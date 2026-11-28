//! A pallet in a window of its own (desktop): its title bar drags the window
//! back into the main window, where it docks or floats as a pallet floating
//! there would, or anywhere else on the screen; a double-click or its menu
//! brings it back; with its pin off it folds down to its title bar while the
//! pointer is away.
//!
//! While the title bar is held, the pallet's window keeps getting the
//! pointer even outside it (the system's pointer capture), as raw events
//! (`tool_windows::set_pointer_feed`). Its points are in that window's
//! logical space; with the window's place on the screen (physical pixels,
//! kept by its Moved events) they turn into screen points, and with the main
//! window's place into points of the main window's dock workspace, where
//! the usual drop targets apply:
//!
//! ```text
//! screen     = pallet_window_at + local * unit(pallet_scale)
//! main_local = (screen - main_window_at * unit(main_scale)) / unit(main_scale)
//! workspace  = main_local - workspace_origin
//! ```
//!
//! The screen's unit is the physical pixel on Windows and X11, and the point
//! on macOS, whose screens share one space in points whatever their scale
//! (`unit`, `SCREEN_IN_POINTS`).
//!
//! Native Wayland leaves the move to the compositor: no window learns where
//! it is there (see `tool_windows::can_drag_dock`).

use crate::app::tool_windows::{Detached, WindowPlacement};
use crate::app::{Message, OpenCADStudio};
use crate::ui::dock::{DockDrag, DockMsg, DragSource, DropTarget, PanelId};
use iced::{Point, Size, Task, Vector};

/// Pointer travel (logical px) before a press on a pallet window's title bar
/// moves the window, so a click (or the first of a double-click) never does.
const WINDOW_DRAG_THRESHOLD: f32 = 5.0;

/// How far past the main window's edge (logical px) a dragged pallet has to
/// be before it is torn out into a window of its own, so a drag along the
/// edge does not open and close windows.
const TEAR_OUT_MARGIN: f32 = 12.0;

/// Whether the screen is measured in points (macOS) rather than physical
/// pixels (Windows, X11). macOS lays its screens out in points: a window's
/// physical pixels (points × its own scale) mean different places on screens
/// of different scales.
pub(crate) const SCREEN_IN_POINTS: bool = cfg!(target_os = "macos");

/// Screen units per logical pixel of a window on a screen of `scale`.
pub(crate) fn unit(scale: f32, points: bool) -> f32 {
    if points {
        1.0
    } else {
        scale
    }
}

/// A window's place in screen units, from its placement (physical px, see
/// `WindowPlacement`) on a screen of `scale`.
pub(crate) fn placement_on_screen(px: Point, scale: f32, points: bool) -> Point {
    let k = unit(scale, points) / scale;
    Point::new(px.x * k, px.y * k)
}

/// The point of the main window under screen point `screen` (screen units),
/// with the main window's top-left corner at `main_at` (logical) on a screen
/// of scale `main_scale`.
pub(crate) fn screen_to_main(
    screen: Point,
    main_at: Point,
    main_scale: f32,
    points: bool,
) -> Point {
    let u = unit(main_scale, points);
    Point::new(
        (screen.x - main_at.x * u) / u,
        (screen.y - main_at.y * u) / u,
    )
}

impl OpenCADStudio {
    /// The dock messages of a pallet's window of its own.
    pub(super) fn on_pallet_window(&mut self, m: DockMsg) -> Task<Message> {
        match m {
            DockMsg::WindowBarPress(window) => self.dock_window_press(window),
            DockMsg::WindowBarDoubleClick(window) => {
                self.dock_drag = None;
                self.return_os_window(window)
            }
            DockMsg::WindowMenu(window) => {
                self.dock_window_menu = window;
                self.dock_float_menu_at = self.dock_float_pointer;
                if window.is_some() {
                    self.dock_drag = None;
                }
                Task::none()
            }
            DockMsg::WindowResize(window, direction) => {
                iced::window::drag_resize(window, direction)
            }
            DockMsg::WindowPointer(window, point) if Some(window) == self.main_window => {
                self.dock_main_pointer(point)
            }
            DockMsg::WindowRelease(window) if Some(window) == self.main_window => {
                self.dock_main_release()
            }
            DockMsg::WindowPointer(window, point) => self.dock_window_drag_move(window, point),
            DockMsg::WindowRelease(window) => self.dock_window_drag_release(window),
            DockMsg::WindowAt(window, at) => {
                if let Some(at) = at {
                    self.os_window_moved(window, at);
                }
                Task::none()
            }
            DockMsg::WorkspaceOrigin(origin) => {
                if origin.is_some() {
                    self.dock_workspace_origin = origin;
                }
                Task::none()
            }
            DockMsg::WindowHover(window, inside) => self.dock_window_hover(window, inside),
            DockMsg::WindowFoldSettled(window, fold, gen) => {
                self.dock_window_fold_settled(window, fold, gen)
            }
            DockMsg::DragCancel => self.dock_drag_cancel(),
            _ => Task::none(),
        }
    }

