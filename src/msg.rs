pub struct BlockData {
    block: Vec<u8>,
    index: u8,
    block_index: usize
}

impl BlockData {
    pub fn new(block: Vec<u8>, index: u8, block_index: usize) -> Self {
        BlockData { block, index, block_index }
    }

    pub fn block(&self) -> &Vec<u8> { &self.block }
    pub fn index(&self) -> u8 { self.index }
    pub fn block_index(&self) -> usize { self.block_index }
}

pub enum Messages {
    Payload(BlockData),
    Intermediate(BlockData),
    Plain(BlockData)
}
