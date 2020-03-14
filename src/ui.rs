use crate::Messages;

use std::sync::Mutex;
use std::time::{ Duration, Instant };

use tui::terminal::Frame;
use tui::backend::Backend;
use tui::widgets::{ Block, Borders, Text, Paragraph, Widget };
use tui::layout::{ Layout, Alignment, Direction, Constraint };

use crossterm::event;
use crossterm::event::{ Event, KeyCode };

use log::trace;

pub type BlkTxt<'a> = Mutex<Vec<Text<'a>>>;
pub struct ScreenCtx<'a> {
    pyld_txt: BlkTxt<'a>,
    inter_txt: BlkTxt<'a>,
    plain_txt: BlkTxt<'a>,
    blks: u16,
    blksz: u16
}

impl<'a> ScreenCtx<'a> {
    pub fn new(blks: u16, blksz: u16) -> Self {
        let mut init_txt: String = vec!['.'; blksz as usize * 2].iter().collect();
        init_txt.push('\n');
        log::debug!("gui blocks: {}", blks);

        let ublks = blks as usize;
        ScreenCtx {
            pyld_txt: Mutex::new(vec![Text::raw(init_txt.clone()); ublks]),
            inter_txt: Mutex::new(vec![Text::raw(init_txt.clone()); ublks]),
            plain_txt: Mutex::new(vec![Text::raw(init_txt.clone()); ublks]),
            blks, blksz
        }
    }

    pub fn update(&self, msg: Messages) {
        let now = Instant::now();
        match msg {
            Messages::Payload(p) => {
                log::debug!("received block: {}", p.block_index());
                let mut txt = hex::encode(p.block());
                txt.push('\n');
                self.pyld_txt.lock().unwrap()[p.block_index()] = Text::raw(txt);
            }
            Messages::Intermediate(i) => {
                let mut txt = hex::encode(i.block());
                txt.push('\n');
                self.inter_txt.lock().unwrap()[i.block_index()] = Text::raw(txt);
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
                self.plain_txt.lock().unwrap()[p.block_index()] = Text::raw(txt);
            }
        };
        trace!("updating screen ctx took {:?}", now.elapsed());
    }

    fn handle_keys() {
        if event::poll(Duration::from_secs(0)).unwrap() {
            match event::read().unwrap() {
                Event::Key(e) => match e.code {
                    KeyCode::Esc => std::process::exit(0),
                    _ => ()
                },
                _ => ()
            };
        }
    }

    pub fn draw<F: Backend>(&self, mut f: Frame<F>) {
        ScreenCtx::handle_keys();
        let screen = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(self.blks + 4),
                          Constraint::Length(0)]
                          .as_ref())
            .split(f.size());

        let blocks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Length((self.blksz * 2) + 4),
                          Constraint::Length((self.blksz * 2) + 4),
                          Constraint::Length((self.blksz * 3) + 5),
                          Constraint::Length(0)]
                          .as_ref())
            .split(screen[0]);

        let pyld_txt;
        let inter_txt;
        let plain_txt;
        std::thread::sleep(Duration::from_millis(1));
        let now = Instant::now();
        { pyld_txt = self.pyld_txt.lock().unwrap().clone(); }
        { inter_txt = self.inter_txt.lock().unwrap().clone(); }
        { plain_txt = self.plain_txt.lock().unwrap().clone(); }
        trace!("copying states took {:?}", now.elapsed());

        let now = Instant::now();
        Paragraph::new(pyld_txt.iter())
            .alignment(Alignment::Center)
            .block(Block::default().title("payload").borders(Borders::ALL))
            .render(&mut f, blocks[0]);
        Paragraph::new(inter_txt.iter())
            .alignment(Alignment::Center)
            .block(Block::default().title("intermediate").borders(Borders::ALL))
            .render(&mut f, blocks[1]);
        Paragraph::new(plain_txt.iter())
            .alignment(Alignment::Center)
            .block(Block::default().title("plain").borders(Borders::ALL))
            .render(&mut f, blocks[2]);
        trace!("rendering took {:?}", now.elapsed());
    }
}
