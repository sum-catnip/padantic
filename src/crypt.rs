use crate::{ CmdOracleCtx, CmdOracle, PrioQueue };
use crate::msg::{ Messages, pyld, inter, plain };
use crate::oracle;

use std::u8;

use snafu::{ Snafu, ResultExt, ensure };
use crossbeam::thread;
use crossbeam::thread::ScopedJoinHandle;

type Result<T, E = Error> = std::result::Result<T, E>;
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("error in oracle: {}", source))]
    Oracle { source: oracle::Error },
    #[snafu(display("tried all 256 bytes without success"))]
    Tries,
    #[snafu(display("invalid input: {}", reason))]
    Input { reason: String }
}

#[derive(Debug)]
pub struct Dec {
    pub intermediate: Vec<u8>,
    pub plain: Vec<u8>
}

impl Dec {
    pub fn new(intermediate: Vec<u8>, plain: Vec<u8>) -> Self {
        Dec { intermediate, plain }
    }
}

type DecResult = Result<Dec>;
trait ProgressCb = Fn(Messages) + Sync + Send;

struct BlockDecypt<'a, F: ProgressCb> {
    b_pyld: Vec<u8>,
    b_inter: Vec<u8>,
    b_plain: Vec<u8>,
    iv: &'a [u8],
    oracle: CmdOracle,
    chars: &'a PrioQueue,
    last: bool,
    block: usize,
    blksz: u8,
    cb: &'a F
}

// yes, thats how lazy i am
macro_rules! i { ($x:expr) => { $x as usize }; }

impl<'a, F: ProgressCb> BlockDecypt<'a, F> {
    pub fn new(blk: &[u8], iv: &'a [u8],
               oracle: CmdOracle,
               chars: &'a PrioQueue,
               cb: &'a F, block: usize,
               last: bool) -> Result<Self> {

        ensure!(blk.len() <= u8::MAX as usize,
                Input { reason: "blocksize must be below 256" });

        let blksz = blk.len();
        let mut b_pyld = vec![0u8; blksz * 2];
        b_pyld[blksz..].copy_from_slice(blk);
        Ok(BlockDecypt {
            b_inter: vec![0u8; blksz],
            b_plain: vec![0u8; blksz],
            blksz: blksz as u8,
            b_pyld, iv, oracle, chars, last, cb, block
        })
    }

    fn decrypt_byte(&mut self, i: u8, mut chars: impl Iterator<Item = u8>)
        -> Result<u8, Error> {
        let pad = self.blksz - i;
        
        self.b_pyld.iter_mut()
            .zip(self.b_inter.iter())
            .skip(i as usize +1)
            .for_each(|(p, i)| *p = pad ^ i);

        chars.try_find(|b| {
            self.b_pyld[i!(i)] = *b ^ (pad ^ self.iv[i!(i)]);
            (self.cb)(pyld(self.b_pyld[0..i!(self.blksz)].to_vec(),
                           i as u8, self.block));
            self.oracle.request(&self.b_pyld).context(Oracle)
        })?
        .ok_or(Error::Tries)
    }

    fn skip_pad(&mut self) -> Result<u8, Error> {
        match self.last {
            true => {
                let res = self.decrypt_byte(self.blksz -1, 1..self.blksz -1)?;
                (self.blksz - res .. self.blksz)
                    .map(|i| i as usize)
                    .for_each(|i| {
                        self.b_inter[i] = res ^ self.iv[i];
                        self.b_plain[i] = res;
                    });

                Ok(self.blksz - res)
            },
            false => Ok(self.blksz)
        }
    }

    pub fn decrypt(mut self) -> DecResult {
        for i in (0 .. self.skip_pad()?).rev() {
            let res = self.decrypt_byte(i, self.chars.iter())?;

            self.b_inter[i!(i)] = res ^ self.iv[i!(i)];
            self.b_plain[i!(i)] = res;
            self.chars.hit(res);

            (self.cb)(inter(self.b_inter.clone(), i, self.block));
            (self.cb)(plain(self.b_plain.clone(), i, self.block));
        }
        Ok(Dec::new(self.b_inter, self.b_plain))
    }
}

pub fn decrypt<F>(cipher: &[u8],
                  blksz: u8,
                  oracle: &CmdOracleCtx,
                  prog: F,
                  chars: &[u8; 256],
                  iv: bool) -> Vec<DecResult>
    where F: Fn(Messages) + Sync + Send {

    let mut blocks = cipher
        .chunks(blksz as usize)
        .collect::<Vec<&[u8]>>();

    let ivblk: Vec<u8>;
    if !iv {
        ivblk = vec![0; blksz as usize];
        blocks.insert(0, &ivblk);
    }

    let chars = PrioQueue::new(chars.to_vec());
    let res = thread::scope(|s| {
        let blkc = blocks.len();
        let chars = &chars;
        let prog = &prog;
        (1..blocks.len())
            .map(|i| {
                let blk1 = blocks[i];
                let blk0 = blocks[i-1];
                s.spawn(move |_|
                    BlockDecypt::new(blk1, blk0,
                                     oracle.spawn().context(Oracle)?,
                                     chars, prog, i -1,
                                     i == blkc -1 && (iv || blkc > 2))?
                                     .decrypt())})
            .collect::<Vec<ScopedJoinHandle<_>>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect()
    }).unwrap();
    (prog)(Messages::Done);
    res
}
