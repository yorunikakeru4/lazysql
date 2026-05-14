use crate::db::repo::tables_repo::TableField;
use crate::state::records::{MAX_CELL_LEN, RecordsState};

/// Truncates a cell value to `MAX_CELL_LEN` characters and appends `...` if needed.
fn truncate_cell(s: &str) -> String {
    if s.chars().count() <= MAX_CELL_LEN {
        return s.to_string();
    }
    let boundary = s
        .char_indices()
        .nth(MAX_CELL_LEN - 3)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    format!("{}...", &s[..boundary])
}

/// Formats records as a table with aligned columns.
pub(crate) fn format_records_table(records: &RecordsState) -> String {
    const COL_GAP: &str = "   ";

    if records.columns.is_empty() {
        return "No data".to_string();
    }

    let mut widths: Vec<usize> = records.columns.iter().map(|c| c.name.len()).collect();
    for row in &records.rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                let cell_len = cell
                    .as_ref()
                    .map(|s| s.chars().count().min(MAX_CELL_LEN))
                    .unwrap_or(4);
                widths[i] = widths[i].max(cell_len);
            }
        }
    }

    let header: String = records
        .columns
        .iter()
        .enumerate()
        .map(|(i, col)| format!("{:<width$}", col.name, width = widths[i]))
        .collect::<Vec<_>>()
        .join(COL_GAP);

    let separator = "-".repeat(widths.iter().sum::<usize>() + COL_GAP.len() * (widths.len() - 1));

    let rows: String = records
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(i, cell)| {
                    let raw = cell.as_ref().map(|s| s.as_str()).unwrap_or("NULL");
                    let val = truncate_cell(raw);
                    let w = widths.get(i).copied().unwrap_or(val.chars().count());
                    format!("{:<width$}", val, width = w)
                })
                .collect::<Vec<_>>()
                .join(COL_GAP)
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("{}\n{}\n{}", header, separator, rows)
}

/// Formats records using the table layout when it fits, otherwise a vertical layout.
pub(crate) fn format_records(records: &RecordsState, available_width: u16) -> String {
    if records.min_table_width > available_width {
        return format_records_vertical(records);
    }

    format_records_table(records)
}

