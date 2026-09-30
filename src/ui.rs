use crossterm::event::{KeyCode, KeyModifiers};

pub fn truncate(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Converts a terminal coordinate to `u16`, saturating instead of truncating.
pub fn to_u16(n: usize) -> u16 {
    u16::try_from(n).unwrap_or(u16::MAX)
}

pub fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Single-line text input with readline/emacs-style editing keys.
#[derive(Default)]
pub struct LineEditor {
    chars: Vec<char>,
    cursor: usize,
    /// Text removed by the last kill command, for `Ctrl-Y`.
    killed: Vec<char>,
}

impl LineEditor {
    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    pub fn clear(&mut self) {
        self.chars.clear();
        self.cursor = 0;
    }

    /// Text before the cursor, the char under it (if any), and the text after it.
    pub fn split_at_cursor(&self) -> (String, Option<char>, String) {
        (
            self.chars[..self.cursor].iter().collect(),
            self.chars.get(self.cursor).copied(),
            self.chars.iter().skip(self.cursor + 1).collect(),
        )
    }

    fn word_start(&self) -> usize {
        let mut i = self.cursor;
        while i > 0 && !self.chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
        while i > 0 && self.chars[i - 1].is_alphanumeric() {
            i -= 1;
        }
        i
    }

    fn word_end(&self) -> usize {
        let mut i = self.cursor;
        while i < self.chars.len() && !self.chars[i].is_alphanumeric() {
            i += 1;
        }
        while i < self.chars.len() && self.chars[i].is_alphanumeric() {
            i += 1;
        }
        i
    }

    /// Removes `range`, remembering it for yanking.
    fn kill(&mut self, from: usize, to: usize) {
        if from < to {
            self.killed = self.chars.drain(from..to).collect();
            self.cursor = from;
        }
    }

    /// Applies an editing key. Returns whether the text changed; unhandled keys change nothing.
    pub fn handle(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        let ctrl = modifiers.contains(KeyModifiers::CONTROL);
        let alt = modifiers.contains(KeyModifiers::ALT);
        let len = self.chars.len();
        match code {
            KeyCode::Char('a') | KeyCode::Home if ctrl || code == KeyCode::Home => {
                self.cursor = 0;
            }
            KeyCode::Char('e') | KeyCode::End if ctrl || code == KeyCode::End => {
                self.cursor = len;
            }
            KeyCode::Char('b') if ctrl => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Left if ctrl || alt => self.cursor = self.word_start(),
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Char('f') if ctrl => self.cursor = (self.cursor + 1).min(len),
            KeyCode::Right if ctrl || alt => self.cursor = self.word_end(),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(len),
            KeyCode::Char('b') if alt => self.cursor = self.word_start(),
            KeyCode::Char('f') if alt => self.cursor = self.word_end(),
            KeyCode::Char('h') if ctrl => return self.backspace(),
            KeyCode::Backspace if alt => {
                self.kill(self.word_start(), self.cursor);
                return true;
            }
            KeyCode::Backspace => return self.backspace(),
            KeyCode::Char('d') if ctrl => return self.delete(),
            KeyCode::Delete => return self.delete(),
            KeyCode::Char('d') if alt => {
                self.kill(self.cursor, self.word_end());
                return true;
            }
            KeyCode::Char('w') if ctrl => {
                self.kill(self.word_start(), self.cursor);
                return true;
            }
            KeyCode::Char('k') if ctrl => {
                self.kill(self.cursor, len);
                return true;
            }
            KeyCode::Char('u') if ctrl => {
                self.kill(0, self.cursor);
                return true;
            }
            KeyCode::Char('y') if ctrl => {
                let yanked = self.killed.clone();
                self.chars
                    .splice(self.cursor..self.cursor, yanked.iter().copied());
                self.cursor += yanked.len();
                return !yanked.is_empty();
            }
            KeyCode::Char(c) if !ctrl && !alt => {
                self.chars.insert(self.cursor, c);
                self.cursor += 1;
                return true;
            }
            _ => {}
        }
        false
    }

    fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.chars.remove(self.cursor);
        true
    }

    fn delete(&mut self) -> bool {
        if self.cursor >= self.chars.len() {
            return false;
        }
        self.chars.remove(self.cursor);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_str(e: &mut LineEditor, s: &str) {
        for c in s.chars() {
            e.handle(KeyCode::Char(c), KeyModifiers::NONE);
        }
    }

    fn ctrl(e: &mut LineEditor, c: char) -> bool {
        e.handle(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn edits_like_readline() {
        let mut e = LineEditor::default();
        type_str(&mut e, "feature/foo bar");
        ctrl(&mut e, 'w');
        assert_eq!(e.text(), "feature/foo ");
        ctrl(&mut e, 'a');
        ctrl(&mut e, 'k');
        assert_eq!(e.text(), "");
        ctrl(&mut e, 'y');
        assert_eq!(e.text(), "feature/foo ");
        ctrl(&mut e, 'a');
        e.handle(KeyCode::Char('f'), KeyModifiers::ALT);
        type_str(&mut e, "!");
        assert_eq!(e.text(), "feature!/foo ");
        ctrl(&mut e, 'e');
        ctrl(&mut e, 'u');
        assert_eq!(e.text(), "");
        assert!(!e.handle(KeyCode::Backspace, KeyModifiers::NONE));
    }
}
