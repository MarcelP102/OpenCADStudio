//! Fill style per layer, kept in the layer's extension dictionary.
//!
//! The style lives in an XRecord named [`LAYER_FILL_RECORD`] owned by the
//! layer table record, the same place AutoCAD keeps per-layer overrides. Other
//! programs keep the record without reading it, so a drawing stays a plain
//! DWG: the fills themselves are ordinary associative hatches.

use document::objects::{XRecordEntry, XRecordValue};
use document::types::Color;
use document::Document;

/// XRecord in a layer's extension dictionary that holds its fill style.
pub(crate) const LAYER_FILL_RECORD: &str = "OCS_LAYER_FILL";

/// Version written by this build. A newer version is still read; fields this
/// build does not know are ignored.
const VERSION: i16 = 1;

const CODE_VERSION: i32 = 70;
const CODE_KIND: i32 = 71;
const CODE_PATTERN: i32 = 2;
const CODE_COLOR_INDEX: i32 = 62;
const CODE_COLOR_RGB: i32 = 420;
const CODE_BACKGROUND_INDEX: i32 = 63;
const CODE_BACKGROUND_RGB: i32 = 421;
const CODE_TRANSPARENCY: i32 = 440;
const CODE_SCALE: i32 = 41;
const CODE_ANGLE: i32 = 50;

/// What a layer fills its closed shapes with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FillKind {
    None,
    Solid,
    Pattern,
}

impl FillKind {
    fn code(self) -> i16 {
        match self {
            Self::None => 0,
            Self::Solid => 1,
            Self::Pattern => 2,
        }
    }

    fn from_code(code: i32) -> Option<Self> {
        match code {
            0 => Some(Self::None),
            1 => Some(Self::Solid),
            2 => Some(Self::Pattern),
            _ => None,
        }
    }
}

/// A layer's fill style.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LayerFill {
    pub kind: FillKind,
    /// Hatch pattern name (`ANSI31`, `AR-B816`); unused for a solid fill.
    pub pattern: String,
    /// Foreground colour; `ByLayer` follows the layer colour.
    pub color: Color,
    /// Background colour behind a pattern; `None` is transparent.
    pub background: Option<Color>,
    /// 0–90 percent.
    pub transparency: u8,
    pub scale: f64,
    pub angle_deg: f64,
}

impl Default for LayerFill {
    fn default() -> Self {
        Self {
            kind: FillKind::Solid,
            pattern: "ANSI31".into(),
            color: Color::ByLayer,
            background: None,
            transparency: 0,
            scale: 1.0,
            angle_deg: 0.0,
        }
    }
}

impl LayerFill {
    /// Whether shapes on the layer get a fill at all.
    pub(crate) fn fills(&self) -> bool {
        match self.kind {
            FillKind::None => false,
            FillKind::Solid => true,
            FillKind::Pattern => !self.pattern.trim().is_empty(),
        }
    }

    /// Short text for the Layer Manager column and the command line.
    pub(crate) fn summary(&self) -> String {
        match self.kind {
            FillKind::None => "None".into(),
            FillKind::Solid => "Solid".into(),
            FillKind::Pattern => self.pattern.trim().to_string(),
        }
    }
}

fn layer_handle(doc: &Document, layer: &str) -> Option<document::Handle> {
    doc.layers
        .get(layer)
        .map(|record| record.handle)
        .filter(|handle| !handle.is_null())
}

fn int(entry: &XRecordEntry) -> Option<i32> {
    match entry.value {
        XRecordValue::Int16(value) => Some(i32::from(value)),
        XRecordValue::Int32(value) => Some(value),
        XRecordValue::Int64(value) => i32::try_from(value).ok(),
        XRecordValue::Byte(value) => Some(i32::from(value)),
        _ => None,
    }
}

fn real(entry: &XRecordEntry) -> Option<f64> {
    match entry.value {
        XRecordValue::Double(value) if value.is_finite() => Some(value),
        _ => None,
    }
}

