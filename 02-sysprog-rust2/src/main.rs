use std::io::{self, Read, Write};

const ADDRESS_SIZE: usize = 2; // Memory addresses occupy two bytes.
const INSTRUCTION_POINTER_ADDRESS: usize = 0;
const MEMORY_SIZE: usize = 64 * 1024;

// Decoded operands and instructions, corresponding to the Python data classes.
struct Argument {
    address: Option<usize>,
    value: u32,
}

impl Argument {
    fn new(address: Option<usize>, value: u32) -> Self {
        Self { address, value }
    }
}

struct Instruction {
    address: usize,
    opcode: u8,
    integer_size: usize,
    argument1: Argument,
    argument2: Argument,
}

impl Instruction {
    fn new(
        address: usize,
        opcode: u8,
        integer_size: usize,
        argument1: Argument,
        argument2: Argument,
    ) -> Self {
        Self {
            address,
            opcode,
            integer_size,
            argument1,
            argument2,
        }
    }
}

// The virtual machine owns its memory and tracks the decoding cursor.
struct VM {
    memory: Vec<u8>,
    current_address: usize,
}

impl VM {
    fn new(memory: Vec<u8>) -> Result<Self, &'static str> {
        let mut vm = Self {
            memory,
            current_address: 0,
        };
        vm.current_address = vm.load_data(INSTRUCTION_POINTER_ADDRESS, ADDRESS_SIZE)? as usize;
        Ok(vm)
    }

    /// Start each instruction by reading the pointer stored in memory.
    fn start_instruction(&mut self) -> Result<bool, &'static str> {
        self.current_address = self.load_data(INSTRUCTION_POINTER_ADDRESS, ADDRESS_SIZE)? as usize;
        Ok(self.current_address != 0)
    }

    fn save_instruction_pointer(&mut self) -> Result<(), &'static str> {
        if self.current_address > usize::from(u16::MAX) {
            return Err("Instruction pointer outside 16-bit address range");
        }

        self.store_data(
            INSTRUCTION_POINTER_ADDRESS,
            self.current_address as u32,
            ADDRESS_SIZE,
        )
    }

    // Memory, console, and file-loading helpers.

    /// Read an unsigned little-endian value consisting of size bytes.
    fn load_data(&self, address: usize, size: usize) -> Result<u32, &'static str> {
        let end = match address.checked_add(size) {
            Some(end) => end,
            None => return Err("Address calculation overflow"),
        };

        if end > self.memory.len() {
            return Err("Read outside memory");
        }

        match size {
            1 => Ok(u32::from(self.memory[address])),
            2 => Ok(u32::from(self.read_u16(address))),
            4 => Ok(u32::from_le_bytes([
                self.memory[address],
                self.memory[address + 1],
                self.memory[address + 2],
                self.memory[address + 3],
            ])),
            _ => Err("Unsupported integer size"),
        }
    }

    // load_data checks the bounds before calling this two-byte helper.
    fn read_u16(&self, address: usize) -> u16 {
        let first_byte = self.memory[address];
        let second_byte = self.memory[address + 1];
        u16::from_le_bytes([first_byte, second_byte])
    }

    /// Store an unsigned value after validating its size and destination.
    fn store_data(&mut self, address: usize, value: u32, size: usize) -> Result<(), &'static str> {
        if !matches!(size, 1 | 2 | 4) {
            return Err("Unsupported integer size");
        }

        let end = match address.checked_add(size) {
            Some(end) => end,
            None => return Err("Address calculation overflow"),
        };

        if end > self.memory.len() {
            return Err("Write outside memory");
        }

        let fits = match size {
            1 => value <= u32::from(u8::MAX),
            2 => value <= u32::from(u16::MAX),
            _ => true,
        };

        if !fits {
            return Err("Value too large for integer size");
        }

        let bytes = value.to_le_bytes();
        self.memory[address..end].copy_from_slice(&bytes[..size]);
        Ok(())
    }

    /// Read one byte from standard input, or return 256 at end of input.
    fn read_console_byte() -> Result<u32, &'static str> {
        let mut buffer = [0u8; 1];

        match io::stdin().read(&mut buffer) {
            Ok(0) => Ok(256),
            Ok(_) => Ok(u32::from(buffer[0])),
            Err(_) => Err("Failed to read console input"),
        }
    }

    /// Write the lowest byte and immediately flush standard output.
    fn write_console_byte(value: u32) -> Result<(), &'static str> {
        let mut output = io::stdout();
        let byte = value as u8;

        output
            .write_all(&[byte])
            .map_err(|_| "Failed to write console output")?;
        output.flush().map_err(|_| "Failed to flush console output")
    }

    /// Load a binary program into zero-filled memory and construct the VM.
    fn from_filename(filename: &str) -> Result<Self, &'static str> {
        let program = std::fs::read(filename).map_err(|_| "Failed to read program file")?;

        if program.len() > MEMORY_SIZE {
            return Err("Program does not fit in memory");
        }

        let mut memory = vec![0u8; MEMORY_SIZE];
        memory[..program.len()].copy_from_slice(&program);
        Self::new(memory)
    }

    // Instruction decoding.

    fn decode_integer_size(settings: u8) -> Result<usize, &'static str> {
        let size_code = settings & 0b11;

        match size_code {
            0 => Ok(1),
            1 => Ok(2),
            2 => Ok(4),
            _ => Err("Invalid integer size"),
        }
    }

    fn decode_argument_types(settings: u8) -> (u8, u8) {
        let first_mode = (settings >> 2) & 0b11;
        let second_mode = (settings >> 4) & 0b11;
        (first_mode, second_mode)
    }

    /// Decode one operand and advance past its encoded bytes.
    fn decode_argument(&mut self, integer_size: usize, mode: u8) -> Result<Argument, &'static str> {
        match mode {
            0 => Ok(Argument::new(None, 0)),
            1 => {
                let address = self.current_address;
                let value = self.load_data(address, integer_size)?;
                self.current_address += integer_size;
                Ok(Argument::new(Some(address), value))
            }
            2 => {
                let address = self.load_data(self.current_address, ADDRESS_SIZE)? as usize;
                self.current_address += ADDRESS_SIZE;
                let value = self.load_data(address, integer_size)?;
                Ok(Argument::new(Some(address), value))
            }
            3 => {
                let pointer_address = self.load_data(self.current_address, ADDRESS_SIZE)? as usize;
                self.current_address += ADDRESS_SIZE;
                let address = self.load_data(pointer_address, ADDRESS_SIZE)? as usize;
                let value = self.load_data(address, integer_size)?;
                Ok(Argument::new(Some(address), value))
            }
            _ => Err("Invalid argument mode"),
        }
    }

    /// Decode the current instruction and leave the cursor at the next one.
    fn load_instruction(&mut self) -> Result<Instruction, &'static str> {
        let address = self.current_address;
        let opcode = self.load_data(address, 1)? as u8;
        let settings = self.load_data(address + 1, 1)? as u8;

        if !(1..=13).contains(&opcode) {
            return Err("Invalid opcode");
        }

        let integer_size = Self::decode_integer_size(settings)?;
        let (first_mode, second_mode) = Self::decode_argument_types(settings);
        self.current_address += 2;

        let argument1 = self.decode_argument(integer_size, first_mode)?;
        let argument2 = self.decode_argument(integer_size, second_mode)?;

        Ok(Instruction::new(
            address,
            opcode,
            integer_size,
            argument1,
            argument2,
        ))
    }

    // Instruction execution.

    fn execute_instruction(&mut self, instruction: &Instruction) -> Result<(), &'static str> {
        let opcode = instruction.opcode;
        let first = instruction.argument1.value;
        let second = instruction.argument2.value;

        let result = match opcode {
            1 => second,
            2 => first.checked_add(second).ok_or("Addition overflow")?,
            3 => first.checked_sub(second).ok_or("Subtraction underflow")?,
            4 => first.checked_mul(second).ok_or("Multiplication overflow")?,
            5 => first.checked_div(second).ok_or("Division by zero")?,
            6 => first & second,
            7 => first | second,
            8 => first ^ second,
            9..=11 => {
                let condition = match opcode {
                    9 => first == second,
                    10 => first > second,
                    _ => first < second,
                };

                if !condition {
                    self.load_instruction()?;
                }
                return Ok(());
            }
            12 => Self::read_console_byte()?,
            13 => return Self::write_console_byte(first),
            _ => return Err("Invalid opcode"),
        };

        self.write_result(instruction, result)
    }

    /// Store a result and follow jumps that write to the instruction pointer.
    fn write_result(&mut self, instruction: &Instruction, result: u32) -> Result<(), &'static str> {
        let destination = instruction
            .argument1
            .address
            .ok_or("First operand has no writable address")?;

        self.store_data(destination, result, instruction.integer_size)?;

        if destination < ADDRESS_SIZE {
            self.current_address =
                self.load_data(INSTRUCTION_POINTER_ADDRESS, ADDRESS_SIZE)? as usize;
        }
        Ok(())
    }

    /// Execute instructions until the stored instruction pointer becomes zero.
    fn run(&mut self) -> Result<(), &'static str> {
        while self.start_instruction()? {
            let instruction = self.load_instruction()?;
            self.execute_instruction(&instruction)?;
            self.save_instruction_pointer()?;
        }
        Ok(())
    }
}

// Command-line entry point, corresponding to Python's __main__ block.
fn main() -> Result<(), &'static str> {
    let mut args = std::env::args().skip(1);
    let filename = args.next().ok_or("Usage: vm <program_file>")?;

    if args.next().is_some() {
        return Err("Usage: vm <program_file>");
    }

    let mut vm = VM::from_filename(&filename)?;
    vm.run()
}

#[cfg(test)]
mod tests;
