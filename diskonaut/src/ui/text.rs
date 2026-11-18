//! Measuring text in terminal columns (not bytes or chars: `é` is two
//! bytes, `漢` two columns).

use ::ratatui::layout::Rect;
use ::unicode_width::UnicodeWidthStr;

/// Columns `text` takes on screen.
pub fn width(text: &str) -> u16 {
    u16::try_from(text.width()).unwrap_or(u16::MAX)
}

/// The x at which `text` is centered in `rect` (its left edge when the text
/// is wider than the rect).
pub fn centered_x(rect: &Rect, text: &str) -> u16 {
    rect.x + rect.width.saturating_sub(width(text)).div_ceil(2)
}
