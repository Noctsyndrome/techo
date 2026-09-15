use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use unicode_width::UnicodeWidthChar;

#[derive(Clone, Debug, Default)]
pub struct TextEditor {
    pub text: String,
    pub cursor: usize,
}

impl TextEditor {
    pub fn new(text: String) -> Self {
        let cursor = text.len();
        Self { text, cursor }
    }
    pub fn insert(&mut self, text: &str) {
        let text: String = text
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\t', "    ")
            .chars()
            .filter(|c| *c == '\n' || !c.is_control())
            .collect();
        self.text.insert_str(self.cursor, &text);
        self.cursor += text.len();
    }
    fn previous(&self) -> usize {
        self.text[..self.cursor]
            .char_indices()
            .last()
            .map_or(0, |(i, _)| i)
    }
    fn next(&self) -> usize {
        self.cursor
            + self.text[self.cursor..]
                .chars()
                .next()
                .map_or(0, char::len_utf8)
    }
    fn vertical(&mut self, down: bool) {
        let start = self.text[..self.cursor].rfind('\n').map_or(0, |i| i + 1);
        let column = self.text[start..self.cursor].chars().count();
        let target = if down {
            let Some(end) = self.text[self.cursor..].find('\n') else {
                return;
            };
            self.cursor + end + 1
        } else {
            if start == 0 {
                return;
            }
            self.text[..start - 1].rfind('\n').map_or(0, |i| i + 1)
        };
        let line = self.text[target..].split('\n').next().unwrap_or("");
        self.cursor = target
            + line
                .char_indices()
                .nth(column)
                .map_or(line.len(), |(i, _)| i);
    }
    fn line_start(&self) -> usize {
        self.text[..self.cursor].rfind('\n').map_or(0, |i| i + 1)
    }
    fn line_end(&self) -> usize {
        self.cursor
            + self.text[self.cursor..]
                .find('\n')
                .unwrap_or(self.text.len() - self.cursor)
    }
    /// Movement follows readline: Ctrl+A and Ctrl+E reach the ends of the line on
    /// every keyboard, including those without Home and End.
    pub fn key(&mut self, key: KeyEvent, multiline: bool) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Left => self.cursor = self.previous(),
            KeyCode::Right => self.cursor = self.next(),
            KeyCode::Up => self.vertical(false),
            KeyCode::Down => self.vertical(true),
            KeyCode::Home if ctrl => self.cursor = 0,
            KeyCode::End if ctrl => self.cursor = self.text.len(),
            KeyCode::Home => self.cursor = self.line_start(),
            KeyCode::End => self.cursor = self.line_end(),
            KeyCode::Char('a') if ctrl => self.cursor = self.line_start(),
            KeyCode::Char('e') if ctrl => self.cursor = self.line_end(),
            KeyCode::Backspace => {
                let previous = self.previous();
                self.text.drain(previous..self.cursor);
                self.cursor = previous;
            }
            KeyCode::Delete => {
                let next = self.next();
                self.text.drain(self.cursor..next);
            }
            KeyCode::Enter if multiline => self.insert("\n"),
            KeyCode::Tab if multiline => self.insert("    "),
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.insert(&c.to_string())
            }
            _ => {}
        }
    }
    /// Explicit character wrapping keeps cursor coordinates identical to rendered lines.
    pub fn visual(&self, width: u16) -> (Vec<String>, (u16, usize)) {
        let width = width.max(1);
        let mut lines = vec![String::new()];
        let mut x = 0;
        let mut cursor = (0, 0);
        for (i, c) in self.text.char_indices() {
            let w = c.width().unwrap_or(0) as u16;
            if c != '\n' && x + w > width && x > 0 {
                lines.push(String::new());
                x = 0;
            }
            if i == self.cursor {
                cursor = (x, lines.len() - 1);
            }
            if c == '\n' {
                lines.push(String::new());
                x = 0;
            } else {
                lines.last_mut().unwrap().push(c);
                x += w;
            }
        }
        if self.cursor == self.text.len() {
            if x >= width {
                lines.push(String::new());
                x = 0;
            }
            cursor = (x, lines.len() - 1);
        }
        (lines, cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }
    #[test]
    fn chinese_editing_and_multiline_navigation() {
        let mut e = TextEditor::new("中文\n日志".into());
        e.key(key(KeyCode::Up), true);
        assert_eq!(e.cursor, 6);
        e.key(key(KeyCode::Left), true);
        e.key(key(KeyCode::Delete), true);
        assert_eq!(e.text, "中\n日志");
        e.insert("测试");
        assert_eq!(e.text, "中测试\n日志");
        e.key(key(KeyCode::Backspace), true);
        assert_eq!(e.text, "中测\n日志");
    }
    #[test]
    fn wrapping_tracks_wide_characters_and_trailing_newlines() {
        let e = TextEditor::new("中文ab\n\n".into());
        let (lines, cursor) = e.visual(4);
        assert_eq!(lines, ["中文", "ab", "", ""]);
        assert_eq!(cursor, (0, 3));
        let e = TextEditor::new("中文".into());
        assert_eq!(e.visual(4).1, (0, 1));
    }
    #[test]
    fn ctrl_a_and_ctrl_e_reach_the_ends_of_the_line() {
        let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
        let mut e = TextEditor::new("first\n中文 line".into());
        e.key(ctrl('a'), true);
        assert_eq!(e.cursor, 6);
        e.key(ctrl('e'), true);
        assert_eq!(e.cursor, e.text.len());
        e.key(key(KeyCode::Up), true);
        e.key(ctrl('a'), true);
        assert_eq!(e.cursor, 0);
        e.key(ctrl('e'), true);
        assert_eq!(e.cursor, 5);
        // Ctrl+Home and Ctrl+End still cross lines.
        e.key(KeyEvent::new(KeyCode::End, KeyModifiers::CONTROL), true);
        assert_eq!(e.cursor, e.text.len());
        e.key(KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL), true);
        assert_eq!(e.cursor, 0);
        // Neither inserts a letter.
        assert_eq!(e.text, "first\n中文 line");
    }
    #[test]
    fn paste_normalizes_terminal_controls() {
        let mut e = TextEditor::default();
        e.insert("a\r\nb\t\u{1b}c");
        assert_eq!(e.text, "a\nb    c");
    }
}
