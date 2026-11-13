//! The quiet text lines of a palette: what shows when there is nothing to
//! list, and a status line at the bottom.

use crate::app::Message;
use crate::ui::style::{chrome, palette_metrics as pm, text};
use iced::widget::{container, text as label};
use iced::{Background, Element, Length, Padding, Theme};

/// What a palette shows when there is nothing to list ("No selection",
/// "No blocks in this drawing"): one muted line, inset like the grid.
pub fn empty_state<'a>(message: impl Into<String>) -> Element<'a, Message> {
    container(
        label(message.into())
            .size(text::UI_SMALL)
            .line_height(text::line())
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(chrome::muted(theme)),
            }),
    )
    .padding(Padding {
        top: pm::TOOL_GAP,
        bottom: pm::TOOL_GAP,
        left: pm::GRID_INSET_L,
        right: pm::GRID_INSET_R,
    })
    .width(Length::Fill)
    .into()
}

/// A palette's status line at its bottom ("Current layer: 0 · 1 of 1
/// layers shown"): one row high, on the category-bar colour.
pub fn footer<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .height(Length::Fixed(pm::ROW_H))
        .width(Length::Fill)
        .padding(Padding::ZERO.left(pm::GRID_INSET_L).right(pm::GRID_INSET_R))
        .align_y(iced::Center)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(chrome::palette_bar(theme))),
            text_color: Some(chrome::muted(theme)),
            ..Default::default()
        })
        .into()
}

/// Footer text: [`text::UI`] on a whole-pixel line.
pub fn footer_text<'a>(value: impl Into<String>) -> Element<'a, Message> {
    label(value.into())
        .size(text::UI)
        .line_height(text::line())
        .wrapping(iced::widget::text::Wrapping::None)
        .into()
}
