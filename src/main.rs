use std::{
    fs,
    io::{self, stdout},
    path::PathBuf,
    time::Duration,
};

use chrono::{Datelike, Local, NaiveDate, Weekday};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

const QUOTES: [&str; 4] = [
    "The secret of getting ahead is getting started. — Mark Twain",
    "A day is a little life. — Arthur Schopenhauer",
    "What you do every day matters more than what you do once in a while. — Gretchen Rubin",
    "The future depends on what you do today. — Mahatma Gandhi",
];

#[derive(Clone, Debug)]
struct Task {
    done: bool,
    text: String,
}

#[derive(Clone, Debug)]
struct ScheduleEntry {
    /// Minutes after the paper day starts at 04:00, in the range 0..1440.
    offset_minutes: u16,
    text: String,
}

#[derive(Clone, Debug)]
struct Journal {
    tasks: Vec<Task>,
    schedule: Vec<ScheduleEntry>,
    free_memo: String,
}

fn schedule_offset(hour: u16, minute: u16) -> Option<u16> {
    if hour > 23 || minute > 59 {
        return None;
    }
    let clock_minutes = hour * 60 + minute;
    Some(if clock_minutes >= 4 * 60 {
        clock_minutes - 4 * 60
    } else {
        clock_minutes + 20 * 60
    })
}

fn parse_schedule_time(value: &str) -> Option<u16> {
    let time = value.split_whitespace().next()?;
    let (hour, minute) = time.split_once(':')?;
    schedule_offset(hour.parse().ok()?, minute.parse().ok()?)
}

fn format_schedule_time(offset_minutes: u16) -> String {
    let clock_minutes = (offset_minutes + 4 * 60) % (24 * 60);
    let suffix = if offset_minutes + 4 * 60 >= 24 * 60 {
        " (+1)"
    } else {
        ""
    };
    format!(
        "{:02}:{:02}{suffix}",
        clock_minutes / 60,
        clock_minutes % 60
    )
}

impl Journal {
    fn blank() -> Self {
        Self {
            tasks: Vec::new(),
            schedule: Vec::new(),
            free_memo: String::new(),
        }
    }

    fn append_schedule_line(&mut self, offset_minutes: u16, line: &str) {
        if let Some(entry) = self
            .schedule
            .iter_mut()
            .find(|entry| entry.offset_minutes == offset_minutes)
        {
            if !entry.text.is_empty() {
                entry.text.push('\n');
            }
            entry.text.push_str(line);
        } else {
            self.schedule.push(ScheduleEntry {
                offset_minutes,
                text: line.to_string(),
            });
        }
    }

    fn from_markdown(markdown: &str) -> Self {
        enum Section {
            Other,
            Todo,
            Schedule,
            LegacyMemo,
            LegacyTimeline,
            FreeMemo,
        }

        let mut journal = Self::blank();
        let mut section = Section::Other;
        let mut schedule_time = None;

        for line in markdown.lines() {
            match line {
                "## TODO" => {
                    section = Section::Todo;
                }
                "## Schedule" => {
                    section = Section::Schedule;
                    schedule_time = None;
                }
                "## Timeline Memo" => {
                    section = Section::LegacyMemo;
                }
                "## Timeline" => {
                    section = Section::LegacyTimeline;
                    schedule_time = None;
                }
                "## Free Memo" => {
                    section = Section::FreeMemo;
                }
                _ => match section {
                    Section::Todo => {
                        if let Some(text) = line.strip_prefix("- [ ] ") {
                            journal.tasks.push(Task {
                                done: false,
                                text: text.to_string(),
                            });
                        } else if let Some(text) = line.strip_prefix("- [x] ") {
                            journal.tasks.push(Task {
                                done: true,
                                text: text.to_string(),
                            });
                        }
                    }
                    Section::Schedule | Section::LegacyTimeline => {
                        if let Some(time) = line.strip_prefix("### ") {
                            schedule_time = parse_schedule_time(time);
                        } else if !line.trim().is_empty() {
                            journal.append_schedule_line(schedule_time.unwrap_or(120), line);
                        }
                    }
                    Section::FreeMemo => {
                        if !journal.free_memo.is_empty() {
                            journal.free_memo.push('\n');
                        }
                        journal.free_memo.push_str(line);
                    }
                    Section::LegacyMemo => {
                        if !line.trim().is_empty() {
                            let (offset_minutes, content) = line
                                .split_once("  ")
                                .and_then(|(time, text)| {
                                    parse_schedule_time(time).map(|offset| (offset, text))
                                })
                                .unwrap_or((120, line));
                            journal.append_schedule_line(offset_minutes, content);
                        }
                    }
                    Section::Other => {}
                },
            }
        }
        journal.free_memo = journal.free_memo.trim_end_matches('\n').to_string();
        journal.schedule.sort_by_key(|entry| entry.offset_minutes);
        journal
    }

