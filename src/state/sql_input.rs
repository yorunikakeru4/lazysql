/// Result of SQL execution displayed in a popup.
#[derive(Debug, Clone)]
pub enum SqlResult {
    Success {
        rows_affected: u64,
    },
    /// Fallback: sqlparser missed a returning query; rows were returned but not displayed.
    Rows {
        count: usize,
    },
    Error(String),
}

/// State for the SQL command input bar.
#[derive(Debug, Default)]
pub struct SqlInputState {
    pub query: String,
    pub cursor_pos: usize,
    pub active: bool,
    pub result: Option<SqlResult>,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
}

impl SqlInputState {
    /// Activates the SQL input bar.
    pub fn open(&mut self) {
        self.active = true;
    }

    /// Closes the SQL input bar (keeps query for execution).
    pub fn close(&mut self) {
        self.active = false;
    }

    /// Resets all state (cancel/dismiss).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Dismisses the result popup only.
    pub fn dismiss_result(&mut self) {
        self.result = None;
    }

    /// Returns true if a result popup is showing.
    pub fn has_result(&self) -> bool {
        self.result.is_some()
    }

    /// Insert char at cursor_pos.
    pub fn insert_char(&mut self, c: char) {
        self.query.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    /// Insert newline at cursor_pos.
    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    /// Delete char before cursor (Backspace).
    pub fn delete_before(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let prev = self.prev_char_boundary();
        self.query.drain(prev..self.cursor_pos);
        self.cursor_pos = prev;
    }

    /// Delete char at cursor (Delete key).
    pub fn delete_at(&mut self) {
        if self.cursor_pos >= self.query.len() {
            return;
        }
        let next = self.next_char_boundary();
        self.query.drain(self.cursor_pos..next);
    }

    /// Move cursor one character to the left.
    pub fn move_left(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        self.cursor_pos = self.prev_char_boundary();
    }

    /// Move cursor one character to the right.
    pub fn move_right(&mut self) {
        if self.cursor_pos >= self.query.len() {
            return;
        }
        self.cursor_pos = self.next_char_boundary();
    }

    /// Move cursor one line up, preserving column.
    pub fn move_up(&mut self) {
        let (line, col) = self.cursor_line_col();
        if line == 0 {
            return;
        }
        let lines: Vec<&str> = self.query.split('\n').collect();
        let new_col = col.min(lines[line - 1].len());
        self.cursor_pos = lines[..line - 1].iter().map(|l| l.len() + 1).sum::<usize>() + new_col;
    }

    /// Move cursor one line down, preserving column.
    pub fn move_down(&mut self) {
        let (line, col) = self.cursor_line_col();
        let lines: Vec<&str> = self.query.split('\n').collect();
        if line + 1 >= lines.len() {
            return;
        }
        let new_col = col.min(lines[line + 1].len());
        self.cursor_pos =
            lines[..line + 1].iter().map(|l| l.len() + 1).sum::<usize>() + new_col;
    }

    /// Move cursor to the start of the current line.
    pub fn move_line_start(&mut self) {
        let before = &self.query[..self.cursor_pos];
        self.cursor_pos = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    }

    /// Move cursor to the end of the current line.
    pub fn move_line_end(&mut self) {
        let after = &self.query[self.cursor_pos..];
        self.cursor_pos += after.find('\n').unwrap_or(after.len());
    }

    /// Push current query to history (deduplicated, max 100).
    pub fn push_history(&mut self) {
        let q = self.query.trim().to_string();
        if q.is_empty() {
            return;
        }
        self.history.retain(|h| h != &q);
        self.history.push(q);
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        self.history_idx = None;
    }

    /// Load previous history entry (older).
    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_idx {
            None => self.history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.history_idx = Some(idx);
        self.query = self.history[idx].clone();
        self.cursor_pos = self.query.len();
    }

    /// Load next history entry (newer). Clears buffer if past end.
    pub fn history_next(&mut self) {
        let Some(idx) = self.history_idx else {
            return;
        };
        if idx + 1 >= self.history.len() {
            self.history_idx = None;
            self.query.clear();
            self.cursor_pos = 0;
        } else {
            self.history_idx = Some(idx + 1);
            self.query = self.history[idx + 1].clone();
            self.cursor_pos = self.query.len();
        }
    }

    /// Returns (line_index, col_index) of cursor_pos.
    pub fn cursor_line_col(&self) -> (usize, usize) {
        let before = &self.query[..self.cursor_pos.min(self.query.len())];
        let line = before.chars().filter(|&c| c == '\n').count();
        let col = before
            .rfind('\n')
            .map(|i| before.len() - i - 1)
            .unwrap_or(before.len());
        (line, col)
    }

    fn prev_char_boundary(&self) -> usize {
        let mut p = self.cursor_pos.saturating_sub(1);
        while p > 0 && !self.query.is_char_boundary(p) {
            p -= 1;
        }
        p
    }

    fn next_char_boundary(&self) -> usize {
        let mut p = self.cursor_pos + 1;
        while p < self.query.len() && !self.query.is_char_boundary(p) {
            p += 1;
        }
        p.min(self.query.len())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn default_state_is_inactive() {
        let state = SqlInputState::default();
        assert!(!state.active);
        assert!(state.query.is_empty());
        assert!(state.result.is_none());
    }

    #[test]
    fn open_sets_active() {
        let mut state = SqlInputState::default();
        state.open();
        assert!(state.active);
    }

    #[test]
    fn close_preserves_query() {
        let mut state = SqlInputState::default();
        state.query = "SELECT 1".to_string();
        state.open();
        state.close();
        assert!(!state.active);
        assert_eq!(state.query, "SELECT 1");
    }

    #[test]
    fn reset_clears_all() {
        let mut state = SqlInputState::default();
        state.query = "SELECT 1".to_string();
        state.active = true;
        state.result = Some(SqlResult::Rows { count: 1 });
        state.reset();
        assert!(!state.active);
        assert!(state.query.is_empty());
        assert!(state.result.is_none());
    }

    #[test]
    fn dismiss_result_clears_only_result() {
        let mut state = SqlInputState::default();
        state.query = "SELECT 1".to_string();
        state.result = Some(SqlResult::Rows { count: 1 });
        state.dismiss_result();
        assert!(state.result.is_none());
        assert_eq!(state.query, "SELECT 1");
    }

    #[test]
    fn insert_char_appends_and_advances_cursor() {
        let mut s = SqlInputState::default();
        s.insert_char('a');
        s.insert_char('b');
        assert_eq!(s.query, "ab");
        assert_eq!(s.cursor_pos, 2);
    }

    #[test]
    fn insert_newline_splits_lines() {
        let mut s = SqlInputState::default();
        s.insert_char('a');
        s.insert_newline();
        s.insert_char('b');
        assert_eq!(s.query, "a\nb");
        assert_eq!(s.cursor_pos, 3);
    }

    #[test]
    fn delete_before_removes_char() {
        let mut s = SqlInputState::default();
        s.query = "ab".into();
        s.cursor_pos = 2;
        s.delete_before();
        assert_eq!(s.query, "a");
        assert_eq!(s.cursor_pos, 1);
    }

    #[test]
    fn delete_before_noop_at_start() {
        let mut s = SqlInputState::default();
        s.query = "a".into();
        s.cursor_pos = 0;
        s.delete_before();
        assert_eq!(s.query, "a");
        assert_eq!(s.cursor_pos, 0);
    }

    #[test]
    fn move_left_decrements_cursor() {
        let mut s = SqlInputState::default();
        s.query = "ab".into();
        s.cursor_pos = 2;
        s.move_left();
        assert_eq!(s.cursor_pos, 1);
    }

    #[test]
    fn move_left_stops_at_zero() {
        let mut s = SqlInputState::default();
        s.query = "a".into();
        s.cursor_pos = 0;
        s.move_left();
        assert_eq!(s.cursor_pos, 0);
    }

    #[test]
    fn move_right_increments_cursor() {
        let mut s = SqlInputState::default();
        s.query = "ab".into();
        s.cursor_pos = 0;
        s.move_right();
        assert_eq!(s.cursor_pos, 1);
    }

    #[test]
    fn move_right_stops_at_end() {
        let mut s = SqlInputState::default();
        s.query = "ab".into();
        s.cursor_pos = 2;
        s.move_right();
        assert_eq!(s.cursor_pos, 2);
    }

    #[test]
    fn move_up_goes_to_prev_line_same_col() {
        let mut s = SqlInputState::default();
        s.query = "abc\nde".into();
        s.cursor_pos = 5; // 'e'
        s.move_up();
        assert_eq!(s.cursor_pos, 1); // 'b' in "abc"
    }

    #[test]
    fn move_up_noop_on_first_line() {
        let mut s = SqlInputState::default();
        s.query = "abc".into();
        s.cursor_pos = 2;
        s.move_up();
        assert_eq!(s.cursor_pos, 2);
    }

    #[test]
    fn move_down_goes_to_next_line_same_col() {
        let mut s = SqlInputState::default();
        s.query = "abc\nde".into();
        s.cursor_pos = 1; // 'b'
        s.move_down();
        assert_eq!(s.cursor_pos, 5); // 'e' (col 1 in "de")
    }

    #[test]
    fn move_line_start_goes_to_line_beginning() {
        let mut s = SqlInputState::default();
        s.query = "abc\nde".into();
        s.cursor_pos = 5;
        s.move_line_start();
        assert_eq!(s.cursor_pos, 4); // start of "de"
    }

    #[test]
    fn move_line_end_goes_to_line_end() {
        let mut s = SqlInputState::default();
        s.query = "abc\nde".into();
        s.cursor_pos = 4;
        s.move_line_end();
        assert_eq!(s.cursor_pos, 6); // end of "de"
    }

    #[test]
    fn history_records_on_push() {
        let mut s = SqlInputState::default();
        s.query = "SELECT 1".into();
        s.cursor_pos = 8;
        s.push_history();
        assert_eq!(s.history, vec!["SELECT 1"]);
    }

    #[test]
    fn history_deduplicates() {
        let mut s = SqlInputState::default();
        s.query = "SELECT 1".into();
        s.push_history();
        s.query = "SELECT 1".into();
        s.push_history();
        assert_eq!(s.history.len(), 1);
    }

    #[test]
    fn history_prev_loads_entry() {
        let mut s = SqlInputState::default();
        s.history = vec!["SELECT 1".into(), "SELECT 2".into()];
        s.history_prev();
        assert_eq!(s.query, "SELECT 2");
        assert_eq!(s.cursor_pos, s.query.len());
    }

    #[test]
    fn history_next_after_prev_moves_forward() {
        let mut s = SqlInputState::default();
        s.history = vec!["SELECT 1".into(), "SELECT 2".into()];
        s.history_prev();
        s.history_prev();
        s.history_next();
        assert_eq!(s.query, "SELECT 2");
    }
}
