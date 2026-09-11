use crate::{
    calendar::{parse_date, shift_days, shift_months},
    editor::TextEditor,
    journal::{Journal, ScheduleEntry, Task, clock_time, parse_time},
    storage::Store,
};
use chrono::{Local, NaiveDate};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind,
};
use ratatui::layout::Rect;
use std::{io, path::PathBuf};

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

pub struct App {
    pub store: Store,
    pub date: NaiveDate,
    pub journal: Journal,
    pub original: Option<String>,
    pub focus: Focus,
    pub selected_task: usize,
    pub selected_schedule: usize,
    pub task_offset: usize,
    pub schedule_offset: usize,
    pub memo_scroll: usize,
    pub editing: Option<Editing>,
    pub calendar: Option<NaiveDate>,
    pub calendar_page_size: i32,
    pub hits: Vec<(Rect, Action)>,
    pub status: String,
    pub help: bool,
    pub delete_pending: bool,
    pub quit: bool,
}

impl App {
    pub fn open(dir: PathBuf, date: NaiveDate) -> io::Result<Self> {
        let store = Store::open(dir)?;
        let (journal, original) = store.load(date)?;
        Ok(Self {
            store,
            date,
            journal,
            original,
            focus: Focus::Memo,
            selected_task: 0,
            selected_schedule: 0,
            task_offset: 0,
            schedule_offset: 0,
            memo_scroll: 0,
            editing: None,
            calendar: None,
            calendar_page_size: 1,
            hits: Vec::new(),
            status: String::new(),
            help: false,
            delete_pending: false,
            quit: false,
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
                self.selected_schedule = 0;
                self.task_offset = 0;
                self.schedule_offset = 0;
                self.memo_scroll = 0;
                self.status.clear();
            }
            Err(e) => self.status = format!("Cannot open {date}: {e}"),
        }
    }
    fn persist(&mut self, journal: Journal) -> bool {
        match self.store.save(self.date, &journal, &self.original) {
            Ok(raw) => {
                self.journal = journal;
                self.original = Some(raw);
                self.status = "Saved".into();
                true
            }
            Err(e) => {
                self.status = format!("Save failed: {e}");
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
                let index = (!new && self.selected_schedule < self.journal.schedule.len())
                    .then_some(self.selected_schedule);
                (
                    EditTarget::Schedule(index),
                    index
                        .map(|i| self.journal.schedule[i].text.clone())
                        .unwrap_or_default(),
                    index
                        .map(|i| clock_time(self.journal.schedule[i].offset_minutes))
                        .unwrap_or_default(),
                )
            }
        };
        self.editing = Some(Editing {
            target,
            text: TextEditor::new(text),
            time: TextEditor::new(time),
            time_active: matches!(target, EditTarget::Schedule(_)),
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
                "Please enter an item; Esc cancels without adding it".into();
            return;
        }
        let mut schedule_selection = None;
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
                let Some(time) = parse_time(&edit.time.text) else {
                    let edit = self.editing.as_mut().unwrap();
                    edit.error = "Time must be HH:MM (00:00-23:59)".into();
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
                schedule_selection = Some(i);
            }
            EditTarget::Jump => unreachable!(),
        }
        if self.persist(journal) {
            if target == EditTarget::Task(None) {
                self.selected_task = self.journal.tasks.len() - 1;
            }
            if let Some(i) = schedule_selection {
                self.selected_schedule = i;
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
                self.selected_schedule = self
                    .selected_schedule
                    .saturating_add_signed(delta)
                    .min(self.journal.schedule.len().saturating_sub(1))
            }
            Focus::Memo => self.memo_scroll = self.memo_scroll.saturating_add_signed(delta),
        }
    }
    fn delete(&mut self) {
        let mut journal = self.journal.clone();
        match self.focus {
            Focus::Todo if !journal.tasks.is_empty() => {
                journal.tasks.remove(self.selected_task);
            }
            Focus::Schedule if !journal.schedule.is_empty() => {
                journal.schedule.remove(self.selected_schedule);
            }
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
                Action::Cancel => {
                    self.editing = None;
                    self.status = "Cancelled; no changes saved".into();
                }
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
                    Focus::Schedule => self.selected_schedule = i,
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
    pub fn key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if self.editing.is_some() {
            match key.code {
                KeyCode::Esc => self.action(Action::Cancel),
                KeyCode::Char('s') if ctrl => self.commit(),
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
            self.help = false;
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
                KeyCode::PageUp => {
                    self.calendar = Some(shift_months(date, -self.calendar_page_size))
                }
                KeyCode::PageDown => {
                    self.calendar = Some(shift_months(date, self.calendar_page_size))
                }
                KeyCode::Char('[') => self.calendar = Some(shift_months(date, -12)),
                KeyCode::Char(']') => self.calendar = Some(shift_months(date, 12)),
                KeyCode::Char('t') => self.calendar = Some(Local::now().date_naive()),
                KeyCode::Char('g') => self.start_jump(),
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
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::PageUp => self.move_selection(-10),
            KeyCode::PageDown => self.move_selection(10),
            KeyCode::Char(' ') if self.focus == Focus::Todo && !self.journal.tasks.is_empty() => {
                let mut journal = self.journal.clone();
                journal.tasks[self.selected_task].done = !journal.tasks[self.selected_task].done;
                self.persist(journal);
            }
            KeyCode::Char('d') | KeyCode::Delete if self.focus != Focus::Memo => {
                self.delete_pending = true
            }
            KeyCode::Char('y') => self.calendar = Some(self.date),
            KeyCode::Char('g') => self.start_jump(),
            KeyCode::Char('[') => self.open_date(shift_days(self.date, -1)),
            KeyCode::Char(']') => self.open_date(shift_days(self.date, 1)),
            KeyCode::Home => self.open_date(Local::now().date_naive()),
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
        App::open(test_dir(), parse_date("2026-09-11").unwrap()).unwrap()
    }
    fn cleanup(app: App) {
        let dir = app.store.dir.clone();
        drop(app);
        std::fs::remove_dir_all(dir).unwrap();
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
    fn schedule_requires_time_sorts_and_edits_time() {
        let mut a = app();
        a.focus = Focus::Schedule;
        for time in ["13:00", "09:00", "00:30"] {
            a.start_edit(true);
            let e = a.editing.as_mut().unwrap();
            e.time.insert(time);
            e.text.insert("work\nnotes");
            a.commit();
            assert!(a.editing.is_none());
        }
        assert_eq!(
            a.journal
                .schedule
                .iter()
                .map(|e| clock_time(e.offset_minutes))
                .collect::<Vec<_>>(),
            ["09:00", "13:00", "00:30"]
        );
        a.selected_schedule = 0;
        a.start_edit(false);
        a.editing.as_mut().unwrap().time = TextEditor::new("15:00".into());
        a.commit();
        assert_eq!(a.selected_schedule, 1);
        a.start_edit(true);
        a.editing.as_mut().unwrap().text.insert("item");
        a.commit();
        assert!(a.editing.is_some());
        assert_eq!(a.journal.schedule.len(), 3);
        cleanup(a);
    }
    #[test]
    fn failed_save_retains_draft_and_last_saved_model() {
        let mut a = app();
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
        assert!(a.journal.free_memo.is_empty());
        assert!(!a.store.path(other).exists());
        a.open_date(first);
        assert_eq!(a.journal.free_memo, "中文日志\n## TODO\nline two");
        cleanup(a);
    }
}
