use super::*;

// Build decoder fixtures directly so even short memory and invalid cursors can be tested.
fn vm_at(memory: &[u8], current_address: usize) -> VM {
    VM {
        memory: memory.to_vec(),
        current_address,
    }
}

// Memory-reading helpers.

#[test]
fn read_u16_reads_little_endian_at_an_offset() {
    assert_eq!(vm_at(&[0xaa, 0x34, 0x12, 0xbb], 0).read_u16(1), 0x1234);
}

#[test]
fn load_data_reads_each_width_and_can_end_at_the_memory_boundary() {
    let vm = vm_at(&[0xaa, 0x78, 0x56, 0x34, 0x12], 0);

    for (size, expected) in [(1, 0x78), (2, 0x5678), (4, 0x12345678)] {
        assert_eq!(vm.load_data(1, size), Ok(expected), "size={size}");
    }
    assert_eq!(vm.load_data(4, 1), Ok(0x12));
    assert_eq!(vm.load_data(3, 2), Ok(0x1234));
}

#[test]
fn load_data_preserves_maximum_unsigned_values() {
    for (size, expected) in [(1, 255), (2, 65535), (4, u32::MAX)] {
        assert_eq!(vm_at(&[0xff; 4], 0).load_data(0, size), Ok(expected));
    }
}

#[test]
fn load_data_rejects_unsupported_sizes() {
    // Enough memory is supplied so the size check determines the error.
    for size in [0, 3, 5, 8] {
        assert_eq!(
            vm_at(&[0; 8], 0).load_data(0, size),
            Err("Unsupported integer size")
        );
    }
}

#[test]
fn load_data_rejects_reads_outside_memory() {
    let cases: &[(&[u8], usize, usize)] = &[
        (&[], 0, 1),
        (&[0; 4], 4, 1),
        (&[0; 4], 3, 2),
        (&[0; 4], 1, 4),
        (&[0; 4], 10, 1),
    ];

    for &(memory, address, size) in cases {
        assert_eq!(
            vm_at(memory, 0).load_data(address, size),
            Err("Read outside memory"),
            "address={address}, size={size}"
        );
    }
}

#[test]
fn load_data_rejects_address_overflow() {
    for (address, size) in [(usize::MAX, 1), (usize::MAX - 1, 4)] {
        assert_eq!(
            vm_at(&[0; 4], 0).load_data(address, size),
            Err("Address calculation overflow")
        );
    }
}

// Header settings.

#[test]
fn integer_size_depends_only_on_the_lowest_two_bits() {
    let cases = [
        (0x00, Ok(1)),
        (0xfc, Ok(1)),
        (0x01, Ok(2)),
        (0x39, Ok(2)),
        (0xfd, Ok(2)),
        (0x02, Ok(4)),
        (0x3a, Ok(4)),
        (0xfe, Ok(4)),
        (0x03, Err("Invalid integer size")),
        (0xff, Err("Invalid integer size")),
    ];

    for (settings, expected) in cases {
        assert_eq!(VM::decode_integer_size(settings), expected);
    }
}

#[test]
fn argument_modes_ignore_size_and_unused_bits() {
    for first_mode in 0u8..=3 {
        for second_mode in 0u8..=3 {
            for other_bits in [0, 1, 2, 3, 0x40, 0x80, 0xc3] {
                let settings = (first_mode << 2) | (second_mode << 4) | other_bits;
                assert_eq!(
                    VM::decode_argument_types(settings),
                    (first_mode, second_mode),
                    "settings={settings:#010b}"
                );
            }
        }
    }
}

// Successful operand and instruction decoding.

#[test]
fn decodes_16_bit_literal() {
    let memory = [2, 0, 13, 0b0000_0101, 0x34, 0x12];
    let mut vm = vm_at(&memory, 2);

    let instruction = vm
        .load_instruction()
        .expect("This instruction should decode successfully");

    assert_eq!(instruction.address, 2);
    assert_eq!(instruction.opcode, 13);
    assert_eq!(instruction.integer_size, 2);
    assert_eq!(instruction.argument1.address, Some(4));
    assert_eq!(instruction.argument1.value, 0x1234);
    assert_eq!(instruction.argument2.address, None);
    assert_eq!(instruction.argument2.value, 0);
    assert_eq!(vm.current_address, 6);
}

