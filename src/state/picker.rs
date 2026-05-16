#[derive(Debug, Default, Clone)]
pub struct PickerState {
    pub open: bool,
    pub selected: usize,
    pub query: String,
}

impl PickerState {
    pub fn open(&mut self) {
        self.open = true;
        self.selected = 0;
        self.query.clear();
    }

    pub fn cancel(&mut self) {
        self.open = false;
    }

    pub fn move_next(&mut self, len: usize) {
        if len == 0 {
            self.selected = 0;
            return;
        }

        self.selected = (self.selected + 1).min(len - 1);
    }

    pub fn move_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn push_query_char(&mut self, ch: char) {
        self.query.push(ch);
        self.selected = 0;
    }

    pub fn pop_query(&mut self) {
        self.query.pop();
        self.selected = 0;
    }
}