    fn to_markdown(&self, date: NaiveDate) -> String {
        let mut output = format!(
            "---\ndate: {}\ntags:\n  - techo\n---\n\n# {}\n\n## TODO\n",
            date, date
        );
        for task in &self.tasks {
            let marker = if task.done { "x" } else { " " };
            output.push_str(&format!("- [{marker}] {}\n", task.text));
        }
        output.push_str("\n## Schedule\n");
        for entry in &self.schedule {
            output.push_str(&format!(
                "\n### {}\n{}\n",
                format_schedule_time(entry.offset_minutes),
                entry.text
            ));
        }
        output.push_str("\n## Free Memo\n");
        if !self.free_memo.is_empty() {
            output.push_str(&self.free_memo);
            output.push('\n');
        }
        output
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Schedule,
    Todo,
    FreeMemo,
}

#[derive(Clone, Copy)]
enum Editing {
    Task(usize),
    Schedule(usize),
    FreeMemo,
}

struct App {
    date: NaiveDate,
    journal: Journal,
    path: PathBuf,
    focus: Focus,
    selected_task: usize,
    schedule_cursor: u16,
    editing: Option<(Editing, String)>,
    edit_cursor: usize,
    status: String,
}

impl App {
    fn load() -> io::Result<Self> {
        let date = Local::now().date_naive();
        let path = PathBuf::from("logs").join(format!("{date}.md"));
        let journal = if path.exists() {
            Journal::from_markdown(&fs::read_to_string(&path)?)
        } else {
            Journal::blank()
        };
        let mut app = Self {
            date,
            journal,
            path,
            focus: Focus::Schedule,
            selected_task: 0,
            schedule_cursor: 120,
            editing: None,
            edit_cursor: 0,
            status: String::new(),
        };
        if !app.path.exists() {
            app.save()?;
        }
        app.refresh_status();
        Ok(app)
    }

    fn save(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, self.journal.to_markdown(self.date))
    }

    fn move_selection(&mut self, delta: isize) {
        match self.focus {
            Focus::Todo if !self.journal.tasks.is_empty() => {
                let selected = &mut self.selected_task;
                let length = self.journal.tasks.len();
                *selected = (*selected as isize + delta).clamp(0, length as isize - 1) as usize;
            }
            Focus::Schedule => {
                self.schedule_cursor =
                    (self.schedule_cursor as isize + delta * 30).clamp(0, 1439) as u16;
            }
            _ => {}
        }
    }

    fn start_edit(&mut self, target: Editing) {
        let value = match target {
            Editing::Task(index) => self.journal.tasks[index].text.clone(),
            Editing::Schedule(index) => self.journal.schedule[index].text.clone(),
            Editing::FreeMemo => self.journal.free_memo.clone(),
        };
        self.edit_cursor = value.len();
        self.editing = Some((target, value));
    }

    fn commit_edit(&mut self) -> io::Result<()> {
        let Some((target, value)) = self.editing.take() else {
            return Ok(());
        };
        match target {
            Editing::Task(index) => self.journal.tasks[index].text = value,
            Editing::Schedule(index) => self.journal.schedule[index].text = value,
            Editing::FreeMemo => self.journal.free_memo = value,
        }
        self.save()?;
        self.refresh_status();
        Ok(())
    }

