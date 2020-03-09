use crate::error::{ Result, BrokenOracle };

use snafu::ResultExt;
use tokio::process::Command;

pub struct CmdOracle<'a> {
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
