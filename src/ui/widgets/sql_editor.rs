use crate::state::app::AppState;
use crate::ui::{layout::centered_rect, theme};
use ratatui::{
    Frame,
    layout::Position,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph},
};
use sqlparser::{
    dialect::PostgreSqlDialect,
    keywords::{
        Keyword, RESERVED_FOR_COLUMN_ALIAS, RESERVED_FOR_IDENTIFIER, RESERVED_FOR_TABLE_ALIAS,
        RESERVED_FOR_TABLE_FACTOR,
    },
    tokenizer::{Token, TokenWithSpan, Tokenizer},
};

/// Renders the floating multiline SQL editor overlay.
pub(crate) fn render(frame: &mut Frame, state: &AppState) {
    let area = frame.area();
    let popup = centered_rect(70, 20, area);
    frame.render_widget(Clear, popup);

    let driver = if state.current_db.is_some() {
        "postgres"
    } else {
        "—"
    };
    let title = format!(
        " SQL Editor ─── {driver} ─── Ctrl+E execute · Enter newline · Tab indent · Esc close "
    );

    let query = &state.sql_input.query;
    let (cursor_line, cursor_col) = state.sql_input.cursor_line_col();

    let lines: Vec<Line> = query
        .split('\n')
        .enumerate()
        .map(|(line_idx, line_text)| {
            let line_num = Span::styled(
                format!("{:>3}  ", line_idx + 1),
                Style::new().fg(theme::FG4),
            );

            let mut spans = vec![line_num];
            spans.extend(highlight_sql(line_text));
            Line::from(spans)
        })
        .collect();

    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::bordered()
                    .title(title)
                    .border_style(Style::new().fg(theme::ORANGE)),
            )
            .style(Style::new().bg(theme::BG1)),
        popup,
    );

    // Position terminal cursor (for accessibility / IME)
    let cursor_x = (popup.x + 6 + cursor_col as u16).min(popup.x + popup.width.saturating_sub(2));
    let cursor_y = (popup.y + 1 + cursor_line as u16).min(popup.y + popup.height.saturating_sub(2));
    frame.set_cursor_position(Position::new(cursor_x, cursor_y));
}

/// SQL syntax highlight for a single line segment.
fn highlight_sql(text: &str) -> Vec<Span<'_>> {
    let dialect = PostgreSqlDialect {};
    let Ok(tokens) = Tokenizer::new(&dialect, text)
        .with_unescape(false)
        .tokenize_with_location()
    else {
        return highlight_sql_words(text);
    };

    let mut spans = Vec::new();
    let mut cursor = 0;
    for (idx, token) in tokens.iter().enumerate() {
        let start = byte_index_for_column(text, token.span.start.column);
        let end = byte_index_for_column(text, token.span.end.column);

        if cursor < start {
            spans.push(Span::raw(&text[cursor..start]));
        }
        if start < end {
            spans.push(style_token(
                &text[start..end],
                token,
                next_significant_token(&tokens, idx),
            ));
        }
        cursor = end;
    }
    if cursor < text.len() {
        spans.push(Span::raw(&text[cursor..]));
    }

    spans
}

fn highlight_sql_words(text: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        if starts_with_sql_word_char(remaining) {
            let end = remaining
                .char_indices()
                .find_map(|(i, c)| (!is_sql_word_char(c)).then_some(i))
                .unwrap_or(remaining.len());
            let word = &remaining[..end];
            if is_sql_keyword(word) {
                spans.push(Span::styled(word, Style::new().fg(theme::PURPLE)));
            } else {
                spans.push(Span::raw(word));
            }
            remaining = &remaining[end..];
        } else {
            let end = remaining
                .char_indices()
                .nth(1)
                .map(|(i, _)| i)
                .unwrap_or(remaining.len());
            spans.push(Span::raw(&remaining[..end]));
            remaining = &remaining[end..];
        }
    }
    spans
}

fn style_token<'a>(
    text: &'a str,
    token: &TokenWithSpan,
    next_token: Option<&TokenWithSpan>,
) -> Span<'a> {
    let Some(style) = token_style(&token.token, next_token.map(|token| &token.token)) else {
        return Span::raw(text);
    };
    Span::styled(text, style)
}

