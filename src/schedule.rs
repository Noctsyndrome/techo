use crate::{editor::TextEditor, journal::Journal};

/// Columns used by the gutter: an item's time, "09:00  ".
pub const GUTTER: u16 = 7;

/// One rendered line of the schedule: a line of an entry, in time order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Row {
    pub entry: usize,
    /// First line of its entry.
    pub first: bool,
    /// The gutter shows the time here: the first line of the first entry at that time.
    pub timed: bool,
    pub text: String,
}

/// Lay the entries out as rows, wrapped to `width`. Entries sharing a time gather
/// under one time label; the lines of one entry follow each other.
pub fn rows(journal: &Journal, width: u16) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut previous = None;
    for (i, entry) in journal.schedule.iter().enumerate() {
        let (lines, _) = TextEditor::new(entry.text.clone()).visual(width.max(1));
        for (n, line) in lines.into_iter().enumerate() {
            rows.push(Row {
                entry: i,
                first: n == 0,
                timed: n == 0 && previous != Some(entry.offset_minutes),
                text: line,
            });
        }
        previous = Some(entry.offset_minutes);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::{ScheduleEntry, parse_time};

    fn entry(time: &str, text: &str) -> ScheduleEntry {
        ScheduleEntry {
            offset_minutes: parse_time(time).unwrap(),
            text: text.into(),
        }
    }

    #[test]
    fn same_times_gather_and_lines_follow_their_entry() {
        let journal = Journal {
            schedule: vec![
                entry("09:00", "morning brief"),
                entry("09:00", "insert note"),
                entry("12:10", "lunch with K\ntried the new place"),
            ],
            ..Journal::default()
        };
        let rows = rows(&journal, 40);
        assert_eq!(rows.len(), 4);
        assert_eq!(
            (rows[0].entry, rows[0].first, rows[0].timed),
            (0, true, true)
        );
        assert_eq!(
            (rows[1].entry, rows[1].first, rows[1].timed),
            (1, true, false)
        );
        assert_eq!(
            (rows[2].entry, rows[2].first, rows[2].timed),
            (2, true, true)
        );
        assert_eq!(
            (rows[3].entry, rows[3].first, rows[3].timed),
            (2, false, false)
        );
        assert_eq!(rows[3].text, "tried the new place");
        assert!(super::rows(&Journal::default(), 4).is_empty());
    }
}
