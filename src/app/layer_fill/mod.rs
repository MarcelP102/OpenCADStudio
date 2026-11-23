//! Fill by layer: fills work as line colours do. A closed shape drawn on a
//! layer gets that layer's fill (ByLayer), or the Graphic Attributes
//! palette's custom fill for new objects. A layer whose fill is None fills
//! nothing.
//!
//! - [`store`]: the fill style per layer, an XRecord on the layer.
//! - [`apply`]: the associative fill added when a shape is drawn.

pub(crate) mod apply;
pub(crate) mod store;

use crate::app::{Message, OpenCADStudio};
use iced::Task;
use store::{FillKind, LayerFill};

/// Split a command line into words; a word in double quotes may hold spaces
/// (a layer name).
fn words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut rest = text.trim_start();
    while !rest.is_empty() {
        if let Some(quoted) = rest.strip_prefix('"') {
            let end = quoted.find('"').unwrap_or(quoted.len());
            words.push(quoted[..end].to_string());
            rest = quoted.get(end + 1..).unwrap_or("").trim_start();
        } else {
            let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
            words.push(rest[..end].to_string());
            rest = rest[end..].trim_start();
        }
    }
    words
}

/// A fill colour as typed: an index 1–255 or `r,g,b`. No ByLayer: the
/// fill colour is itself part of the layer's settings.
fn parse_color(text: &str) -> Option<document::types::Color> {
    use document::types::Color;
    let text = text.trim();
    if let Ok(index) = text.parse::<u8>() {
        return (index >= 1).then_some(Color::Index(index));
    }
    let parts: Vec<u8> = text
        .split(',')
        .map(|part| part.trim().parse::<u8>())
        .collect::<Result<_, _>>()
        .ok()?;
    match parts[..] {
        [r, g, b] => Some(Color::Rgb { r, g, b }),
        _ => None,
    }
}

/// Parse `None | Solid | <pattern> [scale] [angle] [transparency]` onto `base`.
fn parse_fill(base: Option<LayerFill>, words: &[String]) -> Result<LayerFill, String> {
    let mut fill = base.unwrap_or_default();
    // `color=<colour>` may stand anywhere; the rest is positional.
    let mut positional = Vec::new();
    for word in words {
        let lower = word.to_ascii_lowercase();
        match lower
            .strip_prefix("color=")
            .or_else(|| lower.strip_prefix("kleur="))
        {
            Some(value) => {
                fill.color = parse_color(value).ok_or_else(|| {
                    format!("LAYERFILL: \"{value}\" is not a colour (1-255 or r,g,b).")
                })?;
            }
            None => positional.push(word.clone()),
        }
    }
    let words = positional.as_slice();
    let Some(kind) = words.first() else {
        return Err("LAYERFILL: enter None, Solid or a pattern name.".into());
    };
    match kind.to_ascii_uppercase().as_str() {
        "NONE" | "GEEN" => fill.kind = FillKind::None,
        "SOLID" => fill.kind = FillKind::Solid,
        name => {
            let known = crate::scene::model::hatch_patterns::catalog()
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(name));
            let Some(entry) = known else {
                return Err(format!("LAYERFILL: no hatch pattern named \"{kind}\"."));
            };
            fill.kind = FillKind::Pattern;
            fill.pattern = entry.name.clone();
        }
    }
    let number = |index: usize, what: &str| -> Result<Option<f64>, String> {
        match words.get(index) {
            None => Ok(None),
            Some(word) => word
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(Some)
                .ok_or_else(|| format!("LAYERFILL: {what} must be a number.")),
        }
    };
    if let Some(scale) = number(1, "the scale")? {
        if scale <= 0.0 {
            return Err("LAYERFILL: the scale must be greater than 0.".into());
        }
        fill.scale = scale;
    }
    if let Some(angle) = number(2, "the angle")? {
        fill.angle_deg = angle;
    }
    if let Some(transparency) = number(3, "the transparency")? {
        if !(0.0..=90.0).contains(&transparency) {
            return Err("LAYERFILL: the transparency must be from 0 to 90.".into());
        }
        fill.transparency = transparency.round() as u8;
    }
    Ok(fill)
}

