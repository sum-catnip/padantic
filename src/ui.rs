use crate::Messages;

use std::sync::Mutex;

use tui::terminal::Frame;
use tui::backend::Backend;
use tui::widgets::{ Block, Borders, Text, Paragraph, Widget };
use tui::layout::{ Layout, Alignment, Direction, Constraint };

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

        let ublks = blks as usize;
        ScreenCtx {
            pyld_txt: Mutex::new(vec![Text::raw(init_txt.clone()); ublks -1]),
            inter_txt: Mutex::new(vec![Text::raw(init_txt.clone()); ublks -1]),
            plain_txt: Mutex::new(vec![Text::raw(init_txt.clone()); ublks -1]),
            blks, blksz
        }
    }

    pub fn update(&self, msg: Messages) {
        match msg {
            Messages::Payload(p) => {
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
    }

    pub fn draw<F: Backend>(&self, mut f: Frame<F>) {
        let size = f.size();
        let screen = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Length(self.blks + 2),
                          Constraint::Length(5)]
                          .as_ref())
            .split(size);

        let blocks = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Length((self.blksz * 2) + 4),
                          Constraint::Length((self.blksz * 2) + 4),
                          Constraint::Length((self.blksz * 3) + 4),
                          Constraint::Length(0)]
                          .as_ref())
            .split(screen[0]);

        {
            Paragraph::new(self.pyld_txt.lock().unwrap().iter())
                .alignment(Alignment::Center)
                .block(Block::default().title("payload").borders(Borders::ALL))
                .render(&mut f, blocks[0]);
        }
        {
            Paragraph::new(self.inter_txt.lock().unwrap().iter())
                .alignment(Alignment::Center)
                .block(Block::default().title("intermediate").borders(Borders::ALL))
                .render(&mut f, blocks[1]);
        }
        {
            Paragraph::new(self.plain_txt.lock().unwrap().iter())
                .alignment(Alignment::Center)
                .block(Block::default().title("plain").borders(Borders::ALL))
                .render(&mut f, blocks[2]);
        }
    }
}
