use crate::msg::Messages;

use std::sync::Mutex;
use std::time::{ Duration, Instant };

use tui::terminal::{ Terminal, Frame };
use tui::backend::Backend;
use tui::widgets::{ Block, Borders, Text, Paragraph, Widget };
use tui::layout::{ Layout, Alignment, Direction, Constraint };
use tui::style::Style;

use crossterm::event;
use crossterm::event::{ Event, KeyCode };

use log::trace;

#[derive(Clone)]
struct StyledText<'a> {
    pub done: Text<'a>,
    pub curr: Text<'a>,
    pub next: Text<'a>
}

impl<'a> StyledText<'a> {
    pub fn new(done: String, curr: String, mut next: String) -> Self {
        next.push('\n');
        StyledText {
            done: Text::raw(done),
            curr: Text::raw(curr),
            next: Text::raw(next)
        }
    }

    pub fn placeholder(size: u16) -> Self {
        let mut init: String = vec!['.'; size as usize].iter().collect();
        init.push('\n');
        StyledText {
            done: Text::raw(""),
            curr: Text::raw(""),
            next: Text::raw(init)
        }
    }

    pub fn iter(self) -> impl Iterator<Item = Text<'a>> {
        let yeet = [self.done, self.curr, self.next];
        yeet.iter()
    }
}

pub type BlkTxt<'a> = Mutex<Vec<StyledText<'a>>>;
pub struct ScreenCtx<'a> {
    pyld_txt: BlkTxt<'a>,
    inter_txt: BlkTxt<'a>,
    plain_txt: BlkTxt<'a>,
    done: Mutex<bool>,
    blks: u16,
    blksz: u16,
    delay_us: u128
}

impl<'a> ScreenCtx<'a> {
    pub fn new(blks: u16, blksz: u16, fps: u64) -> Self {
        let mut init_txt: String = vec!['.'; blksz as usize * 2].iter().collect();
        init_txt.push('\n');

        let ublks = blks as usize;
        ScreenCtx {
            pyld_txt: Mutex::new(vec![StyledText::placeholder(blksz * 2); ublks]),
            inter_txt: Mutex::new(vec![StyledText::placeholder(blksz * 2); ublks]),
            plain_txt: Mutex::new(vec![StyledText::placeholder((blksz * 3) +1); ublks]),
            done: Mutex::new(false),
            delay_us: 1000000 / fps as u128,
            blks, blksz
        }
    }

    pub fn update(&self, msg: Messages) {
        let now = Instant::now();
        match msg {
            Messages::Payload(p) => {
                let mut txt = hex::encode(p.block());
                txt.push('\n');
                let txt = StyledText::new("".to_owned(), "".to_owned(), txt);
                self.pyld_txt.lock().unwrap()[p.block_index()] = txt;
            }
            Messages::Intermediate(i) => {
                let mut txt = hex::encode(i.block());
                txt.push('\n');
                let txt = StyledText::new("".to_owned(), "".to_owned(), txt);
                self.inter_txt.lock().unwrap()[i.block_index()] = txt;
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
                let txt = StyledText::new("".to_owned(), "".to_owned(), txt);
                self.plain_txt.lock().unwrap()[p.block_index()] = txt;
            },
            Messages::Done => *self.done.lock().unwrap() = true
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

    pub fn draw_loop<T: Backend>(&self, term: &mut Terminal<T>) {
        let mut now = Instant::now();
        while !{ *self.done.lock().unwrap() } {
            if now.elapsed().as_micros() > self.delay_us {
                term.draw(|f| self.draw(f)).unwrap();
                now = Instant::now();
            }
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
        let now = Instant::now();
        { pyld_txt = self.pyld_txt.lock().unwrap().clone(); }
        { inter_txt = self.inter_txt.lock().unwrap().clone(); }
        { plain_txt = self.plain_txt.lock().unwrap().clone(); }
        trace!("copying states took {:?}", now.elapsed());

        let now = Instant::now();
        Paragraph::new(pyld_txt.iter().flat_map(|s| [s.done, s.curr, s.next].into_iter()))
            .alignment(Alignment::Center)
            .block(Block::default().title("payload").borders(Borders::ALL))
            .render(&mut f, blocks[0]);
        Paragraph::new(inter_txt.iter().flat_map(|s| [s.done, s.curr, s.next].iter()))
            .alignment(Alignment::Center)
            .block(Block::default().title("intermediate").borders(Borders::ALL))
            .render(&mut f, blocks[1]);
        Paragraph::new(plain_txt.iter().flat_map(|s| [s.done, s.curr, s.next].iter()))
            .alignment(Alignment::Center)
            .block(Block::default().title("plain").borders(Borders::ALL))
            .render(&mut f, blocks[2]);
        trace!("rendering took {:?}", now.elapsed());
    }
}
