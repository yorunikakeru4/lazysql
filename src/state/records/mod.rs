use crate::db::repo::tables_repo::{ColumnInfo, FetchRowsResult, RowData};

/// Maximum number of characters displayed per cell before truncation.
pub const MAX_CELL_LEN: usize = 50;

/// Source of records being viewed.
#[derive(Debug, Clone)]
pub enum RecordsSource {
    Table { schema: String, table: String },
    Query { sql: String },
}

/// State for the paginated records viewer.
#[derive(Debug, Default)]
pub struct RecordsState {
    pub source: Option<RecordsSource>,
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<RowData>,
    pub total_count: u64,
    pub offset: u64,
    pub rows_per_page: u16,
    pub error: Option<String>,
    pub min_table_width: u16,
    pub selected_row: usize,
    pub selected_col: usize,
}

impl RecordsState {
    /// Creates state for viewing a table's records.
    pub fn for_table(schema: String, table: String) -> Self {
        Self {
            source: Some(RecordsSource::Table { schema, table }),
            ..Default::default()
        }
    }

    /// Creates state for viewing a query's results.
    pub fn for_query(sql: String) -> Self {
        Self {
            source: Some(RecordsSource::Query { sql }),
            ..Default::default()
        }
    }

    /// Resets all state.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Returns current page number (1-indexed).
    pub fn current_page(&self) -> u64 {
        if self.rows_per_page == 0 {
            return 1;
        }
        (self.offset / self.rows_per_page as u64) + 1
    }

    /// Returns total number of pages.
    pub fn total_pages(&self) -> u64 {
        if self.rows_per_page == 0 {
            return 1;
        }
        let rpp = self.rows_per_page as u64;
        self.total_count.div_ceil(rpp)
    }

    /// Returns true if there's a next page.
    pub fn has_next_page(&self) -> bool {
        self.current_page() < self.total_pages()
    }

    /// Returns true if there's a previous page.
    pub fn has_prev_page(&self) -> bool {
        self.offset > 0
    }

    /// Advances to next page, returns new offset.
    pub fn next_page(&mut self) -> u64 {
        if self.has_next_page() {
            self.offset += self.rows_per_page as u64;
        }
        self.offset
    }

    /// Goes to previous page, returns new offset.
    pub fn prev_page(&mut self) -> u64 {
        self.offset = self.offset.saturating_sub(self.rows_per_page as u64);
        self.offset
    }

    /// Updates state from a fetch result.
    pub fn update_from_result(&mut self, result: FetchRowsResult) {
        self.columns = result.columns;
        self.rows = result.rows;
        self.total_count = result.total_count;
        self.error = None;
        self.calculate_min_table_width();
    }

    /// Calculates minimum width needed for table display.
    pub fn calculate_min_table_width(&mut self) {
        const COL_GAP: u16 = 3;
        let widths = self.table_column_widths();
        let mut width: u16 = 0;

        for (i, col_width) in widths.iter().enumerate() {
            width = width.saturating_add(*col_width);
            if i < self.columns.len() - 1 {
                width = width.saturating_add(COL_GAP);
            }
        }

        // Add borders (2) + some padding
        self.min_table_width = width.saturating_add(4);
    }

