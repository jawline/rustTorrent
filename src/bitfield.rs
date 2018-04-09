pub struct Bitfield {
    data: Vec<u8>
}

impl Bitfield {
    pub fn new(data: Vec<u8>) -> Bitfield {
        Bitfield {
            data: data
        }
    }

    fn chunk(piece: usize) -> usize {
        piece / 8
    }

    fn bit(piece: usize) -> usize {
        piece % 8
    }

    pub fn get(&self, piece: usize) -> bool {
        let idx = Bitfield::chunk(piece);
        let bit = Bitfield::bit(piece); 
        let mask = (1 << (7 - bit));
        println!("Idx: {} Bit: {} Field {} Has {}", idx, bit, mask, self.data[idx] & mask);
        self.data[idx] & mask != 0
    }

    pub fn set(&self, piece: usize, have: bool) {
    }
}