#[test]
fn literal_zero_has_an_address_while_implicit_zero_does_not() {
    let mut vm = vm_at(&[13, 0x04, 0], 0);
    let instruction = vm.load_instruction().expect("Valid zero literal");

    assert_eq!(instruction.argument1.value, 0);
    assert_eq!(instruction.argument1.address, Some(2));
    assert_eq!(instruction.argument2.value, 0);
    assert_eq!(instruction.argument2.address, None);
    assert_eq!(vm.current_address, 3);
}

#[test]
fn instruction_decoding_ignores_unused_settings_bits() {
    for unused_bits in [0x40, 0x80, 0xc0] {
        let memory = [13, unused_bits | 0x05, 0x34, 0x12];
        let mut vm = vm_at(&memory, 0);
        let instruction = vm.load_instruction().expect("Unused bits are ignored");

        assert_eq!(instruction.integer_size, 2);
        assert_eq!(instruction.argument1.address, Some(2));
        assert_eq!(instruction.argument1.value, 0x1234);
        assert_eq!(instruction.argument2.address, None);
        assert_eq!(instruction.argument2.value, 0);
        assert_eq!(vm.current_address, 4);
    }
}

#[test]
fn decodes_every_combination_of_operand_modes_and_sizes() {
    let cases = [
        (0u8, 1usize, 0xabu32, 0xd2u32),
        (1, 2, 0xcdef, 0xfedc),
        (2, 4, 0x89abcdef, 0xfedcba98),
    ];

    for (size_code, size, first_value, second_value) in cases {
        let first_bytes = first_value.to_le_bytes();
        let second_bytes = second_value.to_le_bytes();

        // Each list is ordered by mode: zero, literal, reference, double reference.
        // Values live at 0x0100 and 0x0120; pointers live at 0x0200 and 0x0220.
        let first_encodings: [&[u8]; 4] = [&[], &first_bytes[..size], &[0x00, 0x01], &[0x00, 0x02]];
        let second_encodings: [&[u8]; 4] =
            [&[], &second_bytes[..size], &[0x20, 0x01], &[0x20, 0x02]];

        for (first_mode, first_encoding) in first_encodings.iter().enumerate() {
            for (second_mode, second_encoding) in second_encodings.iter().enumerate() {
                let mut memory = vec![0; 768];
                let settings = size_code | ((first_mode as u8) << 2) | ((second_mode as u8) << 4);
                memory[..4].copy_from_slice(&[2, 0, 2, settings]);

                let second_start = 4 + first_encoding.len();
                let end = second_start + second_encoding.len();
                memory[4..second_start].copy_from_slice(first_encoding);
                memory[second_start..end].copy_from_slice(second_encoding);
                memory[0x0100..0x0100 + size].copy_from_slice(&first_bytes[..size]);
                memory[0x0120..0x0120 + size].copy_from_slice(&second_bytes[..size]);
                memory[0x0200..0x0202].copy_from_slice(&[0x00, 0x01]);
                memory[0x0220..0x0222].copy_from_slice(&[0x20, 0x01]);

                let mut vm = vm_at(&memory, 2);
                let instruction = vm.load_instruction().unwrap_or_else(|error| {
                    panic!("size={size}, modes=({first_mode}, {second_mode}): {error}")
                });

                let expected_first_address =
                    [None, Some(4), Some(0x0100), Some(0x0100)][first_mode];
                let expected_second_address =
                    [None, Some(second_start), Some(0x0120), Some(0x0120)][second_mode];

                assert_eq!(instruction.address, 2);
                assert_eq!(instruction.opcode, 2);
                assert_eq!(instruction.integer_size, size);
                assert_eq!(instruction.argument1.address, expected_first_address);
                assert_eq!(instruction.argument2.address, expected_second_address);
                assert_eq!(
                    instruction.argument1.value,
                    if first_mode == 0 { 0 } else { first_value }
                );
                assert_eq!(
                    instruction.argument2.value,
                    if second_mode == 0 { 0 } else { second_value }
                );
                assert_eq!(
                    vm.current_address, end,
                    "size={size}, modes=({first_mode}, {second_mode})"
                );
            }
        }
    }
}

#[test]
fn implicit_zero_reads_no_bytes_and_leaves_the_cursor_unchanged() {
    for size in [1, 2, 4] {
        let mut vm = vm_at(&[], usize::MAX);
        let argument = vm.decode_argument(size, 0).expect("Zero needs no memory");

        assert_eq!(argument.address, None);
        assert_eq!(argument.value, 0);
        assert_eq!(vm.current_address, usize::MAX);
    }
}

