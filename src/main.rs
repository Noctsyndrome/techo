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
        Event, KeyCode, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
        supports_keyboard_enhancement,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io::{self, IsTerminal, Write, stdout},
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

/// Whether the kitty keyboard protocol was pushed, so that it is popped again
/// on every way out, including a panic.
static ENHANCED: AtomicBool = AtomicBool::new(false);

struct Screen;
impl Drop for Screen {
    fn drop(&mut self) {
        restore();
    }
}
fn restore() {
    if ENHANCED.swap(false, Ordering::SeqCst) {
        let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
    }
    let _ = disable_raw_mode();
    let _ = execute!(
        stdout(),
        DisableMouseCapture,
        DisableBracketedPaste,
        LeaveAlternateScreen,
        crossterm::cursor::Show
    );
}

/// Ask the terminal to report keys unambiguously (the kitty keyboard protocol):
/// then Ctrl+Enter differs from Enter and a Mac's Command key arrives as Super.
/// Terminals that do not answer are left as they are; every key that matters
/// works without this. Call in raw mode, on the screen the app will use.
fn enhance_keyboard() -> bool {
    let supported = supports_keyboard_enhancement().unwrap_or(false);
    let pushed = supported
        && execute!(
            stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )
        .is_ok();
    ENHANCED.store(pushed, Ordering::SeqCst);
    pushed
}

/// `techo --keys`: print what the terminal delivers for each key, so a binding
/// that does not work can be traced to the terminal or to techo.
fn show_keys() -> io::Result<()> {
    struct Raw;
    impl Drop for Raw {
        fn drop(&mut self) {
            if ENHANCED.swap(false, Ordering::SeqCst) {
                let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
            }
            let _ = disable_raw_mode();
        }
    }
    enable_raw_mode()?;
    let _raw = Raw;
    let mut out = stdout();
    let protocol = if enhance_keyboard() {
        "reports keys with the kitty keyboard protocol"
    } else {
        "reports keys the traditional way (no kitty keyboard protocol)"
    };
    write!(
        out,
        "techō key check: this terminal {protocol}.\r\n\
         Press keys to see what arrives; Ctrl+C ends.\r\n"
    )?;
    out.flush()?;
    loop {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            let modifiers = if key.modifiers.is_empty() {
                "-".to_string()
            } else {
                key.modifiers.to_string()
            };
            write!(out, "{:<24} {modifiers}\r\n", format!("{:?}", key.code))?;
            out.flush()?;
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Ok(());
            }
        }
    }
}
fn main() -> io::Result<()> {
    let mut dir: Option<PathBuf> = None;
    let mut date = Local::now().date_naive();
    let mut mouse = true;
    let mut keys = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "techō — a little space for your day\n\nUsage: techo [--data-dir PATH] [--date YYYY-MM-DD] [--no-mouse]\n\n--data-dir PATH   Journal directory (overrides TECHO_DIR)\n--date DATE       Open a date instead of today\n--no-mouse        Keep terminal mouse selection behavior\n--keys            Show what the terminal sends for each key, then exit\n--version        Show version\n\nDefault files: $XDG_DATA_HOME/techo/journals or ~/.local/share/techo/journals\nWindows: %LOCALAPPDATA%/techo/journals\n\nInside: s schedule, t todo, f free memo, y year, g date, ? keys.\nNote: Ctrl+S saves, Esc cancels. Mouse works over SSH too.\nA words.txt in the journal directory replaces the daily words.\nFor existing checkout journals: techo --data-dir ./logs"
                );
                return Ok(());
            }
            "--keys" => keys = true,
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
    if keys {
        return show_keys();
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
    // The protocol stack belongs to the alternate screen, so ask once it is up.
    app.enhanced = enhance_keyboard();
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
