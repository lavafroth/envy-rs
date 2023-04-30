pub struct BitField {
    mask: u8,
    bytes: Vec<u8>,
}

impl BitField {
    pub fn new(size: usize) -> BitField {
        let (mut quo, rem) = full_division(size, 8);
        if rem != 0 {
            quo += 1;
        }
        BitField {
            mask: (1 << rem) - 1,
            bytes: vec![0; quo],
        }
    }

    pub fn increment(&mut self) {
        for i in 0..self.bytes.len() {
            if self.bytes[i] == 255 {
                self.bytes[i] = 0;
                continue;
            }
            self.bytes[i] += 1;
            break;
        }
    }

    pub fn maxed(&self) -> bool {
        let n = self.bytes.len();
        for i in 0..n - 1 {
            if self.bytes[i] != 255 {
                return false;
            }
        }
        self.bytes[n - 1] == self.mask
    }

    pub fn at(&self, index: usize) -> bool {
        let (quo, rem) = full_division(index, 8);
        let bit_index: u8 = 1 << rem;
        self.bytes[quo] & bit_index == bit_index
    }
}

fn full_division(n: usize, radix: usize) -> (usize, usize) {
    let quo = n / radix;
    (quo, n - radix * quo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let i = BitField::new(17);
        // 17 bits means rounding off to 3 bytes
        // with the last byte having mask = 1
        assert_eq!(i.mask, 1);

        let i = BitField::new(19);
        // 19 bits means rounding off to 3 bytes
        // with the last byte having mask = 7
        assert_eq!(i.mask, 7);

        let i = BitField::new(24);
        // 24 bits means exactly 3 bytes
        // with the last byte having mask = 0
        assert_eq!(i.mask, 0);
    }

    #[test]
    fn test_increment() {
        let mut i = BitField::new(8);
        i.increment();
        // the value for the bitfield now is 1
        assert!(i.at(0));

        // we add 3 to the bitfield
        for _ in 0..3 {
            i.increment();
        }
        // the total value now is 4
        // and 1 << 2 is 4
        assert!(i.at(2));

        // the last two bits must be zero
        assert!(!i.at(1));
        assert!(!i.at(0));
    }

    #[test]
    fn test_maxed() {
        let mut i = BitField::new(8);
        // an 8 bit vector will max at 255
        for _ in 0..=255 {
            i.increment();
        }
        assert!(i.maxed());
    }
}
