use crate::{
    app::{Action, App, EditTarget, Focus},
    calendar::{MONTHS, moon},
    editor::TextEditor,
    journal::format_time,
};
use chrono::{Datelike, Local, NaiveDate};
use crossterm::event::KeyCode;
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};
use unicode_width::UnicodeWidthChar;

fn accent() -> Style {
    Style::default().fg(Color::Rgb(176, 192, 155))
}
fn muted() -> Style {
    Style::default().fg(Color::DarkGray)
}
fn block(title: String, active: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(if active {
            BorderType::Thick
        } else {
            BorderType::Plain
        })
        .border_style(if active { accent() } else { muted() })
        .title(Line::styled(
            title,
            if active {
                accent().bold()
            } else {
                Style::default()
            },
        ))
}
fn panel_title(name: &str, shortcut: char, active: bool) -> String {
    format!(" {}{} [{shortcut}] ", if active { "> " } else { "" }, name)
}
fn put(frame: &mut Frame, rect: Rect, text: impl Into<Text<'static>>) {
    frame.render_widget(Paragraph::new(text), rect);
}
fn clip(text: &str, width: u16) -> String {
    let mut out = String::new();
    let mut used = 0;
    let total = unicode_width::UnicodeWidthStr::width(text);
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
fn button(
    frame: &mut Frame,
    hits: &mut Vec<(Rect, Action)>,
    x: u16,
    y: u16,
    label: &str,
    action: Action,
    max_x: u16,
) -> u16 {
    let width = (label.len() as u16).min(max_x.saturating_sub(x));
    if width > 0 {
        let rect = Rect::new(x, y, width, 1);
        put(
            frame,
            rect,
            Line::styled(label.to_string(), accent().bold()),
        );
        hits.push((rect, action));
    }
    x.saturating_add(width + 1)
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    app.hits.clear();
    let area = frame.area();
    if area.width < 32 || area.height < 14 {
        put(frame, area, "techō\nPlease resize to at least 32 x 14.\nYour draft is retained.\nq: quit (outside editor)".to_string());
        return;
    }
    // A clean editor backdrop also avoids partial wide glyphs at popup edges.
    if app.editing.is_some() {
        draw_editor(frame, app);
        return;
    }
    if app.calendar.is_some() {
        draw_year(frame, app);
    } else {
        draw_day(frame, app);
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
    let rows = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(6),
        Constraint::Length(4),
    ])
    .split(area);
    let (symbol, phase) = moon(app.date);
    let header = block(" techō · a little space for your day ".into(), false);
    let inner = header.inner(rows[0]);
    frame.render_widget(header, rows[0]);
    let weekday = ["月", "火", "水", "木", "金", "土", "日"]
        [app.date.weekday().num_days_from_monday() as usize];
    let date = format!(
        " {} ({}) · day {}",
        app.date.format("%Y 年 %m 月 %d 日"),
        weekday,
        app.date.ordinal()
    );
    put(
        frame,
        Rect::new(inner.x, inner.y, inner.width, 1),
        Line::styled(date, Style::default().bold()),
    );
    let moon = format!(" {symbol} {phase} (approx.)");
    put(
        frame,
        Rect::new(inner.x, inner.y + 1, inner.width, 1),
        Line::styled(moon, accent()),
    );
    if inner.width >= 62 {
        let x = inner.right() - 30;
        let x = button(
            frame,
            &mut app.hits,
            x,
            inner.y + 1,
            "[y Year]",
            Action::Calendar,
            inner.right(),
        );
        button(
            frame,
            &mut app.hits,
            x,
            inner.y + 1,
            "[g Date]",
            Action::Key(KeyCode::Char('g')),
            inner.right(),
        );
    }
    let body = rows[1];
    if area.width >= 86 && body.height >= 10 {
        let columns = Layout::horizontal([Constraint::Min(40), Constraint::Length(32)]).split(body);
        let schedule_h = (app.journal.schedule.len().saturating_add(2).min(10) as u16)
            .clamp(4, (body.height / 3).clamp(4, 10));
        let left = Layout::vertical([Constraint::Length(schedule_h), Constraint::Min(3)])
            .split(columns[0]);
        draw_list(frame, app, left[0], Focus::Schedule);
        draw_memo(frame, app, left[1]);
        if body.height >= 18 {
            let right = Layout::vertical([
                Constraint::Min(4),
                Constraint::Length(10),
                Constraint::Length(3),
            ])
            .split(columns[1]);
            draw_list(frame, app, right[0], Focus::Todo);
            draw_month(
                frame,
                &mut app.hits,
                right[1],
                app.date,
                app.date,
                app.date,
                true,
            );
            frame.render_widget(
                Paragraph::new("A day is a little life.").block(block(" words ".into(), false)),
                right[2],
            );
        } else {
            draw_list(frame, app, columns[1], Focus::Todo);
        }
    } else if area.width >= 60 && body.height >= 10 {
        let sections = Layout::vertical([
            Constraint::Length((body.height / 3).max(4)),
            Constraint::Min(5),
        ])
        .split(body);
        let top = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(sections[0]);
        draw_list(frame, app, top[0], Focus::Schedule);
        draw_list(frame, app, top[1], Focus::Todo);
        draw_memo(frame, app, sections[1]);
    } else {
        // Narrow terminals keep the active panel readable; all sections stay keyboard-accessible.
        match app.focus {
            Focus::Memo => draw_memo(frame, app, body),
            focus => draw_list(frame, app, body, focus),
        }
    }
    let footer = rows[2];
    let mut x = footer.x;
    for (label, action) in [
        ("[s Schedule]", Action::Focus(Focus::Schedule)),
        ("[t Todo]", Action::Focus(Focus::Todo)),
        ("[f Memo]", Action::Focus(Focus::Memo)),
        ("[y Year]", Action::Calendar),
    ] {
        x = button(
            frame,
            &mut app.hits,
            x,
            footer.y,
            label,
            action,
            footer.right(),
        );
    }
    let hint = match app.focus {
        Focus::Schedule => "n Add · Enter Edit · d Delete · Up/Down Select",
        Focus::Todo => "n Add · Enter Edit · Space Check · d Delete · Up/Down Select",
        Focus::Memo => "Enter/e Write · Up/Down or wheel Scroll",
    };
    put(
        frame,
        Rect::new(footer.x, footer.y + 1, footer.width, 1),
        hint.to_string(),
    );
    put(
        frame,
        Rect::new(footer.x, footer.y + 2, footer.width, 1),
        "Tab Switch · y Year · g Date · [/] Day · Home Today · ? Help · q Quit".to_string(),
    );
    put(
        frame,
        Rect::new(footer.x, footer.y + 3, footer.width, 1),
        Line::styled(app.status.clone(), accent()),
    );
}

