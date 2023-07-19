//! This is a tui-rs extension for a file dialog popup.
//!
//! ## Usage
//!
//! See the `examples` directory on how to use this extension. Run
//!
//! ```
//! cargo run --example demo
//! ```
//!
//! to see it in action.
//!
//! First, add a file dialog to the TUI app:
//!
//! ```rust
//! use tui_file_dialog::FileDialog;
//!
//! struct App {
//!     // Other fields of the App...
//!
//!     file_dialog: FileDialog
//! }
//! ```
//!
//! If you want to use the default key bindings provided by this crate, just wrap
//! the event handler of your app in the [`bind_keys!`] macro.
//!
//! ```rust
//! use tui_file_dialog::bind_keys;
//!
//! fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
//!     loop {
//!         terminal.draw(|f| ui(f, &mut app))?;
//!
//!         bind_keys!(
//!             // Expression to use to access the file dialog.
//!             app.file_dialog,
//!             // Event handler of the app, when the file dialog is closed.
//!             if let Event::Key(key) = event::read()? {
//!                 match key.code {
//!                     KeyCode::Char('q') => {
//!                         return Ok(());
//!                     }
//!                     _ => {}
//!                 }
//!             }
//!         )
//!     }
//! }
//! ```
//!
//! Also in the `run_app` function, deal with the selected files:
//!
//! ```rust
//! fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
//!     loop {
//!         // Bindings and app drawing code...
//!
//!         if let Some(selected_files) = app.file_dialog.selected_files() {
//!             app.selected_files = selected_files;
//!         }
//!      }
//! }
//! ```
//!
//! Finally, draw the file dialog:
//!
//! ```rust
//! fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
//!     // Other UI drawing code...
//!
//!     app.file_dialog.draw(f);
//! }
//! ```
//!
//! ## Limitations
//!
//! I've started this crate with a minimalistic approach and new functionality will
//! be added on a use-case basis. For example, it is currently not possible to add
//! styling to the file dialog and just a boring, minimalist block with a list is
//! used to render it.
use std::{
    cmp,
    collections::HashSet,
    ffi::OsString,
    fs,
    io::Result,
    iter,
    path::{Path, PathBuf},
};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// A pattern that can be used to filter the displayed files.
pub enum FilePattern {
    /// Filter by file extension. This filter is case insensitive.
    Extension(String),
    /// Filter by substring. This filter is case sensitive.
    Substring(String),
}

impl FilePattern {
    /// Returns whether the given file name matches the filter.
    pub fn matches(&self, file: &Path) -> bool {
        if file.is_dir() {
            return true;
        }
        match self {
            FilePattern::Extension(ext) => file.extension().map_or(false, |e| {
                e.to_ascii_lowercase() == OsString::from(ext.to_ascii_lowercase())
            }),
            FilePattern::Substring(substr) => file
                .file_name()
                .map_or(false, |n| n.to_string_lossy().contains(substr)),
        }
    }
}

/// The file dialog.
///
/// This manages the state of the file dialog. After selecting a file, the absolute path to that
/// file will be stored in the file dialog.
///
/// The file dialog is opened with the current working directory by default. To start the file
/// dialog with a different directory, use [`FileDialog::set_dir`].
pub struct FileDialog {
    width: u16,
    height: u16,

    filter: Option<FilePattern>,
    open: bool,
    current_dir: PathBuf,
    show_hidden: bool,

    default_bindings: bool,
    multi_selection: bool,

    list_state: ListState,
    items: Vec<String>,
    selected_indices: HashSet<usize>,
}

impl FileDialog {
    /// Create a new file dialog.
    ///
    /// The width and height are the size of the file dialog in percent of the terminal size. They
    /// are clamped to 100%.
    pub fn new(width: u16, height: u16) -> Result<Self> {
        let mut s = Self {
            width: cmp::min(width, 100),
            height: cmp::min(height, 100),

            filter: None,
            open: false,
            current_dir: PathBuf::from(".").canonicalize().unwrap(),
            show_hidden: false,

            default_bindings: false,
            multi_selection: false,

            list_state: ListState::default(),
            items: vec![],
            selected_indices: HashSet::new(),
        };

        s.update_entries()?;

        Ok(s)
    }