fn describe(layer: &str, fill: Option<&LayerFill>) -> String {
    match fill {
        Some(fill) if fill.kind != FillKind::None => format!(
            "LAYERFILL {layer}: {}, colour {}, scale {}, angle {}, transparency {}%",
            fill.summary(),
            crate::ui::color_select::color_display_name(fill.color),
            fill.scale,
            fill.angle_deg,
            fill.transparency
        ),
        _ => format!("LAYERFILL {layer}: None"),
    }
}

/// The Fill column's choices: None, Solid and every hatch pattern.
fn fill_choices() -> &'static [String] {
    static CHOICES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    CHOICES.get_or_init(|| {
        let mut options = vec![
            crate::ui::window::layers::FILL_NONE.to_string(),
            "Solid".to_string(),
        ];
        options.extend(
            crate::scene::model::hatch_patterns::catalog()
                .iter()
                .filter(|entry| {
                    matches!(
                        entry.gpu,
                        crate::scene::model::hatch_model::HatchPattern::Pattern(_)
                    )
                })
                .map(|entry| entry.name.clone()),
        );
        options
    })
}

impl OpenCADStudio {
    /// LAYERFILL, FILLBYLAYERRESET and FILLBYLAYERAPPLY. `None` for any
    /// other command.
    pub(in crate::app) fn dispatch_layer_fill(
        &mut self,
        cmd: &str,
        i: usize,
    ) -> Option<Task<Message>> {
        let all = words(cmd);
        let name = all.first()?.to_ascii_uppercase();
        let rest = all.get(1..).unwrap_or(&[]);
        match name.as_str() {
            "LAYERFILL" => {
                self.layer_fill_command(i, rest);
                Some(self.finish_dispatch(cmd))
            }
            "FILLBYLAYERRESET" => {
                let reset = self.reset_layer_fills(i);
                self.command_line.push_output(&format!(
                    "FILLBYLAYERRESET: {reset} fill(s) follow their layer again."
                ));
                self.refresh_properties();
                Some(self.finish_dispatch(cmd))
            }
            "FILLBYLAYERAPPLY" => {
                self.fill_by_layer_apply_command(i, rest);
                Some(self.finish_dispatch(cmd))
            }
            _ => None,
        }
    }

    /// `LAYERFILL [layer [None|Solid|pattern [scale [angle [transparency]]]]]`:
    /// report or set a layer's fill style. `.` is the current layer.
    fn layer_fill_command(&mut self, i: usize, words: &[String]) {
        let layer = match words.first().map(String::as_str) {
            None | Some(".") => self.tabs[i].active_layer.clone(),
            Some(name) => {
                let found = self.tabs[i]
                    .scene
                    .document
                    .layers
                    .iter()
                    .find(|layer| layer.name.eq_ignore_ascii_case(name))
                    .map(|layer| layer.name.clone());
                let Some(found) = found else {
                    self.command_line
                        .push_rejection(&format!("LAYERFILL: no layer named \"{name}\"."));
                    return;
                };
                found
            }
        };
        if words.len() < 2 {
            let stored = store::read(&self.tabs[i].scene.document, &layer);
            self.command_line
                .push_output(&describe(&layer, stored.as_ref()));
            return;
        }
        let current = Some(self.layer_fill_or_new(i, &layer));
        match parse_fill(current, &words[1..]) {
            Ok(fill) => {
                self.set_layer_fill(i, &layer, Some(&fill));
                self.command_line
                    .push_output(&describe(&layer, Some(&fill)));
            }
            Err(message) => self.command_line.push_rejection(&message),
        }
    }

    /// `FILLBYLAYERAPPLY [layer ...|*]`: fill the closed shapes already on
    /// the layers (the current one by default, `*` all with a fill style).
    fn fill_by_layer_apply_command(&mut self, i: usize, words: &[String]) {
        let document = &self.tabs[i].scene.document;
        let layers: Vec<String> = if words.iter().any(|word| word == "*") {
            document
                .layers
                .iter()
                .filter(|layer| store::read(document, &layer.name).is_some_and(|fill| fill.fills()))
                .map(|layer| layer.name.clone())
                .collect()
        } else if words.is_empty() {
            vec![self.tabs[i].active_layer.clone()]
        } else {
            words.to_vec()
        };
        let added = self.apply_layer_fills(i, &layers);
        self.command_line
            .push_output(&format!("FILLBYLAYERAPPLY: {added} shape(s) filled."));
    }