/// Formats records as vertical per-record field/value blocks.
pub(crate) fn format_records_vertical(records: &RecordsState) -> String {
    if records.columns.is_empty() {
        return "No data".to_string();
    }

    let name_width = records
        .columns
        .iter()
        .map(|col| col.name.chars().count())
        .max()
        .unwrap_or(0);

    records
        .rows
        .iter()
        .enumerate()
        .map(|(row_idx, row)| {
            let mut lines = vec![format!(
                "-[ RECORD {} ]-------------------------",
                row_idx + 1
            )];
            for (col_idx, col) in records.columns.iter().enumerate() {
                let value = row
                    .get(col_idx)
                    .and_then(|cell| cell.as_deref())
                    .unwrap_or("∅");
                lines.push(format!("{:<name_width$} | {}", col.name, value));
            }
            lines.join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Formats table fields as aligned columns.
#[allow(dead_code)]
pub(crate) fn format_fields(fields: &[&TableField]) -> String {
    const COLUMN_GAP: &str = "   ";

    let mut rows: Vec<(&str, &str, &str, String, String)> = Vec::with_capacity(fields.len());
    for f in fields {
        let constraint = f
            .constraint
            .as_ref()
            .map(|c| format!("{c:?}"))
            .unwrap_or_default();
        let default = f.default_value.clone().unwrap_or_default();
        rows.push((&f.name, &f.data_type, &f.is_nullable, constraint, default));
    }

    let name_w = rows
        .iter()
        .map(|(name, _, _, _, _)| name.len())
        .max()
        .unwrap_or(0)
        .max("Column".len());
    let type_w = rows
        .iter()
        .map(|(_, data_type, _, _, _)| data_type.len())
        .max()
        .unwrap_or(0)
        .max("Type".len());
    let nullable_w = rows
        .iter()
        .map(|(_, _, nullable, _, _)| nullable.len())
        .max()
        .unwrap_or(0)
        .max("Nullable".len());
    let constraint_w = rows
        .iter()
        .map(|(_, _, _, constraint, _)| constraint.len())
        .max()
        .unwrap_or(0)
        .max("Constraint".len());

    let header = format!(
        "{:<name_w$}{COLUMN_GAP}{:<type_w$}{COLUMN_GAP}{:<nullable_w$}{COLUMN_GAP}{:<constraint_w$}{COLUMN_GAP}{}\n",
        "Column", "Type", "Nullable", "Constraint", "Default",
    );
    let separator = format!(
        "{}\n",
        "-".repeat(
            name_w + type_w + nullable_w + constraint_w + "Default".len() + COLUMN_GAP.len() * 4,
        )
    );
    let body: String = rows
        .iter()
        .map(|(name, data_type, nullable, constraint, default)| {
            format!(
                "{:<name_w$}{COLUMN_GAP}{:<type_w$}{COLUMN_GAP}{:<nullable_w$}{COLUMN_GAP}{:<constraint_w$}{COLUMN_GAP}{}\n",
                name, data_type, nullable, constraint, default
            )
        })
        .collect();

    format!("{header}{separator}{body}")
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::db::repo::tables_repo::ColumnInfo;

    #[test]
    fn nullable_column_stays_aligned_with_long_type_values() {
        let short = TableField {
            name: "id".to_string(),
            data_type: "integer".to_string(),
            is_nullable: "NO".to_string(),
            constraint: None,
            default_value: None,
        };
        let long = TableField {
            name: "payload".to_string(),
            data_type: "character varying with very very long suffix".to_string(),
            is_nullable: "YES".to_string(),
            constraint: None,
            default_value: None,
        };

        let rendered = format_fields(&[&short, &long]);
        let lines: Vec<&str> = rendered.lines().collect();

        let nullable_short = lines[2].find("NO").unwrap();
        let nullable_long = lines[3].find("YES").unwrap();
        assert_eq!(nullable_short, nullable_long);
    }

    #[test]
    fn columns_have_wider_gap_between_each_other() {
        let field = TableField {
            name: "id".to_string(),
            data_type: "integer".to_string(),
            is_nullable: "NO".to_string(),
            constraint: None,
            default_value: None,
        };

        let rendered = format_fields(&[&field]);
        let header = rendered.lines().next().unwrap();
        let column_end = header.find("Column").unwrap() + "Column".len();
        let type_start = header.find("Type").unwrap();
        let nullable_start = header.find("Nullable").unwrap();
        let constraint_start = header.find("Constraint").unwrap();
        let default_start = header.find("Default").unwrap();

        assert!(type_start - column_end >= 3);
        assert!(nullable_start - (type_start + "Type".len()) >= 3);
        assert!(constraint_start - (nullable_start + "Nullable".len()) >= 3);
        assert!(default_start - (constraint_start + "Constraint".len()) >= 3);
    }

    #[test]
    fn format_records_table_aligns_columns() {
        let records = RecordsState {
            columns: vec![
                ColumnInfo { name: "id".into() },
                ColumnInfo {
                    name: "name".into(),
                },
            ],
            rows: vec![
                vec![Some("1".into()), Some("Alice".into())],
                vec![Some("2".into()), Some("Bob".into())],
            ],
            ..Default::default()
        };
        let output = format_records_table(&records);
        assert!(output.contains("id"));
        assert!(output.contains("name"));
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
    }

    #[test]
    fn format_records_table_handles_null() {
        let records = RecordsState {
            columns: vec![ColumnInfo { name: "val".into() }],
            rows: vec![vec![None]],
            ..Default::default()
        };
        let output = format_records_table(&records);
        assert!(output.contains("NULL"));
    }

    #[test]
    fn format_records_table_empty_returns_no_data() {
        let records = RecordsState::default();
        let output = format_records_table(&records);
        assert_eq!(output, "No data");
    }

    #[test]
    fn format_records_uses_vertical_layout_when_table_is_too_wide() {
        let mut records = RecordsState {
            columns: vec![
                ColumnInfo { name: "id".into() },
                ColumnInfo {
                    name: "dedupe_key".into(),
                },
                ColumnInfo {
                    name: "message".into(),
                },
            ],
            rows: vec![vec![
                Some("6".into()),
                Some("test-1".into()),
                Some("Test message for notification 1. Generated automatically.".into()),
            ]],
            ..Default::default()
        };
        records.calculate_min_table_width();

        let output = format_records(&records, records.min_table_width.saturating_sub(1));

        assert!(output.contains("-[ RECORD 1 ]"));
        assert!(output.contains("id         | 6"));
        assert!(output.contains("dedupe_key | test-1"));
        assert!(
            output
                .contains("message    | Test message for notification 1. Generated automatically.")
        );
    }
}
