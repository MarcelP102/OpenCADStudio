//! The one menu of the palette area: the palette strip's right-click and +
//! menus, the drawing-tab bar's ☰ menu and a tab's hover list of layouts.
//! Same item height, check marks, separators and colours everywhere.

use crate::app::Message;
use crate::ui::style::{chrome, palette_metrics as pm, text as ui_text};
use iced::widget::{column, container, mouse_area, row, text, Space};
use iced::{Background, Border, Element, Length, Theme};

/// The mark before an item's label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// No mark column content.
    None,
    /// A check mark when on (a toggle).
    Check(bool),
    /// A dot when on (one of a set of choices).
    Radio(bool),
    /// A small dot: the drawing has unsaved changes.
    Dirty,
}

/// One entry of a [`menu`].
pub enum Item {
    Action {
        mark: Mark,
        icon: Option<&'static [u8]>,
        label: String,
        /// Dirty dot after the label (an open drawing with unsaved changes).
        dirty: bool,
        /// `None` shows the item disabled.
        on_press: Option<Box<Message>>,
        /// A tooltip beside the item (why a disabled item is disabled).
        tip: Option<String>,
    },
    Separator,
    /// A muted caption over the items after it ("EXPAND BY").
    Heading(String),
}

impl Item {
    /// A plain item.
    pub fn new(label: impl Into<String>, on_press: Message) -> Self {
        Item::Action {
            mark: Mark::None,
            icon: None,
            label: label.into(),
            dirty: false,
            on_press: Some(Box::new(on_press)),
            tip: None,
        }
    }

    /// A greyed item that does nothing, with `reason` as its tooltip.
    pub fn disabled(label: impl Into<String>, reason: impl Into<String>) -> Self {
        Item::Action {
            mark: Mark::None,
            icon: None,
            label: label.into(),
            dirty: false,
            on_press: None,
            tip: Some(reason.into()),
        }
    }

    /// The item with `mark` before its label.
    pub fn mark(mut self, m: Mark) -> Self {
        if let Item::Action { mark, .. } = &mut self {
            *mark = m;
        }
        self
    }

    /// The item with an icon before its label.
    pub fn icon(mut self, art: &'static [u8]) -> Self {
        if let Item::Action { icon, .. } = &mut self {
            *icon = Some(art);
        }
        self
    }

    /// The item greyed out and inert unless `on`.
    pub fn enabled(mut self, on: bool) -> Self {
        if let Item::Action { on_press, .. } = &mut self {
            if !on {
                *on_press = None;
            }
        }
        self
    }

    /// The item with a dirty dot after its label.
    pub fn dirty(mut self, on: bool) -> Self {
        if let Item::Action { dirty, .. } = &mut self {
            *dirty = on;
        }
        self
    }
}

/// A small filled dot, `size` across, in `color`.
pub fn dot<'a>(size: f32, color: fn(&Theme) -> iced::Color) -> Element<'a, Message> {
    container(Space::new().width(size).height(size))
        .style(move |theme: &Theme| container::Style {
            background: Some(Background::Color(color(theme))),
            border: Border {
                radius: (size / 2.0).into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn mark_cell<'a>(mark: Mark) -> Element<'a, Message> {
    let inner: Element<'a, Message> = match mark {
        Mark::Check(true) => {
            crate::ui::icons::tinted(crate::ui::icons::CHECK, MARK_ICON, chrome::text)
        }
        Mark::Radio(true) => dot(RADIO_DOT, chrome::text),
        Mark::Dirty => dot(pm::DIRTY_DOT, chrome::text),
        _ => Space::new().into(),
    };
    container(inner)
        .center_x(Length::Fixed(pm::MENU_CHECK_W))
        .center_y(Length::Fixed(pm::MENU_ITEM_H))
        .into()
}

/// Size of a check mark.
const MARK_ICON: f32 = 12.0;
/// Size of a radio dot.
const RADIO_DOT: f32 = 6.0;
/// Size of an item's icon.
const ITEM_ICON: f32 = 16.0;

/// One item as a row: mark, optional icon, label, optional dirty dot.
fn item<'a>(it: Item) -> Element<'a, Message> {
    match it {
        Item::Separator => container(
            container(Space::new().width(Length::Fill).height(pm::LINE)).style(|theme: &Theme| {
                container::Style {
                    background: Some(Background::Color(chrome::border(theme))),
                    ..Default::default()
                }
            }),
        )
        .center_y(Length::Fixed(pm::MENU_SEPARATOR_H))
        .width(Length::Fill)
        .into(),
        Item::Heading(caption) => container(
            text(caption)
                .size(ui_text::UI_SMALL)
                .line_height(ui_text::line())
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(chrome::muted(theme)),
                }),
        )
        .padding(iced::Padding::ZERO.left(pm::MENU_CHECK_W))
        .center_y(Length::Fixed(pm::MENU_ITEM_H))
        .width(Length::Fill)
        .into(),
        Item::Action {
            mark,
            icon,
            label,
            dirty,
            on_press,
            tip,
        } => {
            let mut content = row![mark_cell(mark)].align_y(iced::Center);
            if let Some(art) = icon {
                content = content.push(
                    container(crate::ui::icons::tinted(art, ITEM_ICON, chrome::text))
                        .padding(iced::Padding::ZERO.right(pm::TOOL_GAP)),
                );
            }
            content = content.push(
                text(label)
                    .size(ui_text::UI)
                    .line_height(ui_text::line())
                    .wrapping(iced::widget::text::Wrapping::None)
                    .width(Length::Fill),
            );
            if dirty {
                content = content.push(dot(pm::DIRTY_DOT, chrome::text));
            }
            // Pressed through a `mouse_area`, not a button: an
            // `iced_aw::ContextMenu` rebuilds its overlay on every view pass,
            // and a button there loses its press before the release lands.
            let enabled = on_press.is_some();
            let lit = move |hovered: bool| {
                move |theme: &Theme| container::Style {
                    background: (hovered && enabled)
                        .then(|| Background::Color(chrome::hover(theme))),
                    text_color: Some(if enabled {
                        chrome::text(theme)
                    } else {
                        chrome::muted(theme)
                    }),
                    ..Default::default()
                }
            };
            let body = |hovered: bool| {
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fixed(pm::MENU_ITEM_H))
                    .style(lit(hovered))
            };
            let face = container(
                content
                    .height(Length::Fixed(pm::MENU_ITEM_H))
                    .padding(iced::Padding::ZERO.right(pm::MENU_PAD_X)),
            )
            .width(Length::Fill)
            .style(lit(false));
            let mut area = mouse_area(iced::widget::stack![
                iced::widget::hover(body(false), body(true)),
                face
            ]);
            if let Some(m) = on_press {
                area = area
                    .on_press(*m)
                    .interaction(iced::mouse::Interaction::Pointer);
            }
            match tip {
                Some(tip) => crate::ui::kit::tooltip(area, tip, crate::ui::kit::TipPosition::Right),
                None => area.into(),
            }
        }
    }
}

