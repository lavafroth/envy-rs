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
            mask: 1 << rem - 1,
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
