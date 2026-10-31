//! The Quick Access Toolbar: the items the user chose (`RibbonLayout::qat`),
//! each drawn by its kind, with the customize button at the end.

use std::time::Duration;

use iced::widget::tooltip::Position as TipPos;
use iced::widget::{button, container, row, text, tooltip, Space};
use iced::{Background, Element, Fill, Length, Theme};

use super::layout::{QatBuiltin, QatItem};
use super::ribbon_menu::{check_row, fit_width, panel_of_width};
use super::widgets::{
    layer_combo_face, make_tip, qat_split_part_style, quick_access_btn, render_history_control,
    split_outline, tip_style, top_hist_btn_style, SMALL_ICON, TOP_HIST_GAP,
};
use super::{Ribbon, REDO_HISTORY_ID, UNDO_HISTORY_ID};
use crate::app::Message;
use crate::modules::IconKind;
use crate::ui::icons;
use crate::ui::style::{chrome, ribbon_metrics as metrics};
use crate::ui::wrap_bar::PosReport;

/// The layer combo in the toolbar; it opens the ribbon combo's list.
pub(crate) const QAT_LAYER_COMBO_ID: &str = "QAT_LAYER_COMBO";
/// The Switch Windows and User Interface menus.
pub(crate) const QAT_SWITCH_ID: &str = "QAT_SWITCH_WINDOWS";
pub(crate) const QAT_UI_ID: &str = "QAT_USER_INTERFACE";
/// Starts the dropdown id of a split item's list.
const SPLIT_PREFIX: &str = "qat:";
const TIP_DELAY: Duration = Duration::from_millis(400);
/// Two overlapping windows.
const SWITCH_WINDOWS_ICON: &str = "M3 5h12v10H3z M9 9h12v10H9z";
/// A window with a bar along its top and a side bar.
const USER_INTERFACE_ICON: &str = "M3 4h18v16H3z M3 9h18 M8 9v11";