    /// Whether the default bindings are used.
    ///
    /// This is set by the [`bind_keys!`] macro automatically.
    pub fn default_bindings(&mut self, used: bool) {
        self.default_bindings = used;
    }
    /// Whether multi selection should be enabled.
    pub fn set_multi_selection(&mut self, enable: bool) {
        self.multi_selection = enable;
    }
    /// Returns true, when multi selection is enabled.
    pub fn multi_selection(&self) -> bool {
        self.multi_selection
    }
    /// The directory to open the file dialog in.
    pub fn set_dir(&mut self, dir: PathBuf) -> Result<()> {
        self.current_dir = dir.canonicalize()?;
        self.update_entries()
    }
    /// Sets the filter to use when browsing files.
    pub fn set_filter(&mut self, filter: FilePattern) -> Result<()> {
        self.filter = Some(filter);
        self.update_entries()
    }
    /// Removes the filter.
    pub fn reset_filter(&mut self) -> Result<()> {
        self.filter.take();
        self.update_entries()
    }
    /// Toggles whether hidden files should be shown.
    ///
    /// This only checks whether the file name starts with a dot.
    pub fn toggle_show_hidden(&mut self) -> Result<()> {
        self.show_hidden = !self.show_hidden;
        self.update_entries()
    }

    /// Opens the file dialog.
    ///
    /// Resets the selected files.
    pub fn open(&mut self) {
        self.selected_indices.clear();
        self.open = true;
    }
    /// Closes the file dialog.
    pub fn close(&mut self) {
        self.open = false;
    }
    /// Returns whether the file dialog is currently open.
    pub fn is_open(&self) -> bool {
        self.open
    }
    /// Draws the file dialog in the TUI application.
    pub fn draw<B: Backend>(&mut self, f: &mut Frame<B>) {
        if self.open {
            let block = Block::default()
                .title(format!("{}", self.current_dir.to_string_lossy()))
                .borders(Borders::ALL);
            let list_items: Vec<ListItem> = self
                .items
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    ListItem::new(format!(
                        "{}{}",
                        if self.multi_selection {
                            if self.selected_indices.contains(&i) {
                                "☑ "
                            } else {
                                "☐ "
                            }
                        } else {
                            ""
                        },
                        s.as_str()
                    ))
                })
                .collect();

