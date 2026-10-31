//! Ribbon panels floating over the drawing (`RibbonLayout::floating`): the
//! layer that holds them and each panel's frame.
//!
//! The layer keeps its panels in a fixed order in the widget tree, so a
//! panel's widget state stays with it when it is raised, and draws them and
//! hands them the pointer by their stacking order. Away from the panels it
//! takes no pointer input, so the drawing under it works as usual.

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{self, Widget};
use iced::advanced::{mouse, overlay, renderer, Renderer as _, Shell};
use iced::widget::container;
use iced::{
    Background, Border, Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
};

use super::draw_panel::{self, SlideLook};
use super::layout::FloatingPanel;
use super::{extras, floating_id, panel_frame, render_group, Ribbon, Strip};
use crate::app::Message;
use crate::modules::RibbonGroup;
use crate::ui::style::{chrome, ribbon_metrics as metrics};
use crate::ui::wrap_bar::PosReport;

impl Ribbon {
    /// The floating panels with their tab's index and group, in the order
    /// `layout.floating` keeps them. A panel whose tab or group is gone is
    /// left out.
    pub(super) fn floating_groups(&self) -> Vec<(usize, &RibbonGroup, &FloatingPanel)> {
        self.layout
            .floating
            .iter()
            .filter_map(|panel| {
                let index = self
                    .modules
                    .iter()
                    .position(|module| module.id() == panel.module_id)?;
                let group = self
                    .ordered_groups(index)
                    .into_iter()
                    .find(|group| extras::panel_key(&panel.module_id, group.title) == panel.key)?;
                Some((index, group, panel))
            })
            .collect()
    }

    /// The layer of floating panels, over the drawing and under the ribbon's
    /// popups. The caller leaves it out on the Start tab, with Clean Screen
    /// and while the ribbon is closed.
    pub fn floating_layer(&self, show_block_palette: bool) -> Option<Element<'_, Message>> {
        let found = self.floating_groups();
        if found.is_empty() {
            return None;
        }
        let ts = self.toggle_state(show_block_palette);
        let mut order: Vec<usize> = (0..found.len()).collect();
        order.sort_by_key(|&i| found[i].2.z);
        let keys = found
            .iter()
            .map(|(_, _, panel)| panel.key.clone())
            .collect();
        let panels = found
            .iter()
            .map(|&(index, group, panel)| {
                let module_id = self.modules[index].id();
                (
                    Point::new(panel.x, panel.y),
                    self.floating_panel(module_id, group, ts),
                )
            })
            .collect();
        Some(
            FloatLayer {
                panels,
                order,
                keys,
            }
            .into(),
        )
    }

    /// One floating panel: its tools at full size over its title strip, on
    /// `panel` with a 1 px `border` outline.
    fn floating_panel<'a>(
        &'a self,
        module_id: &str,
        group: &'a RibbonGroup,
        ts: super::widgets::ToggleState,
    ) -> Element<'a, Message> {
        let key = extras::panel_key(module_id, group.title);
        let style_ctx = self.style_context();
        let body = render_group(
            false,
            group,
            &key,
            Strip::Bare,
            SlideLook::Closed,
            Vec::new(),
            self.lit_tool(),
            &self.open_dropdown,
            &self.last_cmd,
            ts,
            &self.layer_infos,
            &self.active_layer,
            self.active_color,
            &self.active_linetype,
            self.active_lineweight,
            &style_ctx,
        );
        let title = draw_panel::group_title(group.title, &key, SlideLook::Closed, false);
        let framed = panel_frame::PanelFrame::new(body, metrics::TITLE_STRIP_H, title.launcher)
            .title(Some(title.row));
        let framed = container(framed).style(|theme: &Theme| container::Style {
            background: Some(Background::Color(chrome::panel(theme))),
            border: Border {
                color: chrome::border(theme),
                width: crate::ui::icons::hairline(),
                radius: 0.0.into(),
            },
            ..Default::default()
        });
        let grip = Grip {
            child: framed.into(),
            key: key.clone(),
            edge: Edge::Bottom(metrics::TITLE_STRIP_H),
            on_click: None,
            threshold: metrics::FLOAT_MOVE_THRESHOLD,
        };
        PosReport::owned(floating_id(&key), grip).into()
    }

    /// Apply a floating panel message. Returns whether the layout changed
    /// and should be saved.
    pub fn on_float(&mut self, msg: FloatMsg) -> bool {
        match msg {
            FloatMsg::Raise(key) => {
                let top = self.top_float_z();
                match self.layout.floating.iter_mut().find(|p| p.key == key) {
                    Some(panel) if panel.z < top || top == 0 => {
                        panel.z = top + 1;
                        true
                    }
                    _ => false,
                }
            }
            FloatMsg::Press { key, at } => {
                self.float_drag = self
                    .layout
                    .floating
                    .iter()
                    .find(|panel| panel.key == key)
                    .map(|panel| FloatDrag {
                        key,
                        grab: at - Point::new(panel.x, panel.y),
                    });
                false
            }
            FloatMsg::Move { at, .. } => {
                let Some(drag) = &self.float_drag else {
                    return false;
                };
                let corner = at - drag.grab;
                if let Some(panel) = self.layout.floating.iter_mut().find(|p| p.key == drag.key) {
                    panel.x = corner.x;
                    panel.y = corner.y;
                }
                false
            }
            FloatMsg::Release { moved } => self.float_drag.take().is_some() && moved,
        }
    }
}

