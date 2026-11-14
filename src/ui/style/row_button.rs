//! The column-header button style still used by Layer Walk. The palettes
//! otherwise take their row and header buttons from `ui::kit`.

use iced::widget::button;
use iced::{Background, Theme};

/// Column-header button style (layers panel).
pub(crate) fn header_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.palette();
    let highlighted = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let pair = if highlighted {
        palette.background.strong
    } else {
        palette.background.weak
    };
    button::Style {
        background: highlighted.then_some(Background::Color(pair.color)),
        text_color: pair.text,
        ..Default::default()
    }
}
