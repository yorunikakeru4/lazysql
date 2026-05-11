#[derive(Debug, Default)]
pub struct ConnectState {
    pub selected: usize,
    pub error: Option<String>,
}

impl ConnectState {
    /// Moves selection down, wrapping around when reaching the end.
    pub fn select_next(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = (self.selected + 1) % len;
    }

    /// Moves selection up, stopping at index 0.
    pub fn select_prev(&mut self, len: usize) {
        if len == 0 {
            return;
        }
        self.selected = self.selected.saturating_sub(1);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn select_next_wraps() {
        let mut state = ConnectState::default();
        state.selected = 1;
        state.select_next(2);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn select_prev_stops_at_zero() {
        let mut state = ConnectState::default();
        state.select_prev(3);
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn select_noop_when_empty() {
        let mut state = ConnectState::default();
        state.select_next(0);
        state.select_prev(0);
        assert_eq!(state.selected, 0);
    }
}
