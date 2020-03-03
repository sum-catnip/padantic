use std::process::Command;
use std::fs;

use base64;
use hex::FromHexError;
use futures::future;
use async_std::task;
use snafu::{ Snafu, ResultExt, ensure };
use clap::{
    Arg, app_from_crate,
    crate_authors, crate_description,
    crate_name, crate_version
};

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("invalid block size: {}", size))]
    BlockSize { size: usize },
    #[snafu(display("oracle returned `true` for every possible try"))]
    BraveOracle,
    #[snafu(display("oracle returned an error: {}", source))]
    BrokenOracle { source: std::io::Error },
    #[snafu(display("failed to load the characters file `{}`: {}", file, source))]
    CharLoad { source: std::io::Error, file: String },
    #[snafu(display("failed to parse hex bytes for `{}`: {}", field, source))]
    HexParse { source: FromHexError, field: &'static str },
    #[snafu(display("insufficient characters, needs exactly 256"))]
    InsufficiendChars
}

fn xor_slice(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b).map(|(a, b)| a ^ b).collect::<Vec<u8>>()
}

#[derive(Debug)]
struct Dec {
    pub intermediate: Vec<u8>,
    pub plain: Vec<u8>
}

impl Dec {
    pub fn new(cipher: &[u8], intermediate: Vec<u8>) -> Self {
        let plain = cipher
            .iter()
            .zip(&intermediate)
            .map(|(c, i)| c ^ i)
            .collect();

        Dec { intermediate, plain }
    }
}

type Result<T, E = Error> = std::result::Result<T, E>;
type DecResult = Result<Dec>;

fn cmd_oracle(payload: &[u8], cmd: &str, args: &[&str]) -> Result<bool> {
    Ok(Command::new(cmd)
        .args(args)
        .arg(base64::encode(payload))
        .status()
        .context(BrokenOracle {})?
        .success())
}

async fn decrypt_intermediate<F>(blk: &[u8], last: &[u8], oracle: F, chars: [u8; 256]) -> Result<Vec<u8>>
    where F: Fn(&[u8]) -> Result<bool> {
    let blksz = blk.len();
    let mut intermediate = vec![0; blksz];
    let mut payload = vec![0; blksz * 2];
    payload[blksz..].copy_from_slice(blk);
    let mut last_byte = 0;
    for i in (0..blksz).rev() {
        let pad = (blksz - i) as u8;
        (i +1..blksz).rev().for_each(|j| payload[j] = pad ^ intermediate[j]);

        let mut took = 0;
        // TODO predict based on all last chars
        payload[i] = last_byte ^ (last[i] ^ pad);
        if oracle(&payload)? { took = 1 }
        else {
            for (j, b) in chars.iter().enumerate() {
                payload[i] = *b ^ (last[i] ^ pad);
                if oracle(&payload)? { took = j; last_byte = *b; break; }
            }
        }
        println!("oracle took {} tries", took);
        ensure!(took != chars.len(), BraveOracle);
        intermediate[i] = payload[i] ^ pad;
    }

    Ok(intermediate)
}

async fn decrypt<F>(cipher: &[u8], blksz: usize, oracle: F, chars: [u8; 256])
    -> Result<Vec<DecResult>>
    where F: Fn(&[u8]) -> Result<bool> {
    let oracleref = &oracle;
    let blocks = cipher.chunks(blksz).collect::<Vec<&[u8]>>();
    let i = future::join_all(blocks
        .iter()
        .skip(1)
        .zip(blocks[0..blocks.len() -1].iter())
        .map(|(blk1, blk2)| decrypt_intermediate(blk1, blk2, oracleref, chars)))
        .await;

    Ok(blocks
       .iter()
       .zip(i)
       .map(|(c, i)| i.map(|i| Dec::new(c, i) ))
       .collect())
}

fn parse_chars(file: &str) -> Result<[u8; 256]> {
    let chars = fs::read_to_string(file)
        .context(CharLoad { file: file.to_string() })?
        .replace(' ', "");
    let res = hex::decode(chars)
        .context(HexParse { field: "chars" })?;
    let mut out = [0; 256];
    ensure!(res.len() == out.len(), InsufficiendChars);
    out.copy_from_slice(&res);
    Ok(out)
}

fn main() {
    let args = app_from_crate!()
        .arg(Arg::with_name("cipher")
            .short("c").long("cipher").required(true).takes_value(true).index(1)
            .help("target ciphertext (hex encoded)"))
        .arg(Arg::with_name("noiv")
             .long("noiv")
             .help("skip CBC on first block and guess IV interactively"))
        .arg(Arg::with_name("size")
             .long("size").short("s").takes_value(true).default_value("16")
             .help("CBC block size"))
        .arg(Arg::with_name("chars")
             .long("chars").takes_value(true).default_value("english.chars")
             .long_help(concat!("collon seperated list of hex encoded bytes to guess the plaintext. ",
                                "ALL 256 POSSIBLE BYTES MUST BE PRESENT in no particular order. ",
                                "example: 00:01:02 ... 61:62:63 ... 6A:6B ... FF:FF")))
        .arg(Arg::with_name("oracle")
             .long("oracle").short("o").required(true).takes_value(true).index(2).multiple(true)
             .long_help(concat!("the command to run as an oracle. ",
                                "should only return status 0 for valid padding. ",
                                "command will be ran with base64 paylod as first arg. ",
                                "arguments after cmd argument will be prepended BEFORE payload")))
        .get_matches();

    let mut cipher = hex::decode(args.value_of("cipher").unwrap())
        .context(HexParse { field: "cipher" })
        .unwrap();

    let chars = parse_chars(args.value_of("chars").unwrap()).unwrap();

    let blksz: usize = args.value_of("size").unwrap().parse()
        .expect("invalid value for `size`");

    if args.is_present("noiv") {
        let mut tmp = vec![0; blksz];
        tmp.extend(cipher);
        cipher = tmp;
    }

    let mut oracle = args.values_of("oracle").unwrap();
    let cmd = oracle.next().unwrap();
    let cmd_args = oracle.collect::<Vec<&str>>();

    for dec in task::block_on(decrypt(&cipher, blksz, |p| cmd_oracle(p, cmd, &cmd_args), chars)).unwrap() {
        println!("{:?}", dec.unwrap());
    }
}
