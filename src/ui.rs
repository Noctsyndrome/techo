use crate::{
    app::{Action, App, EditTarget, Focus},
    calendar::{MONTHS, moon},
    editor::TextEditor,
    journal::clock_time,
    schedule::{self, GUTTER},
    theme, words,
};
use chrono::{Datelike, Local, NaiveDate};
use crossterm::event::KeyCode;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Wrap},
};
use std::{cell::Cell, collections::HashSet};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

thread_local! {
    /// The colour the page is printed in this month: rules, titles, today.
    static INK: Cell<Color> = const { Cell::new(Color::DarkGray) };
}
fn set_ink(colour: Color) {
    INK.with(|ink| ink.set(colour));
}
/// Everything the planner prints is in the month's colour; what you write is
/// plain, and small print stays grey.
fn ink() -> Style {
    Style::default().fg(INK.with(|ink| ink.get()))
}
fn muted() -> Style {
    Style::default().fg(Color::DarkGray)
}
/// Panels are ruled boxes with a little margin inside. The active one shows its
/// title as a small tab in the month's colour, which also reads without colour.
fn block(title: &str, active: bool) -> Block<'static> {
    let block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .border_type(BorderType::Plain)
        .border_style(ink());
    if title.is_empty() {
        block
    } else {
        block.title(if active {
            // The tab floats on the rule: a gap on either side keeps it from
            // reading as part of the line.
            Line::from(vec![
                Span::raw(" "),
                Span::styled(format!(" {title} "), ink().reversed()),
                Span::raw(" "),
            ])
        } else {
            Line::styled(format!(" {title} "), ink())
        })
    }
}
fn put(frame: &mut Frame, rect: Rect, text: impl Into<Text<'static>>) {
    frame.render_widget(Paragraph::new(text), rect);
}
fn clip(text: &str, width: u16) -> String {
    let mut out = String::new();
    let mut used = 0;
    let total = text.width();
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > width.saturating_sub(u16::from(total > width as usize)) as usize {
            break;
        }
        out.push(c);
        used += w;
    }
    if total > width as usize && width > 0 {
        out.push('…');
    }
    out
}
/// Quiet clickable text; returns the x after it.
fn button(
    frame: &mut Frame,
    hits: &mut Vec<(Rect, Action)>,
    x: u16,
    y: u16,
    label: &str,
    action: Action,
    max_x: u16,
) -> u16 {
    let width = (label.width() as u16).min(max_x.saturating_sub(x));
    if width > 0 {
        let rect = Rect::new(x, y, width, 1);
        put(frame, rect, Line::styled(label.to_string(), muted()));
        hits.push((rect, action));
    }
    x.saturating_add(width + 2)
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    app.hits.clear();
    set_ink(theme::month(app.date).color());
    app.schedule_area = None;
    app.schedule_cursor_y = None;
    app.todo_area = None;
    app.todo_cursor_y = None;
    app.todo_next_y = None;
    app.memo_area = None;
    let area = frame.area();
    if area.width < 32 || area.height < 14 {
        put(
            frame,
            area,
            "techō\nPlease resize to at least 32 x 14.\nYour draft is retained.\nq: quit (outside editor)"
                .to_string(),
        );
        return;
    }
    if app.calendar.is_some() {
        draw_year(frame, app);
    } else if app.turning() {
        // A freshly turned page is blank for a beat before the ink appears.
        let journal = std::mem::take(&mut app.journal);
        draw_day(frame, app);
        app.journal = journal;
    } else {
        draw_day(frame, app);
    }
    // Writing happens on the page itself, in a small note where the cursor is.
    if app.editing.is_some() {
        draw_editor(frame, app);
    }
    if app.help {
        draw_help(frame, app);
    }
    if app.delete_pending {
        draw_delete(frame, app);
    }
}

