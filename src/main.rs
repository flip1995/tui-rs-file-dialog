use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, Result};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
    Frame, Terminal,
};

use tui_rs_file_dialog::{bind_keys, FileDialog};

#[derive(Debug)]
struct App {
    file_dialog: FileDialog,
}

impl App {
    pub fn new(file_dialog: FileDialog) -> Self {
        Self { file_dialog }
    }
}

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, App::new(FileDialog::new(50, 50, None)?));

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
        )
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(f.size());
    let block = Block::default().title("Block 1").borders(Borders::ALL);
    f.render_widget(block, chunks[0]);
    let block = Block::default().title("Block 2").borders(Borders::ALL);
    f.render_widget(block, chunks[1]);

    app.file_dialog.draw(f);
}
