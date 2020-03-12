mod error;
mod prio;
mod oracle;
mod msg;
mod crypt;
mod cli;
mod ui;

use error::Result;
use oracle::{ CmdOracle, CmdOracleCtx };
use prio::PrioQueue;
use ui::ScreenCtx;

use msg::{ BlockData, Messages };

use std::io;
use std::io::Write;

use crossterm::terminal;
use tui::Terminal;
use tui::backend::CrosstermBackend;

use crossbeam::thread;

fn main() {
    let opt = cli::parse();
    let oracle = CmdOracleCtx::new(opt.oracle().to_owned(), opt.oracle_args().to_owned());
    crossterm::execute!(io::stdout(), terminal::EnterAlternateScreen).unwrap();
    terminal::enable_raw_mode().unwrap();
    let backend = CrosstermBackend::new(io::stdout());
    let mut term = Terminal::new(backend).unwrap();
    term.hide_cursor().unwrap();
    term.clear().unwrap();

    let blocks = opt.cipher().len() / opt.size() as usize;
    let blksz = opt.size() as u16;
    let kektop = ScreenCtx::new(blocks as u16, blksz);
    let cb = |msg: Messages| kektop.update(msg);
    
    thread::scope(|s| {
        s.spawn(|_| crypt::decrypt(opt.cipher(), opt.size(), &oracle, &cb, opt.chars(), opt.iv()));
        s.spawn(|_| loop { term.draw(|f| kektop.draw(f)).unwrap() });
    }).unwrap();

    crossterm::execute!(io::stdout(), terminal::LeaveAlternateScreen).unwrap();
    //t.join().unwrap();
}