fn color_from(index: Option<i32>, rgb: Option<i32>) -> Option<Color> {
    if let Some(rgb) = rgb {
        return Some(Color::from_true_color_value(rgb));
    }
    let index = i16::try_from(index?).ok()?;
    (0..=257).contains(&index).then(|| Color::from_index(index))
}

/// The fill style of `layer`; `None` when it has none or the record cannot be
/// read. A damaged record never panics: it reads as "no fill style".
pub(crate) fn read(doc: &Document, layer: &str) -> Option<LayerFill> {
    let record = doc.xrecord(layer_handle(doc, layer)?, LAYER_FILL_RECORD)?;
    let first = |code: i32| record.entries.iter().find(|entry| entry.code == code);
    let version = first(CODE_VERSION).and_then(int)?;
    if version < 1 {
        return None;
    }
    let kind = FillKind::from_code(first(CODE_KIND).and_then(int)?)?;
    let pattern = first(CODE_PATTERN)
        .and_then(|entry| entry.value.as_string())
        .unwrap_or_default()
        .to_string();
    let color = color_from(
        first(CODE_COLOR_INDEX).and_then(int),
        first(CODE_COLOR_RGB).and_then(int),
    )
    .unwrap_or(Color::ByLayer);
    let background = color_from(
        first(CODE_BACKGROUND_INDEX).and_then(int),
        first(CODE_BACKGROUND_RGB).and_then(int),
    );
    let transparency = first(CODE_TRANSPARENCY)
        .and_then(int)
        .map_or(0, |percent| percent.clamp(0, 90) as u8);
    let scale = first(CODE_SCALE)
        .and_then(real)
        .filter(|scale| *scale > 0.0)
        .unwrap_or(1.0);
    let angle_deg = first(CODE_ANGLE).and_then(real).unwrap_or(0.0);
    Some(LayerFill {
        kind,
        pattern,
        color,
        background,
        transparency,
        scale,
        angle_deg,
    })
}

fn color_entries(color: &Color, index_code: i32, rgb_code: i32) -> Vec<XRecordEntry> {
    match color.to_true_color_value() {
        Some(rgb) if color.is_true_color() => vec![
            XRecordEntry::int16(index_code, color.approximate_index()),
            XRecordEntry::int32(rgb_code, rgb),
        ],
        _ => vec![XRecordEntry::int16(index_code, color.approximate_index())],
    }
}

