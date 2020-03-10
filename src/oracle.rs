use std::process::{ Command, Stdio, Child };
use std::ffi::OsStr;
use std::thread;
use std::io::Read;
use std::io::Write;
use std::io::BufReader;
use std::io::BufRead;

use snafu::{ Snafu, ResultExt, ensure };

type Result<T, E = Error> = std::result::Result<T, E>;
#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("oracle io returned an error: {}", source))]
    BrokenIO { source: std::io::Error },
    #[snafu(display("oracle is being a weirdo: {}", reason))]
    BrokenLogic { reason: String }
}

pub struct CmdOracleCtx {
    cmd: String,
    args: Vec<String>
}

impl CmdOracleCtx {
    pub fn new(cmd: String, args: Vec<String>) -> Self {
        CmdOracleCtx { cmd, args }
    }

    pub fn spawn(&self) -> Result<CmdOracle> {
        CmdOracle::new(self)
    }
}

pub struct CmdOracle( Child );
impl CmdOracle {
    fn new(ctx: &CmdOracleCtx) -> Result<Self> {
        let child = CmdOracle(Command::new(&ctx.cmd)
            .args(&ctx.args)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(BrokenIO)?
        );

        thread::spawn(|| {
            BufReader::new(err).lines()
                .for_each(|l| log::error!("stderr: {}", l.unwrap()));

            log::debug!("stderr thread exit");
        });

        Ok(child)
    }

    pub fn request(&self, payload: &[u8]) -> Result<bool> {
        self.0.stdin.unwrap().write_all(payload)
            .context(BrokenIO);
        self.0.stdin.unwrap().write(&['\n' as u8])
            .context(BrokenIO);
        self.0.stdin.unwrap().flush()
            .context(BrokenIO);

        // 'y' or 'n' with a newline
        let mut buf = [0; 2];
        self.0.stdout.unwrap().read_exact(&mut buf)
            .context(BrokenIO);

        ensure!(buf[1] == '\n' as u8, BrokenLogic { reason:
            format!("unexpected output: {:?} (shoud have newline at the end)", buf)
        });

        match buf[0].into() {
            'y' => Ok(true),
            'n' => Ok(false),
            x   => Err(Error::BrokenLogic { reason:
                format!("invalid choice: {:?}. choices are: y/n", x)})
        }
    }
}
