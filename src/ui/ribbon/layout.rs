//! The user's ribbon layout: hidden tabs and panels, the panel order, and the
//! minimize and slide-out preferences. Persisted in `RibbonConfig`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::modules::{CadModule, RibbonGroup};

/// How far the ribbon is minimized.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum RibbonMinimize {
    /// The full ribbon.
    #[default]
    Off,
    /// Only the tab row.
    Tabs,
    /// The tab row and a row of panel titles.
    PanelTitles,
    /// The tab row and a row of one button per panel.
    PanelButtons,
}

/// What the minimize button does.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum MinimizePref {
    /// Step through every minimize state.
    #[default]
    Cycle,
    /// Switch between the full ribbon and the tab row.
    Tabs,
    /// Switch between the full ribbon and the panel titles.
    PanelTitles,
    /// Switch between the full ribbon and the panel buttons.
    PanelButtons,
}

/// One item of the Quick Access Toolbar.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum QatItem {
    /// Any command line, e.g. "QSAVE", "HATCH", "ZOOM E", or the id of a
    /// ribbon dropdown (a split item whose face runs the last one chosen).
    Command(String),
    /// A built-in control with its own UI.
    Builtin(QatBuiltin),
    Separator,
}

impl QatItem {
    /// Whether `self` and `other` are the same item: commands ignore case
    /// and surrounding spaces.
    pub fn same(&self, other: &QatItem) -> bool {
        match (self, other) {
            (QatItem::Command(a), QatItem::Command(b)) => a.trim().eq_ignore_ascii_case(b.trim()),
            _ => self == other,
        }
    }
}

/// The Quick Access Toolbar's controls with a UI of their own.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum QatBuiltin {
    Undo,
    Redo,
    LayerCombo,
    SwitchWindows,
    UserInterface,
}

/// The toolbar a user with no saved one sees: the file commands, then Undo
/// and Redo.
pub fn default_qat() -> Vec<QatItem> {
    let command = |cmd: &str| QatItem::Command(cmd.to_string());
    vec![
        command("QNEW"),
        command("OPEN"),
        command("SAVE"),
        command("SAVEAS"),
        command("PRINT"),
        QatItem::Builtin(QatBuiltin::Undo),
        QatItem::Builtin(QatBuiltin::Redo),
    ]
}

/// The ribbon's persisted layout. Panels are named by their panel key
/// (`extras::panel_key`), tabs by their module id.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RibbonLayout {
    /// Module ids of the tabs the user hid.
    pub hidden_tabs: BTreeSet<String>,
    /// Panel keys of the panels the user hid.
    pub hidden_panels: BTreeSet<String>,
    /// Per module id, its panel keys in the user's order.
    pub panel_order: BTreeMap<String, Vec<String>>,
    pub show_panel_titles: bool,
    pub minimize: RibbonMinimize,
    pub minimize_pref: MinimizePref,
    /// Panel keys of the slide-outs pinned open.
    pub pinned_slideouts: BTreeSet<String>,
    /// The command search box is kept open; collapsed it shows as its
    /// magnifier button.
    pub search_expanded: bool,
    /// The Quick Access Toolbar, left to right.
    pub qat: Vec<QatItem>,
    /// Show the Quick Access Toolbar in its own row below the ribbon.
    pub qat_below: bool,
    /// Per split item, the command its face runs (the last one chosen).
    pub qat_faces: BTreeMap<String, String>,
}

impl Default for RibbonLayout {
    fn default() -> Self {
        Self {
            hidden_tabs: BTreeSet::new(),
            hidden_panels: BTreeSet::new(),
            panel_order: BTreeMap::new(),
            show_panel_titles: true,
            minimize: RibbonMinimize::default(),
            minimize_pref: MinimizePref::default(),
            pinned_slideouts: BTreeSet::new(),
            search_expanded: true,
            qat: default_qat(),
            qat_below: false,
            qat_faces: BTreeMap::new(),
        }
    }
}

/// Whether `key` is the panel key of the panel `title` on module `module_id`.
fn is_key(key: &str, module_id: &str, title: &str) -> bool {
    key.strip_prefix(module_id)
        .and_then(|rest| rest.strip_prefix('/'))
        == Some(title)
}

