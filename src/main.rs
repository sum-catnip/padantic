#![feature(try_find)]
#![feature(trait_alias)]

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

use std::io;
use std::fs::File;
use std::io::Write;

use crossterm::terminal::{ EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode };
use crossterm::execute;
use tui::Terminal;
use tui::backend::CrosstermBackend;

use crossbeam::thread;

use simplelog::{ WriteLogger, LevelFilter, Config };

fn main() {
    let _ = WriteLogger::init(LevelFilter::Trace, Config::default(), File::create("log.log").unwrap());

    let opt = cli::parse();
    let oracle = CmdOracleCtx::new(opt.oracle().to_owned(), opt.oracle_args().to_owned());
    let _ = execute!(io::stdout(), EnterAlternateScreen);
    enable_raw_mode().unwrap();
    let backend = CrosstermBackend::new(io::stdout());
    let mut term = Terminal::new(backend).unwrap();
    term.hide_cursor().unwrap();
    term.clear().unwrap();

    let blocks = opt.cipher().len() / opt.size() as usize;
    let blksz = opt.size() as u16;
    let screen = ScreenCtx::new(blocks as u16 -1, blksz);
    let cb = |msg| screen.update(msg);
    
    term.draw(|f| screen.draw(f)).unwrap();
    thread::scope(|s| {
        s.spawn(|_| crypt::decrypt(opt.cipher(), opt.size(), &oracle, &cb, opt.chars(), opt.iv()));
        s.spawn(|_| screen.draw_loop(&mut term));
    }).unwrap();

    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}
