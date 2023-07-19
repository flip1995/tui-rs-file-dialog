use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{self, Result},
    path::PathBuf,
};
use tui::{
    backend::{Backend, CrosstermBackend},
    widgets::{Block, Borders},
    Frame, Terminal,
};

use tui_file_dialog::{bind_keys, FileDialog, FilePattern};

struct App {
    // 1. Add the `FileDialog` to the tui app.
    file_dialog: FileDialog,

    selected_files: Vec<PathBuf>,
}

impl App {
    pub fn new(file_dialog: FileDialog) -> Self {
        Self {
            file_dialog,
            selected_files: vec![],
        }
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut file_dialog = FileDialog::new(60, 40)?;
    file_dialog.set_multi_selection(true);
    file_dialog.set_filter(FilePattern::Extension("toml".to_string()))?;
    let res = run_app(&mut terminal, App::new(file_dialog));

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        // 2. Use the `bind_keys` macro to overwrite key bindings, when the file dialog is open.
        // The first argument of the macro is the expression that should be used to access the file
        // dialog.
        bind_keys!(
            app.file_dialog,
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('o') if key.modifiers == KeyModifiers::CONTROL => {
                        app.file_dialog.open()
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        );

        // 3. Deal with the result of the file dialog
        if let Some(selected_files) = app.file_dialog.selected_files() {
            app.selected_files = selected_files;
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let block = Block::default()
        .title(format!(
            "Selected files: {}",
            app.selected_files
                .iter()
                .map(|f| f.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .borders(Borders::ALL);
    f.render_widget(block, f.size());

    // 4. Call the draw function of the file dialog in order to render it.
    app.file_dialog.draw(f);
}