    /// The pallet shown in window `window`, and that window.
    fn pallet_window(
        &self,
        window: iced::window::Id,
    ) -> Option<(PanelId, crate::app::tool_windows::OsWindow)> {
        let os = self.os_windows.iter().find(|w| w.id == window).copied()?;
        match os.content {
            Detached::Panel(panel) => Some((panel, os)),
            Detached::Tool(_) => None,
        }
    }

    /// A press on pallet window `window`'s title bar: the drag starts on
    /// the first motion past the threshold. Where the windows are is asked
    /// afresh, as the system may have moved them without telling.
    fn dock_window_press(&mut self, window: iced::window::Id) -> Task<Message> {
        let Some((panel, os)) = self.pallet_window(window) else {
            return Task::none();
        };
        self.dock_window_menu = None;
        let system = !crate::app::tool_windows::can_drag_dock();
        let start = Point::new(os.placement.x, os.placement.y);
        self.dock_drag = Some(DockDrag::moving_window(panel, window, start, system));
        self.dock_tear_undo = None;
        // Where it was pressed, where the system says; else its first motion.
        if let Some(DockDrag::Move { origin, .. }) = self.dock_drag.as_mut() {
            *origin = crate::sys::screen_cursor();
        }
        self.dock_drag_last = None;
        self.dock_edge_menu = None;
        self.dock_float_menu = None;
        if system {
            return Task::none();
        }
        let mut asks = vec![
            iced::window::position(window)
                .map(move |at| Message::Dock(DockMsg::WindowAt(window, at))),
            workspace_origin().map(|at| Message::Dock(DockMsg::WorkspaceOrigin(at))),
        ];
        if let Some(main) = self.main_window {
            asks.push(
                iced::window::position(main)
                    .map(move |at| Message::Dock(DockMsg::WindowAt(main, at))),
            );
        }
        Task::batch(asks)
    }

    /// The pointer moved to `local` (logical, in pallet window `window`)
    /// while its title bar is held: past the threshold the window follows
    /// it, and where it is over the main window's workspace the drop
    /// preview there shows where it would land.
    fn dock_window_drag_move(&mut self, window: iced::window::Id, local: Point) -> Task<Message> {
        let Some(DockDrag::Move {
            source:
                DragSource::OwnWindow {
                    window: dragged,
                    system,
                    torn_out: false,
                    ..
                },
            ..
        }) = self.dock_drag
        else {
            return Task::none();
        };
        if dragged != window {
            return Task::none();
        }
        let Some((_, os)) = self.pallet_window(window) else {
            return Task::none();
        };
        // The window manager moves it: hand the press over on its first
        // motion, so a double-click without moving still reads as one.
        if system {
            self.dock_drag = None;
            return iced::window::drag(window);
        }
        let u = unit(os.scale, SCREEN_IN_POINTS);
        let at = placement_on_screen(
            Point::new(os.placement.x, os.placement.y),
            os.scale,
            SCREEN_IN_POINTS,
        );
        if !(at.x.is_finite() && at.y.is_finite()) {
            return Task::none();
        }
        // The system's own idea of the pointer where it has one; else from
        // the window's place, which can trail the moves asked of it.
        let screen = crate::sys::screen_cursor()
            .unwrap_or_else(|| Point::new(at.x + local.x * u, at.y + local.y * u));
        self.dock_window_follow(window, screen)
    }