    fn schedule_entry_at_cursor(&mut self) -> usize {
        if let Some(index) = self
            .journal
            .schedule
            .iter()
            .position(|entry| entry.offset_minutes == self.schedule_cursor)
        {
            return index;
        }
        self.journal.schedule.push(ScheduleEntry {
            offset_minutes: self.schedule_cursor,
            text: String::new(),
        });
        self.journal
            .schedule
            .sort_by_key(|entry| entry.offset_minutes);
        self.journal
            .schedule
            .iter()
            .position(|entry| entry.offset_minutes == self.schedule_cursor)
            .expect("new schedule entry exists")
    }

    fn refresh_status(&mut self) {
        self.status = match self.focus {
            Focus::Schedule => format!(
                "Schedule {} · ↑/↓: move 30 min · Enter/e: write here · s/t/f: focus · q: quit",
                format_schedule_time(self.schedule_cursor)
            ),
            Focus::Todo => {
                "Todo · ↑/↓: select · n: new · e: edit · Space: check · s/t/f: focus · q: quit"
                    .to_string()
            }
            Focus::FreeMemo => "Free memo · e: edit · s/t/f: focus · q: quit".to_string(),
        };
    }

    fn insert_at_cursor(&mut self, text: &str) {
        if let Some((_, buffer)) = &mut self.editing {
            buffer.insert_str(self.edit_cursor, text);
            self.edit_cursor += text.len();
        }
    }

    fn delete_before_cursor(&mut self) {
        if self.edit_cursor == 0 {
            return;
        }
        if let Some((_, buffer)) = &mut self.editing {
            let previous = buffer[..self.edit_cursor]
                .char_indices()
                .last()
                .map(|(index, _)| index)
                .unwrap_or(0);
            buffer.drain(previous..self.edit_cursor);
            self.edit_cursor = previous;
        }
    }

    fn move_edit_cursor(&mut self, forward: bool) {
        let Some((_, buffer)) = &self.editing else {
            return;
        };
        if forward {
            if let Some(character) = buffer[self.edit_cursor..].chars().next() {
                self.edit_cursor += character.len_utf8();
            }
        } else if self.edit_cursor > 0 {
            self.edit_cursor = buffer[..self.edit_cursor]
                .char_indices()
                .last()
                .map(|(index, _)| index)
                .unwrap_or(0);
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> io::Result<bool> {
        if self.editing.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.editing = None;
                    self.edit_cursor = 0;
                    self.refresh_status();
                }
                KeyCode::Enter if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.commit_edit()?
                }
                KeyCode::Enter => self.insert_at_cursor("\n"),
                KeyCode::Backspace => self.delete_before_cursor(),
                KeyCode::Left => self.move_edit_cursor(false),
                KeyCode::Right => self.move_edit_cursor(true),
                KeyCode::Char(character)
                    if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
                {
                    self.insert_at_cursor(&character.to_string());
                }
                _ => {}
            }
            return Ok(false);
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
            KeyCode::Char('s') if key.modifiers.is_empty() => self.focus = Focus::Schedule,
            KeyCode::Char('t') if key.modifiers.is_empty() => self.focus = Focus::Todo,
            KeyCode::Char('f') if key.modifiers.is_empty() => self.focus = Focus::FreeMemo,
            KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                self.focus = match self.focus {
                    Focus::Schedule => Focus::Todo,
                    Focus::Todo => Focus::FreeMemo,
                    Focus::FreeMemo => Focus::Schedule,
                };
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Char(' ') if self.focus == Focus::Todo && !self.journal.tasks.is_empty() => {
                let task = &mut self.journal.tasks[self.selected_task];
                task.done = !task.done;
                self.save()?;
                self.refresh_status();
            }
            KeyCode::Char('n') if self.focus == Focus::Todo => {
                self.journal.tasks.push(Task {
                    done: false,
                    text: String::new(),
                });
                self.selected_task = self.journal.tasks.len() - 1;
                self.start_edit(Editing::Task(self.selected_task));
            }
            KeyCode::Char('e') => match self.focus {
                Focus::Todo if !self.journal.tasks.is_empty() => {
                    self.start_edit(Editing::Task(self.selected_task));
                }
                Focus::Schedule => {
                    let index = self.schedule_entry_at_cursor();
                    self.start_edit(Editing::Schedule(index));
                }
                Focus::FreeMemo => self.start_edit(Editing::FreeMemo),
                Focus::Todo => {}
            },
            KeyCode::Enter if self.focus == Focus::Schedule => {
                let index = self.schedule_entry_at_cursor();
                self.start_edit(Editing::Schedule(index));
            }
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.save()?;
                self.refresh_status();
            }
            _ => {}
        }
        self.refresh_status();
        Ok(false)
    }
}