fn token_style(token: &Token, next_token: Option<&Token>) -> Option<Style> {
    match token {
        Token::Word(_) if matches!(next_token, Some(Token::LParen)) => {
            Some(Style::new().fg(theme::AQUA))
        }
        Token::Word(word) => keyword_style(word.keyword),
        Token::Number(..)
        | Token::SingleQuotedString(_)
        | Token::DoubleQuotedString(_)
        | Token::TripleSingleQuotedString(_)
        | Token::TripleDoubleQuotedString(_)
        | Token::DollarQuotedString(_)
        | Token::SingleQuotedByteStringLiteral(_)
        | Token::DoubleQuotedByteStringLiteral(_)
        | Token::TripleSingleQuotedByteStringLiteral(_)
        | Token::TripleDoubleQuotedByteStringLiteral(_)
        | Token::SingleQuotedRawStringLiteral(_)
        | Token::DoubleQuotedRawStringLiteral(_)
        | Token::TripleSingleQuotedRawStringLiteral(_)
        | Token::TripleDoubleQuotedRawStringLiteral(_)
        | Token::NationalStringLiteral(_)
        | Token::QuoteDelimitedStringLiteral(_)
        | Token::NationalQuoteDelimitedStringLiteral(_)
        | Token::EscapedStringLiteral(_)
        | Token::UnicodeStringLiteral(_)
        | Token::HexStringLiteral(_) => Some(Style::new().fg(theme::GREEN)),
        Token::Eq
        | Token::Neq
        | Token::Lt
        | Token::Gt
        | Token::LtEq
        | Token::GtEq
        | Token::Spaceship
        | Token::Plus
        | Token::Minus
        | Token::Mul
        | Token::Div
        | Token::DuckIntDiv
        | Token::Mod
        | Token::StringConcat
        | Token::DoubleColon
        | Token::Assignment
        | Token::Ampersand
        | Token::Pipe
        | Token::Caret
        | Token::RArrow
        | Token::Sharp
        | Token::DoubleSharp
        | Token::Tilde
        | Token::TildeAsterisk
        | Token::ExclamationMarkTilde
        | Token::ExclamationMarkTildeAsterisk
        | Token::DoubleTilde
        | Token::DoubleTildeAsterisk
        | Token::ExclamationMarkDoubleTilde
        | Token::ExclamationMarkDoubleTildeAsterisk
        | Token::ShiftLeft
        | Token::ShiftRight
        | Token::Overlap
        | Token::ExclamationMark
        | Token::DoubleExclamationMark
        | Token::AtSign
        | Token::CaretAt
        | Token::PGSquareRoot
        | Token::PGCubeRoot
        | Token::Arrow
        | Token::LongArrow
        | Token::HashArrow
        | Token::HashLongArrow
        | Token::AtArrow
        | Token::ArrowAt
        | Token::HashMinus
        | Token::AtQuestion
        | Token::AtAt
        | Token::Question
        | Token::QuestionAnd
        | Token::QuestionPipe
        | Token::CustomBinaryOperator(_) => Some(Style::new().fg(theme::YELLOW)),
        _ => None,
    }
}

fn next_significant_token(tokens: &[TokenWithSpan], idx: usize) -> Option<&TokenWithSpan> {
    tokens
        .iter()
        .skip(idx + 1)
        .find(|token| !matches!(token.token, Token::Whitespace(_)))
}

