use std::fs;

use base64;
use hex::FromHexError;
use futures::future;
use snafu::{ Snafu, ResultExt, ensure };
use tokio::process::Command;
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

struct CmdOracle<'a> {
    cmd: &'a str,
    args: &'a [&'a str]
}

impl<'a> CmdOracle<'a> {
    pub fn new(cmd: &'a str, args: &'a [&'a str]) -> Self {
        CmdOracle { cmd, args }
    }

    pub async fn request(&self, payload: &[u8]) -> Result<bool> {
        Command::new(self.cmd)
            .args(self.args)
            .arg(base64::encode(payload))
            .status()
            .await
            .context(BrokenOracle {})
            .map(|r| r.success())
    }
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

async fn decrypt_intermediate<'a>(blk: &[u8],
                                  last: &[u8],
                                  oracle: &CmdOracle<'a>,
                                  chars: [u8; 256]) -> Result<Vec<u8>> {
    let blksz = blk.len();
    let mut intermediate = vec![0; blksz];
    let mut payload = vec![0; blksz * 2];
    payload[blksz..].copy_from_slice(blk);
    let mut prio = chars.to_vec();
    for i in (0..blksz).rev() {
        let pad = (blksz - i) as u8;
        (i +1..blksz).rev().for_each(|j| payload[j] = pad ^ intermediate[j]);

        let mut took = 0;
        // TODO predict based on all last chars
        let prio_tmp = prio.to_vec();
        for (j, b) in prio_tmp.iter().enumerate() {
            payload[i] = b ^ (last[i] ^ pad);
            if oracle.request(&payload).await? {
                prio.remove(j);
                prio.insert(0, *b);
                took = j; break;
            }
        }
        println!("oracle took {} tries", took);
        ensure!(took != chars.len(), BraveOracle);
        intermediate[i] = payload[i] ^ pad;
    }

    Ok(intermediate)
}

async fn decrypt<'a>(cipher: &[u8],
                     blksz: usize,
                     oracle: &CmdOracle<'a>,
                     chars: [u8; 256]) -> Result<Vec<DecResult>> {
    let blocks = cipher.chunks(blksz).collect::<Vec<&[u8]>>();
    let i = future::join_all(blocks
        .iter()
        .skip(1)
        .zip(blocks[0..blocks.len() -1].iter())
        .map(|(blk1, blk2)| decrypt_intermediate(blk1, blk2, oracle, chars)))
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
        .trim()
        .replace(' ', "");

    println!("{}", chars);
    let res = hex::decode(chars)
        .context(HexParse { field: "chars" })?;
    let mut out = [0; 256];
    ensure!(res.len() == out.len(), InsufficiendChars);
    out.copy_from_slice(&res);
    Ok(out)
}

#[tokio::main]
async fn main() {
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
             .long_help(concat!("space seperated list of hex encoded bytes to guess the plaintext. ",
                                "ALL 256 POSSIBLE BYTES MUST BE PRESENT in no particular order. ",
                                "example: 00 01 02 ... 61 62 63 ... 6A 6B ... FF FF")))
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

    let oracle = CmdOracle::new(cmd, &cmd_args);
    for dec in decrypt(&cipher, blksz, &oracle, chars).await.unwrap() {
        println!("{:?}", dec.unwrap());
    }
}
