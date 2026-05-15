/// Tracks keyboard picker state for selecting a built-in theme.
#[derive(Debug, Clone, Default)]
pub struct ThemePickerState {
    /// Whether the theme picker is currently visible.
    pub open: bool,
    /// Current filter query.
    pub query: String,
    /// Selected index within the filtered theme list.
    pub selected: usize,
    names: Vec<String>,
}

impl ThemePickerState {
    /// Creates picker state from theme names sorted for display.
    pub fn new(names: Vec<String>) -> Self {
        let mut state = Self {
            names,
            ..Self::default()
        };
        state.sort_names();
        state
    }

    /// Opens the picker and resets transient selection state.
    pub fn open(&mut self) {
        self.open = true;
        self.query.clear();
        self.selected = 0;
    }

    /// Closes the picker and resets transient selection state.
    pub fn cancel(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
    }

    /// Appends a character to the filter query and clamps selection.
    pub fn push_query(&mut self, c: char) {
        self.query.push(c);
        self.clamp_selected();
    }

    /// Removes the final character from the filter query and clamps selection.
    pub fn pop_query(&mut self) {
        self.query.pop();
        self.clamp_selected();
    }

    /// Moves selection to the next filtered name, wrapping at the end.
    pub fn move_next(&mut self) {
        let count = self.filtered_count();
        if count == 0 {
            self.selected = 0;
            return;
        }

        self.selected = (self.selected + 1) % count;
    }

    /// Moves selection to the previous filtered name, wrapping at the start.
    pub fn move_prev(&mut self) {
        let count = self.filtered_count();
        if count == 0 {
            self.selected = 0;
            return;
        }

        self.selected = if self.selected == 0 {
            count - 1
        } else {
            self.selected - 1
        };
    }

    /// Returns sorted theme names containing the current query.
    pub fn filtered_names(&self) -> Vec<&str> {
        let query = self.query.to_lowercase();
        self.names
            .iter()
            .filter(|name| query.is_empty() || name.to_lowercase().contains(&query))
            .map(String::as_str)
            .collect()
    }

    /// Returns the selected filtered theme name.
    pub fn selected_name(&self) -> Option<&str> {
        self.filtered_names().get(self.selected).copied()
    }

    /// Replaces available theme names, sorts them, and clamps selection.
    pub fn set_names(&mut self, names: Vec<String>) {
        self.names = names;
        self.sort_names();
        self.clamp_selected();
    }

    fn sort_names(&mut self) {
        self.names.sort();
    }

    fn filtered_count(&self) -> usize {
        let query = self.query.to_lowercase();
        self.names
            .iter()
            .filter(|name| query.is_empty() || name.to_lowercase().contains(&query))
            .count()
    }

    fn clamp_selected(&mut self) {
        let count = self.filtered_count();
        if count == 0 {
            self.selected = 0;
            return;
        }

        self.selected = self.selected.min(count - 1);
    }
}

#[cfg(test)]
mod test {
    use super::ThemePickerState;

    fn picker(names: &[&str]) -> ThemePickerState {
        ThemePickerState::new(names.iter().map(|name| (*name).to_string()).collect())
    }

    #[test]
    fn filters_theme_names() {
        let mut state = picker(&["Solarized Dark", "Gruvbox", "Solarized Light"]);

        state.push_query('s');
        state.push_query('o');
        state.push_query('l');

        assert_eq!(
            state.filtered_names(),
            vec!["Solarized Dark", "Solarized Light"]
        );
    }

    #[test]
    fn selection_clamps_after_filtering() {
        let mut state = picker(&["Alpha", "Beta", "Gamma"]);
        state.move_prev();

        state.push_query('b');

        assert_eq!(state.selected, 0);
        assert_eq!(state.selected_name(), Some("Beta"));
    }

    #[test]
    fn cancel_closes_without_selection() {
        let mut state = picker(&["Alpha", "Beta"]);
        state.open();
        state.push_query('b');

        state.cancel();

        assert!(!state.open);
        assert!(state.query.is_empty());
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn move_prev_wraps_from_first_to_last() {
        let mut state = picker(&["Alpha", "Beta", "Gamma"]);

        state.move_prev();

        assert_eq!(state.selected_name(), Some("Gamma"));
    }

    #[test]
    fn empty_filtered_list_returns_no_selected_name_and_movement_does_not_panic() {
        let mut state = picker(&["Alpha", "Beta"]);

        state.push_query('z');
        state.move_next();
        state.move_prev();

        assert_eq!(state.filtered_names(), Vec::<&str>::new());
        assert_eq!(state.selected_name(), None);
    }
}
