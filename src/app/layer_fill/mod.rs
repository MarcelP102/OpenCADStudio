//! Fill by layer: a closed shape drawn on a layer gets that layer's fill.
//!
//! - [`store`]: the fill style per layer, an XRecord on the layer.
//! - [`apply`]: the associative fill added when a shape is drawn.
//!
//! FILLBYLAYER (0 off, 1 on; kept with the drawing) turns it on. While it is
//! off, nothing behaves differently.

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

/// Parse `None | Solid | <pattern> [scale] [angle] [transparency]` onto `base`.
fn parse_fill(base: Option<LayerFill>, words: &[String]) -> Result<LayerFill, String> {
    let mut fill = base.unwrap_or_default();
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
            "LAYERFILL {layer}: {}, scale {}, angle {}, transparency {}%",
            fill.summary(),
            fill.scale,
            fill.angle_deg,
            fill.transparency
        ),
        _ => format!("LAYERFILL {layer}: None"),
    }
}

impl OpenCADStudio {
    /// FILLBYLAYER, SETVAR FILLBYLAYER and LAYERFILL. `None` for any other
    /// command.
    pub(in crate::app) fn dispatch_layer_fill(
        &mut self,
        cmd: &str,
        i: usize,
    ) -> Option<Task<Message>> {
        let all = words(cmd);
        let first = all.first()?.to_ascii_uppercase();
        let (name, rest) = if first == "SETVAR" {
            (
                all.get(1)?.to_ascii_uppercase(),
                all.get(2..).unwrap_or(&[]),
            )
        } else {
            (first, all.get(1..).unwrap_or(&[]))
        };
        match name.as_str() {
            apply::FILLBYLAYER => {
                self.fill_by_layer_setvar(i, rest.first().map(String::as_str));
                Some(self.finish_dispatch(cmd))
            }
            "LAYERFILL" if all[0].eq_ignore_ascii_case("LAYERFILL") => {
                self.layer_fill_command(i, rest);
                Some(self.finish_dispatch(cmd))
            }
            "FILLBYLAYERRESET" if all[0].eq_ignore_ascii_case("FILLBYLAYERRESET") => {
                let reset = self.reset_layer_fills(i);
                self.command_line.push_output(&format!(
                    "FILLBYLAYERRESET: {reset} fill(s) follow their layer again."
                ));
                self.refresh_properties();
                Some(self.finish_dispatch(cmd))
            }
            "FILLBYLAYERAPPLY" if all[0].eq_ignore_ascii_case("FILLBYLAYERAPPLY") => {
                self.fill_by_layer_apply_command(i, rest);
                Some(self.finish_dispatch(cmd))
            }
            _ => None,
        }
    }

    fn fill_by_layer_setvar(&mut self, i: usize, value: Option<&str>) {
        let current = u8::from(apply::fill_by_layer(&self.tabs[i].scene.document));
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None => {
                self.command_line.push_output(&format!(
                    "Enter new value for {} <{current}>:",
                    apply::FILLBYLAYER
                ));
                self.pending_setvar = Some(apply::FILLBYLAYER.to_string());
            }
            Some(value @ ("0" | "1")) => {
                if value != current.to_string() {
                    crate::io::set_drawing_variable(
                        &mut self.tabs[i].scene.document,
                        apply::FILLBYLAYER,
                        value,
                    );
                    self.tabs[i].dirty = true;
                }
                self.command_line
                    .push_output(&format!("{} = {value}", apply::FILLBYLAYER));
            }
            Some(_) => {
                self.command_line
                    .push_rejection(crate::t!("Requires 0 or 1 only.").as_ref());
                self.command_line.push_output(&format!(
                    "Enter new value for {} <{current}>:",
                    apply::FILLBYLAYER
                ));
                self.pending_setvar = Some(apply::FILLBYLAYER.to_string());
            }
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
        let current = store::read(&self.tabs[i].scene.document, &layer);
        if words.len() < 2 {
            self.command_line
                .push_output(&describe(&layer, current.as_ref()));
            return;
        }
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
        let current = store::read(&self.tabs[i].scene.document, layer);
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

    /// Hand the Layer Manager the fill style per layer and the choices of
    /// its Fill column.
    pub(in crate::app) fn sync_layer_fill_column(&mut self, i: usize) {
        let document = &self.tabs[i].scene.document;
        let fills = document
            .layers
            .iter()
            .filter_map(|layer| {
                let fill = store::read(document, &layer.name)?;
                (fill.kind != FillKind::None).then(|| (layer.name.to_lowercase(), fill.summary()))
            })
            .collect();
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
        self.tabs[i].layers.set_fills(fills, options);
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