            let list = List::new(list_items).block(block).highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            );

            let mut area = centered_rect(self.width, self.height, f.size());
            if self.default_bindings {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(1), Constraint::Length(1)].as_ref())
                    .split(area);
                area = chunks[0];
                f.render_widget(
                    Paragraph::new(format!(
                        "{}'Enter': open - 'q': quit",
                        if self.multi_selection {
                            "'Space': select - "
                        } else {
                            ""
                        }
                    ))
                    .alignment(tui::layout::Alignment::Right),
                    chunks[1],
                );
            }
            f.render_stateful_widget(list, area, &mut self.list_state);
        }
    }

    /// Get the selected_files.
    ///
    /// Only returns them after the file dialog was closed and will reset them.
    pub fn selected_files(&mut self) -> Option<Vec<PathBuf>> {
        if !self.open {
            let mut files = vec![];
            for i in self.selected_indices.iter() {
                files.push(self.current_dir.join(&self.items[*i]));
            }
            self.selected_indices.clear();
            Some(files)
        } else {
            None
        }
    }

    /// Goes to the next item in the file list.
    pub fn next(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => cmp::min(self.items.len() - 1, i + 1),
            None => cmp::min(self.items.len().saturating_sub(1), 1),
        };
        self.list_state.select(Some(i));
    }
    /// Goes to the previous item in the file list.
    pub fn previous(&mut self) {
        let i = match self.list_state.selected() {
            Some(i) => i.saturating_sub(1),
            None => 0,
        };
        self.list_state.select(Some(i));
    }
    /// Moves one directory up.
    ///
    /// Resets the selected files in multi selection mode.
    pub fn up(&mut self) -> Result<()> {
        self.current_dir.pop();
        self.selected_indices.clear();
        self.update_entries()
    }

    /// Selects an item in the file list.
    ///
    /// If the item is a directory, the file dialog will move into that directory. If the item is a
    /// file, the file will be selected. If multi selection is not enabled, the file dialog will
    /// close and the path to the file can be retrieved through [`FileDialog::selected_files`].
    ///
    /// Resets the selected files when changing directory in multi selection mode.
    pub fn select(&mut self) -> Result<()> {
        let Some(selected) = self.list_state.selected() else {
            self.next();
            return Ok(());
        };

        let path = self.current_dir.join(&self.items[selected]);
        if path.is_file() {
            self.toggle_selection();
            if !self.multi_selection {
                self.close();
            }
            return Ok(());
        }

        self.current_dir = path.canonicalize()?;
        self.selected_indices.clear();
        self.update_entries()
    }

    /// Toggles the selection of the currently selected item.
    ///
    /// This only makes sense in multi selection mode. In single selection mode, use the
    /// [`FileDialog::select`] method.
    pub fn toggle_selection(&mut self) {
        let Some(selected) = self.list_state.selected() else {
            self.next();
            return;
        };

        if self.selected_indices.contains(&selected) {
            self.selected_indices.remove(&selected);
        } else {
            self.selected_indices.insert(selected);
        }
    }

    /// Updates the entries in the file list. This function is called automatically when necessary.
    fn update_entries(&mut self) -> Result<()> {
        self.items = iter::once("..".to_string())
            .chain(
                fs::read_dir(&self.current_dir)?
                    .flatten()
                    .filter(|file| {
                        let file = file.path();
                        if file
                            .file_name()
                            .map_or(false, |n| n.to_string_lossy().starts_with('.'))
                        {
                            return self.show_hidden;
                        }
                        if let Some(ref filter) = self.filter {
                            return filter.matches(&file);
                        }
                        true
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

/// Macro to automatically overwrite the default key bindings of the app, when the file dialog is
/// open.
///
/// This macro only works inside of a function that returns a [`std::io::Result`] or a result that
/// has an error type that implements [`From<std::io::Error>`].
///
/// Default bindings:
///
/// | Key | Action |
/// | --- | --- |
/// | `q`, `Esc` | Close the file dialog. |
/// | `j`, `Down` | Move down in the file list. |
/// | `k`, `Up` | Move up in the file list. |
/// | `Enter` | Open the current item. |
/// | `Space` | Select the current item (if multi selection is enabled). |
/// | `u` | Move one directory up. |
/// | `I` | Toggle showing hidden files. |
///
/// ## Example
///
/// ```
/// bind_keys!(
///     // Expression to use to access the file dialog.
///     app.file_dialog,
///     // Event handler of the app, when the file dialog is closed.
///     if let Event::Key(key) = event::read()? {
///         match key.code {
///             KeyCode::Char('q') => {
///                 return Ok(());
///             }
///             _ => {}
///         }
///     }
/// )
/// ```
#[macro_export]
macro_rules! bind_keys {
    ($file_dialog:expr, $e:expr) => {{
        $file_dialog.default_bindings(true);
        if $file_dialog.is_open() {
            use ::crossterm::event::{self, Event, KeyCode};
            // File dialog events
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        $file_dialog.close();
                    }
                    KeyCode::Char('I') => $file_dialog.toggle_show_hidden()?,
                    KeyCode::Enter => {
                        $file_dialog.select()?;
                    }
                    KeyCode::Char(' ') if $file_dialog.multi_selection() => {
                        $file_dialog.toggle_selection();
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

/// Helper function to create a centered rectangle in the TUI app.
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
