//! The Quick Access Toolbar: the items the user chose (`RibbonLayout::qat`),
//! each drawn by its kind, with the customize button at the end.

use iced::widget::{row, text};
use iced::{Element, Theme};

use super::layout::{QatBuiltin, QatItem};
use super::widgets::{quick_access_btn, render_history_control, TOP_HIST_GAP};
use super::{Ribbon, REDO_HISTORY_ID, UNDO_HISTORY_ID};
use crate::app::Message;
use crate::modules::IconKind;
use crate::ui::icons;

/// What a command item shows.
#[derive(Clone, Copy)]
pub(super) enum FaceIcon {
    /// One of the toolbar's own chrome icons, tinted for the tab row.
    Chrome(&'static [u8]),
    /// A ribbon or menu icon, in its own colours.
    Tool(IconKind),
    /// No icon anywhere: the button shows the command name.
    Text,
}

/// The icon and label of a command item.
#[derive(Clone)]
pub(super) struct Face {
    /// The label, localized, on one line.
    pub label: String,
    pub icon: FaceIcon,
}

/// The toolbar's own file buttons keep the icons and labels they always had.
static FILE_FACES: [(&str, &[u8], &str); 6] = [
    ("QNEW", icons::DOC_NEW, "New"),
    ("OPEN", icons::FOLDER_OPEN, "Open"),
    ("SAVE", icons::SAVE, "Save"),
    ("QSAVE", icons::SAVE, "Save"),
    ("SAVEAS", icons::FILE_EXPORT, "Save As"),
    ("PRINT", icons::PRINT, "Print"),
];

impl Ribbon {
    /// The icon and label of the command item `cmd`: the toolbar's own file
    /// buttons first, then the ribbon button that runs it, then its menu-bar
    /// entry; with none of them its name.
    pub(super) fn command_face(&self, cmd: &str) -> Face {
        let cmd = cmd.trim();
        if let Some(&(_, icon, label)) = FILE_FACES
            .iter()
            .find(|(command, _, _)| command.eq_ignore_ascii_case(cmd))
        {
            return Face {
                label: crate::t!(label).into_owned(),
                icon: FaceIcon::Chrome(icon),
            };
        }
        if let Some((label, icon)) = self.ribbon_face(cmd) {
            return Face {
                label: crate::t!(label).replace('\n', " "),
                icon: FaceIcon::Tool(icon),
            };
        }
        if let Some((label, icon)) = crate::ui::menu_bar::entry_for_line(cmd) {
            return Face {
                label: label.trim_end_matches('.').to_string(),
                icon: icon.map_or(FaceIcon::Text, |icon| FaceIcon::Tool(IconKind::Svg(icon))),
            };
        }
        if let Some(icon) = crate::modules::registry::command_icon(cmd) {
            return Face {
                label: cmd.to_string(),
                icon: FaceIcon::Tool(IconKind::Svg(icon)),
            };
        }
        Face {
            label: cmd.to_string(),
            icon: FaceIcon::Text,
        }
    }

    /// The toolbar's items, left to right. Greyed out and inert on the Start
    /// tab, which has no drawing for them.
    pub(super) fn qat_items(
        &self,
        is_start: bool,
        undo_count: usize,
        redo_count: usize,
    ) -> Vec<Element<'_, Message>> {
        self.layout
            .qat
            .iter()
            .filter_map(|item| match item {
                QatItem::Command(cmd) => Some(quick_access_btn(
                    self.command_face(cmd),
                    cmd.clone(),
                    !is_start,
                )),
                QatItem::Builtin(QatBuiltin::Undo) => Some(render_history_control(
                    "Undo",
                    UNDO_HISTORY_ID,
                    if is_start { 0 } else { undo_count },
                    &self.open_dropdown,
                )),
                QatItem::Builtin(QatBuiltin::Redo) => Some(render_history_control(
                    "Redo",
                    REDO_HISTORY_ID,
                    if is_start { 0 } else { redo_count },
                    &self.open_dropdown,
                )),
                QatItem::Builtin(_) | QatItem::Separator => None,
            })
            .collect()
    }

    /// The toolbar as one flow that wraps.
    pub(super) fn qat_row(
        &self,
        is_start: bool,
        undo_count: usize,
        redo_count: usize,
    ) -> Element<'_, Message> {
        iced::widget::Row::with_children(self.qat_items(is_start, undo_count, redo_count))
            .spacing(TOP_HIST_GAP)
            .align_y(iced::Center)
            .wrap()
            .vertical_spacing(0.0)
            .into()
    }
}

/// The icon of a command item, greyed out unless `enabled`; a command with
/// no icon shows its name.
pub(super) fn face_icon<'a>(face: &Face, size: f32, enabled: bool) -> Element<'a, Message> {
    match face.icon {
        FaceIcon::Chrome(bytes) if enabled => icons::chrome(bytes, size),
        FaceIcon::Chrome(bytes) => icons::chrome_disabled(bytes, size),
        FaceIcon::Tool(IconKind::Svg(bytes)) if enabled => icons::tool(bytes, size),
        FaceIcon::Tool(IconKind::Svg(bytes)) => icons::tool_disabled(bytes, size),
        FaceIcon::Tool(IconKind::Glyph(glyph)) => text(glyph).size(size * 0.7).into(),
        FaceIcon::Text => {
            let label = face.label.clone();
            row![text(label)
                .size(11)
                .wrapping(iced::widget::text::Wrapping::None)
                .style(move |theme: &Theme| text::Style {
                    color: (!enabled).then(|| crate::ui::style::chrome::muted(theme)),
                })]
            .into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_toolbar_keeps_its_icons_and_labels() {
        let ribbon = Ribbon::default();
        let new = ribbon.command_face("QNEW");
        assert_eq!(new.label, "New");
        assert!(matches!(new.icon, FaceIcon::Chrome(icon) if icon == icons::DOC_NEW));
        assert_eq!(ribbon.command_face("PRINT").label, "Print");
    }

    #[test]
    fn a_ribbon_command_takes_the_ribbon_buttons_face() {
        let ribbon = Ribbon::default();
        let face = ribbon.command_face("HATCH");
        assert!(matches!(face.icon, FaceIcon::Tool(_)));
        assert!(!face.label.is_empty());
        // A slide-out tool and a dropdown item are found too.
        assert_eq!(ribbon.command_face("SPLINE").label, "Spline Fit");
        assert_eq!(ribbon.command_face("AREA").label, "Area");
        // A dropdown by its id shows the dropdown.
        assert_eq!(
            ribbon.command_face("MEASURE_MENU").label,
            crate::t!("Measure")
        );
    }

    #[test]
    fn a_menu_command_takes_the_menu_entry_and_an_unknown_one_its_name() {
        let ribbon = Ribbon::default();
        assert_eq!(ribbon.command_face("ZOOM E").label, "Extents");
        let unknown = ribbon.command_face("NOSUCHCOMMAND");
        assert_eq!(unknown.label, "NOSUCHCOMMAND");
        assert!(matches!(unknown.icon, FaceIcon::Text));
    }
}