/// Whether the Quick Access menu `id` works on the Start tab, which has no
/// drawing.
pub(crate) fn works_without_drawing(id: &str) -> bool {
    matches!(id, QAT_SWITCH_ID | QAT_UI_ID)
}

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
        let enabled = !is_start;
        self.layout
            .qat
            .iter()
            .map(|item| match item {
                QatItem::Command(cmd) => match self.ribbon_dropdown(cmd) {
                    Some((id, label, items, default)) => {
                        self.qat_split(id, label, items, default, enabled)
                    }
                    None => quick_access_btn(self.command_face(cmd), cmd.clone(), enabled),
                },
                QatItem::Builtin(QatBuiltin::Undo) => render_history_control(
                    "Undo",
                    UNDO_HISTORY_ID,
                    if is_start { 0 } else { undo_count },
                    &self.open_dropdown,
                ),
                QatItem::Builtin(QatBuiltin::Redo) => render_history_control(
                    "Redo",
                    REDO_HISTORY_ID,
                    if is_start { 0 } else { redo_count },
                    &self.open_dropdown,
                ),
                QatItem::Builtin(QatBuiltin::LayerCombo) => container(layer_combo_face(
                    QAT_LAYER_COMBO_ID,
                    &self.layer_infos,
                    &self.active_layer,
                    self.open_dropdown.as_deref() == Some(QAT_LAYER_COMBO_ID),
                    metrics::QAT_COMBO_W,
                    false,
                ))
                .padding([0.0, metrics::QAT_COMBO_MARGIN])
                .into(),
                QatItem::Builtin(QatBuiltin::SwitchWindows) => self.qat_menu_button(
                    QAT_SWITCH_ID,
                    SWITCH_WINDOWS_ICON,
                    crate::tr!("ribbon-menu", "switch-windows"),
                ),
                QatItem::Builtin(QatBuiltin::UserInterface) => self.qat_menu_button(
                    QAT_UI_ID,
                    USER_INTERFACE_ICON,
                    crate::tr!("ribbon-menu", "user-interface"),
                ),
                QatItem::Separator => separator(),
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

    /// The command the face of the split item `id` runs: the one last
    /// chosen from its list, else the dropdown's default.
    pub(super) fn qat_face_command(
        &self,
        id: &str,
        items: &[(&'static str, &'static str, IconKind)],
        default: &'static str,
    ) -> &'static str {
        self.layout
            .qat_faces
            .get(id)
            .and_then(|chosen| items.iter().find(|(cmd, _, _)| cmd == chosen))
            .map_or(default, |(cmd, _, _)| *cmd)
    }

    /// A split item: its face runs the variant last chosen, its arrow opens
    /// the list the ribbon dropdown `id` shows.
    fn qat_split(
        &self,
        id: &'static str,
        label: &'static str,
        items: &[(&'static str, &'static str, IconKind)],
        default: &'static str,
        enabled: bool,
    ) -> Element<'_, Message> {
        let face_cmd = self.qat_face_command(id, items, default);
        let (face_label, face_icon_kind) = items
            .iter()
            .find(|(cmd, _, _)| *cmd == face_cmd)
            .map_or((label, None), |(_, label, icon)| (*label, Some(*icon)));
        let open_id = split_dropdown_id(id);
        let open = self.open_dropdown.as_deref() == Some(open_id.as_str());
        let face = Face {
            label: crate::t!(face_label).into_owned(),
            icon: face_icon_kind.map_or(FaceIcon::Text, FaceIcon::Tool),
        };
        let face_btn = button(
            container(face_icon(&face, SMALL_ICON, enabled))
                .center_x(Fill)
                .center_y(Fill),
        )
        .on_press_maybe(enabled.then(|| Message::MenuCommand(face_cmd.to_string())))
        .style(move |theme: &Theme, status| qat_split_part_style(theme, enabled, open, status))
        .width(Length::Fixed(metrics::QAT_BUTTON))
        .height(metrics::QAT_BUTTON)
        .padding(0);
        let face_btn = tooltip(
            face_btn,
            make_tip(command_tip(&face.label, face_cmd)),
            TipPos::Bottom,
        )
        .gap(6.0)
        .delay(TIP_DELAY)
        .style(tip_style);
        let arrow = button(
            container(icons::dropdown_arrow())
                .center_x(Fill)
                .center_y(Fill),
        )
        .on_press_maybe(enabled.then(|| Message::ToggleRibbonDropdown(open_id.clone())))
        .style(move |theme: &Theme, status| qat_split_part_style(theme, enabled, open, status))
        .width(Length::Fixed(metrics::QAT_SPLIT_ARROW_W))
        .height(metrics::QAT_BUTTON)
        .padding(0);
        let arrow = tooltip(
            arrow,
            make_tip(crate::t!(label).replace('\n', " ")),
            TipPos::Bottom,
        )
        .gap(6.0)
        .delay(TIP_DELAY)
        .style(tip_style);
        let parts: Element<'_, Message> = row![face_btn, arrow].into();
        let parts = if enabled { split_outline(parts) } else { parts };
        let anchor = super::keytips::quick_access_anchor(id);
        super::keytips::anchor_with(move || anchor, PosReport::owned(open_id, parts))
    }

    /// An icon with a ▾ that opens the menu `id`.
    fn qat_menu_button(
        &self,
        id: &'static str,
        icon: &'static str,
        tip: String,
    ) -> Element<'_, Message> {
        let open = self.open_dropdown.as_deref() == Some(id);
        let btn = button(
            row![icons::line_icon(icon, SMALL_ICON), icons::dropdown_arrow()]
                .spacing(2)
                .align_y(iced::Center),
        )
        .on_press(Message::ToggleRibbonDropdown(id.to_string()))
        .style(move |theme: &Theme, status| top_hist_btn_style(theme, true, open, status))
        .width(Length::Fixed(metrics::QAT_MENU_BUTTON_W))
        .height(metrics::QAT_BUTTON)
        .padding([0.0, metrics::SMALL_PAD]);
        let btn = tooltip(btn, make_tip(tip), TipPos::Bottom)
            .gap(6.0)
            .delay(TIP_DELAY)
            .style(tip_style);
        let anchor = super::keytips::quick_access_anchor(id);
        super::keytips::anchor_with(move || anchor, PosReport::new(id, btn))
    }

    /// The open Quick Access list or menu `open_id`, if it is one: a split
    /// item's variants, the layer list, the open drawings or the user
    /// interface switches. They work without a drawing, except the variants
    /// and the layers.
    pub(super) fn qat_overlay(
        &self,
        open_id: &str,
        win: (f32, f32),
        is_start: bool,
        drawings: &[(usize, String, bool)],
    ) -> Option<Element<'_, Message>> {
        if let Some(id) = open_id.strip_prefix(SPLIT_PREFIX) {
            if is_start {
                return None;
            }
            let (id, _, items, default) = self.ribbon_dropdown(id)?;
            let current = self.qat_face_command(id, items, default);
            let (panel, width) = super::dropdown_list(id, items, current, None, |cmd| {
                Message::RibbonQat(QatMsg::Pick {
                    item: id.to_string(),
                    cmd,
                })
            });
            return Some(self.place_dropdown(open_id, panel, width, win.0));
        }
        match open_id {
            QAT_LAYER_COMBO_ID if !is_start => self.layer_combo_overlay(QAT_LAYER_COMBO_ID, win),
            QAT_SWITCH_ID => {
                let rows: Vec<(String, bool, Option<Message>)> = drawings
                    .iter()
                    .map(|(index, name, current)| {
                        (name.clone(), *current, Some(Message::TabSwitch(*index)))
                    })
                    .collect();
                let rows = if rows.is_empty() {
                    vec![(crate::tr!("ribbon-menu", "no-open-drawings"), false, None)]
                } else {
                    rows
                };
                Some(self.qat_menu(QAT_SWITCH_ID, rows, win))
            }
            QAT_UI_ID => {
                let rows = [
                    (
                        crate::tr!("ribbon-menu", "viewcube"),
                        "NAVVCUBE",
                        self.show_viewcube,
                    ),
                    (
                        crate::tr!("ribbon-menu", "ucs-icon"),
                        "UCSICON",
                        self.show_ucs_icon,
                    ),
                    (
                        crate::tr!("ribbon-menu", "file-tabs"),
                        "FILETAB",
                        self.show_file_tabs,
                    ),
                    (
                        crate::tr!("ribbon-menu", "layout-tabs"),
                        "LAYOUTTAB",
                        self.show_layout_tabs,
                    ),
                    // OCS has no switch for these yet; the command line
                    // says so when they are picked.
                    (
                        crate::tr!("ribbon-menu", "command-line"),
                        "COMMANDLINE",
                        false,
                    ),
                    (crate::tr!("ribbon-menu", "status-bar"), "STATUSBAR", false),
                ]
                .into_iter()
                .map(|(label, cmd, on)| (label, on, Some(Message::MenuCommand(cmd.to_string()))))
                .collect();
                Some(self.qat_menu(QAT_UI_ID, rows, win))
            }
            _ => None,
        }
    }

    /// A Quick Access menu of checkable `rows` under the button `id`.
    fn qat_menu(
        &self,
        id: &str,
        rows: Vec<(String, bool, Option<Message>)>,
        win: (f32, f32),
    ) -> Element<'_, Message> {
        let width = fit_width(rows.iter().map(|(label, _, _)| label.as_str()));
        let items = rows
            .into_iter()
            .map(|(label, checked, on_press)| check_row(label, checked, on_press).into())
            .collect();
        self.place_dropdown(id, panel_of_width(items, width), width, win.0)
    }

    /// Apply a Quick Access message. Returns whether the toolbar changed and
    /// should be saved, and what to run.
    pub fn on_qat(&mut self, msg: QatMsg) -> QatOutcome {
        match msg {
            QatMsg::Pick { item, cmd } => {
                self.close_dropdown();
                self.layout.qat_faces.insert(item, cmd.to_string());
                QatOutcome {
                    save: true,
                    run: Some(Message::MenuCommand(cmd.to_string())),
                }
            }
        }
    }
}

