use snafu::Snafu;
use hex::FromHexError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("invalid block size: {}", size), visibility(pub))]
    BlockSize { size: usize },
    #[snafu(display("oracle returned `true` for every possible try"), visibility(pub))]
    BraveOracle,
    #[snafu(display("oracle returned an error: {}", source), visibility(pub))]
    BrokenOracle { source: std::io::Error },
    #[snafu(display("failed to load the characters file `{}`: {}", file, source), visibility(pub))]
    CharLoad { source: std::io::Error, file: String },
    #[snafu(display("failed to parse hex bytes for `{}`: {}", field, source), visibility(pub))]
    HexParse { source: FromHexError, field: &'static str },
    #[snafu(display("insufficient characters, needs exactly 256"), visibility(pub))]
    InsufficiendChars
}