fn draw_list(frame: &mut Frame, app: &mut App, area: Rect, focus: Focus) {
    let active = app.focus == focus;
    let (name, key, count, selected, offset) = match focus {
        Focus::Schedule => (
            "schedule",
            's',
            app.journal.schedule.len(),
            app.selected_schedule,
            &mut app.schedule_offset,
        ),
        _ => (
            "todo",
            't',
            app.journal.tasks.len(),
            app.selected_task,
            &mut app.task_offset,
        ),
    };
    let title = format!(
        "{} {}/{} ",
        panel_title(name, key, active),
        if count == 0 { 0 } else { selected + 1 },
        count
    );
    let border = block(title, active);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    app.hits.push((area, Action::Focus(focus)));
    let height = inner.height as usize;
    if height == 0 {
        return;
    }
    *offset = (*offset).min(count.saturating_sub(height));
    if selected < *offset {
        *offset = selected;
    }
    if selected >= *offset + height {
        *offset = selected + 1 - height;
    }
    if count == 0 {
        put(
            frame,
            inner,
            Line::styled(
                if focus == Focus::Schedule {
                    "n  Add a time + item"
                } else {
                    "n  Add a todo"
                },
                muted(),
            ),
        );
        return;
    }
    for (row, i) in (*offset..count).take(height).enumerate() {
        let (prefix, text) = if focus == Focus::Schedule {
            let entry = &app.journal.schedule[i];
            (
                format!("{:<12} ", format_time(entry.offset_minutes)),
                entry.text.as_str(),
            )
        } else {
            let task = &app.journal.tasks[i];
            (
                format!("[{}] ", if task.done { 'x' } else { ' ' }),
                task.text.as_str(),
            )
        };
        let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
        let preview = format!(
            "{}{}{}",
            if active && i == selected { "> " } else { "  " },
            prefix,
            first
        );
        let multiline = if text.contains('\n') { " ↵" } else { "" };
        let preview = format!(
            "{}{}",
            clip(
                &preview,
                inner
                    .width
                    .saturating_sub(if multiline.is_empty() { 0 } else { 2 })
            ),
            multiline
        );
        let rect = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);
        let style = if active && i == selected {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
        };
        put(
            frame,
            rect,
            Line::styled(clip(&preview, inner.width), style),
        );
        app.hits.push((rect, Action::Select(focus, i)));
    }
}