    /// The pointer of a pallet window's drag is at screen point `screen`
    /// (screen units): past the threshold the window follows it, and where
    /// it is over the main window's workspace the drop preview there shows
    /// where it would land.
    fn dock_window_follow(&mut self, window: iced::window::Id, screen: Point) -> Task<Message> {
        let Some(DockDrag::Move {
            panel,
            origin,
            bar,
            source:
                DragSource::OwnWindow {
                    window: dragged,
                    start,
                    grab,
                    system,
                    torn_out,
                },
            ..
        }) = self.dock_drag
        else {
            return Task::none();
        };
        if dragged != window || system {
            return Task::none();
        }
        let Some((_, os)) = self.pallet_window(window) else {
            return Task::none();
        };
        let scale = os.scale;
        // Screen units per logical pixel of this window.
        let u = unit(scale, SCREEN_IN_POINTS);
        let at = placement_on_screen(
            Point::new(os.placement.x, os.placement.y),
            scale,
            SCREEN_IN_POINTS,
        );
        if !(at.x.is_finite() && at.y.is_finite()) {
            return Task::none();
        }
        let origin = origin.unwrap_or(screen);
        let grab = match grab {
            Some(grab) => grab,
            None => {
                let moved = (screen.x - origin.x).hypot(screen.y - origin.y) / u;
                if moved < WINDOW_DRAG_THRESHOLD {
                    if let Some(DockDrag::Move { origin: o, .. }) = self.dock_drag.as_mut() {
                        *o = Some(origin);
                    }
                    return Task::none();
                }
                // Held where it was pressed, not where the threshold ran out:
                // the window has not moved yet.
                Vector::new((origin.x - at.x) / u, (origin.y - at.y) / u)
            }
        };
        self.dock_title_hover = None;
        let target = self.dock_window_target(panel, screen, grab, bar);
        let bar = match target {
            Some(
                DropTarget::Edge { side, .. }
                | DropTarget::Join { side, .. }
                | DropTarget::Float { bar: side, .. },
            ) => side,
            None => bar,
        };
        self.dock_drag = Some(DockDrag::Move {
            panel,
            group: None,
            origin: Some(origin),
            target,
            grab,
            bar,
            source: DragSource::OwnWindow {
                window,
                start,
                grab: Some(grab),
                system,
                torn_out,
            },
        });
        // The window's new top-left corner on the screen, and in its own
        // logical units as the system takes them.
        let top_left = Point::new(screen.x - grab.x * u, screen.y - grab.y * u);
        let logical = Point::new(top_left.x / u, top_left.y / u);
        // No move event reliably follows a move the app asks for (X11
        // reports none): it is where the window is from now on. Kept for the
        // next open once the drag ends (`dock_window_drag_release`).
        if let Some(os) = self.os_windows.iter_mut().find(|w| w.id == window) {
            os.placement.x = logical.x * scale;
            os.placement.y = logical.y * scale;
        }
        iced::window::move_to(window, logical)
    }

    /// Where pallet `panel`, held at `grab` in its window, lands with the
    /// pointer at screen point `screen` (screen units): a drop target of the
    /// main window's workspace when it is over that, `None` anywhere else
    /// (it stays a window of its own). With docking off for the pallet, or
    /// Ctrl held, it only floats there.
    pub(crate) fn dock_window_target(
        &mut self,
        panel: PanelId,
        screen: Point,
        grab: Vector,
        bar: crate::app::config::DockSide,
    ) -> Option<DropTarget> {
        let main = screen_to_main(
            screen,
            self.main_window_at?,
            crate::ui::icons::scale_factor(),
            SCREEN_IN_POINTS,
        );
        let (ww, wh) = self.win_size;
        if !(main.x >= 0.0 && main.y >= 0.0 && main.x < ww && main.y < wh) {
            self.dock_drag_last = None;
            return None;
        }
        let origin = self.dock_workspace_origin?;
        let p = Point::new(main.x - origin.x, main.y - origin.y);
        let (sw, sh) = self.dock_workspace_size();
        if !(p.x >= 0.0 && p.y >= 0.0 && p.x < sw && p.y < sh) {
            self.dock_drag_last = None;
            return None;
        }
        // The icon that follows the pointer over a strip.
        self.dock_drag_last = Some(p);
        Some(if self.dock.docking(panel) && !self.ctrl_down {
            self.dock_drop_target(panel, p, grab, bar)
        } else {
            self.dock_float_target(panel, p, grab, bar)
        })
    }