#[test]
fn references_to_address_zero_keep_some_zero_as_the_destination() {
    for mode in [2, 3] {
        let mut memory = [0; 10];
        memory[..2].copy_from_slice(&[0x34, 0x12]);
        // Mode 3 follows address 4, whose two bytes encode address zero.
        memory[8] = if mode == 2 { 0 } else { 4 };
        let mut vm = vm_at(&memory, 8);

        let argument = vm.decode_argument(2, mode).expect("Valid reference");

        assert_eq!(argument.address, Some(0));
        assert_eq!(argument.value, 0x1234);
        assert_eq!(vm.current_address, 10);
    }
}

#[test]
fn references_can_read_values_at_the_end_of_64_kib_memory() {
    for (size_code, size, value) in [(0u8, 1, 0xabu32), (1, 2, 0xcdef), (2, 4, 0x89abcdef)] {
        let mut memory = vec![0; MEMORY_SIZE];
        let address = MEMORY_SIZE - size;
        memory[..2].copy_from_slice(&[13, 0x08 | size_code]);
        memory[2..4].copy_from_slice(&(address as u16).to_le_bytes());
        memory[address..].copy_from_slice(&value.to_le_bytes()[..size]);
        let mut vm = vm_at(&memory, 0);

        let instruction = vm.load_instruction().expect("Valid boundary read");

        assert_eq!(instruction.argument1.address, Some(address));
        assert_eq!(instruction.argument1.value, value);
        assert_eq!(vm.current_address, 4);
    }
}

#[test]
fn decodes_the_python_subtraction_example_with_a_self_reference() {
    let memory = [2, 0, 3, 0x19, 2, 0, 1, 0];
    let mut vm = vm_at(&memory, 2);
    let instruction = vm.load_instruction().expect("Valid subtraction");

    assert_eq!(instruction.address, 2);
    assert_eq!(instruction.opcode, 3);
    assert_eq!(instruction.integer_size, 2);
    assert_eq!(instruction.argument1.address, Some(2));
    assert_eq!(instruction.argument1.value, 0x1903);
    assert_eq!(instruction.argument2.address, Some(6));
    assert_eq!(instruction.argument2.value, 1);
    assert_eq!(vm.current_address, 8);
}

#[test]
fn decodes_consecutive_instructions_of_different_lengths() {
    let mut memory = [0; 20];
    memory[..13].copy_from_slice(&[
        2, 0, // Initial instruction pointer.
        13, 0x04, b'A', // write8: literal A, implicit zero.
        2, 0x1a, 16, 0, 0x78, 0x56, 0x34, 0x12, // add32: reference, literal.
    ]);
    memory[16..20].copy_from_slice(&[4, 3, 2, 1]);
    let mut vm = vm_at(&memory, 2);

    let first = vm.load_instruction().expect("Valid first instruction");
    assert_eq!(first.address, 2);
    assert_eq!(first.opcode, 13);
    assert_eq!(first.argument1.value, u32::from(b'A'));
    assert_eq!(vm.current_address, 5);

    let second = vm.load_instruction().expect("Valid second instruction");
    assert_eq!(second.address, 5);
    assert_eq!(second.opcode, 2);
    assert_eq!(second.integer_size, 4);
    assert_eq!(second.argument1.address, Some(16));
    assert_eq!(second.argument1.value, 0x01020304);
    assert_eq!(second.argument2.address, Some(9));
    assert_eq!(second.argument2.value, 0x12345678);
    assert_eq!(vm.current_address, 13);

    // Decoding advances the cursor; the run loop will later save it to memory.
    assert_eq!(
        vm.load_data(INSTRUCTION_POINTER_ADDRESS, ADDRESS_SIZE),
        Ok(2)
    );
}

#[test]
fn accepts_every_defined_opcode() {
    for opcode in 1..=13 {
        let mut vm = vm_at(&[opcode, 0], 0);
        let instruction = vm.load_instruction().expect("Defined opcode");
        assert_eq!(instruction.opcode, opcode);
        assert_eq!(vm.current_address, 2);
    }
}

// Malformed instructions must return errors instead of indexing outside memory.
// No cursor rollback is required when decoding fails.

#[test]
fn rejects_every_undefined_opcode() {
    for opcode in std::iter::once(0u8).chain(14..=255) {
        let mut vm = vm_at(&[opcode, 0], 0);
        assert_eq!(
            vm.load_instruction().err(),
            Some("Invalid opcode"),
            "opcode={opcode}"
        );
    }
}

