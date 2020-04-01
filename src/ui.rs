use crate::msg::Messages;

use std::sync::Mutex;
use std::time::{ Duration, Instant };
use std::thread;

use tui::terminal::{ Terminal, Frame };
use tui::backend::Backend;
use tui::widgets::{ Block, Borders, Text, Paragraph, Widget };
use tui::layout::{ Layout, Alignment, Direction, Constraint };
use tui::style::{ Color, Style };

use crossterm::event;
use crossterm::event::{ Event, KeyCode, KeyModifiers };

use log::trace;

pub type BlkTxt<'a> = Mutex<Vec<Vec<Text<'a>>>>;
pub struct ScreenCtx<'a> {
    pyld_txt: BlkTxt<'a>,
    inter_txt: BlkTxt<'a>,
    plain_txt: BlkTxt<'a>,
    done: Mutex<bool>,
    blks: u16,
    blksz: u16,
}

fn stylize_hex<'a>(blk: &Vec<u8>, i: usize, blki: usize, lst: &BlkTxt<'a>) {
    let next = hex::encode(&blk[0..i]);
    let next = Text::styled(next, Style::default().fg(Color::LightRed));

    let curr = hex::encode(&blk[i..i +1]);
    let curr = Text::styled(curr, Style::default().fg(Color::LightCyan));

    let done = hex::encode(&blk[i..blk.len() -1]);
    let done = Text::styled(done, Style::default().fg(Color::LightGreen));

    let mut lst = lst.lock().unwrap();
    lst[blki][0] = next;
    lst[blki][1] = curr;
    lst[blki][2] = done;
    lst[blki][3] = Text::raw("\n");
}

impl<'a> ScreenCtx<'a> {
    pub fn new(blks: u16, blksz: u16) -> Self {
        let mut init_txt: String = vec!['.'; blksz as usize * 2].iter().collect();
        init_txt.push('\n');

        let init = vec![vec![Text::raw(init_txt), Text::raw(""), Text::raw(""), Text::raw("")]; blks as usize];
        ScreenCtx {
            pyld_txt: Mutex::new(init.clone()),
            inter_txt: Mutex::new(init.clone()),
            plain_txt: Mutex::new(init.clone()),
            done: Mutex::new(false),
            blks, blksz
        }
    }

    pub fn update(&self, msg: Messages) {
        let now = Instant::now();
        match msg {
            Messages::Payload(p) =>
                stylize_hex(p.block(), p.index() as usize, p.block_index(), &self.pyld_txt),

            Messages::Intermediate(i) =>
                stylize_hex(i.block(), i.index() as usize, i.block_index(), &self.inter_txt),

            Messages::Plain(p) => {
                let mut txt = " ".to_owned();
                txt.push_str(&p.block()
                    .iter()
                    .map(|b| char::from(*b))
                    .map(|c| if c.is_ascii_graphic() { c } else { '.' })
                    .collect::<String>());
                txt.push('\n');
                let txt = Text::raw(txt);
                stylize_hex(p.block(), p.index() as usize, p.block_index(), &self.plain_txt);
                self.plain_txt.lock().unwrap()[p.block_index()][3] = txt;
            },
            Messages::Done => *self.done.lock().unwrap() = true
        };
        trace!("updating screen ctx took {:?}", now.elapsed());
    }

    fn handle_keys() {
        if event::poll(Duration::from_secs(0)).unwrap() {
            match event::read().unwrap() {
                Event::Key(e) => match e.code {
                    KeyCode::Char('c') if e.modifiers.contains(KeyModifiers::CONTROL) =>
                        std::process::exit(0),
                    _ => ()
                },
                _ => ()
            };
        }
    }

    pub fn draw_loop<T: Backend>(&self, term: &mut Terminal<T>) {
        while !{ *self.done.lock().unwrap() } {
            term.draw(|f| self.draw(f)).unwrap();
            thread::sleep(Duration::from_millis(10));
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
        Paragraph::new(pyld_txt.iter().flatten())
            .alignment(Alignment::Center)
            .block(Block::default().title("payload").borders(Borders::ALL))
            .render(&mut f, blocks[0]);
        Paragraph::new(inter_txt.iter().flatten())
            .alignment(Alignment::Center)
            .block(Block::default().title("intermediate").borders(Borders::ALL))
            .render(&mut f, blocks[1]);
        Paragraph::new(plain_txt.iter().flatten())
            .alignment(Alignment::Center)
            .block(Block::default().title("plain").borders(Borders::ALL))
            .render(&mut f, blocks[2]);
        trace!("rendering took {:?}", now.elapsed());
    }
}
