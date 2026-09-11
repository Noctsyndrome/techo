mod app;
mod calendar;
mod editor;
mod journal;
mod schedule;
mod storage;
mod theme;
mod ui;
mod words;

use chrono::Local;
use crossterm::{
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io::{self, IsTerminal, stdout},
    path::PathBuf,
    time::Duration,
};

struct Screen;
impl Drop for Screen {
    fn drop(&mut self) {
        restore();
    }
}
fn restore() {
    let _ = disable_raw_mode();
    let _ = execute!(
        stdout(),
        DisableMouseCapture,
        DisableBracketedPaste,
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
}
fn main() -> io::Result<()> {
    let mut dir: Option<PathBuf> = None;
    let mut date = Local::now().date_naive();
    let mut mouse = true;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "techō — a little space for your day\n\nUsage: techo [--data-dir PATH] [--date YYYY-MM-DD] [--no-mouse]\n\n--data-dir PATH   Journal directory (overrides TECHO_DIR)\n--date DATE       Open a date instead of today\n--no-mouse        Keep terminal mouse selection behavior\n--version        Show version\n\nDefault files: $XDG_DATA_HOME/techo/journals or ~/.local/share/techo/journals\nWindows: %LOCALAPPDATA%/techo/journals\n\nInside: s schedule, t todo, f free memo, y year, g date, ? help.\nEditor: Ctrl+S saves, Esc cancels. Mouse works over SSH too.\nA words.txt in the journal directory replaces the daily words.\nFor existing checkout journals: techo --data-dir ./logs"
                );
                return Ok(());
            }
            "--version" | "-V" => {
                println!("techo {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--data-dir" => {
                dir = Some(
                    args.next()
                        .ok_or_else(|| io::Error::other("--data-dir requires a path"))?
                        .into(),
                )
            }
            "--date" => {
                date = args
                    .next()
                    .as_deref()
                    .and_then(calendar::parse_date)
                    .ok_or_else(|| io::Error::other("--date requires a valid YYYY-MM-DD"))?
            }
            "--no-mouse" => mouse = false,
            _ => {
                return Err(io::Error::other(format!(
                    "Unknown argument: {arg}; use --help"
                )));
            }
        }
    }
    if !io::stdin().is_terminal() || !stdout().is_terminal() {
        return Err(io::Error::other(
            "An interactive terminal is required; use --help for options",
        ));
    }
    let mut app = app::App::open(dir.map(Ok).unwrap_or_else(storage::default_dir)?, date)?;
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        hook(info);
    }));
    enable_raw_mode()?;
    let _screen = Screen;
    execute!(stdout(), EnterAlternateScreen, EnableBracketedPaste)?;
    if mouse {
        execute!(stdout(), EnableMouseCapture)?;
    }
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    while !app.quit {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;
        let wait = if app.turning() { 30 } else { 250 };
        if event::poll(Duration::from_millis(wait))? {
            app.event(event::read()?);
        }
    }
    Ok(())
}
