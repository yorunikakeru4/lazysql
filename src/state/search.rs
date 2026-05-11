/// Holds the state of the `/` search bar.
#[derive(Debug, Default)]
pub struct SearchState {
    pub query: String,
    /// `true` while the search bar is visible and accepting keyboard input.
    pub active: bool,
}

impl SearchState {
    /// Activates the search bar.
    pub fn open(&mut self) {
        self.active = true;
    }

    /// Deactivates the search bar but keeps the current query as an active filter.
    pub fn close(&mut self) {
        self.active = false;
    }

    /// Clears the query and deactivates the search bar.
    pub fn reset(&mut self) {
        *self = SearchState::default();
    }

    /// Returns `true` if `text` contains `self.query` (case-insensitive).
    /// Always returns `true` when the query is empty.
    pub fn matches(&self, text: &str) -> bool {
        if self.query.is_empty() {
            return true;
        }
        text.to_lowercase().contains(&self.query.to_lowercase())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn matches_empty_query_always_true() {
        let s = SearchState::default();
        assert!(s.matches("anything"));
        assert!(s.matches(""));
    }

    #[test]
    fn matches_case_insensitive() {
        let mut s = SearchState::default();
        s.query = "PUB".to_string();
        assert!(s.matches("public"));
        assert!(s.matches("PUBLIC"));
        assert!(!s.matches("auth"));
    }

    #[test]
    fn matches_substring() {
        let mut s = SearchState::default();
        s.query = "ser".to_string();
        assert!(s.matches("users"));
        assert!(!s.matches("posts"));
    }

    #[test]
    fn reset_clears_query_and_deactivates() {
        let mut s = SearchState {
            query: "foo".to_string(),
            active: true,
        };
        s.reset();
        assert!(s.query.is_empty());
        assert!(!s.active);
    }

    #[test]
    fn open_sets_active() {
        let mut s = SearchState::default();
        s.open();
        assert!(s.active);
    }

    #[test]
    fn close_clears_active_keeps_query() {
        let mut s = SearchState {
            query: "foo".to_string(),
            active: true,
        };
        s.close();
        assert!(!s.active);
        assert_eq!(s.query, "foo");
    }
}