    /// The title bar of pallet window `window` came up: over a dock target
    /// the pallet docks there; over the rest of the workspace it floats
    /// there; anywhere else it stays a window of its own, where it is now.
    pub(crate) fn dock_window_drag_release(&mut self, window: iced::window::Id) -> Task<Message> {
        let Some(DockDrag::Move {
            panel,
            target,
            source:
                DragSource::OwnWindow {
                    window: dragged,
                    grab,
                    torn_out,
                    ..
                },
            ..
        }) = self.dock_drag
        else {
            return Task::none();
        };
        if dragged != window {
            return Task::none();
        }
        self.dock_drag = None;
        self.dock_drag_last = None;
        self.dock_tear_undo = None;
        let Some((_, os)) = self.pallet_window(window) else {
            return Task::none();
        };
        // A click: nothing moved.
        if grab.is_none() {
            return Task::none();
        }
        let Some(target) = target else {
            // Stays a window where it was dropped: remembered for next time.
            self.keep_pallet_window_at(window, Point::new(os.placement.x, os.placement.y));
            self.save_config();
            // Just torn out of the main window, it takes the focus now.
            return if torn_out {
                iced::window::gain_focus(window)
            } else {
                Task::none()
            };
        };
        self.dock.detached.remove(&panel);
        match target {
            DropTarget::Edge { side, index } => {
                self.dock.dock(panel, side, index);
            }
            DropTarget::Join { side, group, index } => {
                self.dock.join_group(panel, side, group, index);
            }
            DropTarget::Float { x, y, bar } => {
                let (w, h) = crate::app::tool_windows::pallet_size_in_main(os.placement);
                self.dock.float(crate::ui::dock::FloatPanel {
                    bar,
                    ..crate::ui::dock::FloatPanel::new(panel, x, y, w, h)
                });
                self.dock.raise_float(panel);
            }
        }
        self.dock_icon_hover = None;
        self.dock_grip_hover = None;
        self.dock_grip_lit = None;
        self.dock_reveal(panel);
        self.save_config();
        // Its window closes as the update settles: it is no longer separate.
        self.focus_main_window()
    }

    /// Esc while a pallet's window is dragged: it goes back to where the
    /// drag began, and stays a window of its own.
    pub(crate) fn dock_window_drag_cancel(&mut self) -> Task<Message> {
        let Some(DockDrag::Move {
            source: DragSource::OwnWindow { window, start, .. },
            ..
        }) = self.dock_drag
        else {
            return Task::none();
        };
        self.dock_drag = None;
        self.dock_drag_last = None;
        let Some((_, os)) = self.pallet_window(window) else {
            return Task::none();
        };
        if !(start.x.is_finite() && start.y.is_finite()) {
            return Task::none();
        }
        self.keep_pallet_window_at(window, start);
        iced::window::move_to(window, Point::new(start.x / os.scale, start.y / os.scale))
    }

    /// Pallet window `window` stands at `at` (physical px) now, and opens
    /// there next time. Its own move events may not say so (none follows a
    /// move the app asks for on X11), nor count while it is being placed.
    fn keep_pallet_window_at(&mut self, window: iced::window::Id, at: Point) {
        let Some(os) = self.os_windows.iter_mut().find(|w| w.id == window) else {
            return;
        };
        os.placement.x = at.x;
        os.placement.y = at.y;
        os.placing = false;
        let (content, placement) = (os.content, os.placement);
        self.remember_placement(Some(content), placement);
    }

    /// Where the main window stands, asked when a pallet drag begins in it
    /// while that is not known yet (no move event since it opened): a drag
    /// past its edge needs it.
    pub(crate) fn ask_main_window_at(&self) -> Task<Message> {
        match self.main_window {
            Some(main)
                if self.main_window_at.is_none() && crate::app::tool_windows::can_drag_dock() =>
            {
                iced::window::position(main)
                    .map(move |at| Message::Dock(DockMsg::WindowAt(main, at)))
            }
            _ => Task::none(),
        }
    }

