use ratatui::layout::Rect;

/// Returns a centered rectangle with the requested width percentage and height.
pub(crate) fn centered_rect(percent_width: u16, height: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_width / 100;
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height,
    }
}