fn draw_day(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let rows = Layout::vertical([Constraint::Min(10), Constraint::Length(1)]).split(area);
    let body = rows[0];
    if area.width >= 86 && body.height >= 22 {
        // The planner page: writing on the left, todo, month and words down the right.
        let columns = Layout::horizontal([Constraint::Min(50), Constraint::Length(34)]).split(body);
        let left = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(schedule_height(
                app,
                columns[0].width,
                columns[0].height - 3,
            )),
            Constraint::Min(5),
        ])
        .split(columns[0]);
        draw_header(frame, app, left[0]);
        draw_schedule(frame, app, left[1]);
        draw_memo(frame, app, left[2]);
        let right = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(10),
            Constraint::Length(5),
        ])
        .split(columns[1]);
        draw_todo(frame, app, right[0]);
        draw_month(
            frame,
            &mut app.hits,
            &app.written,
            right[1],
            app.date,
            app.date,
            app.date,
            true,
        );
        draw_words(frame, app, right[2]);
    } else if area.width >= 60 && body.height >= 16 {
        let sections = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(schedule_height(app, body.width * 3 / 5, body.height - 3)),
            Constraint::Min(5),
        ])
        .split(body);
        draw_header(frame, app, sections[0]);
        let top = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(sections[1]);
        draw_schedule(frame, app, top[0]);
        draw_todo(frame, app, top[1]);
        draw_memo(frame, app, sections[2]);
    } else {
        // Narrow terminals keep the active panel readable; all sections stay keyboard-accessible.
        let parts = Layout::vertical([Constraint::Length(1), Constraint::Min(8)]).split(body);
        put(frame, parts[0], header_line(app.date, 1));
        match app.focus {
            Focus::Memo => draw_memo(frame, app, parts[1]),
            Focus::Todo => draw_todo(frame, app, parts[1]),
            Focus::Schedule => draw_schedule(frame, app, parts[1]),
        }
    }
    draw_footer(frame, app, rows[1]);
}

/// The schedule only grows with what is written; the rest of the column stays
/// with free memo.
fn schedule_height(app: &App, width: u16, available: u16) -> u16 {
    let text_width = width.saturating_sub(4 + GUTTER).max(1);
    let needed = schedule::rows(&app.journal, text_width).len() as u16 + 2;
    let ceiling = (available * 3 / 5).max(6).min(available.saturating_sub(5));
    needed.clamp(6, ceiling.max(6))
}

/// The page's date as a planner prints it: `09-11 (金) · day 254 · New moon`.
fn header_line(date: NaiveDate, indent: usize) -> Line<'static> {
    let weekday =
        ["月", "火", "水", "木", "金", "土", "日"][date.weekday().num_days_from_monday() as usize];
    Line::from(vec![
        Span::styled(
            format!(
                "{}{} ({}) · day {}",
                " ".repeat(indent),
                date.format("%m-%d"),
                weekday,
                date.ordinal()
            ),
            Style::default().bold(),
        ),
        Span::raw(format!(" · {}", moon(date))),
    ])
}

fn draw_header(frame: &mut Frame, app: &mut App, area: Rect) {
    let header = block("techō", false);
    let inner = header.inner(area);
    frame.render_widget(header, area);
    if inner.height == 0 {
        return;
    }
    put(
        frame,
        Rect::new(inner.x, inner.y, inner.width, 1),
        header_line(app.date, 0),
    );
}

/// The day's items in time order; items at the same time gather under one label.
/// A small mark on the rule where the page continues: on the bottom rule when
/// more lies below, on the top rule when the view has moved past the start.
fn overflow(frame: &mut Frame, area: Rect, above: bool, below: bool) {
    if area.width < 8 || area.height < 2 {
        return;
    }
    let x = area.right() - 5;
    if above {
        put(
            frame,
            Rect::new(x, area.y, 3, 1),
            Line::styled(" ⋯ ", ink()),
        );
    }
    if below {
        put(
            frame,
            Rect::new(x, area.bottom() - 1, 3, 1),
            Line::styled(" ⋯ ", ink()),
        );
    }
}

