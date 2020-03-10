mod error;
mod prio;
mod oracle;
mod msg;
mod crypt;
mod cli;

use error::Result;
use oracle::{ CmdOracle, CmdOracleCtx };
use prio::PrioQueue;

use msg::{ ProgressMsg, Messages };

use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;

use snafu::{ Snafu, ResultExt, ensure };
use base64;
use crossterm::{ event, terminal };
use tui::Terminal;
use tui::backend::CrosstermBackend;
use tui::widgets::{ Text, Paragraph, Widget };
use tui::layout::Alignment;

fn main() {
    let opt = cli::parse();
    let oracle = CmdOracleCtx::new(opt.oracle().to_owned(), opt.oracle_args().to_owned());
    terminal::enable_raw_mode().unwrap();
    let stdout = std::io::stdout();
    // crossterm::execute!(stdout, terminal::EnterAlternateScreen).unwrap();
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend).unwrap();
    term.hide_cursor().unwrap();
    term.clear().unwrap();
    
    let (tx, rx) = mpsc::channel::<Messages>();
    let t = thread::spawn(move || {
        let mut blocks = HashMap::<usize, ProgressMsg>::new();
        let mut text: Vec<Text> = Vec::new();
        loop {
            if let Ok(msg) = rx.try_recv() {
                match msg {
                    Messages::Done() => break,
                    Messages::Prog(p) => { blocks.insert(p.block(), p); }
                }
                text = blocks
                    .values()
                    .map(|p| {
                        let mut hex = hex::encode(p.payload());
                        hex.push('\n');
                        Text::raw(hex)
                    })
                    .collect();
            }
            term.draw(|mut f| {
                let size = f.size();
                Paragraph::new(text.iter())
                    .alignment(Alignment::Center)
                    .render(&mut f, size);
            }).unwrap();
        }
    });

    for dec in crypt::decrypt(opt.cipher(), opt.size(), &oracle, tx, opt.chars(), opt.iv()) {
        println!("{:?}", dec.unwrap());
    }

    t.join().unwrap();
}