/// A floating panel message.
#[derive(Clone, Debug)]
pub enum FloatMsg {
    /// A press on the panel with this key: it comes to the front.
    Raise(String),
    /// A press on its title strip, at this window point.
    Press { key: String, at: Point },
    /// The pointer moved far enough from the press: the panel follows it.
    /// `free` (Alt held) moves it without snapping.
    Move { at: Point, free: bool },
    /// The press let go; `moved` once it had moved the panel.
    Release { moved: bool },
}

/// A floating panel being moved by its title strip.
#[derive(Clone, Debug)]
pub(super) struct FloatDrag {
    pub key: String,
    /// The press point within the panel.
    pub grab: Vector,
}

/// Where on a panel its title strip is: along the bottom or the right, this
/// many px.
#[derive(Clone, Copy, Debug)]
enum Edge {
    Bottom(f32),
    #[allow(dead_code)]
    Right(f32),
}

impl Edge {
    fn strip(self, bounds: Rectangle) -> Rectangle {
        match self {
            Edge::Bottom(h) => Rectangle {
                y: bounds.y + bounds.height - h,
                height: h,
                ..bounds
            },
            Edge::Right(w) => Rectangle {
                x: bounds.x + bounds.width - w,
                width: w,
                ..bounds
            },
        }
    }
}

/// A floating panel's title strip, `edge` of `child`: a press there that no
/// button inside took moves the panel once the pointer has gone
/// `threshold` px, and without that movement is a click (`on_click`).
/// The pointer stays the plain arrow.
struct Grip<'a> {
    child: Element<'a, Message>,
    key: String,
    edge: Edge,
    on_click: Option<Message>,
    threshold: f32,
}

#[derive(Default)]
struct GripState {
    press: Option<Point>,
    moving: bool,
    alt: bool,
}