fn draw_memo(frame: &mut Frame, app: &mut App, area: Rect) {
    let border = block(
        panel_title("free memo", 'f', app.focus == Focus::Memo),
        app.focus == Focus::Memo,
    );
    let inner = border.inner(area);
    frame.render_widget(border, area);
    app.hits.push((area, Action::Focus(Focus::Memo)));
    if app.journal.free_memo.is_empty() {
        put(
            frame,
            inner,
            Line::styled(
                "Enter to write. A thought, an experiment, a little of today.",
                muted(),
            ),
        );
        return;
    }
    let (lines, _) = TextEditor::new(app.journal.free_memo.clone()).visual(inner.width);
    app.memo_scroll = app
        .memo_scroll
        .min(lines.len().saturating_sub(inner.height as usize));
    let text = lines
        .into_iter()
        .skip(app.memo_scroll)
        .take(inner.height as usize)
        .map(Line::from)
        .collect::<Vec<_>>();
    put(frame, inner, text);
}

fn draw_month(
    frame: &mut Frame,
    hits: &mut Vec<(Rect, Action)>,
    area: Rect,
    month: NaiveDate,
    selected: NaiveDate,
    opened: NaiveDate,
    main: bool,
) {
    let title = if main {
        format!(" {} · y Year ", month.format("%Y-%m"))
    } else {
        format!(" {} ", MONTHS[month.month0() as usize])
    };
    let border = block(title, !main && selected.month() == month.month());
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if main {
        hits.push((area, Action::Calendar));
    }
    put(
        frame,
        Rect::new(inner.x, inner.y, inner.width, 1),
        " Mo  Tu  We  Th  Fr  Sa  Su".to_string(),
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
        let label = if date == selected {
            format!("[{day:02}]")
        } else if date == today {
            format!("({day:02})")
        } else if date == opened {
            format!("{{{day:02}}}")
        } else {
            format!(" {day:02} ")
        };
        let style = if date == selected {
            accent().reversed().bold()
        } else if date == today {
            accent().bold()
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
        Constraint::Length(2),
        Constraint::Min(8),
        Constraint::Length(3),
    ])
    .split(area);
    put(
        frame,
        Rect::new(area.x, area.y, area.width, 1),
        Line::styled(
            format!(" techō · {} · choose a day", selected.year()),
            accent().bold(),
        ),
    );
    let mut x = area.x;
    for (label, key) in [
        ("[<Year]", '['),
        ("[Year>]", ']'),
        ("[Today]", 't'),
        ("[g Date]", 'g'),
    ] {
        x = button(
            frame,
            &mut app.hits,
            x,
            area.y + 1,
            label,
            Action::Key(KeyCode::Char(key)),
            area.right(),
        );
    }
    let grid = rows[1];
    let cols = (grid.width / 30).clamp(1, 4);
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
        draw_month(
            frame,
            &mut app.hits,
            rect,
            NaiveDate::from_ymd_opt(selected.year(), month, 1).unwrap(),
            selected,
            app.date,
            false,
        );
    }
    let footer = rows[2];
    let x = button(
        frame,
        &mut app.hits,
        footer.x,
        footer.y,
        "[PgUp]",
        Action::Key(KeyCode::PageUp),
        footer.right(),
    );
    let x = button(
        frame,
        &mut app.hits,
        x,
        footer.y,
        "[PgDn]",
        Action::Key(KeyCode::PageDown),
        footer.right(),
    );
    button(
        frame,
        &mut app.hits,
        x,
        footer.y,
        "[Esc Back]",
        Action::Key(KeyCode::Esc),
        footer.right(),
    );
    put(
        frame,
        Rect::new(footer.x, footer.y + 1, footer.width, 1),
        if footer.width < 60 {
            "Arrows: pick · Enter/click: open".into()
        } else {
            format!("{} · Arrows: select · Enter/click: open", selected)
        },
    );
    put(
        frame,
        Rect::new(footer.x, footer.y + 2, footer.width, 1),
        if app.status.is_empty() {
            if footer.width < 60 {
                "[dd] Selected · (dd) Today".into()
            } else {
                "[dd] Selected · (dd) Today · {dd} Open day".into()
            }
        } else {
            app.status.clone()
        },
    );
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