impl RibbonLayout {
    /// `groups` of module `module_id` in the user's order. The panels the
    /// order lists take the places the listed panels have by default, sorted
    /// as listed; a panel it does not list (a new one) keeps its place.
    pub(super) fn order<'g>(
        &self,
        module_id: &str,
        mut groups: Vec<&'g RibbonGroup>,
    ) -> Vec<&'g RibbonGroup> {
        let Some(order) = self.panel_order.get(module_id) else {
            return groups;
        };
        let rank = |group: &RibbonGroup| {
            order
                .iter()
                .position(|key| is_key(key, module_id, group.title))
        };
        let slots: Vec<usize> = (0..groups.len())
            .filter(|&i| rank(groups[i]).is_some())
            .collect();
        let mut listed: Vec<&RibbonGroup> = slots.iter().map(|&i| groups[i]).collect();
        listed.sort_by_key(|group| rank(group));
        for (slot, group) in slots.into_iter().zip(listed) {
            groups[slot] = group;
        }
        groups
    }

    /// Back to the default panel order, every tab and panel shown and panel
    /// titles on. The minimize state and pinned slide-outs stay.
    pub(super) fn reset(&mut self) {
        self.hidden_tabs.clear();
        self.hidden_panels.clear();
        self.panel_order.clear();
        self.show_panel_titles = true;
    }

    /// Whether the Quick Access Toolbar holds `item`.
    pub(super) fn qat_has(&self, item: &QatItem) -> bool {
        self.qat.iter().any(|held| held.same(item))
    }

    /// Add `item` at the end of the Quick Access Toolbar unless it is there
    /// already; whether it was added. Separators are always added.
    pub(super) fn qat_add(&mut self, item: QatItem) -> bool {
        if item != QatItem::Separator && self.qat_has(&item) {
            return false;
        }
        self.qat.push(item);
        true
    }

    /// Take `item` off the Quick Access Toolbar, with a separator it leaves
    /// at either end or doubled; whether it was there.
    pub(super) fn qat_remove(&mut self, item: &QatItem) -> bool {
        let before = self.qat.len();
        self.qat.retain(|held| !held.same(item));
        self.tidy_qat_separators();
        self.qat.len() != before
    }

    /// Add `item` if the toolbar lacks it, else take it off.
    pub(super) fn qat_toggle(&mut self, item: QatItem) {
        if !self.qat_remove(&item) {
            self.qat_add(item);
        }
    }

    /// Drop separators at the ends of the toolbar and doubled ones.
    fn tidy_qat_separators(&mut self) {
        let mut tidy: Vec<QatItem> = Vec::with_capacity(self.qat.len());
        for item in self.qat.drain(..) {
            let is_separator = item == QatItem::Separator;
            if is_separator && tidy.last().is_none_or(|last| *last == QatItem::Separator) {
                continue;
            }
            tidy.push(item);
        }
        if tidy.last() == Some(&QatItem::Separator) {
            tidy.pop();
        }
        self.qat = tidy;
    }

    pub(super) fn panel_hidden(&self, module_id: &str, title: &str) -> bool {
        self.hidden_panels
            .iter()
            .any(|key| is_key(key, module_id, title))
    }

    /// Whether `modules` show as they do by default: no tab or panel of
    /// theirs hidden, every panel in its default place and panel titles on.
    /// Entries naming a tab or panel that no longer exists do not count.
    pub(super) fn is_default(&self, modules: &[Box<dyn CadModule>]) -> bool {
        self.show_panel_titles
            && modules.iter().all(|module| {
                let id = module.id();
                let groups: Vec<&RibbonGroup> = module.ribbon_groups().iter().collect();
                !self.hidden_tabs.contains(id)
                    && groups
                        .iter()
                        .all(|group| !self.panel_hidden(id, group.title))
                    && self
                        .order(id, groups.clone())
                        .iter()
                        .zip(&groups)
                        .all(|(a, b)| std::ptr::eq(*a, *b))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group(title: &'static str) -> RibbonGroup {
        RibbonGroup {
            title,
            tools: vec![],
        }
    }

    fn titles(groups: &[&RibbonGroup]) -> Vec<&'static str> {
        groups.iter().map(|group| group.title).collect()
    }

    struct Tab(Vec<RibbonGroup>);

    impl CadModule for Tab {
        fn id(&self) -> &'static str {
            "draw"
        }
        fn title(&self) -> &'static str {
            "Draw"
        }
        fn ribbon_groups(&self) -> &[RibbonGroup] {
            &self.0
        }
    }

    fn order_of(keys: &[&str]) -> RibbonLayout {
        let mut layout = RibbonLayout::default();
        layout.panel_order.insert(
            "draw".to_string(),
            keys.iter().map(|key| key.to_string()).collect(),
        );
        layout
    }

    #[test]
    fn the_quick_access_toolbar_adds_once_and_removes_ignoring_case() {
        let mut layout = RibbonLayout::default();
        let hatch = QatItem::Command("HATCH".into());
        assert!(!layout.qat_has(&hatch));
        assert!(layout.qat_add(hatch.clone()));
        assert!(!layout.qat_add(QatItem::Command(" hatch ".into())));
        assert_eq!(layout.qat.last(), Some(&hatch));

        layout.qat_toggle(QatItem::Command("hatch".into()));
        assert!(!layout.qat_has(&hatch));
        layout.qat_toggle(QatItem::Builtin(QatBuiltin::LayerCombo));
        assert_eq!(
            layout.qat.last(),
            Some(&QatItem::Builtin(QatBuiltin::LayerCombo))
        );
    }

    #[test]
    fn removing_an_item_drops_the_separators_it_strands() {
        let mut layout = RibbonLayout::default();
        let save = QatItem::Command("SAVE".into());
        let print = QatItem::Command("PRINT".into());
        layout.qat = vec![
            save.clone(),
            QatItem::Separator,
            print.clone(),
            QatItem::Separator,
            QatItem::Builtin(QatBuiltin::Undo),
        ];
        layout.qat_remove(&print);
        assert_eq!(
            layout.qat,
            [
                save.clone(),
                QatItem::Separator,
                QatItem::Builtin(QatBuiltin::Undo)
            ]
        );
        layout.qat_remove(&save);
        assert_eq!(layout.qat, [QatItem::Builtin(QatBuiltin::Undo)]);
    }

    #[test]
    fn the_order_sorts_the_panels_it_lists() {
        let (a, b, c) = (group("A"), group("B"), group("C"));
        let layout = order_of(&["draw/C", "draw/A", "draw/B"]);
        assert_eq!(
            titles(&layout.order("draw", vec![&a, &b, &c])),
            ["C", "A", "B"]
        );
        // Another tab's order does not apply.
        assert_eq!(
            titles(&layout.order("insert", vec![&a, &b, &c])),
            ["A", "B", "C"]
        );
    }

    #[test]
    fn a_panel_the_order_does_not_list_keeps_its_place() {
        let (a, n, b, c) = (group("A"), group("New"), group("B"), group("C"));
        let layout = order_of(&["draw/C", "draw/A", "draw/B", "draw/Gone"]);
        assert_eq!(
            titles(&layout.order("draw", vec![&a, &n, &b, &c])),
            ["C", "New", "A", "B"]
        );
    }

    #[test]
    fn a_hidden_panel_is_named_by_tab_and_title() {
        let mut layout = RibbonLayout::default();
        layout.hidden_panels.insert("draw/A".to_string());
        layout.hidden_panels.insert("insert/B".to_string());
        assert!(layout.panel_hidden("draw", "A"));
        assert!(!layout.panel_hidden("draw", "B"));
        assert!(!layout.panel_hidden("insert", "A"));
        assert!(!layout.panel_hidden("dra", "w/A"));
    }

    #[test]
    fn the_default_layout_ignores_stale_entries() {
        let modules: Vec<Box<dyn CadModule>> = vec![Box::new(Tab(vec![group("A"), group("B")]))];
        let mut layout = RibbonLayout::default();
        assert!(layout.is_default(&modules));

        // Naming panels and tabs that do not exist, or listing the default
        // order, changes nothing.
        layout.hidden_panels.insert("draw/Gone".to_string());
        layout.hidden_tabs.insert("gone".to_string());
        layout
            .panel_order
            .insert("draw".to_string(), vec!["draw/A".into(), "draw/B".into()]);
        assert!(layout.is_default(&modules));

        let mut moved = layout.clone();
        moved
            .panel_order
            .insert("draw".to_string(), vec!["draw/B".into(), "draw/A".into()]);
        assert!(!moved.is_default(&modules));

        let mut hidden = layout.clone();
        hidden.hidden_panels.insert("draw/B".to_string());
        assert!(!hidden.is_default(&modules));

        let mut hidden_tab = layout.clone();
        hidden_tab.hidden_tabs.insert("draw".to_string());
        assert!(!hidden_tab.is_default(&modules));

        let mut no_titles = layout;
        no_titles.show_panel_titles = false;
        assert!(!no_titles.is_default(&modules));
    }
}