fn weekday_japanese(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "月",
        Weekday::Tue => "火",
        Weekday::Wed => "水",
        Weekday::Thu => "木",
        Weekday::Fri => "金",
        Weekday::Sat => "土",
        Weekday::Sun => "日",
    }
}

fn month_lines(date: NaiveDate) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            format!("{} 年 {} 月", date.year(), date.month()),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("月  火  水  木  金  土  日"),
    ];
    let first = date.with_day(1).expect("first day exists");
    let padding = first.weekday().num_days_from_monday() as usize;
    let last = (28..=31)
        .rfind(|day| NaiveDate::from_ymd_opt(date.year(), date.month(), *day).is_some())
        .unwrap_or(28);
    let mut cells = (0..padding).map(|_| None).collect::<Vec<_>>();
    cells.extend((1..=last).map(Some));
    while cells.len() % 7 != 0 {
        cells.push(None);
    }
    for week in cells.chunks(7) {
        let spans = week
            .iter()
            .enumerate()
            .map(|(index, day)| match day {
                Some(day) if *day == date.day() => Span::styled(
                    format!("{:>2} ", day),
                    Style::default()
                        .bg(Color::Magenta)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                ),
                Some(day) => {
                    let color = if index >= 5 { Color::Red } else { Color::Gray };
                    Span::styled(format!("{:>2} ", day), Style::default().fg(color))
                }
                None => Span::raw("   "),
            })
            .collect::<Vec<_>>();
        lines.push(Line::from(spans));
    }
    lines
}

fn schedule_row(offset_minutes: u16, height: u16) -> usize {
    ((offset_minutes as usize * height as usize) / 1440).min(height.saturating_sub(1) as usize)
}

fn schedule_labels(height: u16, cursor: u16, active: bool) -> Vec<Line<'static>> {
    let mut labels = vec![String::new(); height as usize];
    for (offset, label) in [
        (120, "6"),
        (300, "9"),
        (480, "12"),
        (660, "15"),
        (840, "18"),
        (1020, "21"),
        (1200, "0"),
        (1380, "3"),
    ] {
        labels[schedule_row(offset, height)] = format!("{:>2}", label);
    }
    let cursor_row = schedule_row(cursor, height);
    labels[cursor_row] = if active {
        format!("›{}", labels[cursor_row])
    } else {
        format!(" {}", labels[cursor_row])
    };
    labels
        .into_iter()
        .map(|label| Line::from(Span::styled(label, Style::default().fg(Color::Cyan))))
        .collect()
}

