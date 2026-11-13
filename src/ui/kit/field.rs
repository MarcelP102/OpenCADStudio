//! The boxes at the top of a palette: a search field and a selector (the
//! Properties object list box).

use crate::app::Message;
use crate::ui::style::{chrome, palette_metrics as pm, text};
use iced::widget::{container, text_input};
use iced::{Background, Element, Length, Padding, Theme};

/// Vertical padding of a 22 px box around a 14 px text line and its 1 px
/// border.
const BOX_PAD_Y: f32 = (pm::TOOL_H - text::UI_LINE - 2.0 * pm::LINE) / 2.0;

/// Padding of a text input or combo filling a 22 px box (a toolbar-height
/// field, as opposed to a [`super::FIELD_PAD`] grid cell).
pub const BOX_PAD: Padding = Padding {
    top: BOX_PAD_Y,
    bottom: BOX_PAD_Y,
    left: pm::CELL_PAD_X,
    right: pm::CELL_PAD_X,
};

/// A search field: the `palette_bar` box, 22 px, 11 px text. Returned as a
/// text input so callers can add `on_submit`, an `id`, ...
pub fn search_field<'a>(
    placeholder: &str,
    value: &str,
    on_input: impl Fn(String) -> Message + 'a,
) -> text_input::TextInput<'a, Message> {
    text_input(placeholder, value)
        .on_input(on_input)
        .size(text::UI)
        .line_height(text::line())
        .padding(BOX_PAD)
        .width(Length::Fill)
        .style(super::combo_style)
}

/// A selector box (the object list at the top of Properties): `content`
/// in the 22 px `palette_bar` box, filling the width.
pub fn select_box<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fixed(pm::TOOL_H))
        .padding(Padding::ZERO.left(pm::CELL_PAD_X).right(pm::CELL_PAD_X))
        .align_y(iced::Center)
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(chrome::palette_bar(theme))),
            text_color: Some(chrome::text(theme)),
            ..Default::default()
        })
        .into()
}

/// A button that opens a list (a picker head, an L / B / C mode button):
/// the `palette_bar` box, `hover` while hovered, the accent edge while its
/// list is `open`.
pub fn box_button_style(
    open: bool,
) -> impl Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style {
    move |theme: &Theme, status: iced::widget::button::Status| {
        use iced::widget::button::Status;
        let hovered = matches!(status, Status::Hovered | Status::Pressed);
        iced::widget::button::Style {
            background: Some(Background::Color(if hovered {
                chrome::hover(theme)
            } else {
                chrome::palette_bar(theme)
            })),
            text_color: chrome::text(theme),
            border: iced::Border {
                color: if open {
                    chrome::accent(theme)
                } else {
                    chrome::palette_bar(theme)
                },
                width: pm::LINE,
                radius: 0.0.into(),
            },
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_box_is_22_px_on_whole_pixels() {
        assert_eq!(BOX_PAD_Y.fract(), 0.0);
        assert_eq!(2.0 * BOX_PAD_Y + text::UI_LINE + 2.0 * pm::LINE, pm::TOOL_H);
    }
}
