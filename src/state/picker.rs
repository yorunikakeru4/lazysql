#[derive(Debug, Default, Clone)]
pub struct PickerState {
    pub open: bool,
    pub selected: usize,
    pub query: String,
}

impl PickerState {
    /// Opens the picker and clears query and selection.
    pub fn open(&mut self) {
        self.open = true;
        self.selected = 0;
        self.query.clear();
    }

    /// Closes the picker and clears query and selection.
    pub fn cancel(&mut self) {
        self.open = false;
    }

    /// Moves selection down, stopping at the end.
    pub fn move_next(&mut self, len: usize) {
        if len == 0 {
            self.selected = 0;
            return;
        }

        self.selected = (self.selected + 1).min(len - 1);
    }

    /// Moves selection up, stopping at the start.
    pub fn move_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Adds a character to the end of the query and resets selection to the top.
    pub fn push_query_char(&mut self, ch: char) {
        self.query.push(ch);
        self.selected = 0;
    }

    /// Removes the last character from the query, if any.
    pub fn pop_query(&mut self) {
        self.query.pop();
        self.selected = 0;
    }

    /// Clears query and selection.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn clamp_selection(&mut self, len: usize) {
        if len == 0 {
            self.selected = 0;
            return;
        }
        self.selected = self.selected.min(len - 1);
    }
}
