use crate::{ CmdOracleCtx, CmdOracle, PrioQueue, Messages, BlockData };
use crate::oracle;

use snafu::{ Snafu, ResultExt, ensure };
use crossbeam::thread;

type Result<T, E = Error> = std::result::Result<T, E>;
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("error in oracle: {}", source))]
    Oracle { source: oracle::Error },
    #[snafu(display("tried all 256 bytes without success"))]
    Tries
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

fn decrypt_blk<F>(blk: &[u8],
               last: &[u8],
               mut oracle: CmdOracle,
               chars: &PrioQueue,
               prog: &F,
               block: usize,
               is_last: bool) -> DecResult
    where F: Fn(Messages) + Sync + Send {
    let blksz = blk.len();
    let mut intermediate = vec![0; blksz];
    let mut plain = vec![0u8; blksz];
    let mut payload = vec![0; blksz * 2];
    payload[blksz..].copy_from_slice(blk);
    let mut end = 0;
    // TODO this code is fucking disgusting and i should feel ashamed of myself
    if is_last {
        for b in 1..blksz +1 {
            payload[blksz -1] = b as u8 ^ (1 ^ last[blksz -1]);
            //println!("trying pad {}", b);
            if oracle.request(&payload).context(Oracle)? {
                //println!("found pad byte: {}", b);
                for i in blksz - b .. blksz {
                    intermediate[i] = b as u8 ^ last[i];
                    plain[i] = b as u8;
                }
                end = b;
                break;
            }
        }
    }
    for i in (0..blksz - end).rev() {
        let pad = (blksz - i) as u8;
        (i +1..blksz).rev().for_each(|j| payload[j] = pad ^ intermediate[j]);

        let mut took = 0;
        for (j, b) in chars.iter().enumerate() {
            payload[i] = b ^ (pad ^ last[i]);
            prog(Messages::Payload(BlockData::new(payload[0..blksz].to_vec(), i as u8, block)));
            if oracle.request(&payload).context(Oracle)? {
                intermediate[i] = b ^ last[i];
                plain[i] = b;
                took = j;
                chars.hit(b);
                break;
            }
        }
        ensure!(took != 255, Tries);
        prog(Messages::Intermediate(BlockData::new(intermediate.clone(), i as u8, block)));
        prog(Messages::Plain(BlockData::new(plain.clone(), i as u8, block)));
        //println!("took {} oracle calls", took);
    }
    Ok(Dec::new(intermediate, plain))
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
    thread::scope(|s| {
        let mut handles = Vec::new();
        let blkc = blocks.len();
        let chars = &chars;
        let prog = &prog;
        for i in 1..blocks.len() {
            let blk1 = blocks[i];
            let blk0 = blocks[i-1];
            handles.push(s.spawn(move |_| {
                decrypt_blk(blk1, blk0,
                            oracle.spawn().context(Oracle)?,
                            chars, prog, i -1,
                            i == blkc -2 && iv)
            }));
        }
        let mut res = Vec::new();
        for handle in handles {
            res.push(handle.join().unwrap())
        }

        res
    }).unwrap()
}
