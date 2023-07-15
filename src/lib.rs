use std::{cmp, ffi::OsString, fs, io::Result, iter, path::PathBuf};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

#[derive(Debug)]
pub enum FilePattern {
    Extension(String),
    Name(String),
    Substring(String),
}

#[derive(Debug)]
pub struct FileDialog {
    width: u16,
    height: u16,
    filter: Option<FilePattern>,

    pub selected_file: Option<PathBuf>,

    open: bool,
    current_dir: PathBuf,
    show_hidden: bool,

    list_state: ListState,
    items: Vec<String>,
}

impl FileDialog {
    pub fn new(width: u16, height: u16, filter: Option<FilePattern>) -> Result<Self> {
        let mut s = Self {
            width: cmp::min(width, 100),
            height: cmp::min(height, 100),
            filter,

            selected_file: None,

            open: false,
            current_dir: PathBuf::from(".").canonicalize().unwrap(),
            show_hidden: false,

            list_state: ListState::default(),
            items: vec![],
        };

        s.update_entries()?;

        Ok(s)
    }
    pub fn set_dir(&mut self, dir: PathBuf) -> Result<()> {
        self.current_dir = dir.canonicalize()?;
        self.update_entries()
    }
    pub fn set_filter(&mut self, filter: FilePattern) -> Result<()> {
        self.filter = Some(filter);
        self.update_entries()
    }
    pub fn reset_filter(&mut self) -> Result<()> {
        self.filter.take();
        self.update_entries()
    }
    pub fn invert_show_hidden(&mut self) -> Result<()> {
        self.show_hidden = !self.show_hidden;
        self.update_entries()
    }

    pub fn open(&mut self) {
        self.open = true;
    }
    pub fn close(&mut self) {
        self.open = false;
    }
    pub fn is_open(&self) -> bool {
        self.open
    }
    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
        if self.open {
            let block = Block::default()
                .title(format!("{}", self.current_dir.to_string_lossy()))
                .borders(Borders::ALL);
            let list_items: Vec<ListItem> = self
                .items
                .iter()
                .map(|s| ListItem::new(s.as_str()))
                .collect();

            let list = List::new(list_items).block(block).highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            );

            let area = centered_rect(self.height, self.width, f.size());
            f.render_stateful_widget(list, area, &mut self.list_state);
        }
    }

    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => cmp::min(self.items.len() - 1, i + 1),
            None => cmp::min(self.items.len().saturating_sub(1), 1),
        };
        self.list_state.select(Some(i));
    }
    pub fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    pub fn up(&mut self) -> Result<()> {
        self.current_dir.pop();
        self.update_entries()
    }

    pub fn select(&mut self) -> Result<()> {
        let Some(selected) = self.list_state.selected() else {
            self.next();
            return Ok(());
        };

        let path = self.current_dir.join(&self.items[selected]);
        if path.is_file() {
            self.selected_file = Some(path);
            self.close();
            return Ok(());
        }

        self.current_dir = path;
        self.update_entries()
    }

    fn update_entries(&mut self) -> Result<()> {
        self.items = iter::once("..".to_string())
            .chain(
                fs::read_dir(&self.current_dir)?
                    .flatten()
                    .filter(|e| {
                        let e = e.path();
                        if e.file_name()
                            .map_or(false, |n| n.to_string_lossy().starts_with('.'))
                        {
                            return self.show_hidden;
                        }
                        if e.is_dir() || self.filter.is_none() {
                            return true;
                        }
                        match self.filter.as_ref().unwrap() {
                            FilePattern::Extension(ext) => e.extension().map_or(false, |e| {
                                e.to_ascii_lowercase() == OsString::from(ext.to_ascii_lowercase())
                            }),
                            FilePattern::Name(name) => {
                                e.file_name().map_or(false, |n| n == OsString::from(name))
                            }
                            FilePattern::Substring(substr) => e
                                .file_name()
                                .map_or(false, |n| n.to_string_lossy().contains(substr)),
                        }
                    })
                    .map(|file| {
                        let file_name = file.file_name();
                        if matches!(file.file_type(), Ok(t) if t.is_dir()) {
                            format!("{}/", file_name.to_string_lossy())
                        } else {
                            file_name.to_string_lossy().to_string()
                        }
                    }),
            )
            .collect();
        self.items.sort_by(|a, b| {
            if a == ".." {
                return cmp::Ordering::Less;
            }
            if b == ".." {
                return cmp::Ordering::Greater;
            }
            match (a.chars().last().unwrap(), b.chars().last().unwrap()) {
                ('/', '/') => a.cmp(b),
                ('/', _) => cmp::Ordering::Less,
                (_, '/') => cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });
        self.list_state.select(None);
        self.next();
        Ok(())
    }
}

#[macro_export]
macro_rules! bind_keys {
    ($file_dialog:expr, $e:expr) => {{
        if $file_dialog.is_open() {
            use ::crossterm::event::{self, Event, KeyCode};
            // File dialog events
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        $file_dialog.close();
                    }
                    KeyCode::Char('I') => $file_dialog.invert_show_hidden()?,
                    KeyCode::Enter => {
                        $file_dialog.select()?;
                    }
                    KeyCode::Char('u') => {
                        $file_dialog.up()?;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        $file_dialog.previous();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        $file_dialog.next();
                    }
                    _ => {}
                }
            }
        } else {
            $e
        }
    }};
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
