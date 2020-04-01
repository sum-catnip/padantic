#![feature(try_find)]
#![feature(trait_alias)]

mod prio;
mod oracle;
mod msg;
mod crypt;
mod cli;
mod ui;

use oracle::{ CmdOracle, CmdOracleCtx };
use prio::PrioQueue;
use ui::ScreenCtx;

use std::io;
use std::io::Write;
use std::fs::File;

use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen,
    enable_raw_mode, disable_raw_mode
};
use crossterm::execute;
use tui::Terminal;
use tui::backend::CrosstermBackend;

use crossbeam::thread;

use simplelog::{ WriteLogger, LevelFilter, Config };

fn main() {
    let opt = cli::parse();
    if let Some(file) = opt.logfile() {
        let lvl = match opt.loglvl() {
            1 => LevelFilter::Error,
            2 => LevelFilter::Warn,
            3 => LevelFilter::Info,
            4 => LevelFilter::Debug,
            5 => LevelFilter::Trace,
            _ => panic!("invalid log level! max is `vvvv` for `trace`")
        };
        let _ = WriteLogger::init(lvl, Config::default(),
                                  File::create(&file)
                                      .expect("error creating logfile"));
    }

    let mut f_inter = None;
    let mut f_plain = None;

    if let Some(f) = opt.outfile() {
        f_plain = Some(File::open(format!("{}.plain", f))
            .expect("error opening plaintext output file"))
    }

    if let Some(f) = opt.outfile() {
        f_inter = Some(File::open(format!("{}.inter", f))
            .expect("error opening intermediate output file"))
    }

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
    let res = thread::scope(|s| {
        s.spawn(|_| screen.draw_loop(&mut term));
        s.spawn(|_| crypt::decrypt(opt.cipher(), opt.size(), &oracle, &cb, opt.chars(), opt.iv()))
            .join()
            .unwrap()
    }).unwrap();

    let _ = execute!(io::stdout(), LeaveAlternateScreen);
    disable_raw_mode().unwrap();

    println!("+>> recovered plaintext ::: intermediate bytes\n");
    for blk in res {
        match blk {
            Ok(blk) => cli::block_output(blk, &mut f_inter, &mut f_plain),
            Err(e) => eprintln!("error decrypting block: {:?}", e)
        }
    }
}