fn schedule_lines(
    entries: &[ScheduleEntry],
    height: u16,
    cursor: u16,
    active: bool,
) -> Vec<Line<'static>> {
    let mut lines = vec![String::new(); height as usize];
    for (entry_index, entry) in entries.iter().enumerate() {
        let start = schedule_row(entry.offset_minutes, height);
        let next_entry = entries
            .get(entry_index + 1)
            .map(|next| next.offset_minutes)
            .unwrap_or(1440);
        let next_major_mark = if entry.offset_minutes < 120 {
            120
        } else {
            (((entry.offset_minutes - 120) / 180) + 1) * 180 + 120
        }
        .min(1440);
        let end = schedule_row(next_entry.min(next_major_mark), height);
        let available_rows = end.saturating_sub(start).max(1);
        let entry_lines = entry.text.lines().collect::<Vec<_>>();

        for line_index in 0..available_rows {
            let row = start + line_index;
            if row >= lines.len() {
                break;
            }
            let Some(text) = entry_lines.get(line_index).copied() else {
                break;
            };
            if !lines[row].is_empty() {
                lines[row].push_str("  ");
            }
            lines[row].push_str(text);
            if line_index + 1 == available_rows && entry_lines.len() > available_rows {
                lines[row].push_str("  …");
            }
        }
    }
    lines
        .into_iter()
        .enumerate()
        .map(|(row, text)| {
            let selected = active && row == schedule_row(cursor, height);
            Line::from(Span::styled(
                text,
                if selected {
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            ))
        })
        .collect()
}

fn panel_border(active: bool) -> Style {
    Style::default().fg(if active {
        Color::Magenta
    } else {
        Color::DarkGray
    })
}

fn render_editor(frame: &mut Frame, area: Rect, title: String, buffer: &str, cursor: usize) {
    let mut content = buffer.to_string();
    content.insert(cursor, '█');
    let popup = centered_rect(76, 58, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta))
        .title(title);
    let inner = block.inner(popup);
    let sections = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    frame.render_widget(block, popup);
    frame.render_widget(
        Paragraph::new(content).wrap(Wrap { trim: false }),
        sections[0],
    );
    frame.render_widget(
        Paragraph::new("Enter: new line  ·  ←/→: move cursor  ·  Ctrl+Enter: save  ·  Esc: cancel")
            .style(Style::default().fg(Color::DarkGray)),
        sections[1],
    );
}

