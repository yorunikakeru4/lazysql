/// Tracks cursor position in the Inspect screen fields table.
#[derive(Debug, Default)]
pub struct InspectState {
    pub selected: usize,
}

impl InspectState {
    /// Resets cursor to top.
    pub fn reset(&mut self) {
        self.selected = 0;
    }

    /// Moves cursor down within `max` visible items.
    pub fn move_down(&mut self, max: usize) {
        if max > 0 {
            self.selected = (self.selected + 1).min(max - 1);
        }
    }

    /// Moves cursor up, stops at 0.
    pub fn move_up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Clamps cursor after the visible list shrinks (e.g. search filter narrows results).
    pub fn clamp(&mut self, max: usize) {
        if max == 0 {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(max - 1);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn move_down_advances_cursor() {
        let mut s = InspectState::default();
        s.move_down(5);
        assert_eq!(s.selected, 1);
    }

    #[test]
    fn move_down_stops_at_last() {
        let mut s = InspectState { selected: 4 };
        s.move_down(5);
        assert_eq!(s.selected, 4);
    }

    #[test]
    fn move_down_noop_on_empty() {
        let mut s = InspectState::default();
        s.move_down(0);
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn move_up_decrements_cursor() {
        let mut s = InspectState { selected: 3 };
        s.move_up();
        assert_eq!(s.selected, 2);
    }

    #[test]
    fn move_up_stops_at_zero() {
        let mut s = InspectState::default();
        s.move_up();
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn clamp_caps_to_last_item() {
        let mut s = InspectState { selected: 10 };
        s.clamp(3);
        assert_eq!(s.selected, 2);
    }

    #[test]
    fn clamp_on_empty_resets_to_zero() {
        let mut s = InspectState { selected: 5 };
        s.clamp(0);
        assert_eq!(s.selected, 0);
    }

    #[test]
    fn reset_sets_zero() {
        let mut s = InspectState { selected: 7 };
        s.reset();
        assert_eq!(s.selected, 0);
    }
}
