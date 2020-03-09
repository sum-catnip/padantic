pub struct ProgressMsg {
    payload: Vec<u8>,
    index: u8,
    block: usize
}

impl ProgressMsg {
    pub fn new(payload: Vec<u8>, index: u8, block: usize) -> Self {
        ProgressMsg { payload, index, block }
    }

    pub fn payload(&self) -> &Vec<u8> { &self.payload }
    pub fn index(&self) -> u8 { self.index }
    pub fn block(&self) -> usize { self.block }
}

pub enum Messages {
    Prog(ProgressMsg),
    Done()
}