fn ui(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(20),
        Constraint::Length(1),
    ])
    .split(area);

    let header = Paragraph::new(Line::from(Span::styled(
        format!(
            "  {} 年 {} 月 {} 日 ({})  ·  day {}",
            app.date.year(),
            app.date.month(),
            app.date.day(),
            weekday_japanese(app.date.weekday()),
            app.date.ordinal()
        ),
        Style::default().add_modifier(Modifier::BOLD),
    )))
    .block(Block::default().borders(Borders::ALL).title("techo"));
    frame.render_widget(header, chunks[0]);

    let page = Layout::horizontal([Constraint::Percentage(68), Constraint::Percentage(32)])
        .split(chunks[1]);
    let left =
        Layout::vertical([Constraint::Percentage(70), Constraint::Percentage(30)]).split(page[0]);
    let right = Layout::vertical([
        Constraint::Percentage(43),
        Constraint::Percentage(34),
        Constraint::Percentage(23),
    ])
    .split(page[1]);

    let schedule_block = Block::default()
        .borders(Borders::ALL)
        .border_style(panel_border(app.focus == Focus::Schedule))
        .title(" schedule ");
    let schedule_inner = schedule_block.inner(left[0]);
    frame.render_widget(schedule_block, left[0]);
    let schedule =
        Layout::horizontal([Constraint::Length(5), Constraint::Min(1)]).split(schedule_inner);
    frame.render_widget(
        Paragraph::new(schedule_labels(
            schedule[0].height,
            app.schedule_cursor,
            app.focus == Focus::Schedule,
        )),
        schedule[0],
    );
    frame.render_widget(
        Paragraph::new(schedule_lines(
            &app.journal.schedule,
            schedule[1].height,
            app.schedule_cursor,
            app.focus == Focus::Schedule,
        ))
        .wrap(Wrap { trim: false }),
        schedule[1],
    );

    let free_memo = if app.journal.free_memo.is_empty() {
        "A space outside the clock — reflections, sketches, and anything else."
    } else {
        &app.journal.free_memo
    };
    let free_style = if app.journal.free_memo.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };
    frame.render_widget(
        Paragraph::new(free_memo)
            .style(free_style)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(panel_border(app.focus == Focus::FreeMemo))
                    .title(" free memo "),
            )
            .wrap(Wrap { trim: false }),
        left[1],
    );

    let todo_items = if app.journal.tasks.is_empty() {
        vec![ListItem::new(Span::styled(
            "□  Press n to add a todo",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        app.journal
            .tasks
            .iter()
            .enumerate()
            .map(|(index, task)| {
                let marker = if task.done { "■" } else { "□" };
                let pointer = if app.focus == Focus::Todo && index == app.selected_task {
                    "›"
                } else {
                    " "
                };
                let style = if task.done {
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::CROSSED_OUT)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(Span::styled(
                    format!("{pointer} {marker} {}", task.text),
                    style,
                )))
            })
            .collect()
    };
    frame.render_widget(
        List::new(todo_items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(panel_border(app.focus == Focus::Todo))
                .title(" todo "),
        ),
        right[0],
    );

    frame.render_widget(
        Paragraph::new(month_lines(app.date))
            .block(Block::default().borders(Borders::ALL).title(" month view ")),
        right[1],
    );
    let quote = QUOTES[app.date.ordinal0() as usize % QUOTES.len()];
    frame.render_widget(
        Paragraph::new(quote)
            .block(Block::default().borders(Borders::ALL).title(" words "))
            .wrap(Wrap { trim: true }),
        right[2],
    );
    frame.render_widget(
        Paragraph::new(app.status.as_str()).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );

    if let Some((target, buffer)) = &app.editing {
        let title = match target {
            Editing::Task(_) => " edit todo ".to_string(),
            Editing::Schedule(index) => format!(
                " schedule · {} ",
                format_schedule_time(app.journal.schedule[*index].offset_minutes)
            ),
            Editing::FreeMemo => " free memo ".to_string(),
        };
        render_editor(frame, area, title, buffer, app.edit_cursor);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::load()?;
    loop {
        terminal.draw(|frame| ui(frame, &app))?;
        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
            && app.handle_key(key)?
        {
            return Ok(());
        }
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    let result = run(&mut terminal);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_round_trip_keeps_todos_and_schedule_entries() {
        let date = NaiveDate::from_ymd_opt(2026, 7, 31).unwrap();
        let original = Journal {
            tasks: vec![Task {
                done: true,
                text: "Read Ratatui docs".into(),
            }],
            schedule: vec![
                ScheduleEntry {
                    offset_minutes: 120,
                    text: "Build techo".into(),
                },
                ScheduleEntry {
                    offset_minutes: 330,
                    text: "Take a break".into(),
                },
            ],
            free_memo: "A loose thought.".into(),
        };
        let loaded = Journal::from_markdown(&original.to_markdown(date));
        assert!(loaded.tasks[0].done);
        assert_eq!(loaded.tasks[0].text, "Read Ratatui docs");
        assert_eq!(loaded.schedule.len(), 2);
        assert_eq!(loaded.schedule[0].offset_minutes, 120);
        assert_eq!(loaded.schedule[1].offset_minutes, 330);
        assert_eq!(loaded.schedule[1].text, "Take a break");
        assert_eq!(loaded.free_memo, "A loose thought.");
    }

    #[test]
    fn legacy_hourly_timeline_migrates_to_schedule_entries() {
        let journal =
            Journal::from_markdown("## Timeline\n\n### 09:00\nBuild techo\n\n### 12:00\nLunch\n");
        assert_eq!(journal.schedule.len(), 2);
        assert_eq!(journal.schedule[0].offset_minutes, 300);
        assert_eq!(journal.schedule[0].text, "Build techo");
        assert_eq!(journal.schedule[1].offset_minutes, 480);
        assert_eq!(journal.schedule[1].text, "Lunch");
    }

    #[test]
    fn paper_day_runs_from_four_am_to_the_next_four_am() {
        assert_eq!(format_schedule_time(0), "04:00");
        assert_eq!(format_schedule_time(1230), "00:30 (+1)");
    }
}