impl<'a> Widget<Message, Theme, Renderer> for Grip<'a> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<GripState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(GripState::default())
    }

    fn diff(&mut self, tree: &mut widget::Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.child));
    }

    fn size(&self) -> Size<Length> {
        self.child.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.child
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let moving = tree.state.downcast_ref::<GripState>().moving;
        // While the panel moves, what is on it takes no pointer input.
        let child_cursor = if moving {
            mouse::Cursor::Unavailable
        } else {
            cursor
        };
        self.child.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            child_cursor,
            renderer,
            shell,
            viewport,
        );
        let state = tree.state.downcast_mut::<GripState>();
        match event {
            Event::Keyboard(iced::keyboard::Event::ModifiersChanged(modifiers)) => {
                state.alt = modifiers.alt();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if shell.is_event_captured() {
                    return;
                }
                let Some(at) = cursor.position_over(self.edge.strip(layout.bounds())) else {
                    return;
                };
                state.press = Some(at);
                state.moving = false;
                shell.publish(Message::RibbonFloat(FloatMsg::Press {
                    key: self.key.clone(),
                    at,
                }));
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let Some(press) = state.press else {
                    return;
                };
                if !state.moving && position.distance(press) >= self.threshold {
                    state.moving = true;
                }
                if state.moving {
                    shell.publish(Message::RibbonFloat(FloatMsg::Move {
                        at: *position,
                        free: state.alt,
                    }));
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.press.take().is_none() {
                    return;
                }
                let moved = std::mem::take(&mut state.moving);
                shell.publish(Message::RibbonFloat(FloatMsg::Release { moved }));
                if !moved
                    && cursor
                        .position_over(self.edge.strip(layout.bounds()))
                        .is_some()
                {
                    if let Some(message) = self.on_click.clone() {
                        shell.publish(message);
                    }
                }
                shell.capture_event();
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.child.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        self.child
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.child.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.child.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a> From<Grip<'a>> for Element<'a, Message> {
    fn from(grip: Grip<'a>) -> Self {
        Element::new(grip)
    }
}

/// The floating panels, each at its top-left corner. `order` lists them
/// back to front; the widget tree keeps them in `panels`' order.
pub(super) struct FloatLayer<'a> {
    panels: Vec<(Point, Element<'a, Message>)>,
    order: Vec<usize>,
    /// The panels' keys, in `panels`' order.
    keys: Vec<String>,
}

impl FloatLayer<'_> {
    /// The front-most panel under the cursor.
    fn hit(&self, layout: Layout<'_>, cursor: mouse::Cursor) -> Option<usize> {
        let at = cursor.position()?;
        let bounds: Vec<Rectangle> = layout.children().map(|child| child.bounds()).collect();
        self.order
            .iter()
            .rev()
            .copied()
            .find(|&i| bounds.get(i).is_some_and(|b| b.contains(at)))
    }
}

impl<'a> Widget<Message, Theme, Renderer> for FloatLayer<'a> {
    fn diff(&mut self, tree: &mut widget::Tree) {
        let mut panels: Vec<&mut Element<'a, Message>> =
            self.panels.iter_mut().map(|(_, panel)| panel).collect();
        tree.diff_children_custom(
            &mut panels,
            |tree, panel| tree.diff(panel.as_widget_mut()),
            |panel| widget::Tree::new(panel.as_widget()),
        );
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let natural = layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, f32::INFINITY));
        let nodes = self
            .panels
            .iter_mut()
            .zip(&mut tree.children)
            .map(|((at, panel), tree)| {
                panel
                    .as_widget_mut()
                    .layout(tree, renderer, &natural)
                    .move_to(*at)
            })
            .collect();
        layout::Node::with_children(limits.max(), nodes)
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let hit = self.hit(layout, cursor);
        // Any press on a panel brings it to the front.
        if let (Some(i), Event::Mouse(mouse::Event::ButtonPressed(_))) = (hit, event) {
            if self.order.last() != Some(&i) {
                shell.publish(Message::RibbonFloat(FloatMsg::Raise(self.keys[i].clone())));
            }
        }
        let layouts: Vec<Layout<'_>> = layout.children().collect();
        // Front to back; a panel under another sees no pointer there.
        for &i in self.order.iter().rev() {
            let panel_cursor = match hit {
                Some(top) if top != i => mouse::Cursor::Unavailable,
                _ => cursor,
            };
            self.panels[i].1.as_widget_mut().update(
                &mut tree.children[i],
                event,
                layouts[i],
                panel_cursor,
                renderer,
                shell,
                viewport,
            );
            if shell.is_event_captured() {
                return;
            }
        }
        // A panel is opaque: what is under it gets no presses or wheel.
        if hit.is_some()
            && matches!(
                event,
                Event::Mouse(
                    mouse::Event::ButtonPressed(_)
                        | mouse::Event::ButtonReleased(_)
                        | mouse::Event::WheelScrolled { .. }
                )
            )
        {
            shell.capture_event();
        }
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        let Some(i) = self.hit(layout, cursor) else {
            // Off the panels the drawing under the layer has the pointer.
            return mouse::Interaction::None;
        };
        let child = layout.children().nth(i).expect("panel layout");
        match self.panels[i].1.as_widget().mouse_interaction(
            &tree.children[i],
            child,
            cursor,
            viewport,
            renderer,
        ) {
            mouse::Interaction::None => mouse::Interaction::Idle,
            interaction => interaction,
        }
    }

    fn operate(
        &mut self,
        tree: &mut widget::Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn widget::Operation,
    ) {
        for (((_, panel), tree), layout) in self
            .panels
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
        {
            panel
                .as_widget_mut()
                .operate(tree, layout, renderer, operation);
        }
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let hit = self.hit(layout, cursor);
        let layouts: Vec<Layout<'_>> = layout.children().collect();
        for &i in &self.order {
            let panel_cursor = match hit {
                Some(top) if top != i => mouse::Cursor::Unavailable,
                _ => cursor,
            };
            // Each panel on its own layer, so a panel on top covers the
            // text of the ones under it.
            renderer.with_layer(layouts[i].bounds(), |renderer| {
                self.panels[i].1.as_widget().draw(
                    &tree.children[i],
                    renderer,
                    theme,
                    style,
                    layouts[i],
                    panel_cursor,
                    viewport,
                );
            });
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        // The panels' tooltips.
        let overlays: Vec<_> = self
            .panels
            .iter_mut()
            .zip(&mut tree.children)
            .zip(layout.children())
            .filter_map(|(((_, panel), tree), layout)| {
                panel
                    .as_widget_mut()
                    .overlay(tree, layout, renderer, viewport, translation)
            })
            .collect();
        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

impl<'a> From<FloatLayer<'a>> for Element<'a, Message> {
    fn from(layer: FloatLayer<'a>) -> Self {
        Element::new(layer)
    }
}

#[cfg(test)]
mod tests {
    use super::super::layout::{FloatingPanel, RibbonLayout};
    use super::Ribbon;

    fn floating(key: &str, z: u32) -> FloatingPanel {
        let (module_id, _) = key.split_once('/').unwrap();
        FloatingPanel {
            key: key.into(),
            module_id: module_id.into(),
            z,
            ..FloatingPanel::default()
        }
    }

    #[test]
    fn the_layer_shows_the_floating_panels_that_still_exist() {
        let mut ribbon = Ribbon::default();
        assert!(ribbon.floating_layer(false).is_none());
        let layout = RibbonLayout {
            floating: vec![
                floating("draw/Modify", 2),
                floating("draw/Gone", 5),
                floating("draw/Draw", 1),
            ],
            ..RibbonLayout::default()
        };
        ribbon.set_layout(layout);
        let found: Vec<_> = ribbon
            .floating_groups()
            .iter()
            .map(|(_, group, panel)| (group.title, panel.z))
            .collect();
        assert_eq!(found, [("Modify", 2), ("Draw", 1)]);
        assert!(ribbon.floating_layer(false).is_some());
    }

    #[test]
    fn a_press_raises_a_panel_and_its_strip_moves_it() {
        use super::FloatMsg;
        use iced::Point;
        let mut ribbon = Ribbon::default();
        let mut draw = floating("draw/Draw", 1);
        draw.x = 100.0;
        draw.y = 200.0;
        ribbon.set_layout(RibbonLayout {
            floating: vec![draw, floating("draw/Modify", 2)],
            ..RibbonLayout::default()
        });
        // Raising the one on top changes nothing; the other goes above it.
        assert!(!ribbon.on_float(FloatMsg::Raise("draw/Modify".into())));
        assert!(ribbon.on_float(FloatMsg::Raise("draw/Draw".into())));
        assert_eq!(ribbon.layout.floating[0].z, 3);

        // The press point keeps its place on the panel while it moves.
        ribbon.on_float(FloatMsg::Press {
            key: "draw/Draw".into(),
            at: Point::new(150.0, 290.0),
        });
        assert!(!ribbon.on_float(FloatMsg::Move {
            at: Point::new(250.0, 390.0),
            free: true,
        }));
        let panel = &ribbon.layout.floating[0];
        assert_eq!((panel.x, panel.y), (200.0, 300.0));
        // Saved on release once it moved; a click is not a move.
        assert!(ribbon.on_float(FloatMsg::Release { moved: true }));
        ribbon.on_float(FloatMsg::Press {
            key: "draw/Draw".into(),
            at: Point::new(250.0, 390.0),
        });
        assert!(!ribbon.on_float(FloatMsg::Release { moved: false }));
        assert!(ribbon.float_drag.is_none());
    }

    #[test]
    fn a_floating_panels_dropdown_opens_from_any_tab() {
        let mut ribbon = Ribbon::default();
        ribbon.set_layout(RibbonLayout {
            floating: vec![floating("draw/Draw", 1)],
            ..RibbonLayout::default()
        });
        let insert = ribbon
            .modules
            .iter()
            .position(|module| module.id() == "insert")
            .unwrap();
        ribbon.select(insert);
        ribbon.toggle_dropdown("CIRCLE");
        let list = ribbon.dropdown_overlay(&[], &[], (1400.0, 900.0), false, &[], &[], &[]);
        assert!(list.is_some());
    }
}