/// The menu: its items in a raised frame as wide as its longest item
/// (see [`width`]).
pub fn menu<'a>(items: Vec<Item>) -> Element<'a, Message> {
    let width = width(&items);
    container(column(items.into_iter().map(item)).width(Length::Fill))
        .width(Length::Fixed(width))
        .padding([pm::MENU_PAD_Y, 0.0])
        .style(frame_style)
        .into()
}

/// Width of a menu holding `items`: its longest item — the mark column,
/// the icon, the label as the UI font measures it, a dirty dot — plus the
/// right padding and the frame's border.
pub fn width(items: &[Item]) -> f32 {
    let widest = items
        .iter()
        .filter_map(|it| match it {
            Item::Separator => None,
            Item::Heading(caption) => Some(
                pm::MENU_CHECK_W
                    + crate::ui::style::text_metrics::width(caption, ui_text::UI_SMALL, false)
                        .ceil(),
            ),
            Item::Action {
                icon, label, dirty, ..
            } => {
                let icon = if icon.is_some() {
                    ITEM_ICON + pm::TOOL_GAP
                } else {
                    0.0
                };
                let dot = if *dirty {
                    pm::TAB_INNER_GAP + pm::DIRTY_DOT
                } else {
                    0.0
                };
                let text = crate::ui::style::text_metrics::width(label, ui_text::UI, false).ceil();
                Some(pm::MENU_CHECK_W + icon + text + dot)
            }
        })
        .fold(0.0_f32, f32::max);
    (widest + pm::MENU_PAD_X + 2.0 * pm::LINE).ceil()
}

/// Height of a menu holding `items`, for keeping it on screen.
pub fn height(items: &[Item]) -> f32 {
    2.0 * pm::MENU_PAD_Y
        + 2.0 * pm::LINE
        + items
            .iter()
            .map(|i| match i {
                Item::Separator => pm::MENU_SEPARATOR_H,
                Item::Heading(_) | Item::Action { .. } => pm::MENU_ITEM_H,
            })
            .sum::<f32>()
}

/// The menu frame: the palette body with a 1 px edge, no rounding.
pub fn frame_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(chrome::palette_body(theme))),
        text_color: Some(chrome::text(theme)),
        border: Border {
            color: chrome::edge(theme),
            width: pm::LINE,
            radius: 0.0.into(),
        },
        shadow: iced::Shadow {
            color: iced::Color::BLACK.scale_alpha(MENU_SHADOW_ALPHA),
            offset: iced::Vector::new(0.0, MENU_SHADOW_Y),
            blur_radius: MENU_SHADOW_BLUR,
        },
        ..Default::default()
    }
}

/// The menu's drop shadow.
const MENU_SHADOW_ALPHA: f32 = 0.35;
const MENU_SHADOW_Y: f32 = 2.0;
const MENU_SHADOW_BLUR: f32 = 8.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_counts_items_and_separators() {
        let items = vec![
            Item::new("A", Message::Noop),
            Item::Separator,
            Item::new("B", Message::Noop).mark(Mark::Check(true)),
        ];
        assert_eq!(
            height(&items),
            2.0 * pm::MENU_PAD_Y + 2.0 * pm::LINE + 2.0 * pm::MENU_ITEM_H + pm::MENU_SEPARATOR_H
        );
    }

    #[test]
    fn a_menu_is_as_wide_as_its_longest_item() {
        let short = vec![Item::new("Start", Message::Noop)];
        let long = vec![
            Item::new("Start", Message::Noop),
            Item::Separator,
            Item::new("Tabs above the palettes", Message::Noop),
        ];
        let label =
            crate::ui::style::text_metrics::width("Tabs above the palettes", ui_text::UI, false)
                .ceil();
        assert!(width(&long) > width(&short));
        assert_eq!(
            width(&long),
            (pm::MENU_CHECK_W + label + pm::MENU_PAD_X + 2.0 * pm::LINE).ceil()
        );
        // An icon and a dirty dot add their room.
        let more = vec![Item::new("Tabs above the palettes", Message::Noop)
            .icon(crate::ui::icons::PLUS)
            .dirty(true)];
        assert!(width(&more) > width(&long));
    }
}