/// A Quick Access message.
#[derive(Clone, Debug)]
pub enum QatMsg {
    /// From the list of the split item `item`: run `cmd`, which its face
    /// shows from now on.
    Pick { item: String, cmd: &'static str },
}

/// What the app does after a Quick Access message.
#[derive(Default)]
pub struct QatOutcome {
    /// The toolbar changed and should be saved.
    pub save: bool,
    /// A message to run.
    pub run: Option<Message>,
}

/// The dropdown id of the list of the split item `id`.
fn split_dropdown_id(id: &str) -> String {
    format!("{SPLIT_PREFIX}{id}")
}

/// A command's tooltip: its label and the command it runs.
pub(super) fn command_tip(label: &str, cmd: &str) -> String {
    format!("{label}\n{} {cmd}", crate::t!("Command:"))
}

/// A 1 px line between two items.
fn separator<'a>() -> Element<'a, Message> {
    container(
        container(
            Space::new()
                .width(icons::hairline())
                .height(metrics::QAT_SEPARATOR_H),
        )
        .style(|theme: &Theme| container::Style {
            background: Some(Background::Color(chrome::border(theme))),
            ..Default::default()
        }),
    )
    .padding([0.0, metrics::QAT_SEPARATOR_MARGIN])
    .into()
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

    #[test]
    fn a_split_item_runs_the_variant_last_chosen() {
        let mut ribbon = Ribbon::default();
        ribbon.layout.qat = vec![QatItem::Command("MEASURE_MENU".into())];
        let (id, _, items, default) = ribbon.ribbon_dropdown("measure_menu").unwrap();
        assert_eq!(id, "MEASURE_MENU");
        assert_eq!(ribbon.qat_face_command(id, items, default), "DIST");

        let out = ribbon.on_qat(QatMsg::Pick {
            item: id.to_string(),
            cmd: "AREA",
        });
        assert!(out.save);
        assert!(matches!(out.run, Some(Message::MenuCommand(ref cmd)) if cmd == "AREA"));
        let (id, _, items, default) = ribbon.ribbon_dropdown(id).unwrap();
        assert_eq!(ribbon.qat_face_command(id, items, default), "AREA");

        // A saved face the list no longer has falls back to the default.
        ribbon
            .layout
            .qat_faces
            .insert("MEASURE_MENU".to_string(), "GONE".to_string());
        let (id, _, items, default) = ribbon.ribbon_dropdown("MEASURE_MENU").unwrap();
        assert_eq!(ribbon.qat_face_command(id, items, default), "DIST");
    }

    #[test]
    fn a_plain_command_is_not_a_split_item() {
        let ribbon = Ribbon::default();
        assert!(ribbon.ribbon_dropdown("LINE").is_none());
        // Hatch has variants on the ribbon, so it is one.
        assert!(ribbon.ribbon_dropdown("HATCH").is_some());
        assert!(ribbon.ribbon_dropdown("ZOOM E").is_none());
    }

    #[test]
    fn the_window_menus_work_without_a_drawing_and_the_lists_need_one() {
        assert!(super::super::works_without_drawing(QAT_SWITCH_ID));
        assert!(super::super::works_without_drawing(QAT_UI_ID));
        assert!(!super::super::works_without_drawing(QAT_LAYER_COMBO_ID));
        assert!(!super::super::works_without_drawing(&split_dropdown_id(
            "MEASURE_MENU"
        )));
    }
}
