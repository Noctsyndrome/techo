use crate::{
    calendar::{parse_date, shift_days, shift_months},
    editor::TextEditor,
    journal::{Journal, ScheduleEntry, Task, clock_time, parse_time_input},
    schedule::Row,
    storage::Store,
};
use chrono::{Local, NaiveDate, Timelike};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::layout::Rect;
use std::{
    collections::HashSet,
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Schedule,
    Todo,
    Memo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditTarget {
    Schedule(Option<usize>),
    Task(Option<usize>),
    Memo,
    Jump,
}

pub struct Editing {
    pub target: EditTarget,
    pub text: TextEditor,
    pub time: TextEditor,
    pub time_active: bool,
    pub error: String,
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Focus(Focus),
    Select(Focus, usize),
    Calendar,
    Date(NaiveDate),
    Key(KeyCode),
    TimeField,
    BodyField,
    Save,
    Cancel,
}

/// How long a freshly opened page stays blank, like the beat of turning paper.
const PAGE_TURN: Duration = Duration::from_millis(45);
/// How long a message such as "Saved" stays in the footer before the hints return.
const STATUS_SHOWN: Duration = Duration::from_millis(1500);
/// Where a new item lands on a day that is not today.
const DEFAULT_TIME: u16 = 300;

pub struct App {
    pub store: Store,
    pub date: NaiveDate,
    pub journal: Journal,
    pub original: Option<String>,
    pub focus: Focus,
    pub selected_task: usize,
    pub task_offset: usize,
    pub schedule_row: usize,
    pub schedule_offset: usize,
    pub schedule_rows: Vec<Row>,
    /// Entry the schedule cursor should land on once the rows are laid out again.
    pub schedule_jump: Option<usize>,
    pub memo_scroll: usize,
    pub editing: Option<Editing>,
    pub calendar: Option<NaiveDate>,
    pub calendar_page_size: i32,
    pub hits: Vec<(Rect, Action)>,
    pub status: String,
    pub status_at: Option<Instant>,
    pub help: bool,
    /// First line of the key reference shown when it is taller than the terminal.
    pub help_scroll: usize,
    /// The terminal reports keys with the kitty protocol: modified Enter and, on a
    /// Mac, the Command key reach the app. Discovered at start, never assumed.
    pub enhanced: bool,
    pub delete_pending: bool,
    pub quit: bool,
    pub written: HashSet<NaiveDate>,
    pub words: Vec<String>,
    pub turned: Option<Instant>,
    /// Where the panels were drawn last, so a note can open right on the cursor.
    pub schedule_area: Option<Rect>,
    pub schedule_cursor_y: Option<u16>,
    pub todo_area: Option<Rect>,
    pub todo_cursor_y: Option<u16>,
    pub todo_next_y: Option<u16>,
    pub memo_area: Option<Rect>,
}

impl App {
    pub fn open(dir: PathBuf, date: NaiveDate) -> io::Result<Self> {
        let store = Store::open(dir)?;
        let (journal, original) = store.load(date)?;
        let written = store.written_dates();
        let words = store.words();
        Ok(Self {
            store,
            date,
            journal,
            original,
            focus: Focus::Schedule,
            selected_task: 0,
            task_offset: 0,
            schedule_row: 0,
            schedule_offset: 0,
            schedule_rows: Vec::new(),
            schedule_jump: None,
            memo_scroll: 0,
            editing: None,
            calendar: None,
            calendar_page_size: 1,
            hits: Vec::new(),
            status: String::new(),
            status_at: None,
            help: written.is_empty(),
            help_scroll: 0,
            enhanced: false,
            delete_pending: false,
            quit: false,
            written,
            words,
            turned: None,
            schedule_area: None,
            schedule_cursor_y: None,
            todo_area: None,
            todo_cursor_y: None,
            todo_next_y: None,
            memo_area: None,
        })
    }
    pub fn open_date(&mut self, date: NaiveDate) {
        if self.editing.is_some() {
            return;
        }
        match self.store.load(date) {
            Ok((journal, original)) => {
                self.date = date;
                self.journal = journal;
                self.original = original;
                self.calendar = None;
                self.selected_task = 0;
                self.task_offset = 0;
                self.schedule_row = 0;
                self.schedule_offset = 0;
                self.schedule_jump = None;
                self.memo_scroll = 0;
                self.status.clear();
                self.turned = Some(Instant::now());
            }
            Err(e) => self.say(format!("Cannot open {date}: {e}")),
        }
    }
    pub fn turning(&self) -> bool {
        self.turned.is_some_and(|t| t.elapsed() < PAGE_TURN)
    }
    /// Show a short message in the footer; it fades and the hints come back.
    pub fn say(&mut self, message: impl Into<String>) {
        self.status = message.into();
        self.status_at = Some(Instant::now());
    }
    pub fn status_line(&self) -> Option<&str> {
        (!self.status.is_empty() && self.status_at.is_some_and(|t| t.elapsed() < STATUS_SHOWN))
            .then_some(self.status.as_str())
    }
    pub fn cursor_entry(&self) -> Option<usize> {
        self.schedule_rows
            .get(self.schedule_row)
            .map(|row| row.entry)
            .filter(|i| *i < self.journal.schedule.len())
    }
    /// A new item takes the time of the item under the cursor, else now on today's
    /// page, else a morning hour.
    fn new_item_time(&self) -> u16 {
        if let Some(i) = self.cursor_entry() {
            return self.journal.schedule[i].offset_minutes;
        }
        let now = Local::now();
        if now.date_naive() == self.date {
            ((now.hour() * 60 + now.minute() + 1200) % 1440) as u16
        } else {
            DEFAULT_TIME
        }
    }
    fn persist(&mut self, journal: Journal) -> bool {
        match self.store.save(self.date, &journal, &self.original) {
            Ok(raw) => {
                self.journal = journal;
                self.original = Some(raw);
                self.written.insert(self.date);
                self.say("Saved");
                true
            }
            Err(e) => {
                self.say(format!("Save failed: {e}"));
                false
            }
        }
    }
    pub fn start_edit(&mut self, new: bool) {
        let (target, text, time) = match self.focus {
            Focus::Memo => (
                EditTarget::Memo,
                self.journal.free_memo.clone(),
                String::new(),
            ),
            Focus::Todo => {
                let index = (!new && self.selected_task < self.journal.tasks.len())
                    .then_some(self.selected_task);
                (
                    EditTarget::Task(index),
                    index
                        .map(|i| self.journal.tasks[i].text.clone())
                        .unwrap_or_default(),
                    String::new(),
                )
            }
            Focus::Schedule => {
                let index = if new { None } else { self.cursor_entry() };
                let time = index
                    .map(|i| self.journal.schedule[i].offset_minutes)
                    .unwrap_or_else(|| self.new_item_time());
                (
                    EditTarget::Schedule(index),
                    index
                        .map(|i| self.journal.schedule[i].text.clone())
                        .unwrap_or_default(),
                    clock_time(time),
                )
            }
        };
        self.editing = Some(Editing {
            target,
            text: TextEditor::new(text),
            time: TextEditor::new(time),
            time_active: false,
            error: String::new(),
        });
        self.status.clear();
    }
    fn start_jump(&mut self) {
        self.editing = Some(Editing {
            target: EditTarget::Jump,
            text: TextEditor::default(),
            time: TextEditor::default(),
            time_active: false,
            error: String::new(),
        });
    }
    pub fn commit(&mut self) {
        let Some(edit) = &self.editing else {
            return;
        };
        if edit.target == EditTarget::Jump {
            if let Some(date) = parse_date(&edit.text.text) {
                self.editing = None;
                self.open_date(date);
            } else {
                self.editing.as_mut().unwrap().error =
                    "Use a valid YYYY-MM-DD date (0001-9999)".into();
            }
            return;
        }
        let target = edit.target;
        let mut journal = self.journal.clone();
        if !matches!(target, EditTarget::Memo) && edit.text.text.trim().is_empty() {
            self.editing.as_mut().unwrap().error =
                "Nothing written yet; Esc leaves without adding it".into();
            return;
        }
        let mut jump = None;
        match target {
            EditTarget::Memo => journal.free_memo = edit.text.text.clone(),
            EditTarget::Task(index) => {
                if let Some(i) = index {
                    journal.tasks[i].text = edit.text.text.clone();
                } else {
                    journal.tasks.push(Task {
                        done: false,
                        text: edit.text.text.clone(),
                    });
                }
            }
            EditTarget::Schedule(index) => {
                let Some(time) = parse_time_input(&edit.time.text) else {
                    let edit = self.editing.as_mut().unwrap();
                    edit.error = "Time: 9, 930 or 09:30 (00:00-23:59)".into();
                    edit.time_active = true;
                    return;
                };
                let entry = ScheduleEntry {
                    offset_minutes: time,
                    text: edit.text.text.clone(),
                };
                // Remove and insert preserves independent entries with the same time.
                if let Some(i) = index {
                    journal.schedule.remove(i);
                }
                let i = journal
                    .schedule
                    .partition_point(|e| e.offset_minutes <= time);
                journal.schedule.insert(i, entry);
                jump = Some(i);
            }
            EditTarget::Jump => unreachable!(),
        }
        if self.persist(journal) {
            if target == EditTarget::Task(None) {
                self.selected_task = self.journal.tasks.len() - 1;
            }
            if jump.is_some() {
                self.schedule_jump = jump;
            }
            self.editing = None;
        } else {
            self.editing.as_mut().unwrap().error = self.status.clone();
        }
    }
    pub fn move_selection(&mut self, delta: isize) {
        match self.focus {
            Focus::Todo => {
                self.selected_task = self
                    .selected_task
                    .saturating_add_signed(delta)
                    .min(self.journal.tasks.len().saturating_sub(1))
            }
            Focus::Schedule => {
                // Move by item, landing on its first line.
                let count = self.journal.schedule.len();
                let target = self
                    .cursor_entry()
                    .unwrap_or(0)
                    .saturating_add_signed(delta)
                    .min(count.saturating_sub(1));
                self.schedule_row = self
                    .schedule_rows
                    .iter()
                    .position(|row| row.entry == target)
                    .unwrap_or(0);
            }
            Focus::Memo => self.memo_scroll = self.memo_scroll.saturating_add_signed(delta),
        }
    }
    /// On the last item of the schedule or todo, or on an empty one.
    fn at_end(&self) -> bool {
        match self.focus {
            Focus::Schedule => self
                .cursor_entry()
                .is_none_or(|i| i + 1 >= self.journal.schedule.len()),
            Focus::Todo => self.selected_task + 1 >= self.journal.tasks.len(),
            Focus::Memo => false,
        }
    }
    /// Going down past the end starts a new item; on today's page it takes the time now.
    fn append(&mut self) {
        self.start_edit(true);
        let now = Local::now();
        if now.date_naive() == self.date
            && let Some(edit) = &mut self.editing
            && matches!(edit.target, EditTarget::Schedule(None))
        {
            edit.time = TextEditor::new(clock_time(
                ((now.hour() * 60 + now.minute() + 1200) % 1440) as u16,
            ));
        }
    }
    fn can_delete(&self) -> bool {
        match self.focus {
            Focus::Todo => !self.journal.tasks.is_empty(),
            Focus::Schedule => self.cursor_entry().is_some(),
            Focus::Memo => false,
        }
    }
    fn delete(&mut self) {
        let mut journal = self.journal.clone();
        match self.focus {
            Focus::Todo if !journal.tasks.is_empty() => {
                journal.tasks.remove(self.selected_task);
            }
            Focus::Schedule => match self.cursor_entry() {
                Some(i) => {
                    journal.schedule.remove(i);
                    self.schedule_jump = Some(i.min(journal.schedule.len().saturating_sub(1)));
                }
                None => return,
            },
            _ => return,
        }
        if self.persist(journal) {
            self.move_selection(0);
        }
    }
    pub fn action(&mut self, action: Action) {
        if self.editing.is_some() {
            match action {
                Action::Save => self.commit(),
                Action::Cancel => self.editing = None,
                Action::TimeField => self.editing.as_mut().unwrap().time_active = true,
                Action::BodyField => self.editing.as_mut().unwrap().time_active = false,
                _ => {}
            }
            return;
        }
        match action {
            Action::Focus(focus) => self.focus = focus,
            Action::Select(focus, i) => {
                self.focus = focus;
                match focus {
                    Focus::Todo => self.selected_task = i,
                    Focus::Schedule => self.schedule_row = i,
                    _ => {}
                }
            }
            Action::Calendar => {
                self.calendar = Some(self.date);
                self.status.clear();
            }
            Action::Date(date) => self.open_date(date),
            Action::Key(code) => self.key(KeyEvent::new(code, KeyModifiers::NONE)),
            _ => {}
        }
    }
    pub fn event(&mut self, event: Event) {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => self.key(key),
            Event::Paste(text) => {
                if let Some(edit) = &mut self.editing {
                    if edit.time_active {
                        edit.time.insert(&text.replace(['\r', '\n'], ""));
                    } else if edit.target == EditTarget::Jump {
                        edit.text.insert(&text.replace(['\r', '\n'], ""));
                    } else {
                        edit.text.insert(&text);
                    }
                }
            }
            Event::Mouse(mouse) => {
                let action = self
                    .hits
                    .iter()
                    .rev()
                    .find(|(rect, _)| rect.contains((mouse.column, mouse.row).into()))
                    .map(|(_, a)| *a);
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        if let Some(action) = action {
                            self.action(action);
                        }
                    }
                    MouseEventKind::ScrollDown if self.help => self.scroll_help(1),
                    MouseEventKind::ScrollUp if self.help => self.scroll_help(-1),
                    MouseEventKind::ScrollDown | MouseEventKind::ScrollUp
                        if self.editing.is_none() && !self.help && !self.delete_pending =>
                    {
                        let delta = if mouse.kind == MouseEventKind::ScrollDown {
                            1
                        } else {
                            -1
                        };
                        if let Some(date) = self.calendar {
                            self.calendar =
                                Some(shift_months(date, delta * self.calendar_page_size));
                        } else if let Some(Action::Focus(f) | Action::Select(f, _)) = action {
                            self.focus = f;
                            self.move_selection(delta as isize * 3);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    /// Whether ⌘S is worth mentioning: only on a Mac, and only once the terminal
    /// has agreed to report the Command key.
    pub fn command_saves(&self) -> bool {
        cfg!(target_os = "macos") && self.enhanced
    }
    /// The reference is drawn clamped, so scrolling only moves the wish; the page
    /// settles it against the terminal's height.
    pub fn scroll_help(&mut self, delta: isize) {
        self.help_scroll = self.help_scroll.saturating_add_signed(delta);
    }
    pub fn key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        // Super is the Command key; it only arrives on a Mac terminal that speaks
        // the kitty protocol, so it is an extra way to save, never the only one.
        let sup = key.modifiers.contains(KeyModifiers::SUPER);
        if self.editing.is_some() {
            match key.code {
                KeyCode::Esc => self.action(Action::Cancel),
                KeyCode::Char('s') if ctrl || sup => self.commit(),
                KeyCode::Enter if ctrl => self.commit(),
                KeyCode::Enter if self.editing.as_ref().unwrap().target == EditTarget::Jump => {
                    self.commit()
                }
                KeyCode::Tab | KeyCode::BackTab
                    if matches!(
                        self.editing.as_ref().unwrap().target,
                        EditTarget::Schedule(_)
                    ) =>
                {
                    let e = self.editing.as_mut().unwrap();
                    e.time_active = !e.time_active;
                }
                KeyCode::Enter if self.editing.as_ref().unwrap().time_active => {
                    self.editing.as_mut().unwrap().time_active = false
                }
                _ => {
                    let e = self.editing.as_mut().unwrap();
                    if e.time_active {
                        e.time.key(key, false);
                    } else {
                        e.text.key(key, e.target != EditTarget::Jump);
                    }
                }
            }
            return;
        }
        if self.help {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => self.scroll_help(1),
                KeyCode::Up | KeyCode::Char('k') => self.scroll_help(-1),
                KeyCode::PageDown => self.scroll_help(10),
                KeyCode::PageUp => self.scroll_help(-10),
                _ => {
                    self.help = false;
                    self.help_scroll = 0;
                }
            }
            return;
        }
        if self.delete_pending {
            self.delete_pending = false;
            if key.code == KeyCode::Char('y') {
                self.delete();
            }
            return;
        }
        if let Some(date) = self.calendar {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.calendar = None;
                    self.status.clear();
                }
                KeyCode::Enter => self.open_date(date),
                KeyCode::Left => self.calendar = Some(shift_days(date, -1)),
                KeyCode::Right => self.calendar = Some(shift_days(date, 1)),
                KeyCode::Up => self.calendar = Some(shift_days(date, -7)),
                KeyCode::Down => self.calendar = Some(shift_days(date, 7)),
                // `,` and `.` page like PgUp/PgDn for keyboards without those keys.
                KeyCode::PageUp | KeyCode::Char(',') => {
                    self.calendar = Some(shift_months(date, -self.calendar_page_size))
                }
                KeyCode::PageDown | KeyCode::Char('.') => {
                    self.calendar = Some(shift_months(date, self.calendar_page_size))
                }
                KeyCode::Char('[') => self.calendar = Some(shift_months(date, -12)),
                KeyCode::Char(']') => self.calendar = Some(shift_months(date, 12)),
                KeyCode::Char('t' | 'T') | KeyCode::Home => {
                    self.calendar = Some(Local::now().date_naive())
                }
                KeyCode::Char('g') => self.start_jump(),
                KeyCode::Char('?') | KeyCode::F(1) => self.help = true,
                _ => {}
            }
            return;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('c') if ctrl => self.quit = true,
            KeyCode::Char('s') if !ctrl => self.focus = Focus::Schedule,
            KeyCode::Char('t') => self.focus = Focus::Todo,
            KeyCode::Char('f') => self.focus = Focus::Memo,
            KeyCode::Tab | KeyCode::Right => {
                self.focus = match self.focus {
                    Focus::Schedule => Focus::Todo,
                    Focus::Todo => Focus::Memo,
                    Focus::Memo => Focus::Schedule,
                }
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.focus = match self.focus {
                    Focus::Schedule => Focus::Memo,
                    Focus::Todo => Focus::Schedule,
                    Focus::Memo => Focus::Todo,
                }
            }
            KeyCode::Char('n') => self.start_edit(true),
            KeyCode::Char('e') | KeyCode::Enter => self.start_edit(false),
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') if self.at_end() => self.append(),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            // `J` and `K` leap like PgDn/PgUp for keyboards without those keys.
            KeyCode::PageUp | KeyCode::Char('K') => self.move_selection(-10),
            KeyCode::PageDown | KeyCode::Char('J') => self.move_selection(10),
            KeyCode::Char(' ') if self.focus == Focus::Todo && !self.journal.tasks.is_empty() => {
                let mut journal = self.journal.clone();
                journal.tasks[self.selected_task].done = !journal.tasks[self.selected_task].done;
                self.persist(journal);
            }
            KeyCode::Char('d') | KeyCode::Delete if self.can_delete() => self.delete_pending = true,
            KeyCode::Char('y') => self.calendar = Some(self.date),
            KeyCode::Char('g') => self.start_jump(),
            KeyCode::Char('[') => self.open_date(shift_days(self.date, -1)),
            KeyCode::Char(']') => self.open_date(shift_days(self.date, 1)),
            // `T` opens today, as `t` does in the year; lower-case t is the todo panel.
            KeyCode::Home | KeyCode::Char('T') => self.open_date(Local::now().date_naive()),
            KeyCode::Char('?') | KeyCode::F(1) => self.help = true,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_dir;
    fn app() -> App {
        let mut app = App::open(test_dir(), parse_date("2026-09-11").unwrap()).unwrap();
        app.help = false;
        app
    }
    fn cleanup(app: App) {
        let dir = app.store.dir.clone();
        drop(app);
        std::fs::remove_dir_all(dir).unwrap();
    }
    fn times(app: &App) -> Vec<String> {
        app.journal
            .schedule
            .iter()
            .map(|e| clock_time(e.offset_minutes))
            .collect()
    }
    fn write_item(a: &mut App, time: &str, text: &str) {
        a.focus = Focus::Schedule;
        a.start_edit(true);
        let e = a.editing.as_mut().unwrap();
        e.time = TextEditor::new(time.into());
        e.text.insert(text);
        a.commit();
        assert!(a.editing.is_none());
        a.schedule_rows = crate::schedule::rows(&a.journal, 40);
    }
    #[test]
    fn first_launch_shows_help_once() {
        let a = App::open(test_dir(), parse_date("2026-09-11").unwrap()).unwrap();
        assert!(a.help);
        assert_eq!(a.focus, Focus::Schedule);
        cleanup(a);
    }
    #[test]
    fn the_command_key_saves_when_the_terminal_reports_it() {
        let mut a = app();
        a.focus = Focus::Todo;
        a.start_edit(true);
        a.editing.as_mut().unwrap().text.insert("call");
        a.key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::SUPER));
        assert!(a.editing.is_none());
        assert_eq!(a.journal.tasks[0].text, "call");
        // Plain s while writing is a letter, and Ctrl+S still saves.
        a.start_edit(true);
        a.key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert_eq!(a.editing.as_ref().unwrap().text.text, "s");
        a.key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert!(a.editing.is_none());
        assert!(!a.command_saves() || cfg!(target_os = "macos"));
        cleanup(a);
    }
    #[test]
    fn letters_stand_in_for_home_and_the_page_keys() {
        let mut a = app();
        for i in 0..15 {
            a.journal.tasks.push(Task {
                done: false,
                text: format!("task {i}"),
            });
        }
        a.focus = Focus::Todo;
        a.key(KeyEvent::new(KeyCode::Char('J'), KeyModifiers::SHIFT));
        assert_eq!(a.selected_task, 10);
        a.key(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT));
        assert_eq!(a.selected_task, 0);
        // Lower-case t is still the todo panel; T is today.
        a.open_date(parse_date("2024-02-29").unwrap());
        a.key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
        assert_eq!(a.date, Local::now().date_naive());
        // In the year, `,` and `.` page the months like PgUp and PgDn.
        a.calendar = Some(parse_date("2026-09-11").unwrap());
        a.calendar_page_size = 4;
        a.key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::NONE));
        assert_eq!(a.calendar, parse_date("2027-01-11"));
        a.key(KeyEvent::new(KeyCode::Char(','), KeyModifiers::NONE));
        assert_eq!(a.calendar, parse_date("2026-09-11"));
        a.key(KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT));
        assert_eq!(a.calendar, Some(Local::now().date_naive()));
        cleanup(a);
    }
    #[test]
    fn the_key_reference_scrolls_and_closes() {
        let mut a = app();
        a.key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert!(a.help);
        a.key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        a.key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        a.key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
        assert!(a.help);
        assert_eq!(a.help_scroll, 12);
        a.key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(a.help_scroll, 11);
        a.key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(!a.help);
        assert_eq!(a.help_scroll, 0);
        // Whatever closed it did nothing else: focus and page unchanged.
        assert_eq!(a.focus, Focus::Schedule);
        cleanup(a);
    }
    #[test]
    fn messages_fade_from_the_footer() {
        let mut a = app();
        assert_eq!(a.status_line(), None);
        a.say("Saved");
        assert_eq!(a.status_line(), Some("Saved"));
        a.status_at = Some(Instant::now() - STATUS_SHOWN);
        assert_eq!(a.status_line(), None);
        cleanup(a);
    }
    #[test]
    fn cancelled_new_items_never_touch_model_or_disk() {
        let mut a = app();
        for focus in [Focus::Todo, Focus::Schedule] {
            a.focus = focus;
            a.start_edit(true);
            a.action(Action::Cancel);
        }
        assert_eq!(a.journal, Journal::default());
        assert!(!a.store.path(a.date).exists());
        cleanup(a);
    }
    #[test]
    fn items_sort_gather_by_time_and_move_by_item() {
        let mut a = app();
        write_item(&mut a, "13:00", "work\nnotes");
        assert_eq!(a.schedule_jump, Some(0));
        write_item(&mut a, "9", "brief");
        write_item(&mut a, "0030", "late");
        assert_eq!(times(&a), ["09:00", "13:00", "00:30"]);
        // A new item next to an existing one takes its time.
        a.schedule_row = a.schedule_rows.iter().position(|r| r.entry == 1).unwrap();
        a.start_edit(true);
        assert_eq!(a.editing.as_ref().unwrap().time.text, "13:00");
        a.editing.as_mut().unwrap().text.insert("same hour");
        a.commit();
        a.schedule_rows = crate::schedule::rows(&a.journal, 40);
        assert_eq!(times(&a), ["09:00", "13:00", "13:00", "00:30"]);
        assert_eq!(a.schedule_jump, Some(2));
        // Up/Down step over an item's lines.
        a.schedule_row = a.schedule_rows.iter().position(|r| r.entry == 1).unwrap();
        a.move_selection(1);
        assert_eq!(a.cursor_entry(), Some(2));
        a.move_selection(-1);
        assert_eq!(a.cursor_entry(), Some(1));
        assert!(a.schedule_rows[a.schedule_row].first);
        // Enter on an item edits it; changing the time re-sorts.
        a.start_edit(false);
        assert_eq!(
            a.editing.as_ref().unwrap().target,
            EditTarget::Schedule(Some(1))
        );
        a.editing.as_mut().unwrap().time = TextEditor::new("15:00".into());
        a.commit();
        assert_eq!(times(&a), ["09:00", "13:00", "15:00", "00:30"]);
        assert_eq!(a.schedule_jump, Some(2));
        // A time that cannot be read keeps the note open.
        a.start_edit(true);
        let e = a.editing.as_mut().unwrap();
        e.time = TextEditor::new("25".into());
        e.text.insert("item");
        a.commit();
        assert!(a.editing.is_some());
        assert!(a.editing.as_ref().unwrap().time_active);
        assert_eq!(a.journal.schedule.len(), 4);
        cleanup(a);
    }
    #[test]
    fn down_past_the_end_starts_a_new_item() {
        // On today's page, whatever day that is: the new item takes the time now.
        let mut a = App::open(test_dir(), Local::now().date_naive()).unwrap();
        a.help = false;
        let down = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        a.focus = Focus::Todo;
        a.key(down);
        assert_eq!(a.editing.as_ref().unwrap().target, EditTarget::Task(None));
        a.action(Action::Cancel);
        write_item(&mut a, "09:00", "brief");
        write_item(&mut a, "13:00", "lunch");
        a.schedule_row = 0;
        a.key(down);
        assert!(a.editing.is_none());
        assert_eq!(a.cursor_entry(), Some(1));
        let clock = |t: chrono::DateTime<Local>| {
            clock_time(((t.hour() * 60 + t.minute() + 1200) % 1440) as u16)
        };
        let before = clock(Local::now());
        a.key(down);
        let after = clock(Local::now());
        let e = a.editing.as_ref().unwrap();
        assert_eq!(e.target, EditTarget::Schedule(None));
        assert!([before, after].contains(&e.time.text), "{}", e.time.text);
        assert!(!e.time_active);
        a.action(Action::Cancel);
        // The wheel never opens a note.
        a.move_selection(3);
        assert!(a.editing.is_none());
        cleanup(a);
    }
    #[test]
    fn new_items_on_other_days_start_in_the_morning() {
        let mut a = app();
        a.open_date(parse_date("2000-01-06").unwrap());
        a.focus = Focus::Schedule;
        a.start_edit(true);
        let e = a.editing.as_ref().unwrap();
        assert_eq!(e.target, EditTarget::Schedule(None));
        assert_eq!(e.time.text, "09:00");
        assert!(!e.time_active);
        cleanup(a);
    }
    #[test]
    fn delete_only_applies_to_an_item_under_the_cursor() {
        let mut a = app();
        a.focus = Focus::Schedule;
        a.key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(!a.delete_pending);
        write_item(&mut a, "10:00", "gone soon");
        a.schedule_row = 0;
        a.key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert!(a.delete_pending);
        a.key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert!(a.journal.schedule.is_empty());
        assert!(a.written.contains(&a.date));
        cleanup(a);
    }
    #[test]
    fn failed_save_retains_draft_and_last_saved_model() {
        let mut a = app();
        a.focus = Focus::Memo;
        a.start_edit(false);
        a.editing.as_mut().unwrap().text.insert("precious draft");
        std::fs::write(a.store.path(a.date), "external change").unwrap();
        a.commit();
        assert_eq!(a.editing.as_ref().unwrap().text.text, "precious draft");
        assert_eq!(a.journal.free_memo, "");
        assert_eq!(
            std::fs::read_to_string(a.store.path(a.date)).unwrap(),
            "external change"
        );
        cleanup(a);
    }

    #[test]
    fn io_failure_can_be_retried_without_losing_draft() {
        let mut a = app();
        a.focus = Focus::Memo;
        a.start_edit(false);
        a.editing.as_mut().unwrap().text.insert("retry this draft");
        let temporary = a
            .store
            .path(a.date)
            .with_extension(format!("md.{}.tmp", std::process::id()));
        std::fs::create_dir(&temporary).unwrap();
        a.commit();
        assert!(a.editing.is_some());
        assert!(!a.store.path(a.date).exists());
        std::fs::remove_dir(&temporary).unwrap();
        a.commit();
        assert!(a.editing.is_none());
        assert_eq!(
            a.store.load(a.date).unwrap().0.free_memo,
            "retry this draft"
        );
        cleanup(a);
    }
    #[test]
    fn dates_are_isolated_and_editing_blocks_navigation() {
        let mut a = app();
        a.focus = Focus::Memo;
        let first = a.date;
        let other = parse_date("2027-02-01").unwrap();
        a.start_edit(false);
        a.editing
            .as_mut()
            .unwrap()
            .text
            .insert("中文日志\n## TODO\nline two");
        a.open_date(other);
        assert_eq!(a.date, first);
        a.commit();
        a.open_date(other);
        assert!(a.turning());
        assert!(a.journal.free_memo.is_empty());
        assert!(!a.store.path(other).exists());
        a.open_date(first);
        assert_eq!(a.journal.free_memo, "中文日志\n## TODO\nline two");
        cleanup(a);
    }
}