fn byte_index_for_column(text: &str, column: u64) -> usize {
    if column == 0 {
        return 0;
    }
    text.char_indices()
        .nth(column.saturating_sub(1) as usize)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

fn starts_with_sql_word_char(text: &str) -> bool {
    text.chars().next().is_some_and(is_sql_word_char)
}

fn is_sql_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn is_sql_keyword(word: &str) -> bool {
    let Token::Word(word) = Token::make_keyword(word) else {
        return false;
    };
    is_highlighted_keyword(word.keyword)
}

fn keyword_style(keyword: Keyword) -> Option<Style> {
    if is_ddl_keyword(keyword) {
        return Some(Style::new().fg(theme::ORANGE));
    }
    if is_clause_keyword(keyword) {
        return Some(Style::new().fg(theme::BLUE));
    }
    if is_dml_keyword(keyword) || is_highlighted_keyword(keyword) {
        return Some(Style::new().fg(theme::PURPLE));
    }
    None
}

fn is_ddl_keyword(keyword: Keyword) -> bool {
    matches!(
        keyword,
        Keyword::CREATE
            | Keyword::ALTER
            | Keyword::DROP
            | Keyword::TRUNCATE
            | Keyword::TABLE
            | Keyword::INDEX
            | Keyword::VIEW
            | Keyword::SCHEMA
            | Keyword::DATABASE
            | Keyword::EXTENSION
            | Keyword::TYPE
            | Keyword::DOMAIN
            | Keyword::SEQUENCE
            | Keyword::FUNCTION
            | Keyword::PROCEDURE
            | Keyword::TRIGGER
    )
}

fn is_dml_keyword(keyword: Keyword) -> bool {
    matches!(
        keyword,
        Keyword::SELECT
            | Keyword::INSERT
            | Keyword::UPDATE
            | Keyword::DELETE
            | Keyword::MERGE
            | Keyword::COPY
            | Keyword::CALL
            | Keyword::VALUES
            | Keyword::RETURNING
    )
}

fn is_clause_keyword(keyword: Keyword) -> bool {
    matches!(
        keyword,
        Keyword::FROM
            | Keyword::WHERE
            | Keyword::JOIN
            | Keyword::LEFT
            | Keyword::RIGHT
            | Keyword::INNER
            | Keyword::OUTER
            | Keyword::FULL
            | Keyword::CROSS
            | Keyword::ON
            | Keyword::GROUP
            | Keyword::BY
            | Keyword::ORDER
            | Keyword::HAVING
            | Keyword::LIMIT
            | Keyword::OFFSET
            | Keyword::WITH
            | Keyword::UNION
            | Keyword::EXCEPT
            | Keyword::INTERSECT
    )
}

fn is_highlighted_keyword(keyword: Keyword) -> bool {
    RESERVED_FOR_TABLE_ALIAS.contains(&keyword)
        || RESERVED_FOR_COLUMN_ALIAS.contains(&keyword)
        || RESERVED_FOR_TABLE_FACTOR.contains(&keyword)
        || RESERVED_FOR_IDENTIFIER.contains(&keyword)
        || matches!(
            keyword,
            Keyword::AS
                | Keyword::AND
                | Keyword::OR
                | Keyword::NOT
                | Keyword::NULL
                | Keyword::IS
                | Keyword::IN
                | Keyword::LIKE
                | Keyword::ILIKE
                | Keyword::BETWEEN
                | Keyword::DISTINCT
                | Keyword::COUNT
                | Keyword::SUM
                | Keyword::AVG
                | Keyword::MAX
                | Keyword::MIN
                | Keyword::ALL
                | Keyword::INSERT
                | Keyword::UPDATE
                | Keyword::DELETE
                | Keyword::CREATE
                | Keyword::DROP
                | Keyword::ALTER
                | Keyword::TABLE
                | Keyword::INDEX
                | Keyword::BY
                | Keyword::CASE
                | Keyword::WHEN
                | Keyword::THEN
                | Keyword::ELSE
                | Keyword::CAST
        )
}

#[cfg(test)]
mod test {
    use super::*;

    fn highlighted_text(text: &str) -> Vec<String> {
        highlight_sql(text)
            .into_iter()
            .filter(|span| span.style.fg == Some(theme::PURPLE))
            .map(|span| span.content.into_owned())
            .collect()
    }

    fn text_with_color(text: &str, color: ratatui::style::Color) -> Vec<String> {
        highlight_sql(text)
            .into_iter()
            .filter(|span| span.style.fg == Some(color))
            .map(|span| span.content.into_owned())
            .collect()
    }

    #[test]
    fn highlights_sqlparser_keyword_not_in_local_legacy_list() {
        assert_eq!(highlighted_text("name ILIKE '%foo%'"), vec!["ILIKE"]);
    }

    #[test]
    fn does_not_highlight_keyword_suffix_inside_identifier() {
        assert!(highlighted_text("sas").is_empty());
    }

    #[test]
    fn does_not_highlight_keyword_prefix_inside_identifier() {
        assert!(highlighted_text("selectable").is_empty());
    }

    #[test]
    fn highlights_ddl_keywords_with_ddl_color() {
        assert_eq!(
            text_with_color("CREATE TABLE users (id int)", theme::ORANGE),
            vec!["CREATE", "TABLE"]
        );
    }

    #[test]
    fn highlights_clause_keywords_with_clause_color() {
        assert_eq!(
            text_with_color("SELECT id FROM users WHERE id = 1", theme::BLUE),
            vec!["FROM", "WHERE"]
        );
    }

    #[test]
    fn highlights_function_calls_with_function_color() {
        assert_eq!(
            text_with_color("SELECT count(id), custom_fn(id) FROM users", theme::AQUA),
            vec!["count", "custom_fn"]
        );
    }

    #[test]
    fn highlights_literals_and_operators_with_their_own_colors() {
        assert_eq!(
            text_with_color("SELECT id = 42 AND name = 'Ada'", theme::GREEN),
            vec!["42", "'Ada'"]
        );
        assert_eq!(
            text_with_color("SELECT id = 42 AND name = 'Ada'", theme::YELLOW),
            vec!["=", "="]
        );
    }
}
