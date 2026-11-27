//! The app side of docking: pallets docked on the drawing view's edges (one
//! group shown per edge, listed in an icon strip) or floating over it.
//!
//! The persisted layout and its pure geometry live in [`crate::ui::dock`];
//! this module holds the transient state handling (`update`) and drawing
//! (`view`).

/// Pallet-menu listing cap: the first N plugin panels per edge, sorted by
/// title. Registration beyond the cap still works (commands, value updates)
/// and reports itself on stderr (gui.log); see docs/plugin-pallet-menu.md.
pub(crate) const PLUGIN_PALLET_MENU_MAX_ROWS: usize = 32;

/// The dock workspace in the main window (see `dock_overlay`).
pub(crate) const DOCK_WORKSPACE_ID: &str = "dock-workspace";
/// The title bar of a pallet's window of its own.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) const PALLET_WINDOW_BAR_ID: &str = "pallet-window-bar";

mod update;
mod view;
#[cfg(not(target_arch = "wasm32"))]
mod window;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tool_palettes_tests;
