use std::sync::Mutex;
use std::collections::HashMap;

#[derive(Debug)]
pub struct PrioQueue (Mutex<HashMap<u8, usize>>);
impl PrioQueue {
    pub fn new(init: Vec<u8>) -> Self {
        PrioQueue (Mutex::new(init
            .iter()
            .rev()
            .enumerate()
            .map(|(i, b)| (*b, i))
            .collect()))
    }

    pub fn hit(&self, byte: u8) {
        let mut q = self.0.lock().unwrap();
        let new = q[&byte] +5;
        q.insert(byte, new);
    }

    pub fn iter(&self) -> impl Iterator<Item = u8> {
        let q = self.0.lock().unwrap();
        let mut tmp = q.clone().into_iter().collect::<Vec<_>>();
        tmp.sort_by(|x, y| y.1.cmp(&x.1));
        tmp.into_iter().map(|(k, _)| k)
    }
}