    /// Returns compact display widths for visible table columns.
    pub fn table_column_widths(&self) -> Vec<u16> {
        self.columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let row_width = self
                    .rows
                    .iter()
                    .map(|r| {
                        r.get(i)
                            .and_then(|v| v.as_ref())
                            .map(|s| s.chars().count().min(MAX_CELL_LEN))
                            .unwrap_or(4)
                    })
                    .max()
                    .unwrap_or(0);
                col.name.chars().count().max(row_width) as u16
            })
            .collect()
    }

    /// Move cursor down one row (stays on last row at boundary).
    pub fn move_row_down(&mut self) {
        if self.rows.is_empty() {
            return;
        }
        if self.selected_row + 1 < self.rows.len() {
            self.selected_row += 1;
        }
    }

    /// Move cursor up one row.
    pub fn move_row_up(&mut self) {
        self.selected_row = self.selected_row.saturating_sub(1);
    }

    /// Move cursor right one column.
    pub fn move_col_right(&mut self) {
        if self.columns.is_empty() {
            return;
        }
        if self.selected_col + 1 < self.columns.len() {
            self.selected_col += 1;
        }
    }

    /// Move cursor left one column.
    pub fn move_col_left(&mut self) {
        self.selected_col = self.selected_col.saturating_sub(1);
    }

    /// Returns the name of the currently selected column, if any.
    pub fn current_col_name(&self) -> Option<&str> {
        self.columns.get(self.selected_col).map(|c| c.name.as_str())
    }

    /// Returns the number of rows that fit in the current terminal layout.
    pub fn rows_per_page_for_terminal(&self, terminal_height: u16, terminal_width: u16) -> u16 {
        if self.columns.is_empty() {
            return 1;
        }

        if self.min_table_width > terminal_width {
            let content_height = terminal_height.saturating_sub(5).max(1);
            let row_height = (self.columns.len() as u16).saturating_add(1).max(1);
            return (content_height / row_height).max(1);
        }

        terminal_height.saturating_sub(7).max(1)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pagination_calculates_pages_correctly() {
        let mut state = RecordsState::default();
        state.rows_per_page = 10;
        state.total_count = 25;
        assert_eq!(state.total_pages(), 3);
        assert_eq!(state.current_page(), 1);
    }

    #[test]
    fn next_page_advances_offset() {
        let mut state = RecordsState::default();
        state.rows_per_page = 10;
        state.total_count = 25;
        state.next_page();
        assert_eq!(state.offset, 10);
        assert_eq!(state.current_page(), 2);
    }

    #[test]
    fn prev_page_decrements_offset() {
        let mut state = RecordsState::default();
        state.rows_per_page = 10;
        state.total_count = 25;
        state.offset = 20;
        state.prev_page();
        assert_eq!(state.offset, 10);
    }

    #[test]
    fn prev_page_stops_at_zero() {
        let mut state = RecordsState::default();
        state.rows_per_page = 10;
        state.offset = 5;
        state.prev_page();
        assert_eq!(state.offset, 0);
    }

    #[test]
    fn reset_clears_all_state() {
        let mut state = RecordsState::for_table("public".into(), "users".into());
        state.offset = 10;
        state.total_count = 100;
        state.reset();
        assert!(state.source.is_none());
        assert_eq!(state.offset, 0);
        assert_eq!(state.total_count, 0);
    }

    #[test]
    fn for_table_sets_source() {
        let state = RecordsState::for_table("public".into(), "users".into());
        match state.source {
            Some(RecordsSource::Table { schema, table }) => {
                assert_eq!(schema, "public");
                assert_eq!(table, "users");
            }
            _ => panic!("Expected Table source"),
        }
    }

    #[test]
    fn for_query_sets_source() {
        let state = RecordsState::for_query("SELECT 1".into());
        match state.source {
            Some(RecordsSource::Query { sql }) => {
                assert_eq!(sql, "SELECT 1");
            }
            _ => panic!("Expected Query source"),
        }
    }

    #[test]
    fn has_next_page_true_when_more_pages() {
        let mut state = RecordsState::default();
        state.rows_per_page = 10;
        state.total_count = 25;
        state.offset = 0;
        assert!(state.has_next_page());
    }

    #[test]
    fn has_next_page_false_on_last_page() {
        let mut state = RecordsState::default();
        state.rows_per_page = 10;
        state.total_count = 25;
        state.offset = 20;
        assert!(!state.has_next_page());
    }

    #[test]
    fn has_prev_page_false_at_start() {
        let mut state = RecordsState::default();
        state.offset = 0;
        assert!(!state.has_prev_page());
    }

    #[test]
    fn update_from_result_populates_state() {
        let mut state = RecordsState::default();
        let result = FetchRowsResult {
            columns: vec![ColumnInfo { name: "id".into() }],
            rows: vec![vec![Some("1".into())]],
            total_count: 1,
        };
        state.update_from_result(result);
        assert_eq!(state.columns.len(), 1);
        assert_eq!(state.rows.len(), 1);
        assert_eq!(state.total_count, 1);
    }

    #[test]
    fn rows_per_page_shrinks_for_vertical_layout() {
        let mut state = RecordsState {
            columns: (0..12)
                .map(|i| ColumnInfo {
                    name: format!("col_{i}"),
                })
                .collect(),
            ..Default::default()
        };
        state.min_table_width = 200;

        assert_eq!(state.rows_per_page_for_terminal(57, 80), 4);
    }

    #[test]
    fn rows_per_page_uses_table_layout_when_it_fits() {
        let mut state = RecordsState {
            columns: vec![ColumnInfo { name: "id".into() }],
            ..Default::default()
        };
        state.min_table_width = 20;

        assert_eq!(state.rows_per_page_for_terminal(57, 80), 50);
    }

    #[test]
    fn selected_row_moves_down() {
        let mut state = RecordsState::default();
        state.rows = vec![vec![Some("a".into())], vec![Some("b".into())]];
        state.move_row_down();
        assert_eq!(state.selected_row, 1);
        state.move_row_down(); // at end, stays
        assert_eq!(state.selected_row, 1);
    }

    #[test]
    fn move_row_up_stops_at_zero() {
        let mut state = RecordsState::default();
        state.rows = vec![vec![Some("a".into())]];
        state.move_row_up();
        assert_eq!(state.selected_row, 0);
    }

    #[test]
    fn move_col_right_stops_at_last() {
        let mut state = RecordsState::default();
        state.columns = vec![
            crate::db::repo::tables_repo::ColumnInfo { name: "a".into() },
            crate::db::repo::tables_repo::ColumnInfo { name: "b".into() },
        ];
        state.move_col_right();
        assert_eq!(state.selected_col, 1);
        state.move_col_right();
        assert_eq!(state.selected_col, 1);
    }

    #[test]
    fn reset_clears_selection() {
        let mut state = RecordsState::for_table("s".into(), "t".into());
        state.selected_row = 5;
        state.selected_col = 3;
        state.reset();
        assert_eq!(state.selected_row, 0);
        assert_eq!(state.selected_col, 0);
    }

    #[test]
    fn table_column_widths_follow_visible_content_with_cap() {
        let mut state = RecordsState::default();
        state.columns = vec![
            ColumnInfo { name: "id".into() },
            ColumnInfo {
                name: "description".into(),
            },
        ];
        state.rows = vec![vec![
            Some("100".into()),
            Some("x".repeat(MAX_CELL_LEN + 20)),
        ]];

        assert_eq!(state.table_column_widths(), vec![3, MAX_CELL_LEN as u16]);
    }
}