fn draw_schedule(frame: &mut Frame, app: &mut App, area: Rect) {
    let active = app.focus == Focus::Schedule;
    let border = block("schedule", active);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    app.hits.push((area, Action::Focus(Focus::Schedule)));
    if inner.height == 0 || inner.width <= GUTTER + 1 {
        return;
    }
    let text_width = inner.width - GUTTER;
    app.schedule_area = Some(inner);
    app.schedule_rows = schedule::rows(&app.journal, text_width);
    if let Some(entry) = app.schedule_jump.take()
        && let Some(row) = app.schedule_rows.iter().position(|row| row.entry == entry)
    {
        app.schedule_row = row;
    }
    let count = app.schedule_rows.len();
    let height = inner.height as usize;
    app.schedule_row = app.schedule_row.min(count.saturating_sub(1));
    app.schedule_offset = app.schedule_offset.min(count.saturating_sub(height));
    if app.schedule_row < app.schedule_offset {
        app.schedule_offset = app.schedule_row;
    }
    if app.schedule_row >= app.schedule_offset + height {
        app.schedule_offset = app.schedule_row + 1 - height;
    }
    // Keep the whole item under the cursor in view when it fits.
    if let Some(entry) = app.cursor_entry()
        && let Some(last) = app.schedule_rows.iter().rposition(|row| row.entry == entry)
        && last >= app.schedule_offset + height
        && last + 1 - app.schedule_row <= height
    {
        app.schedule_offset = last + 1 - height;
    }
    overflow(
        frame,
        area,
        app.schedule_offset > 0,
        app.schedule_offset + height < count,
    );
    let cursor = app.cursor_entry();
    for (line, i) in (app.schedule_offset..count).take(height).enumerate() {
        let (entry, timed, text) = {
            let row = &app.schedule_rows[i];
            (row.entry, row.timed, clip(&row.text, text_width))
        };
        let y = inner.y + line as u16;
        if i == app.schedule_row {
            app.schedule_cursor_y = Some(y);
        }
        let selected = active && Some(entry) == cursor;
        let gutter = if timed {
            format!(
                "{:<width$}",
                clock_time(app.journal.schedule[entry].offset_minutes),
                width = GUTTER as usize
            )
        } else {
            " ".repeat(GUTTER as usize)
        };
        put(
            frame,
            Rect::new(inner.x, y, GUTTER, 1),
            Line::styled(
                gutter,
                if selected {
                    Style::default().bold()
                } else {
                    muted()
                },
            ),
        );
        if selected {
            let shown = if text.is_empty() { " ".into() } else { text };
            let width = (shown.width() as u16).min(text_width);
            put(
                frame,
                Rect::new(inner.x + GUTTER, y, width, 1),
                Line::styled(shown, Style::default().reversed()),
            );
        } else {
            put(frame, Rect::new(inner.x + GUTTER, y, text_width, 1), text);
        }
        app.hits.push((
            Rect::new(inner.x, y, inner.width, 1),
            Action::Select(Focus::Schedule, i),
        ));
    }
}

fn draw_todo(frame: &mut Frame, app: &mut App, area: Rect) {
    let active = app.focus == Focus::Todo;
    let border = block("todo", active);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    app.hits.push((area, Action::Focus(Focus::Todo)));
    app.todo_area = Some(inner);
    let height = inner.height as usize;
    let count = app.journal.tasks.len();
    if height == 0 {
        return;
    }
    let offset = &mut app.task_offset;
    *offset = (*offset).min(count.saturating_sub(height));
    let selected = app.selected_task.min(count.saturating_sub(1));
    if selected < *offset {
        *offset = selected;
    }
    if selected >= *offset + height {
        *offset = selected + 1 - height;
    }
    let (above, below) = (*offset > 0, *offset + height < count);
    overflow(frame, area, above, below);
    app.todo_next_y = Some(inner.y + (count.saturating_sub(*offset).min(height - 1)) as u16);
    for (row, i) in (*offset..count).take(height).enumerate() {
        let task = &app.journal.tasks[i];
        let first = task
            .text
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("");
        let more = if task.text.contains('\n') { " ↵" } else { "" };
        let preview = format!(
            "{}{}",
            clip(
                &format!("[{}] {}", if task.done { 'x' } else { ' ' }, first),
                inner.width.saturating_sub(more.width() as u16)
            ),
            more
        );
        let rect = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
        if i == selected {
            app.todo_cursor_y = Some(rect.y);
        }
        let style = if active && i == selected {
            Style::default().reversed()
        } else if task.done {
            muted()
        } else {
            Style::default()
        };
        put(frame, rect, Line::styled(preview, style));
        app.hits.push((rect, Action::Select(Focus::Todo, i)));
    }
}

fn draw_memo(frame: &mut Frame, app: &mut App, area: Rect) {
    let border = block("free memo", app.focus == Focus::Memo);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    app.hits.push((area, Action::Focus(Focus::Memo)));
    app.memo_area = Some(inner);
    if app.journal.free_memo.is_empty() || inner.height == 0 {
        return;
    }
    let (lines, _) = TextEditor::new(app.journal.free_memo.clone()).visual(inner.width);
    app.memo_scroll = app
        .memo_scroll
        .min(lines.len().saturating_sub(inner.height as usize));
    // While writing in place the editor scrolls on its own, so the marks rest.
    if !matches!(&app.editing, Some(edit) if edit.target == EditTarget::Memo) {
        overflow(
            frame,
            area,
            app.memo_scroll > 0,
            app.memo_scroll + (inner.height as usize) < lines.len(),
        );
    }
    let text = lines
        .into_iter()
        .skip(app.memo_scroll)
        .take(inner.height as usize)
        .map(Line::from)
        .collect::<Vec<_>>();
    put(frame, inner, text);
}

fn draw_words(frame: &mut Frame, app: &mut App, area: Rect) {
    let border = block("words", false);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    frame.render_widget(
        Paragraph::new(words::for_date(&app.words, app.date)).wrap(Wrap { trim: true }),
        inner,
    );
}

