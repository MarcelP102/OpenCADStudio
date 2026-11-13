//! Segmented tabs at the top of a palette (Block Palette, Tool Palettes,
//! Authoring Palettes, Sheet Set): equal parts on a `palette_bar` track,
//! the active one on the palette body.

use crate::app::Message;
use crate::ui::style::{chrome, palette_metrics as pm, text};
use iced::widget::{button, container, row, text as label};
use iced::{Background, Element, Length, Padding, Theme};

/// The tabs `items` (label, message), `active` lit, in a palette band.
pub fn tabs<'a>(items: Vec<(String, Message)>, active: usize) -> Element<'a, Message> {
    let parts = items.into_iter().enumerate().map(|(i, (name, message))| {
        let is_active = i == active;
        button(
            container(
                label(name)
                    .size(text::UI)
                    .line_height(text::line())
                    .wrapping(iced::widget::text::Wrapping::None),
            )
            .center(Length::Fill),
        )
        .on_press(message)
        .width(Length::Fill)
        .height(Length::Fixed(pm::TOOL_H - 2.0 * pm::LINE))
        .padding(0)
        .style(move |theme: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if is_active {
                    Some(Background::Color(chrome::palette_body(theme)))
                } else if hovered {
                    Some(Background::Color(chrome::hover(theme)))
                } else {
                    None
                },
                text_color: if is_active || hovered {
                    chrome::strong(theme)
                } else {
                    chrome::tab_text(theme)
                },
                ..Default::default()
            }
        })
        .into()
    });
    super::band(
        container(row(parts).spacing(pm::LINE))
            .padding(Padding::new(pm::LINE))
            .width(Length::Fill)
            .style(|theme: &Theme| container::Style {
                background: Some(Background::Color(chrome::palette_bar(theme))),
                ..Default::default()
            }),
    )
}