fn draw_editor(frame: &mut Frame, app: &mut App) {
    app.hits.clear();
    let edit = app.editing.as_ref().unwrap();
    let schedule = matches!(edit.target, EditTarget::Schedule(_));
    let jump = edit.target == EditTarget::Jump;
    let area = popup(
        frame.area(),
        92,
        if jump {
            9
        } else {
            frame.area().height.saturating_sub(2)
        },
    );
    frame.render_widget(Clear, area);
    let title = match edit.target {
        EditTarget::Schedule(None) => " Add schedule ",
        EditTarget::Schedule(_) => " Edit schedule ",
        EditTarget::Task(None) => " Add todo ",
        EditTarget::Task(_) => " Edit todo ",
        EditTarget::Memo => " Write free memo ",
        EditTarget::Jump => " Go to date · YYYY-MM-DD ",
    };
    let border = block(title.into(), true);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let parts = Layout::vertical([
        Constraint::Length(if schedule { 3 } else { 0 }),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(inner);
    if schedule {
        let time_border = block(
            format!(
                " {}Time HH:MM · 00:00-03:59 = next day ",
                if edit.time_active { "> " } else { "" }
            ),
            edit.time_active,
        );
        let field = time_border.inner(parts[0]);
        frame.render_widget(time_border, parts[0]);
        put(frame, field, edit.time.text.clone());
        if edit.time_active && field.width > 0 {
            frame.set_cursor_position((
                field.x + (edit.time.cursor as u16).min(field.width - 1),
                field.y,
            ));
        }
        app.hits.push((parts[0], Action::TimeField));
    }
    let body_border = block(
        if schedule {
            format!(
                " {}Item · Enter for new line ",
                if edit.time_active { "" } else { "> " }
            )
        } else {
            " Text ".into()
        },
        !edit.time_active,
    );
    let body = body_border.inner(parts[1]);
    frame.render_widget(body_border, parts[1]);
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
    if !edit.time_active && body.width > 0 && body.height > 0 {
        frame.set_cursor_position((
            body.x + cx.min(body.width - 1),
            body.y + (cy - scroll) as u16,
        ));
    }
    app.hits.push((parts[1], Action::BodyField));
    let foot = parts[2];
    put(
        frame,
        Rect::new(foot.x, foot.y, foot.width, 1),
        Line::styled(edit.error.clone(), Style::default().fg(Color::Red)),
    );
    let x = button(
        frame,
        &mut app.hits,
        foot.x,
        foot.y + 1,
        if jump {
            "[Enter Open]"
        } else {
            "[Ctrl+S Save]"
        },
        Action::Save,
        foot.right(),
    );
    button(
        frame,
        &mut app.hits,
        x,
        foot.y + 1,
        "[Esc Cancel]",
        Action::Cancel,
        foot.right(),
    );
    put(
        frame,
        Rect::new(foot.x, foot.y + 2, foot.width, 1),
        if schedule {
            "Tab: time / item · Enter: new line · Arrows/Home/End: cursor"
        } else {
            "Enter: new line · Arrows/Home/End: cursor · Ctrl+S: save"
        }
        .to_string(),
    );
}

fn draw_help(frame: &mut Frame, app: &mut App) {
    app.hits.clear();
    let area = popup(frame.area(), 78, 20);
    frame.render_widget(Clear, area);
    let text = format!(
        "s Schedule   t Todo   f Free memo   Tab/Shift+Tab Switch\nClick a panel to select it; click a list row to select an item.\nn Add   Enter/e Edit   d Delete   Space Check todo\nUp/Down or mouse wheel: select items / scroll memo\n\ny Year calendar   g Go to YYYY-MM-DD   [/] Previous/next day\nHome Today   q Quit\nYear: arrows select; Enter/click opens; [/] changes year.\nPgUp/PgDn or wheel: calendar pages. Esc returns.\n\nEditor: Ctrl+S saves; Esc cancels. Enter adds a line.\nSchedule: Tab switches time / item. 00:00-03:59 is next day.\nPaste supported. Long text scrolls with the cursor.\nMoon: approximate phase at 12:00 UTC on the selected date.\n\nFiles: {}\n\nPress any key to close.",
        app.store.dir.display()
    );
    frame.render_widget(
        Paragraph::new(text).block(block(" Help ".into(), true)),
        area,
    );
    app.hits.push((area, Action::Key(KeyCode::Esc)));
}

fn draw_delete(frame: &mut Frame, app: &mut App) {
    app.hits.clear();
    let area = popup(frame.area(), 46, 5);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new("Delete the selected item?\nThis change will be saved immediately.")
            .block(block(" Confirm delete ".into(), true)),
        area,
    );
    let x = button(
        frame,
        &mut app.hits,
        area.x + 1,
        area.bottom() - 2,
        "[y Delete]",
        Action::Key(KeyCode::Char('y')),
        area.right() - 1,
    );
    button(
        frame,
        &mut app.hits,
        x,
        area.bottom() - 2,
        "[Esc Cancel]",
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
        App::open(test_dir(), parse_date("2026-09-11").unwrap()).unwrap()
    }
    fn click(app: &mut App, rect: Rect) {
        app.event(Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: rect.x,
            row: rect.y,
            modifiers: KeyModifiers::NONE,
        }));
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
            app.calendar = Some(app.date);
            terminal.draw(|f| draw(f, &mut app)).unwrap();
            app.calendar = None;
        }
        let dir = app.store.dir.clone();
        drop(app);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn mouse_panel_and_calendar_hit_testing_after_resize() {
        let mut app = app();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let rect = app
            .hits
            .iter()
            .find(|(_, a)| matches!(a, Action::Focus(Focus::Schedule)))
            .unwrap()
            .0;
        click(&mut app, rect);
        assert_eq!(app.focus, Focus::Schedule);
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("> schedule"));
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
    fn long_lists_scroll_without_overlapping_other_panels() {
        let mut app = app();
        for i in 0..30 {
            app.journal.schedule.push(crate::journal::ScheduleEntry {
                offset_minutes: i * 30,
                text: format!("Item {i}\n{}", "Long body ".repeat(80)),
            });
        }
        app.focus = Focus::Schedule;
        app.selected_schedule = 29;
        app.journal.free_memo = "Memo stays visible".into();
        let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("Item 29"));
        assert!(text.contains("Memo stays visible"));
        assert!(!text.contains("Long body"));
        app.start_edit(false);
        terminal.draw(|f| draw(f, &mut app)).unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("Long body"));
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
                    x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
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