    /// The pointer entered or left pallet window `window`: where the pallet
    /// auto-hides, after the hover delay the window unfolds or folds down to
    /// its title bar.
    fn dock_window_hover(&mut self, window: iced::window::Id, inside: bool) -> Task<Message> {
        let Some((panel, _)) = self.pallet_window(window) else {
            return Task::none();
        };
        let auto_hides = self.dock.auto_collapse(panel);
        let Some(os) = self.os_windows.iter_mut().find(|w| w.id == window) else {
            return Task::none();
        };
        // A newer hover of this window always wins over a pending one; other
        // windows' hovers leave it be.
        os.fold_gen = os.fold_gen.wrapping_add(1);
        if !auto_hides || os.folded != inside {
            return Task::none();
        }
        let gen = os.fold_gen;
        let fold = !inside;
        Task::perform(
            crate::app::update::rest(crate::ui::dock::DOCK_HOVER_DELAY),
            move |()| Message::Dock(DockMsg::WindowFoldSettled(window, fold, gen)),
        )
    }

    fn dock_window_fold_settled(
        &mut self,
        window: iced::window::Id,
        fold: bool,
        gen: u64,
    ) -> Task<Message> {
        let Some((panel, os)) = self.pallet_window(window) else {
            return Task::none();
        };
        if gen != os.fold_gen {
            return Task::none();
        }
        if fold {
            if !self.dock.auto_collapse(panel) || self.dock_drag.is_some() {
                return Task::none();
            }
            // Its menu, or a popup of the pallet, keeps it open until it
            // closes; then it folds unless the pointer is back.
            if self.dock_window_menu == Some(window) || self.dock_popup_open() {
                return self.dock_window_hover(window, false);
            }
        }
        self.fold_pallet_window(window, fold)
    }

    /// Fold pallet window `window` down to its title bar, or open it to its
    /// full size again. Resizing keeps its top-left corner, and the bar is
    /// on its left, so the bar stays where it is.
    pub(crate) fn fold_pallet_window(
        &mut self,
        window: iced::window::Id,
        fold: bool,
    ) -> Task<Message> {
        let Some(os) = self.os_windows.iter_mut().find(|w| w.id == window) else {
            return Task::none();
        };
        if os.folded == fold {
            return Task::none();
        }
        os.folded = fold;
        let (w, h) = (os.placement.w, os.placement.h);
        let bar = crate::ui::dock::DOCK_FLOAT_BAR_W;
        let least = Size::new(WindowPlacement::MIN_W, WindowPlacement::MIN_H);
        if fold {
            iced::window::set_min_size(window, Some(Size::new(bar, least.height)))
                .chain(iced::window::resize(window, Size::new(bar, h)))
        } else {
            iced::window::resize(window, Size::new(w, h))
                .chain(iced::window::set_min_size(window, Some(least)))
        }
    }

    /// Pallet `panel`'s pin was switched on: a window of its own that folded
    /// opens again.
    pub(crate) fn unfold_pallet_windows(&mut self, panel: PanelId) -> Task<Message> {
        let folded: Vec<iced::window::Id> = self
            .os_windows
            .iter()
            .filter(|w| w.content == Detached::Panel(panel) && w.folded)
            .map(|w| w.id)
            .collect();
        Task::batch(
            folded
                .into_iter()
                .map(|w| self.fold_pallet_window(w, false)),
        )
    }
}

// ── Dragging a pallet out of the main window ────────────────────────────────

impl OpenCADStudio {
    /// The window whose raw pointer feeds the dock: a pallet's window while
    /// its title bar is held, the main window while a single pallet is
    /// dragged in it (to see the pointer leave it) or was torn out of it.
    pub(crate) fn dock_pointer_feed(&self) -> Option<iced::window::Id> {
        match self.dock_drag? {
            DockDrag::Move {
                source: DragSource::OwnWindow { torn_out: true, .. },
                ..
            } => self.main_window,
            drag if drag.own_window().is_some() => drag.own_window(),
            _ => self.dock_drag_out_possible().then_some(self.main_window?),
        }
    }