/// Store `fill` as the fill style of `layer`, or remove it for `None`.
/// Returns whether the document changed.
pub(crate) fn write(doc: &mut Document, layer: &str, fill: Option<&LayerFill>) -> bool {
    let Some(owner) = layer_handle(doc, layer) else {
        return false;
    };
    let Some(fill) = fill else {
        let Some(record) = doc.xrecord_mut(owner, LAYER_FILL_RECORD) else {
            return false;
        };
        let changed = !record.entries.is_empty();
        record.entries.clear();
        record.raw_data.clear();
        record.entries_complete = true;
        return changed;
    };
    if read(doc, layer).as_ref() == Some(fill) {
        return false;
    }
    let mut entries = vec![
        XRecordEntry::int16(CODE_VERSION, VERSION),
        XRecordEntry::int16(CODE_KIND, fill.kind.code()),
        XRecordEntry::string(CODE_PATTERN, fill.pattern.trim()),
    ];
    entries.extend(color_entries(&fill.color, CODE_COLOR_INDEX, CODE_COLOR_RGB));
    if let Some(background) = &fill.background {
        entries.extend(color_entries(
            background,
            CODE_BACKGROUND_INDEX,
            CODE_BACKGROUND_RGB,
        ));
    }
    entries.push(XRecordEntry::int32(
        CODE_TRANSPARENCY,
        i32::from(fill.transparency.min(90)),
    ));
    entries.push(XRecordEntry::double(
        CODE_SCALE,
        if fill.scale.is_finite() && fill.scale > 0.0 {
            fill.scale
        } else {
            1.0
        },
    ));
    entries.push(XRecordEntry::double(
        CODE_ANGLE,
        if fill.angle_deg.is_finite() {
            fill.angle_deg
        } else {
            0.0
        },
    ));
    doc.ensure_xrecord(owner, LAYER_FILL_RECORD);
    let Some(record) = doc.xrecord_mut(owner, LAYER_FILL_RECORD) else {
        return false;
    };
    record.entries = entries;
    // The typed entries are now the whole payload.
    record.raw_data.clear();
    record.raw_dwg_data = None;
    record.entries_complete = true;
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_with_layer(name: &str) -> Document {
        let mut doc = Document::new();
        let mut layer = document::tables::Layer::new(name);
        layer.handle = doc.allocate_handle();
        let _ = doc.layers.add(layer);
        doc
    }

    #[test]
    fn a_fill_style_reads_back_as_written() {
        let mut doc = doc_with_layer("Walls");
        assert_eq!(read(&doc, "Walls"), None);
        let fill = LayerFill {
            kind: FillKind::Pattern,
            pattern: "AR-B816".into(),
            color: Color::Rgb {
                r: 200,
                g: 40,
                b: 10,
            },
            background: Some(Color::Index(9)),
            transparency: 40,
            scale: 2.5,
            angle_deg: 45.0,
        };
        assert!(write(&mut doc, "Walls", Some(&fill)));
        assert_eq!(read(&doc, "Walls"), Some(fill.clone()));
        // The same style again is no change.
        assert!(!write(&mut doc, "Walls", Some(&fill)));
        assert!(write(&mut doc, "Walls", None));
        assert_eq!(read(&doc, "Walls"), None);
    }

    #[test]
    fn a_layer_that_does_not_exist_has_no_style() {
        let mut doc = doc_with_layer("Walls");
        assert!(!write(&mut doc, "Doors", Some(&LayerFill::default())));
        assert_eq!(read(&doc, "Doors"), None);
    }

    #[test]
    fn a_damaged_record_reads_as_no_style() {
        let mut doc = doc_with_layer("Walls");
        let owner = doc.layers.get("Walls").map(|layer| layer.handle).unwrap();
        doc.ensure_xrecord(owner, LAYER_FILL_RECORD);
        let record = doc.xrecord_mut(owner, LAYER_FILL_RECORD).unwrap();
        // A kind this build does not know, and a string where a number goes.
        record.entries = vec![
            XRecordEntry::int16(CODE_VERSION, 1),
            XRecordEntry::string(CODE_KIND, "solid"),
        ];
        assert_eq!(read(&doc, "Walls"), None);
        let record = doc.xrecord_mut(owner, LAYER_FILL_RECORD).unwrap();
        record.entries = vec![
            XRecordEntry::int16(CODE_VERSION, 1),
            XRecordEntry::int16(CODE_KIND, 7),
        ];
        assert_eq!(read(&doc, "Walls"), None);
    }

    #[test]
    fn a_newer_version_with_unknown_fields_still_reads() {
        let mut doc = doc_with_layer("Walls");
        assert!(write(&mut doc, "Walls", Some(&LayerFill::default())));
        let owner = doc.layers.get("Walls").map(|layer| layer.handle).unwrap();
        let record = doc.xrecord_mut(owner, LAYER_FILL_RECORD).unwrap();
        record.entries[0] = XRecordEntry::int16(CODE_VERSION, 2);
        record
            .entries
            .push(XRecordEntry::string(1, "gradient later"));
        record
            .entries
            .push(XRecordEntry::double(CODE_SCALE, f64::NAN));
        let fill = read(&doc, "Walls").unwrap();
        assert_eq!(fill.kind, FillKind::Solid);
        assert_eq!(fill.scale, 1.0);
    }
}
