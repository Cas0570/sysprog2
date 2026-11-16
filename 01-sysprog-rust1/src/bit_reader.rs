pub struct BitReader {
    /// The input data array
    bytes: Vec<u8>,
    /// The next byte to read from
    byte_offset: usize,
    // The next bit in that byte to read from
    bit_offset: usize,
}

impl BitReader {
    pub fn new(bytes: Vec<u8>) -> BitReader {
        Self { 
            bytes, 
            byte_offset: 0, 
            bit_offset: 7
        }
    }

    /// Returns 0 or 1 and moves on to the next bit.
    /// For each byte, the most significant bit is returned first.
    pub fn get_bit(&mut self) -> u8 {
        let current_byte = self.bytes[self.byte_offset];
        let bit = (current_byte >> self.bit_offset) & 1;
        if self.bit_offset == 0 {
            self.bit_offset = 7;
            self.byte_offset += 1;
        } else {
            self.bit_offset -= 1;
        }
        
        bit
    }

    /// Reads `bit_count` bits and returns a (optionally signed) number from it.
    /// Like get_bit(), the most significant bits are read first.
    pub fn get_number(&mut self, bit_count: u8, signed: bool) -> i64 {
        let mut number: i64 = 0;

        for _ in 0..bit_count {
            let bit = self.get_bit();
            number = (number << 1) | i64::from(bit);
        }

        if signed && bit_count > 0 {
            let sign_bit = 1 << (bit_count - 1);

            if number & sign_bit != 0 {
                number -= 1 << bit_count;
            }
        }

        number
    }
}

// Although we haven't explored Rust unit tests yet, the tests below should
// look very familiar. Run them using: `cargo test`

#[cfg(test)]
mod tests {
    use crate::bit_reader::BitReader;

    #[test]
    fn test_bit() {
        let mut br = BitReader::new(Vec::from([0x0f, 0xf0]));
        for bit in 0..4 {
            assert_eq!(br.get_bit(), 0, "Bit {} in sequence 1", bit);
        }
        for bit in 0..8 {
            assert_eq!(br.get_bit(), 1, "Bit {} in sequence 2", bit);
        }
        for bit in 0..4 {
            assert_eq!(br.get_bit(), 0, "Bit {} in sequence 3", bit);
        }
    }

    #[test]
    fn test_unsigned() {
        let mut br = BitReader::new(Vec::from([0x12, 0x34, 0x56]));
        assert_eq!(br.get_number(4, false), 0x1);
        assert_eq!(br.get_number(8, false), 0x23);
        assert_eq!(br.get_number(12, false), 0x456);
    }

    #[test]
    fn test_signed() {
        let mut br = BitReader::new(Vec::from([0x80, 0xff, 0x00, 0x7f]));
        assert_eq!(br.get_number(8, true), -128);
        assert_eq!(br.get_number(8, true), -1);
        assert_eq!(br.get_number(8, true), 0);
        assert_eq!(br.get_number(8, true), 127);
    }

    #[test]
    #[should_panic]
    fn test_panic() {
        let mut br = BitReader::new(Vec::from([0x12, 0x34, 0x56]));
        assert_eq!(br.get_number(24, false), 0x123456);
        br.get_number(1, false);
    }
}