    /// Whether the drag in the main window can take its pallet out of it:
    /// one pallet (a whole group cannot go into a window), where the app
    /// knows where the main window is on the screen.
    fn dock_drag_out_possible(&self) -> bool {
        matches!(
            self.dock_drag,
            Some(DockDrag::Move {
                group: None,
                source: DragSource::Workspace,
                ..
            })
        ) && self.main_window_at.is_some()
            && crate::app::tool_windows::can_drag_dock()
    }

    /// The main window's raw pointer at `p` (its logical space) while a
    /// pallet is dragged in it. Once the pointer is past the window's edge by
    /// `TEAR_OUT_MARGIN`, the pallet is torn out: it opens in a window of its
    /// own under the pointer, held where it was grabbed, and the drag goes on
    /// as that window's (the main window keeps feeding the pointer). After a
    /// tear-out, it moves that window.
    fn dock_main_pointer(&mut self, p: Point) -> Task<Message> {
        let Some(main_at) = self.main_window_at else {
            return Task::none();
        };
        let main_scale = crate::ui::icons::scale_factor();
        if let Some(DockDrag::Move {
            source:
                DragSource::OwnWindow {
                    window,
                    torn_out: true,
                    ..
                },
            ..
        }) = self.dock_drag
        {
            let u = unit(main_scale, SCREEN_IN_POINTS);
            let screen = crate::sys::screen_cursor()
                .unwrap_or_else(|| Point::new((main_at.x + p.x) * u, (main_at.y + p.y) * u));
            return self.dock_window_follow(window, screen);
        }
        if !self.dock_drag_out_possible() {
            return Task::none();
        }
        let (ww, wh) = self.win_size;
        let m = TEAR_OUT_MARGIN;
        if !(p.x < -m || p.y < -m || p.x > ww + m || p.y > wh + m) {
            return Task::none();
        }
        let Some(DockDrag::Move {
            panel,
            target: Some(_),
            grab,
            bar,
            ..
        }) = self.dock_drag
        else {
            // Not past the drag threshold yet: nothing to tear out.
            return Task::none();
        };
        let undo = Box::new(self.dock.clone());
        let at = window_place_for_pointer(p, grab, main_at, main_scale);
        self.detach_panel_at(panel, at);
        let opened = self.reconcile_os_windows();
        let Some(window) = self
            .os_windows
            .iter()
            .find(|w| w.content == Detached::Panel(panel))
            .map(|w| w.id)
        else {
            // No window could open (no main window): leave it as it was.
            self.dock = *undo;
            return opened;
        };
        self.dock_tear_undo = Some(undo);
        self.dock_drag = Some(DockDrag::Move {
            panel,
            group: None,
            origin: None,
            target: None,
            grab,
            bar,
            source: DragSource::OwnWindow {
                window,
                start: at,
                grab: Some(grab),
                system: false,
                torn_out: true,
            },
        });
        self.dock_drag_last = None;
        self.dock_drag_window = None;
        self.dock_fold_on_release = None;
        self.dock_title_hover = None;
        self.dock_icon_hover = None;
        self.dock_grip_hover = None;
        self.dock_grip_lit = None;
        // Back over the main window it docks or floats as a pallet window
        // would: the workspace's place is needed for that.
        Task::batch([
            opened,
            workspace_origin().map(|at| Message::Dock(DockMsg::WorkspaceOrigin(at))),
        ])
    }

    /// The button came up in the main window (maybe outside it) while a
    /// pallet was dragged there, or torn out from there.
    fn dock_main_release(&mut self) -> Task<Message> {
        if let Some(window) = self.dock_drag.and_then(|d| d.own_window()) {
            return self.dock_window_drag_release(window);
        }
        // Over the workspace the capture layer hears the release too;
        // whichever comes first lands the pallet.
        if matches!(self.dock_drag, Some(DockDrag::Move { .. })) {
            self.dock_drag_release();
        }
        Task::none()
    }

