use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{io, thread, time::Duration};
use tui::{
    backend::CrosstermBackend,
    widgets::{Block, Borders},
    Terminal,
};

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    terminal.draw(|f| {
        let mut graph_size = dbg!(f.size());
        let mut devices_size = f.size();
        let graph = Block::default().title("Graph").borders(Borders::ALL);
        let devices = Block::default().title("Devices").borders(Borders::ALL);

        graph_size.width /= 2;
        devices_size.y = devices_size.width / 2;
        devices_size.width /= 2;

        f.render_widget(graph, graph_size);
        f.render_widget(devices, dbg!(devices_size));
    })?;

    thread::sleep(Duration::from_secs(5));

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
