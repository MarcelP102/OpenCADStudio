//! The one tooltip of the app: the ribbon's (panel colour, 1 px border,
//! 11 px text), used by every palette.

use crate::app::Message;
use crate::ui::style::{palette_metrics as pm, text};
use iced::widget::{container, text as label, tooltip as iced_tooltip};
use iced::Element;

/// Where a tooltip opens.
pub use iced::widget::tooltip::Position as TipPosition;

/// `content` with `tip` shown beside it at `position`.
pub fn tooltip<'a>(
    content: impl Into<Element<'a, Message>>,
    tip: impl Into<String>,
    position: TipPosition,
) -> Element<'a, Message> {
    iced_tooltip(
        content,
        container(label(tip.into()).size(text::UI).line_height(text::line()))
            .padding([pm::TOOL_GAP, pm::MENU_PAD_X]),
        position,
    )
    .gap(pm::TOOL_GAP)
    .style(crate::ui::ribbon::tooltip_style)
    .into()
}
