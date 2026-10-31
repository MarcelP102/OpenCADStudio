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
        Some(FloatLayer { panels, order }.into())
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
        PosReport::owned(floating_id(&key), framed).into()
    }
}

/// The floating panels, each at its top-left corner. `order` lists them
/// back to front; the widget tree keeps them in `panels`' order.
pub(super) struct FloatLayer<'a> {
    panels: Vec<(Point, Element<'a, Message>)>,
    order: Vec<usize>,
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
