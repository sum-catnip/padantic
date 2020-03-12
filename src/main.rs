mod error;
mod prio;
mod oracle;
mod msg;
mod crypt;
mod cli;

use error::Result;
use oracle::{ CmdOracle, CmdOracleCtx };
use prio::PrioQueue;

use msg::{ BlockData, Messages };

use std::sync::{ Arc, Mutex };
use std::thread;
use std::char;

use crossterm::terminal;
use tui::Terminal;
use tui::backend::CrosstermBackend;
use tui::widgets::{ Block, Borders, Text, Paragraph, Widget };
use tui::layout::{ Layout, Alignment, Direction, Constraint };

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

    let blocks = opt.cipher().len() / opt.size() as usize;
    let blksz = opt.size() as usize;
    let mut init_txt: String = vec!['.'; blksz * 2].iter().collect();
    init_txt.push('\n');
    let payload_txt = Arc::new(Mutex::new(vec![Text::raw(init_txt.clone()); blocks -1]));
    let intermediate_txt = Arc::new(Mutex::new(vec![Text::raw(init_txt.clone()); blocks -1]));
    let plain_txt = Arc::new(Mutex::new(vec![Text::raw(init_txt.clone()); blocks -1]));
    let cb = |msg: Messages| {
        match msg {
            Messages::Payload(p) => {
                let mut txt = hex::encode(p.block());
                txt.push('\n');
                payload_txt.lock().unwrap()[p.block_index()] = Text::raw(txt);
            }
            Messages::Intermediate(i) => {
                let mut txt = hex::encode(i.block());
                txt.push('\n');
                intermediate_txt.lock().unwrap()[i.block_index()] = Text::raw(txt);
            }
            Messages::Plain(p) => {
                let mut txt = hex::encode(p.block());
                txt.push(' ');
                txt.push_str(&p.block()
                    .iter()
                    .map(|b| char::from(*b))
                    .map(|c| if c.is_ascii_graphic() { c } else { '.' })
                    .collect::<String>());
                txt.push('\n');
                plain_txt.lock().unwrap()[p.block_index()] = Text::raw(txt);
            }
        };
    };
    
    let payload_txt = payload_txt.clone();
    let intermediate_txt = intermediate_txt.clone();
    let plain_txt = plain_txt.clone();
    let t = thread::spawn(move || {
        let blksz = blksz as u16;
        loop {
            term.draw(|mut f| {
                let size = f.size();
                let screen = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([Constraint::Length(blocks as u16 + 2),
                                  Constraint::Length(5)]
                                  .as_ref())
                    .split(size);

                let blocks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([Constraint::Length((blksz * 2) + 4),
                                  Constraint::Length((blksz * 2) + 4),
                                  Constraint::Length((blksz * 3) + 4),
                                  Constraint::Length(0)]
                                  .as_ref())
                    .split(screen[0]);

                {
                    Paragraph::new(payload_txt.lock().unwrap().iter())
                        .alignment(Alignment::Center)
                        .block(Block::default().title("payload").borders(Borders::ALL))
                        .render(&mut f, blocks[0]);
                }
                {
                    Paragraph::new(intermediate_txt.lock().unwrap().iter())
                        .alignment(Alignment::Center)
                        .block(Block::default().title("intermediate").borders(Borders::ALL))
                        .render(&mut f, blocks[1]);
                }
                {
                    Paragraph::new(plain_txt.lock().unwrap().iter())
                        .alignment(Alignment::Center)
                        .block(Block::default().title("plain").borders(Borders::ALL))
                        .render(&mut f, blocks[2]);
                }
            }).unwrap();
        }
    });

    for dec in crypt::decrypt(opt.cipher(), opt.size(), &oracle, &cb, opt.chars(), opt.iv()) {
        //println!("{:?}", dec.unwrap());
    }

    //t.join().unwrap();
}