/// While writing, the footer carries the few keys that matter and any error.
fn draw_editing_footer(frame: &mut Frame, app: &mut App, area: Rect) -> bool {
    let Some(edit) = &app.editing else {
        return false;
    };
    if !edit.error.is_empty() {
        put(
            frame,
            area,
            Line::styled(format!(" {}", edit.error), Style::default().fg(Color::Red)),
        );
        return true;
    }
    let jump = edit.target == EditTarget::Jump;
    let schedule = matches!(edit.target, EditTarget::Schedule(_));
    let time_active = edit.time_active;
    let mut x = button(
        frame,
        &mut app.hits,
        area.x + 1,
        area.y,
        if jump {
            "Enter open"
        } else {
            "Ctrl+S or Ctrl+Enter save"
        },
        Action::Save,
        area.right(),
    );
    x = button(
        frame,
        &mut app.hits,
        x,
        area.y,
        "Esc cancel",
        Action::Cancel,
        area.right(),
    );
    if schedule {
        button(
            frame,
            &mut app.hits,
            x,
            area.y,
            if time_active {
                "Enter back to text"
            } else {
                "Tab time"
            },
            if time_active {
                Action::BodyField
            } else {
                Action::TimeField
            },
            area.right(),
        );
    }
    true
}

/// The keys that matter on the focused panel, in one muted line.
fn hints(app: &App) -> &'static str {
    match app.focus {
        Focus::Schedule => {
            "Enter write · n new · d delete · Up/Down move · Tab panel · [ ] day · y year"
        }
        Focus::Todo => "Enter edit · n new · Space check · d delete · Tab panel · [ ] day · y year",
        Focus::Memo => "Enter write · Up/Down scroll · Tab panel · [ ] day · y year",
    }
}

/// One quiet line: a message for a moment, otherwise the hints, and `?`.
fn draw_footer(frame: &mut Frame, app: &mut App, area: Rect) {
    if area.height == 0 || draw_editing_footer(frame, app, area) {
        return;
    }
    let text = app.status_line().unwrap_or_else(|| hints(app)).to_string();
    put(
        frame,
        area,
        Line::styled(
            clip(&format!(" {text}"), area.width.saturating_sub(3)),
            muted(),
        ),
    );
    if area.width > 4 {
        let rect = Rect::new(area.right() - 2, area.y, 1, 1);
        put(frame, rect, Line::styled("?", muted()));
        app.hits.push((rect, Action::Key(KeyCode::Char('?'))));
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_month(
    frame: &mut Frame,
    hits: &mut Vec<(Rect, Action)>,
    written: &HashSet<NaiveDate>,
    area: Rect,
    month: NaiveDate,
    selected: NaiveDate,
    opened: NaiveDate,
    main: bool,
) {
    let title = if main {
        // The month's traditional name sits on the calendar, the tab a planner prints there.
        format!("{} · {}", month.format("%Y-%m"), theme::month(month).name)
    } else {
        MONTHS[month.month0() as usize].to_string()
    };
    let border = block(&title, !main && selected.month() == month.month());
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if main {
        hits.push((area, Action::Calendar));
    }
    put(
        frame,
        Rect::new(inner.x, inner.y, inner.width, 1),
        Line::styled(" Mo  Tu  We  Th  Fr  Sa  Su", muted()),
    );
    let first = month.with_day(1).unwrap();
    let padding = first.weekday().num_days_from_monday();
    let today = Local::now().date_naive();
    for day in 1..=31 {
        let Some(date) = NaiveDate::from_ymd_opt(month.year(), month.month(), day) else {
            break;
        };
        let cell = padding + day - 1;
        let x = inner.x + (cell % 7) as u16 * 4;
        let y = inner.y + 1 + (cell / 7) as u16;
        if x + 4 > inner.right() || y >= inner.bottom() {
            continue;
        }
        // A dot after the number is the ink of a written day.
        let label = format!(
            " {day:02}{}",
            if written.contains(&date) { "·" } else { " " }
        );
        // Today is a small tab in the month's colour, like the active panel's title;
        // the open page, when it is another day, is a plain one.
        let style = if main {
            if date == today {
                ink().reversed()
            } else if date == opened {
                Style::default().reversed()
            } else {
                Style::default()
            }
        } else if date == selected {
            Style::default().reversed()
        } else if date == today {
            ink().reversed()
        } else if date == opened {
            Style::default().bold()
        } else {
            Style::default()
        };
        let rect = Rect::new(x, y, 4, 1);
        put(frame, rect, Line::styled(label, style));
        hits.push((rect, Action::Date(date)));
    }
}

fn draw_year(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let selected = app.calendar.unwrap();
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(1),
    ])
    .split(area);
    let title = rows[0];
    let x = button(
        frame,
        &mut app.hits,
        title.x + 1,
        title.y,
        "‹",
        Action::Key(KeyCode::Char('[')),
        title.right(),
    );
    let year = selected.year().to_string();
    put(
        frame,
        Rect::new(x, title.y, year.width() as u16, 1),
        Line::styled(year.clone(), Style::default().bold()),
    );
    button(
        frame,
        &mut app.hits,
        x + year.width() as u16 + 2,
        title.y,
        "›",
        Action::Key(KeyCode::Char(']')),
        title.right(),
    );
    let grid = rows[1];
    let cols = (grid.width / 32).clamp(1, 4);
    let rows_count = (grid.height / 9).clamp(1, 4);
    let per_page = (cols * rows_count).min(12) as u32;
    app.calendar_page_size = per_page as i32;
    let first_month = selected.month0() / per_page * per_page;
    let cell_width = grid.width / cols;
    let cell_height = grid.height / rows_count;
    for index in 0..per_page {
        let month = first_month + index + 1;
        if month > 12 {
            break;
        }
        let rect = Rect::new(
            grid.x + index as u16 % cols * cell_width,
            grid.y + index as u16 / cols * cell_height,
            cell_width,
            cell_height,
        );
        // Each month in its own colour, like the coloured tabs along a planner's edge.
        set_ink(theme::MONTHS[month as usize - 1].color());
        draw_month(
            frame,
            &mut app.hits,
            &app.written,
            rect,
            NaiveDate::from_ymd_opt(selected.year(), month, 1).unwrap(),
            selected,
            app.date,
            false,
        );
    }
    set_ink(theme::month(app.date).color());
    let footer = rows[2];
    if draw_editing_footer(frame, app, footer) {
        return;
    }
    let text = match app.status_line() {
        Some(status) => format!(" {status}"),
        None => format!(
            " {selected}   arrows move · Enter open · [ ] year · PgUp/PgDn page · t today · Esc back"
        ),
    };
    put(
        frame,
        footer,
        Line::styled(clip(&text, footer.width.saturating_sub(3)), muted()),
    );
    if footer.width > 4 {
        let rect = Rect::new(footer.right() - 2, footer.y, 1, 1);
        put(frame, rect, Line::styled("?", muted()));
        app.hits.push((rect, Action::Key(KeyCode::Char('?'))));
    }
}

