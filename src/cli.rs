use crate::error::{ HexParse, CharLoad, InsufficiendChars };
use crate::Result;

use std::fs;

use snafu::{ ensure, ResultExt };
use clap::{
    Arg, app_from_crate,
    crate_authors, crate_description,
    crate_name, crate_version
};

pub struct Options {
    cipher: Vec<u8>,
    iv: bool,
    size: u8,
    chars: [u8; 256],
    oracle: String,
    oracle_args: Vec<String>
}

impl Options {
    pub fn oracle(&self) -> &String { &self.oracle }
    pub fn cipher(&self) -> &[u8] { &self.cipher }
    pub fn iv(&self) -> bool { self.iv }
    pub fn size(&self) -> u8 { self.size }
    pub fn chars(&self) -> &[u8; 256] { &self.chars }
    pub fn oracle_args(&self) -> &Vec<String> { &self.oracle_args }
}

fn parse_chars(file: &str) -> Result<[u8; 256]> {
    let chars = fs::read_to_string(file)
        .context(CharLoad { file: file.to_string() })?
        .trim()
        .replace(' ', "");

    //println!("{}", chars);
    let res = hex::decode(chars)
        .context(HexParse { field: "chars" })?;
    let mut out = [0; 256];
    ensure!(res.len() == out.len(), InsufficiendChars);
    out.copy_from_slice(&res);
    Ok(out)
}

pub fn parse() -> Options {
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
             .long_help(concat!("(space seperated) list of hex encoded bytes to guess the plaintext. ",
                                "ALL 256 POSSIBLE BYTES MUST BE PRESENT in no particular order. ",
                                "example: 00 01 02 ... 61 62 63 ... 6A 6B ... FF FF")))
        .arg(Arg::with_name("oracle")
             .long("oracle").short("o").required(true).takes_value(true).index(2).multiple(true)
             .long_help(concat!("the command to run as an oracle. ",
                                "should only return status 0 for valid padding. ",
                                "command will be ran with base64 paylod as first arg. ",
                                "arguments after cmd argument will be prepended BEFORE payload")))
        .get_matches();

    let cipher = hex::decode(args.value_of("cipher").unwrap())
        .context(HexParse { field: "cipher" })
        .unwrap();

    let chars = parse_chars(args.value_of("chars").unwrap()).unwrap();

    let size: u8 = args.value_of("size").unwrap().parse()
        .expect("invalid value for `size`");

    let iv = !args.is_present("noiv");

    let mut oracle = args.values_of("oracle").unwrap();
    let cmd = oracle.next().unwrap().to_owned();
    let cmd_args = oracle.map(|s| s.to_owned()).collect::<Vec<String>>();

    Options {
        cipher, iv, size, chars,
        oracle: cmd, oracle_args: cmd_args
    }
}
