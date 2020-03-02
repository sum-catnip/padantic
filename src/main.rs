use std::process::Command;

use base64;
use futures::future;
use async_std::task;

fn xor_slice(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b).map(|(a, b)| a ^ b).collect::<Vec<u8>>()
}

fn oracle(payload: &[u8]) -> bool {
    Command::new("curl")
        .arg("-f")
        .arg("-s")
        .arg(format!("127.0.0.1:666/{}", base64::encode(payload)))
        .status().unwrap()
        .success()
}

async fn decrypt_intermediate(blk: &[u8]) -> Vec<u8> {
    let blksz = blk.len();
    let mut intermediate = vec![0; blksz];
    let mut payload = vec![0; blksz * 2];
    payload[blksz..].copy_from_slice(blk);
    println!("{:x?}", payload);
    for i in (0..blksz).rev() {
        let pad = (blksz - i) as u8;
        (i +1..blksz).rev().for_each(|j| payload[j] = pad ^ intermediate[j]);

        let mut found = false;
        for b in 0..256 {
            payload[i] = b as u8;
            if oracle(&payload) { found = true; break; }
        }

        if !found { println!("oof"); return intermediate; }
        intermediate[i] = payload[i] ^ pad;
    }

    intermediate
}

async fn decrypt(cipher: &[u8]) {
    let blocks = cipher.chunks(16).collect::<Vec<&[u8]>>();
    let intermediates = future::join_all(blocks
        .iter()
        .skip(1)
        .map(|blk| decrypt_intermediate(blk)))
        .await;

    for (c, i) in blocks[0..intermediates.len()].iter().zip(intermediates) {
        println!("intermediate: {:x?}", i);
        println!("plain: {:x?}", xor_slice(c, &i));
    }
}

fn main() {
    let cipher = base64::decode("R4ROGORdXls96ubhBv/8Ui+Cd1LCtmgR8mUTOYKQkJE=").unwrap();
    task::block_on(decrypt(&cipher)); 
}