fn popup(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

/// A note that opens where the cursor is: on the schedule row, on the todo row,
/// or inside free memo itself. The page stays around it.
fn note_rect(
    panel: Option<Rect>,
    row_y: Option<u16>,
    indent: u16,
    wanted: u16,
    area: Rect,
) -> Rect {
    match panel {
        Some(inner) if inner.height >= 4 && inner.width > indent + 8 => {
            let height = wanted.clamp(4, inner.height);
            let mut y = row_y.unwrap_or(inner.y).clamp(inner.y, inner.bottom() - 1);
            if y + height > inner.bottom() {
                y = inner.bottom() - height;
            }
            Rect::new(inner.x + indent, y, inner.width - indent, height)
        }
        _ => popup(
            area,
            area.width.saturating_sub(4).clamp(20, 72),
            area.height.saturating_sub(2).clamp(4, 10),
        ),
    }
}

fn draw_editor(frame: &mut Frame, app: &mut App) {
    let edit = app.editing.as_ref().unwrap();
    let area = frame.area();
    if edit.target == EditTarget::Memo
        && let Some(inner) = app.memo_area
        && inner.height > 0
    {
        // Free memo is written in place.
        frame.render_widget(Clear, inner);
        draw_text(frame, edit, inner);
        app.hits.push((inner, Action::BodyField));
        return;
    }
    let lines = edit
        .text
        .visual(area.width.saturating_sub(4).max(1))
        .0
        .len() as u16
        + 2;
    let (rect, title) = match edit.target {
        EditTarget::Schedule(_) => (
            note_rect(
                app.schedule_area,
                app.schedule_cursor_y,
                GUTTER - 1,
                lines,
                area,
            ),
            edit.time.text.clone(),
        ),
        EditTarget::Task(index) => (
            note_rect(
                app.todo_area,
                if index.is_some() {
                    app.todo_cursor_y
                } else {
                    app.todo_next_y
                },
                0,
                lines,
                area,
            ),
            "todo".into(),
        ),
        EditTarget::Memo => (
            popup(
                area,
                area.width.saturating_sub(4).clamp(20, 72),
                area.height.saturating_sub(2).clamp(4, 12),
            ),
            "free memo".into(),
        ),
        EditTarget::Jump => (popup(area, 24, 3), "YYYY-MM-DD".into()),
    };
    let schedule = matches!(edit.target, EditTarget::Schedule(_));
    frame.render_widget(Clear, rect);
    let note = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(ink())
        .padding(Padding::horizontal(1))
        .title(Line::styled(
            format!(" {title} "),
            if schedule && edit.time_active {
                ink().bold().reversed()
            } else {
                ink().bold()
            },
        ));
    let body = note.inner(rect);
    frame.render_widget(note, rect);
    draw_text(frame, edit, body);
    if schedule {
        // The time lives in the note's title; Tab moves the cursor up there.
        if edit.time_active {
            frame.set_cursor_position((
                (rect.x + 2 + edit.time.cursor as u16).min(rect.right().saturating_sub(2)),
                rect.y,
            ));
        }
        app.hits
            .push((Rect::new(rect.x, rect.y, rect.width, 1), Action::TimeField));
    }
    app.hits.push((body, Action::BodyField));
}

fn draw_text(frame: &mut Frame, edit: &crate::app::Editing, body: Rect) {
    if body.width == 0 || body.height == 0 {
        return;
    }
    let (lines, (cx, cy)) = edit.text.visual(body.width);
    let scroll = cy.saturating_sub(body.height.saturating_sub(1) as usize);
    put(
        frame,
        body,
        lines
            .into_iter()
            .skip(scroll)
            .take(body.height as usize)
            .map(Line::from)
            .collect::<Vec<_>>(),
    );
    if !edit.time_active {
        frame.set_cursor_position((
            body.x + cx.min(body.width - 1),
            body.y + (cy - scroll) as u16,
        ));
    }
}

fn draw_help(frame: &mut Frame, app: &mut App) {
    app.hits.clear();
    let area = popup(frame.area(), 78, 20);
    frame.render_widget(Clear, area);
    let month = theme::month(app.date);
    let text = format!(
        "s schedule   t todo   f free memo   Tab switch   or click a panel\n\
         Up/Down move   Enter write or edit   n new   d delete   Space check a todo\n\
         Down past the last item starts a new one, at the time now on today's page.\n\
         schedule: n writes an item; its time is the note's title, Tab to change it.\n\
         Type 9, 930 or 09:30. Items at the same time gather together.\n\
         The paper day runs 04:00 to 03:59, so 00:00-03:59 belongs to the night after.\n\
         \n\
         [ ] previous / next day   Home today   g go to a date   y the year\n\
         year: arrows move, Enter opens, [ ] change year, PgUp/PgDn page, Esc back\n\
         \n\
         editor: Ctrl+S or Ctrl+Enter saves, Esc cancels, Enter adds a line, paste works.\n\
         moon: an approximate phase for the date; the date's day number is day N.\n\
         this month is {} ({}), and its pages are printed in {}.\n\
         \n\
         files: {}\n\
         words: words.txt beside the journals, one line per day, if you want your own.\n\
         \n\
         ? or F1 shows this again. Any key closes it.",
        month.name,
        month.reading,
        month.colour,
        app.store.dir.display()
    );
    frame.render_widget(
        Paragraph::new(text)
            .wrap(Wrap { trim: false })
            .block(block("techō", false)),
        area,
    );
    app.hits.push((area, Action::Key(KeyCode::Esc)));
}

fn draw_delete(frame: &mut Frame, app: &mut App) {
    app.hits.clear();
    let area = popup(frame.area(), 40, 5);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new("Delete this item?").block(block("", false)),
        area,
    );
    let x = button(
        frame,
        &mut app.hits,
        area.x + 2,
        area.bottom() - 2,
        "y delete",
        Action::Key(KeyCode::Char('y')),
        area.right() - 1,
    );
    button(
        frame,
        &mut app.hits,
        x,
        area.bottom() - 2,
        "Esc keep",
        Action::Key(KeyCode::Esc),
        area.right() - 1,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{calendar::parse_date, storage::test_dir};
    use crossterm::event::{Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use ratatui::backend::TestBackend;
    fn app() -> App {
        let mut app = App::open(test_dir(), parse_date("2026-09-11").unwrap()).unwrap();
        app.help = false;
        app
    }
    fn click(app: &mut App, rect: Rect) {
        app.event(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.x,
            row: rect.y,
            modifiers: KeyModifiers::NONE,
        }));
    }
    fn screen(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect()
    }
    #[test]
    fn all_views_survive_small_and_large_sizes() {
        let mut app = app();
        for (w, h) in [
            (0, 0),
            (1, 1),
            (80, 1),
            (31, 13),
            (32, 14),
            (60, 24),
            (80, 24),
            (120, 40),
            (200, 60),
        ] {
            let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
            for focus in [Focus::Schedule, Focus::Todo, Focus::Memo] {
                app.focus = focus;
                terminal.draw(|f| draw(f, &mut app)).unwrap();
                app.start_edit(true);
                terminal.draw(|f| draw(f, &mut app)).unwrap();
                app.editing = None;
            }
            app.help = true;
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            app.help = false;
            app.calendar = Some(app.date);
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            app.calendar = None;
        }
        let dir = app.store.dir.clone();
        drop(app);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn page_is_quiet_and_marks_the_active_panel_without_words() {
        let mut app = app();
        app.focus = Focus::Memo;
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let text = screen(&terminal);
        for noise in [
            "[s]",
            "[t]",
            "[f]",
            "0/0",
            "> ",
            "little space",
            "y Year",
            "⋯",
        ] {
            assert!(!text.contains(noise), "{noise} is on the page");
        }
        assert!(text.contains("2026-09"));
        let rect = app
            .hits
            .iter()
            .find(|(_, a)| matches!(a, Action::Focus(Focus::Schedule)))
            .unwrap()
            .0;
        let tab = |terminal: &Terminal<TestBackend>| {
            terminal.backend().buffer()[(rect.x + 2, rect.y)]
                .modifier
                .contains(Modifier::REVERSED)
        };
        assert_eq!(terminal.backend().buffer()[(rect.x, rect.y)].symbol(), "┌");
        assert!(!tab(&terminal));
        click(&mut app, rect);
        assert_eq!(app.focus, Focus::Schedule);
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        assert_eq!(terminal.backend().buffer()[(rect.x, rect.y)].symbol(), "┌");
        assert!(tab(&terminal));
        let dir = app.store.dir.clone();
        drop(app);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn mouse_picks_items_and_calendar_days_after_resize() {
        let mut app = app();
        for (time, text) in [(300, "brief"), (300, "second"), (840, "mail")] {
            app.journal.schedule.push(crate::journal::ScheduleEntry {
                offset_minutes: time,
                text: text.into(),
            });
        }
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let text = screen(&terminal);
        assert!(text.contains("09:00  brief"));
        assert!(text.contains("       second"));
        assert!(text.contains("18:00  mail"));
        let rect = app
            .hits
            .iter()
            .find(|(_, a)| matches!(a, Action::Select(Focus::Schedule, 2)))
            .unwrap()
            .0;
        click(&mut app, rect);
        assert_eq!(app.focus, Focus::Schedule);
        assert_eq!(app.cursor_entry(), Some(2));
        app.calendar = Some(parse_date("2024-02-29").unwrap());
        terminal.backend_mut().resize(60, 24);
        terminal.resize(Rect::new(0, 0, 60, 24)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let rect = app
            .hits
            .iter()
            .find(|(_, a)| matches!(a, Action::Date(d) if *d == parse_date("2024-02-29").unwrap()))
            .unwrap()
            .0;
        click(&mut app, rect);
        assert_eq!(app.date, parse_date("2024-02-29").unwrap());
        assert!(app.calendar.is_none());
        assert!(!app.store.path(app.date).exists());
        let dir = app.store.dir.clone();
        drop(app);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn six_week_months_remain_clickable_in_minimum_terminal() {
        let mut app = app();
        let mut terminal = Terminal::new(TestBackend::new(32, 14)).unwrap();
        for month in 1..=12 {
            let date = (28..=31)
                .rev()
                .find_map(|day| NaiveDate::from_ymd_opt(2026, month, day))
                .unwrap();
            app.calendar = Some(date);
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            assert!(
                app.hits
                    .iter()
                    .any(|(_, action)| matches!(action, Action::Date(d) if *d == date)),
                "last day of month {month} is off screen"
            );
        }
        let dir = app.store.dir.clone();
        drop(app);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn written_days_are_dotted_in_the_calendar() {
        let mut app = app();
        app.focus = Focus::Memo;
        app.start_edit(false);
        app.editing.as_mut().unwrap().text.insert("ink");
        app.commit();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let text = screen(&terminal);
        assert!(text.contains(" 11·"));
        assert!(!text.contains(" 12·"));
        let dir = app.store.dir.clone();
        drop(app);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn long_days_scroll_and_show_their_whole_text() {
        let mut app = app();
        for i in 0..30 {
            app.journal.schedule.push(crate::journal::ScheduleEntry {
                offset_minutes: i * 30,
                text: format!("Item {i}\n{}", "Long body ".repeat(8)),
            });
        }
        app.focus = Focus::Schedule;
        app.schedule_jump = Some(29);
        app.journal.free_memo = "Memo stays visible".into();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let text = screen(&terminal);
        assert!(text.contains("Item 29"));
        assert!(text.contains("Long body"));
        assert!(text.contains("Memo stays visible"));
        assert!(!text.contains("Item 0"));
        assert_eq!(app.cursor_entry(), Some(29));
        // Scrolled to the end: the page continues above, not below.
        let schedule = app
            .hits
            .iter()
            .find(|(_, a)| matches!(a, Action::Focus(Focus::Schedule)))
            .unwrap()
            .0;
        let mark = |terminal: &Terminal<TestBackend>, y: u16| {
            terminal.backend().buffer()[(schedule.right() - 4, y)].symbol() == "⋯"
        };
        assert!(mark(&terminal, schedule.y));
        assert!(!mark(&terminal, schedule.bottom() - 1));
        app.schedule_jump = Some(0);
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let text = screen(&terminal);
        assert!(text.contains("04:00  Item 0"));
        assert!(!text.contains("Item 29"));
        assert!(!mark(&terminal, schedule.y));
        assert!(mark(&terminal, schedule.bottom() - 1));
        let dir = app.store.dir.clone();
        drop(app);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn writing_happens_on_the_page() {
        let mut app = app();
        app.journal.schedule.push(crate::journal::ScheduleEntry {
            offset_minutes: 480,
            text: "lunch".into(),
        });
        app.focus = Focus::Schedule;
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        app.start_edit(true);
        app.editing.as_mut().unwrap().text.insert("walk");
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let text = screen(&terminal);
        for kept in [
            " 12:00 ",
            "walk",
            "todo",
            "free memo",
            "2026-09",
            "Ctrl+S",
            "Tab time",
        ] {
            assert!(text.contains(kept), "{kept} missing while writing");
        }
        let note_y = app
            .hits
            .iter()
            .rev()
            .find(|(_, a)| matches!(a, Action::TimeField))
            .unwrap()
            .0
            .y;
        assert_eq!(Some(note_y), app.schedule_cursor_y);
        app.commit();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let text = screen(&terminal);
        assert!(text.contains("12:00  lunch"));
        assert!(text.contains("       walk"));
        app.focus = Focus::Memo;
        app.start_edit(false);
        app.editing.as_mut().unwrap().text.insert("in place");
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let text = screen(&terminal);
        assert!(text.contains("in place"));
        assert!(text.contains("12:00  lunch"));
        assert!(!text.contains("╭"));
        let dir = app.store.dir.clone();
        drop(app);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Optional render artifact for reviewing layout without touching personal journals.
    #[test]
    #[ignore = "writes previews to TECHO_PREVIEW_DIR"]
    fn export_previews() {
        let output = std::path::PathBuf::from(
            std::env::var_os("TECHO_PREVIEW_DIR").expect("set TECHO_PREVIEW_DIR"),
        );
        std::fs::create_dir_all(&output).unwrap();
        let mut app = app();
        app.journal.schedule = vec![
            crate::journal::ScheduleEntry {
                offset_minutes: 330,
                text: "Review experiment results\nCompare yesterday's run".into(),
            },
            crate::journal::ScheduleEntry {
                offset_minutes: 660,
                text: "Read a paper".into(),
            },
            crate::journal::ScheduleEntry {
                offset_minutes: 1230,
                text: "Late-night observation".into(),
            },
        ];
        app.journal.tasks = vec![
            crate::journal::Task {
                done: true,
                text: "Start the long experiment".into(),
            },
            crate::journal::Task {
                done: false,
                text: "Write down what changed".into(),
            },
        ];
        app.journal.free_memo = "实验日志 / a little of today\n\n09:30  The overnight run finished.\nThe result is worth another look, but first: write down what happened.\n\n## Questions for tomorrow\n\n- What changes when the initial conditions change?\n- Keep the raw observations alongside the interpretation.\n\nA thought does not need to be a task.\nThere is room here for unfinished ideas, too.".into();
        for (name, width, height, year, editor) in [
            ("day", 120, 40, false, false),
            ("day-small", 80, 24, false, false),
            ("year", 120, 40, true, false),
            ("year-small", 32, 14, true, false),
            ("schedule-editor", 80, 24, false, true),
        ] {
            app.calendar = year.then_some(app.date);
            app.editing = None;
            if editor {
                app.focus = Focus::Schedule;
                app.start_edit(false);
            }
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            let buffer = terminal.backend().buffer();
            let mut text = String::new();
            for y in 0..height {
                let mut x = 0;
                while x < width {
                    let symbol = buffer[(x, y)].symbol();
                    text.push_str(symbol);
                    x += symbol.width().max(1) as u16;
                }
                text.push('\n');
            }
            std::fs::write(output.join(format!("{name}.txt")), text).unwrap();
        }
        let dir = app.store.dir.clone();
        drop(app);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