#[test]
fn rejects_the_reserved_integer_size() {
    for settings in [0x03, 0x3f, 0xff] {
        let mut vm = vm_at(&[13, settings], 0);
        assert_eq!(vm.load_instruction().err(), Some("Invalid integer size"));
    }
}

#[test]
fn rejects_missing_header_bytes() {
    let cases: &[(&[u8], usize)] = &[(&[], 0), (&[13], 0), (&[13, 0], 2), (&[0, 13], 1)];

    for &(memory, start) in cases {
        let mut vm = vm_at(memory, start);
        assert_eq!(vm.load_instruction().err(), Some("Read outside memory"));
    }
}

#[test]
fn rejects_an_overflowing_instruction_cursor() {
    let mut vm = vm_at(&[13, 0], usize::MAX);
    assert_eq!(
        vm.load_instruction().err(),
        Some("Address calculation overflow")
    );
}

#[test]
fn rejects_truncated_literals_in_either_operand() {
    for (size_code, size) in [(0u8, 1), (1, 2), (2, 4)] {
        for second_operand in [false, true] {
            for available in 0..size {
                let modes = if second_operand { 0x14 } else { 0x04 };
                let mut memory = vec![13, modes | size_code];
                if second_operand {
                    memory.extend(vec![0; size]); // A complete first literal.
                }
                memory.extend(vec![0; available]);
                let mut vm = vm_at(&memory, 0);

                assert_eq!(
                    vm.load_instruction().err(),
                    Some("Read outside memory"),
                    "size={size}, second_operand={second_operand}, available={available}"
                );
            }
        }
    }
}

#[test]
fn rejects_truncated_encoded_addresses_in_either_operand() {
    for (size_code, size) in [(0u8, 1), (1, 2), (2, 4)] {
        for mode in [2, 3] {
            for second_operand in [false, true] {
                for available in 0..ADDRESS_SIZE {
                    let modes = if second_operand {
                        0x04 | (mode << 4)
                    } else {
                        mode << 2
                    };
                    let mut memory = vec![13, modes | size_code];
                    if second_operand {
                        memory.extend(vec![0; size]);
                    }
                    memory.extend(vec![0; available]);
                    let mut vm = vm_at(&memory, 0);

                    assert_eq!(
                        vm.load_instruction().err(),
                        Some("Read outside memory"),
                        "size={size}, mode={mode}, second_operand={second_operand}, available={available}"
                    );
                }
            }
        }
    }
}

#[test]
fn rejects_truncated_intermediate_pointers_in_either_operand() {
    for size_code in 0..=2 {
        for shift in [2, 4] {
            // Eight bytes of memory: address 7 has only one byte; 8 has none.
            for pointer_address in [7, 8] {
                let mut memory = [0; 8];
                memory[..4].copy_from_slice(&[13, size_code | (3 << shift), pointer_address, 0]);
                let mut vm = vm_at(&memory, 0);

                assert_eq!(
                    vm.load_instruction().err(),
                    Some("Read outside memory"),
                    "size_code={size_code}, shift={shift}, pointer_address={pointer_address}"
                );
            }
        }
    }
}

#[test]
fn rejects_truncated_referenced_values_in_either_operand() {
    for (size_code, size) in [(0u8, 1), (1, 2), (2, 4)] {
        for mode in [2, 3] {
            for shift in [2, 4] {
                for available in 0..size {
                    let mut memory = [0; 16];
                    let value_address = (memory.len() - available) as u8;
                    let encoded_address = if mode == 2 { value_address } else { 6 };
                    memory[..4].copy_from_slice(&[
                        13,
                        size_code | (mode << shift),
                        encoded_address,
                        0,
                    ]);
                    memory[6..8].copy_from_slice(&[value_address, 0]);
                    let mut vm = vm_at(&memory, 0);

                    assert_eq!(
                        vm.load_instruction().err(),
                        Some("Read outside memory"),
                        "size={size}, mode={mode}, shift={shift}, available={available}"
                    );
                }
            }
        }
    }
}

#[test]
fn rejects_invalid_argument_modes_when_called_directly() {
    for mode in [4, 255] {
        let mut vm = vm_at(&[], 0);
        assert_eq!(
            vm.decode_argument(1, mode).err(),
            Some("Invalid argument mode")
        );
    }
}