    /// Layer Manager Fill column: `None`, `Solid` or a pattern name. The
    /// other properties of the style (colour, scale, angle) are kept.
    pub(in crate::app) fn on_layer_fill_set(&mut self, layer: &str, choice: &str) -> Task<Message> {
        let i = self.active_tab;
        let current = Some(self.layer_fill_or_new(i, layer));
        match parse_fill(current, &[choice.to_string()]) {
            Ok(fill) if fill.kind == FillKind::None => {
                let _ = self.set_layer_fill(i, layer, None);
            }
            Ok(fill) => {
                let _ = self.set_layer_fill(i, layer, Some(&fill));
            }
            Err(message) => self.command_line.push_rejection(&message),
        }
        Task::none()
    }

    /// Layer Manager Fill color column (or the Select Color window): give the
    /// layer's fill its own colour. Fills that follow the layer take it in the
    /// same undo step.
    pub(in crate::app) fn on_layer_fill_color_set(
        &mut self,
        layer: &str,
        color: document::types::Color,
    ) -> Task<Message> {
        let i = self.active_tab;
        self.tabs[i].layers.fill_color_picker_row = None;
        let Some(mut fill) = store::read(&self.tabs[i].scene.document, layer) else {
            return Task::none();
        };
        fill.color = color;
        let _ = self.set_layer_fill(i, layer, Some(&fill));
        Task::none()
    }

    pub(in crate::app) fn on_layer_fill_color_toggle(&mut self, layer: String) -> Task<Message> {
        let i = self.active_tab;
        let panel = &mut self.tabs[i].layers;
        panel.color_picker_row = None;
        panel.fill_color_picker_row =
            if panel.fill_color_picker_row.as_deref() == Some(layer.as_str()) {
                None
            } else {
                Some(layer)
            };
        Task::none()
    }

    /// The layer's fill style, or a new one whose colour is the layer's own
    /// colour (a fill colour is concrete, never ByLayer).
    fn layer_fill_or_new(&self, i: usize, layer: &str) -> LayerFill {
        let document = &self.tabs[i].scene.document;
        store::read(document, layer).unwrap_or_else(|| LayerFill {
            color: document
                .layers
                .get(layer)
                .map_or(document::types::Color::Index(7), |record| record.color),
            ..LayerFill::default()
        })
    }

    /// Hand the Layer Manager the fill style per layer and the choices of
    /// its Fill column.
    pub(in crate::app) fn sync_layer_fill_column(&mut self, i: usize) {
        let document = &self.tabs[i].scene.document;
        let fills = document
            .layers
            .iter()
            .filter_map(|layer| {
                let fill = store::read(document, &layer.name)?;
                (fill.kind != FillKind::None)
                    .then(|| (layer.name.to_lowercase(), (fill.summary(), fill.color)))
            })
            .collect();
        self.tabs[i].layers.set_fills(fills, fill_choices());
    }

    /// Set (or with `None` remove) a layer's fill style as one undo step.
    pub(in crate::app) fn set_layer_fill(
        &mut self,
        i: usize,
        layer: &str,
        fill: Option<&LayerFill>,
    ) -> bool {
        if store::read(&self.tabs[i].scene.document, layer).as_ref() == fill {
            return false;
        }
        // Layer records are not delta-safe: undo from a structure snapshot.
        let pending = self.begin_undo(
            i,
            crate::app::tab_document::HistoryLabel::Message("Layer fill"),
            0,
            false,
        );
        let changed = store::write(&mut self.tabs[i].scene.document, layer, fill);
        if changed {
            // Fills that follow the layer take the new style in this step.
            self.restyle_layer_fills(i, layer);
            self.tabs[i].dirty = true;
            self.refresh_layer_panel();
        }
        if let Some(pending) = pending {
            self.commit_undo_delta(i, pending);
        }
        changed
    }
}

#[cfg(test)]
mod tests;
