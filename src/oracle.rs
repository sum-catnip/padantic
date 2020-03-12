use std::process::{ Command, Stdio, Child, ChildStdin, ChildStdout };
use std::io::{ Write, BufReader, BufRead, BufWriter };
use std::time::{ Duration, Instant };
use std::thread;

use snafu::{ Snafu, ResultExt };
use base64;

use log::trace;

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

pub struct CmdOracle{
    child: Child,
    writer: BufWriter<ChildStdin>,
    reader: BufReader<ChildStdout>
}

impl Drop for CmdOracle {
    fn drop(&mut self) {
        // kill forcefully, ignore errors
        let _ = self.child.kill();
    }
}

impl CmdOracle {
    fn new(ctx: &CmdOracleCtx) -> Result<Self> {
        let mut child = Command::new(&ctx.cmd)
            .args(&ctx.args)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context(BrokenIO)?;

        let err = child.stderr.take().unwrap();
        thread::spawn(|| {
            BufReader::new(err).lines()
                .for_each(|l| log::error!("stderr: {}", l.unwrap()));

            log::debug!("stderr thread exit");
        });

        let writer = BufWriter::new(child.stdin.take().unwrap());
        let reader = BufReader::new(child.stdout.take().unwrap());
        Ok(CmdOracle { child, writer, reader })
    }

    pub fn request(&mut self, payload: &[u8]) -> Result<bool> {
        let now = Instant::now();
        self.writer.write_all(base64::encode(payload).as_bytes()).context(BrokenIO)?;
        self.writer.write(&['\n' as u8]).context(BrokenIO)?;
        self.writer.flush().context(BrokenIO)?;

        // 'y' or 'n' with a newline
        let mut line = String::new();
        self.reader.read_line(&mut line)
            .context(BrokenIO)?;

        trace!("oracle took {:?}", now.elapsed());
        match line.trim() {
            "yes" => Ok(true),
            "no"  => Ok(false),
            x => Err(Error::BrokenLogic { reason:
                format!("invalid choice: {:?}. choices are: yes/no", x)})
        }
    }
}
