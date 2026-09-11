use chrono::NaiveDate;
use std::io;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Task {
    pub done: bool,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleEntry {
    /// Minutes after 04:00, including the following morning until 03:59.
    pub offset_minutes: u16,
    pub text: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Journal {
    pub tasks: Vec<Task>,
    pub schedule: Vec<ScheduleEntry>,
    pub free_memo: String,
}

pub fn parse_time(value: &str) -> Option<u16> {
    let value = value.trim();
    let (hour, minute) = value.split_once(':')?;
    if hour.len() != 2
        || minute.len() != 2
        || !value.bytes().all(|b| b.is_ascii_digit() || b == b':')
    {
        return None;
    }
    let (hour, minute) = (hour.parse::<u16>().ok()?, minute.parse::<u16>().ok()?);
    (hour < 24 && minute < 60).then(|| (hour * 60 + minute + 1200) % 1440)
}

/// Read a time the way a person types it: "9", "09", "930", "0930", "9:30", "09:30".
pub fn parse_time_input(value: &str) -> Option<u16> {
    let value: String = value.chars().filter(|c| !c.is_whitespace()).collect();
    let (hour, minute) = match value.split_once(':') {
        Some((h, m)) => (h.to_string(), m.to_string()),
        None => match value.len() {
            1 | 2 => (value.clone(), "00".into()),
            3 => (value[..1].into(), value[1..].into()),
            4 => (value[..2].into(), value[2..].into()),
            _ => return None,
        },
    };
    if hour.is_empty() || hour.len() > 2 || minute.len() != 2 {
        return None;
    }
    parse_time(&format!(
        "{:02}:{}",
        hour.parse::<u16>().ok().filter(|h| *h < 24)?,
        minute
    ))
}

pub fn clock_time(offset: u16) -> String {
    let clock = (offset + 240) % 1440;
    format!("{:02}:{:02}", clock / 60, clock % 60)
}

pub fn format_time(offset: u16) -> String {
    format!(
        "{}{}",
        clock_time(offset),
        if offset >= 1200 { " (+1)" } else { "" }
    )
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl Journal {
    pub fn to_markdown(&self, date: NaiveDate) -> String {
        let mut out = format!(
            "---\ndate: {date}\ntecho-format: 2\ntags:\n  - techo\n---\n\n# {date}\n\n## TODO\n"
        );
        for task in &self.tasks {
            let mut lines = task.text.split('\n');
            out.push_str(&format!(
                "- [{}] {}\n",
                if task.done { "x" } else { " " },
                lines.next().unwrap_or("")
            ));
            for line in lines {
                out.push_str(&format!("  {line}\n"));
            }
        }
        out.push_str("\n## Schedule\n");
        for entry in &self.schedule {
            out.push_str(&format!("### {}\n", format_time(entry.offset_minutes)));
            // Indentation makes headings inside the text unambiguous Markdown code blocks.
            for line in entry.text.split('\n') {
                out.push_str(&format!("    {line}\n"));
            }
        }
        out.push_str("\n## Free Memo\n");
        out.push_str(&self.free_memo);
        out
    }

    pub fn from_markdown(raw: &str) -> io::Result<Self> {
        let text = raw.replace("\r\n", "\n");
        let frontmatter = text
            .strip_prefix("---\n")
            .and_then(|s| s.split_once("\n---\n").map(|p| p.0));
        let version =
            frontmatter.and_then(|s| s.lines().find_map(|l| l.strip_prefix("techo-format: ")));
        match version {
            Some("2") => Self::parse_v2(&text),
            Some(_) => Err(invalid("Unsupported techo format; file left untouched")),
            None => Self::parse_legacy(&text),
        }
    }

    fn parse_v2(text: &str) -> io::Result<Self> {
        let (_, body) = text
            .split_once("\n## TODO\n")
            .ok_or_else(|| invalid("Missing TODO section"))?;
        let (todos, body) = body
            .split_once("\n## Schedule\n")
            .ok_or_else(|| invalid("Missing Schedule section"))?;
        let (schedule, memo) = body
            .split_once("\n## Free Memo\n")
            .ok_or_else(|| invalid("Missing Free Memo section"))?;
        let mut journal = Self {
            free_memo: memo.to_string(),
            ..Self::default()
        };
        for line in todos.lines() {
            if let Some(text) = line
                .strip_prefix("- [ ] ")
                .or_else(|| line.strip_prefix("- [x] "))
            {
                journal.tasks.push(Task {
                    done: line.starts_with("- [x]"),
                    text: text.into(),
                });
            } else if let Some(text) = line.strip_prefix("  ") {
                let task = journal
                    .tasks
                    .last_mut()
                    .ok_or_else(|| invalid("TODO continuation without a task"))?;
                task.text.push('\n');
                task.text.push_str(text);
            } else if !line.is_empty() {
                return Err(invalid(
                    "Unrecognized TODO content; use indented continuation lines",
                ));
            }
        }
        let mut body_lines = 0;
        for line in schedule.lines() {
            if let Some(time) = line.strip_prefix("### ") {
                let time = time.strip_suffix(" (+1)").unwrap_or(time);
                let offset_minutes =
                    parse_time(time).ok_or_else(|| invalid("Invalid schedule time"))?;
                journal.schedule.push(ScheduleEntry {
                    offset_minutes,
                    text: String::new(),
                });
                body_lines = 0;
            } else if let Some(text) = line.strip_prefix("    ") {
                let entry = journal
                    .schedule
                    .last_mut()
                    .ok_or_else(|| invalid("Schedule text without a time"))?;
                if body_lines > 0 {
                    entry.text.push('\n');
                }
                entry.text.push_str(text);
                body_lines += 1;
            } else if !line.is_empty() {
                return Err(invalid(
                    "Unrecognized schedule content; indent body by four spaces",
                ));
            }
        }
        journal.schedule.sort_by_key(|e| e.offset_minutes);
        Ok(journal)
    }

    fn parse_legacy(text: &str) -> io::Result<Self> {
        let mut journal = Self::default();
        let mut section = "";
        let mut recognized = false;
        let mut memo_lines = Vec::new();
        for line in text.lines() {
            // Free Memo is the final section: its own headings are ordinary content.
            if section == "memo" {
                memo_lines.push(line);
                continue;
            }
            match line {
                "## TODO" => {
                    section = "todo";
                    recognized = true;
                }
                "## Schedule" | "## Timeline" => {
                    section = "schedule";
                    recognized = true;
                }
                "## Timeline Memo" => {
                    section = "legacy";
                    recognized = true;
                }
                "## Free Memo" => {
                    section = "memo";
                    recognized = true;
                }
                _ if section == "todo" => {
                    if let Some(text) = line
                        .strip_prefix("- [ ] ")
                        .or_else(|| line.strip_prefix("- [x] "))
                    {
                        journal.tasks.push(Task {
                            done: line.starts_with("- [x]"),
                            text: text.into(),
                        });
                    } else if !line.is_empty()
                        && let Some(task) = journal.tasks.last_mut()
                    {
                        task.text.push('\n');
                        task.text.push_str(line);
                    }
                }
                _ if section == "schedule" => {
                    if let Some(time) = line
                        .strip_prefix("### ")
                        .and_then(|s| s.split_whitespace().next())
                        .and_then(parse_time)
                    {
                        journal.schedule.push(ScheduleEntry {
                            offset_minutes: time,
                            text: String::new(),
                        });
                    } else if !line.is_empty() {
                        if journal.schedule.is_empty() {
                            journal.schedule.push(ScheduleEntry {
                                offset_minutes: 120,
                                text: String::new(),
                            });
                        }
                        let entry = journal.schedule.last_mut().unwrap();
                        if !entry.text.is_empty() {
                            entry.text.push('\n');
                        }
                        entry.text.push_str(line);
                    }
                }
                _ if section == "legacy" && !line.is_empty() => {
                    let (offset_minutes, text) = line
                        .split_once("  ")
                        .and_then(|(t, s)| parse_time(t).map(|t| (t, s)))
                        .unwrap_or((120, line));
                    journal.schedule.push(ScheduleEntry {
                        offset_minutes,
                        text: text.into(),
                    });
                }
                _ => {}
            }
        }
        if !recognized && !text.trim().is_empty() {
            return Err(invalid("Not a techo journal; refusing to replace it"));
        }
        journal.free_memo = memo_lines.join("\n").trim_matches('\n').into();
        journal.schedule.sort_by_key(|e| e.offset_minutes);
        Ok(journal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arbitrary_multiline_text_round_trips() {
        let text =
            "\n中文 📝\n## TODO\n## Schedule\n### 09:00\n- [x] nested\n  indented\n\nend\n\n";
        let journal = Journal {
            tasks: vec![Task {
                done: true,
                text: text.into(),
            }],
            schedule: vec![
                ScheduleEntry {
                    offset_minutes: 1230,
                    text: text.into(),
                },
                ScheduleEntry {
                    offset_minutes: 1230,
                    text: "same time".into(),
                },
            ],
            free_memo: text.into(),
        };
        assert_eq!(
            Journal::from_markdown(
                &journal.to_markdown(NaiveDate::from_ymd_opt(2026, 9, 11).unwrap())
            )
            .unwrap(),
            journal
        );
    }
    #[test]
    fn legacy_preserves_multiline_task_and_memo_headings() {
        let j = Journal::from_markdown("## TODO\n- [ ] first\nsecond\n\n## Timeline\n### 09:00\nwork\n\n## Free Memo\nnotes\n## TODO\nmore").unwrap();
        assert_eq!(j.tasks[0].text, "first\nsecond");
        assert_eq!(j.schedule[0].offset_minutes, 300);
        assert_eq!(j.free_memo, "notes\n## TODO\nmore");
    }
    #[test]
    fn typed_times_are_read_leniently() {
        for (typed, clock) in [
            ("9", "09:00"),
            ("09", "09:00"),
            ("930", "09:30"),
            ("0930", "09:30"),
            ("9:30", "09:30"),
            (" 21:05 ", "21:05"),
            ("2130", "21:30"),
            ("0", "00:00"),
        ] {
            assert_eq!(
                parse_time_input(typed).map(clock_time),
                Some(clock.into()),
                "{typed}"
            );
        }
        for invalid in ["", "25", "960", "12:60", "1:5", "12345", "ab", "9:"] {
            assert_eq!(parse_time_input(invalid), None, "{invalid}");
        }
    }
    #[test]
    fn strict_times_and_paper_day() {
        for invalid in ["24:00", "12:60", "9:00", "09:00 junk", "-1:00"] {
            assert_eq!(parse_time(invalid), None);
        }
        assert_eq!(parse_time("04:00"), Some(0));
        assert_eq!(format_time(parse_time("00:30").unwrap()), "00:30 (+1)");
    }
}