    /// Esc in the window that feeds a pallet drag. A pallet torn out of the
    /// main window goes back where it was and its window closes; a pallet
    /// window goes back to where its drag began; a drag in the main window
    /// just ends, changing nothing.
    pub(crate) fn dock_drag_cancel(&mut self) -> Task<Message> {
        match self.dock_drag {
            Some(DockDrag::Move {
                source: DragSource::OwnWindow { torn_out: true, .. },
                ..
            }) => {
                self.dock_drag = None;
                self.dock_drag_last = None;
                if let Some(undo) = self.dock_tear_undo.take() {
                    // Its window closes as the update settles.
                    self.dock = *undo;
                }
                self.focus_main_window()
            }
            Some(DockDrag::Move {
                source: DragSource::OwnWindow { .. },
                ..
            }) => self.dock_window_drag_cancel(),
            Some(DockDrag::Move { .. }) => {
                self.dock_drag = None;
                self.dock_drag_last = None;
                self.dock_fold_on_release = None;
                Task::none()
            }
            _ => Task::none(),
        }
    }
}

/// Where a pallet's window opens (top-left, physical px) for a pointer at
/// `p` in the main window (logical), held at `grab` from the pallet's
/// top-left corner, with the main window at `main_at` (logical) on a screen
/// of scale `scale`.
pub(crate) fn window_place_for_pointer(
    p: Point,
    grab: Vector,
    main_at: Point,
    scale: f32,
) -> Point {
    Point::new(
        (main_at.x + p.x - grab.x) * scale,
        (main_at.y + p.y - grab.y) * scale,
    )
}

/// Where the dock workspace's top-left corner is in the main window.
fn workspace_origin() -> Task<Option<Point>> {
    use iced::advanced::widget::operation::Outcome;
    use iced::advanced::widget::{Id, Operation};
    struct Find {
        id: Id,
        found: Option<Point>,
    }
    impl Operation<Option<Point>> for Find {
        fn container(&mut self, id: Option<&Id>, bounds: iced::Rectangle) {
            if self.found.is_none() && id == Some(&self.id) {
                self.found = Some(bounds.position());
            }
        }
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<Option<Point>>)) {
            operate(self);
        }
        fn finish(&self) -> Outcome<Option<Point>> {
            Outcome::Some(self.found)
        }
    }
    iced::advanced::widget::operate(Find {
        id: Id::new(super::DOCK_WORKSPACE_ID),
        found: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Windows / X11: physical pixels. A pallet window at (2000, 100) px on
    /// a screen of scale 1.5, the main window at the origin at scale 1.
    #[test]
    fn a_pointer_in_a_pallet_window_maps_into_the_main_window_in_pixels() {
        let at = placement_on_screen(Point::new(2000.0, 100.0), 1.5, false);
        assert_eq!(at, Point::new(2000.0, 100.0));
        let local = Point::new(-1000.0, 20.0);
        let u = unit(1.5, false);
        let screen = Point::new(at.x + local.x * u, at.y + local.y * u);
        assert_eq!(
            screen_to_main(screen, Point::ORIGIN, 1.0, false),
            Point::new(500.0, 130.0)
        );
    }

    /// macOS: points. A pallet window on a Retina screen (scale 2) at
    /// (1500, 100) points, stored as (3000, 200) px, and the main window on
    /// a scale 1 screen: the same point of the screen is found in both,
    /// which pixels would place twice as far.
    #[test]
    fn a_pointer_in_a_pallet_window_maps_into_the_main_window_in_points() {
        let at = placement_on_screen(Point::new(3000.0, 200.0), 2.0, true);
        assert_eq!(at, Point::new(1500.0, 100.0));
        let local = Point::new(-1000.0, 20.0);
        let u = unit(2.0, true);
        let screen = Point::new(at.x + local.x * u, at.y + local.y * u);
        assert_eq!(
            screen_to_main(screen, Point::new(100.0, 0.0), 1.0, true),
            Point::new(400.0, 120.0)
        );
        // A main window on the Retina screen too: still points.
        assert_eq!(
            screen_to_main(screen, Point::new(100.0, 0.0), 2.0, true),
            Point::new(400.0, 120.0)
        );
    }
}
