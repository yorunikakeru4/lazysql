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
    pub active: bool,
    pub result: Option<SqlResult>,
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
}
