/// Vim-style modal state for the TUI.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AppMode {
    #[default]
    Normal,
    Insert,
    Command,
    Visual,
    Result,
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn default_is_normal() {
        assert!(matches!(AppMode::default(), AppMode::Normal));
    }
    #[test]
    fn variants_are_distinct() {
        assert_ne!(AppMode::Normal, AppMode::Insert);
        assert_ne!(AppMode::Insert, AppMode::Command);
    }
}
