//! Fill a closed shape as it is drawn, the way its lines get their colour:
//! by layer (the layer's fill style) or custom (the Graphic Attributes
//! palette's fill for new objects).
//!
//! The fill is an ordinary associative hatch placed behind the shape, marked
//! with XDATA [`FILL_APP`]: 1 follows the layer, 2 is custom. A hatch
//! without that XDATA is a hatch the user made on its own.

use super::store::{self, FillKind, LayerFill};
use crate::app::OpenCADStudio;
use crate::command::CadCommand;
use document::xdata::{ExtendedDataRecord, XDataValue};
use document::{EntityType, Handle};

/// XDATA application on a fill made from a layer's fill style.
pub(crate) const FILL_APP: &str = "OCS_FILL";

/// [`FILL_APP`] value: the fill follows its layer's fill style.
pub(crate) const FOLLOWS_LAYER: i16 = 1;
/// [`FILL_APP`] value: the user changed this fill; it keeps its own style.
/// Written when the Graphic Attributes palette changes such a fill.
pub(crate) const OVERRIDDEN: i16 = 2;

/// The drawing commands whose closed shapes get their layer's fill. Paste,
/// import, blocks, xrefs and plugins are not among them.
const DRAW_COMMANDS: &[&str] = &[
    "PLINE",
    "RECT",
    "RECT_ROT",
    "RECT_CEN",
    "POLY",
    "POLY_C",
    "POLY_E",
    "CIRCLE",
    "CIRCLE_CD",
    "CIRCLE_2P",
    "CIRCLE_3P",
    "CIRCLE_TTR",
    "CIRCLE_TTT",
    "ELLIPSE",
    "ELLIPSE_CENTER",
    "SPLINE",
    "SPLINECV",
];

/// Whether `name` is a drawing command whose shapes are filled.
pub(crate) fn is_draw_command(name: &str) -> bool {
    DRAW_COMMANDS.contains(&name)
}

/// Whether `entity` is a closed, flat shape a fill can follow.
pub(crate) fn is_closed_shape(entity: &EntityType) -> bool {
    match entity {
        EntityType::LwPolyline(polyline) => polyline.is_closed && polyline.vertices.len() >= 2,
        EntityType::Polyline2D(polyline) => polyline.is_closed() && polyline.vertices.len() >= 2,
        EntityType::Circle(_) => true,
        EntityType::Ellipse(ellipse) => {
            let sweep = (ellipse.end_parameter - ellipse.start_parameter).abs();
            sweep < 1.0e-9 || (sweep - std::f64::consts::TAU).abs() < 1.0e-9
        }
        EntityType::Spline(spline) => spline.flags.closed,
        _ => false,
    }
}

/// The [`FILL_APP`] value of a hatch; `None` for a hatch the user made.
pub(crate) fn fill_mark(entity: &EntityType) -> Option<i16> {
    match entity {
        EntityType::Hatch(hatch) => hatch_mark(hatch),
        _ => None,
    }
}

/// The [`FILL_APP`] value of a hatch entity.
pub(crate) fn hatch_mark(hatch: &document::entities::Hatch) -> Option<i16> {
    hatch
        .common
        .extended_data
        .get_record(FILL_APP)?
        .values
        .iter()
        .find_map(|value| match value {
            XDataValue::Integer16(mark) => Some(*mark),
            _ => None,
        })
}

/// A fill that followed its layer and is now changed by hand becomes custom.
pub(crate) fn override_mark(entity: &mut EntityType) {
    if let EntityType::Hatch(hatch) = entity {
        if hatch_mark(hatch) == Some(FOLLOWS_LAYER) {
            set_fill_mark(hatch, OVERRIDDEN);
        }
    }
}

/// How the fill of a set of objects is set, as the L / C / V button shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FillMode {
    /// The fill is the layer's fill style (none when the layer has none).
    ByLayer,
    /// The fill is the object's own.
    Custom,
    Varies,
}

/// The fill mode of `handles`: a shape with a fill that follows its layer, or
/// with no fill on a layer without a fill style, is by layer; any other fill,
/// or no fill where the layer has one, is custom. `None` when nothing among
/// `handles` can be filled.
pub(crate) fn fill_mode(doc: &document::Document, handles: &[Handle]) -> Option<FillMode> {
    let picked: rustc_hash::FxHashSet<Handle> = handles.iter().copied().collect();
    // The marks of the fills bound to each picked shape.
    let mut fills_of: rustc_hash::FxHashMap<Handle, Vec<Option<i16>>> = Default::default();
    for entity in doc.entities() {
        let EntityType::Hatch(hatch) = entity else {
            continue;
        };
        let mark = hatch_mark(hatch);
        for shape in hatch.paths.iter().flat_map(|path| &path.boundary_handles) {
            if picked.contains(shape) {
                fills_of.entry(*shape).or_default().push(mark);
            }
        }
    }
    let mut modes = handles.iter().filter_map(|handle| {
        let entity = doc.get_entity(*handle)?;
        if let EntityType::Hatch(hatch) = entity {
            // A fill picked with its shape is judged through the shape.
            let shape_picked = hatch
                .paths
                .iter()
                .flat_map(|path| &path.boundary_handles)
                .any(|shape| picked.contains(shape));
            return (!shape_picked).then(|| match hatch_mark(hatch) {
                Some(FOLLOWS_LAYER) => FillMode::ByLayer,
                _ => FillMode::Custom,
            });
        }
        if !is_closed_shape(entity) {
            return None;
        }
        Some(match fills_of.get(handle).map(Vec::as_slice) {
            Some(marks) if marks.iter().all(|mark| *mark == Some(FOLLOWS_LAYER)) => {
                FillMode::ByLayer
            }
            Some(_) => FillMode::Custom,
            None => {
                let layer_fills =
                    store::read(doc, &entity.common().layer).is_some_and(|fill| fill.fills());
                if layer_fills {
                    FillMode::Custom
                } else {
                    FillMode::ByLayer
                }
            }
        })
    });
    let first = modes.next()?;
    Some(if modes.all(|mode| mode == first) {
        first
    } else {
        FillMode::Varies
    })
}

/// Set the [`FILL_APP`] value of a hatch.
pub(crate) fn set_fill_mark(hatch: &mut document::entities::Hatch, mark: i16) {
    let mut record = ExtendedDataRecord::new(FILL_APP);
    record.add_value(XDataValue::Integer16(mark));
    hatch.common.extended_data.upsert_record(record);
}

/// `handles` plus the marked fills of the shapes among them and the shapes
/// of the marked fills among them: a shape and its fill select, move, copy
/// and erase as one. A hatch without the mark is not added.
pub(crate) fn with_layer_fills(doc: &document::Document, mut handles: Vec<Handle>) -> Vec<Handle> {
    // No fill was ever made in this drawing: nothing to look up.
    if handles.is_empty() || !doc.app_ids.contains(FILL_APP) {
        return handles;
    }
    let picked: rustc_hash::FxHashSet<Handle> = handles.iter().copied().collect();
    let mut added = Vec::new();
    for entity in doc.entities() {
        if fill_mark(entity).is_none() {
            continue;
        }
        let EntityType::Hatch(hatch) = entity else {
            continue;
        };
        let fill = entity.common().handle;
        let shapes = hatch
            .paths
            .iter()
            .flat_map(|path| path.boundary_handles.iter().copied());
        if picked.contains(&fill) {
            added.extend(shapes.filter(|shape| doc.get_entity(*shape).is_some()));
        } else if shapes.into_iter().any(|shape| picked.contains(&shape)) {
            added.push(fill);
        }
    }
    added.retain(|handle| !picked.contains(handle));
    added.sort_unstable_by_key(Handle::value);
    added.dedup();
    handles.extend(added);
    handles
}

/// The render pattern of a fill style; `None` when the pattern is unknown.
fn render_pattern(
    fill: &LayerFill,
) -> Option<(String, crate::scene::model::hatch_model::HatchPattern)> {
    use crate::scene::model::hatch_model::HatchPattern;
    match fill.kind {
        FillKind::None => None,
        FillKind::Solid => Some(("SOLID".into(), HatchPattern::Solid)),
        FillKind::Pattern => crate::scene::model::hatch_patterns::catalog()
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(fill.pattern.trim()))
            .map(|entry| (entry.name.clone(), entry.gpu.clone())),
    }
}

/// Everything a new fill is made from.
pub(crate) struct FillSpec {
    pub name: String,
    pub pattern: crate::scene::model::hatch_model::HatchPattern,
    pub scale: f64,
    pub angle_deg: f64,
    pub color: document::types::Color,
    pub transparency: document::types::Transparency,
    pub background: Option<document::types::Color>,
}

impl FillSpec {
    /// A layer's fill style; `None` when it fills nothing.
    pub(crate) fn of_layer(fill: &LayerFill) -> Option<Self> {
        if !fill.fills() {
            return None;
        }
        let (name, pattern) = render_pattern(fill)?;
        Some(Self {
            name,
            pattern,
            scale: fill.scale,
            angle_deg: fill.angle_deg,
            color: fill.color,
            transparency: style_transparency(fill),
            background: fill.background,
        })
    }
}

fn style_transparency(fill: &LayerFill) -> document::types::Transparency {
    if fill.transparency == 0 {
        document::types::Transparency::BY_LAYER
    } else {
        document::types::Transparency::from_percent(f64::from(fill.transparency.min(90)) / 100.0)
    }
}

/// Put a layer fill style's colour, background and transparency on `hatch`.
pub(crate) fn apply_style(hatch: &mut document::entities::Hatch, fill: &LayerFill) {
    hatch.common.color = fill.color;
    hatch.common.transparency = style_transparency(fill);
    if let Some(background) = &fill.background {
        crate::entities::hatch::set_background_color(hatch, background);
    }
}

impl OpenCADStudio {
    /// The layer a committed entity ends up on.
    fn commit_layer_of(&self, i: usize, entity: &EntityType, preserve_layer: bool) -> String {
        let layer = &entity.common().layer;
        if preserve_layer && !layer.trim().is_empty() {
            layer.clone()
        } else {
            self.tabs[i].active_layer.clone()
        }
    }

    /// The custom fill new objects get (Graphic Attributes, nothing
    /// selected, mode C); `None` when new fills follow the layer or the custom
    /// fill is None.
    fn creation_custom_fill(&self) -> Option<FillSpec> {
        let creation = &self.graphic_attributes.creation_fill;
        if creation.by_layer {
            return None;
        }
        let (name, pattern) = crate::app::update::graphic_attributes::fill_pattern(creation.kind)?;
        Some(FillSpec {
            name,
            pattern,
            scale: 1.0,
            angle_deg: 0.0,
            color: creation.color,
            transparency: creation.transparency,
            background: None,
        })
    }

    /// Whether committing `entity` from the running command may add a fill.
    /// The fill reorders the sort table and may register its XDATA
    /// application, so such a step is undone from a structure snapshot.
    pub(in crate::app) fn layer_fill_may_apply(
        &self,
        i: usize,
        entity: &EntityType,
        preserve_layer: bool,
    ) -> bool {
        let document = &self.tabs[i].scene.document;
        let fills = if self.graphic_attributes.creation_fill.by_layer {
            store::read(document, &self.commit_layer_of(i, entity, preserve_layer))
                .is_some_and(|fill| fill.fills())
        } else {
            self.creation_custom_fill().is_some()
        };
        fills
            && self.tabs[i]
                .active_cmd
                .as_ref()
                .is_some_and(|command| is_draw_command(command.name()))
            && is_closed_shape(entity)
    }

    /// Fill the closed shapes among `handles`, just committed by a drawing
    /// command: with their layer's fill style, or with the palette's custom
    /// fill. Runs inside the commit's undo step, so one undo removes shape
    /// and fill together. A layer (or custom fill) of None fills nothing.
    pub(in crate::app) fn auto_fill_committed(&mut self, i: usize, handles: &[Handle]) {
        if !self.tabs[i]
            .active_cmd
            .as_ref()
            .is_some_and(|command| is_draw_command(command.name()))
        {
            return;
        }
        let custom = self.creation_custom_fill();
        let by_layer = self.graphic_attributes.creation_fill.by_layer;
        for handle in handles {
            let _ = match &custom {
                _ if by_layer => self.add_layer_fill(i, *handle, FOLLOWS_LAYER),
                Some(spec) => self.add_fill(i, *handle, spec, OVERRIDDEN),
                None => None,
            };
        }
    }

    /// PLINE builds its polyline live and closes the undo step when it ends,
    /// so its fill is added afterwards as a step of its own.
    pub(in crate::app) fn auto_fill_live_entity(&mut self, i: usize, handle: Handle) {
        let Some(entity) = self.tabs[i].scene.document.get_entity(handle) else {
            return;
        };
        if !self.layer_fill_may_apply(i, entity, true) {
            return;
        }
        let label = self.history_label_from_active_cmd(i, "ENTITY");
        let pending = self.begin_undo(i, label, 1, false);
        self.auto_fill_committed(i, &[handle]);
        if let Some(pending) = pending {
            self.commit_undo_delta(i, pending);
        }
    }

    /// The plane a new fill lies in: the working plane in model space, the
    /// sheet in paper space. Matches the Graphic Attributes palette.
    fn layer_fill_plane(&self, i: usize) -> crate::command::WorkingPlane {
        let working = if self.tabs[i].editing_model_space() {
            self.tabs[i].ucs_xform().working_plane()
        } else {
            crate::command::WorkingPlane::default()
        };
        let normal = working.z.normalize_or(glam::DVec3::Z);
        let storage = crate::entities::curve::ocs_plane(
            document::types::Vector3::new(normal.x, normal.y, normal.z),
            working.origin.dot(normal),
        );
        crate::command::WorkingPlane::new(
            glam::DVec3::from_array(storage.origin),
            glam::DVec3::from_array(storage.x_axis),
            glam::DVec3::from_array(storage.y_axis),
        )
    }

    /// Add one associative fill to the closed shape `shape` in its layer's
    /// fill style, behind the shape, marked `mark`. Returns the new hatch.
    pub(in crate::app) fn add_layer_fill(
        &mut self,
        i: usize,
        shape: Handle,
        mark: i16,
    ) -> Option<Handle> {
        let layer = self.tabs[i]
            .scene
            .document
            .get_entity(shape)?
            .common()
            .layer
            .clone();
        let fill = store::read(&self.tabs[i].scene.document, &layer)?;
        let spec = FillSpec::of_layer(&fill)?;
        self.add_fill(i, shape, &spec, mark)
    }

    /// Add one associative fill made from `spec` to the closed shape
    /// `shape`, on the shape's layer, behind it, marked `mark`.
    pub(in crate::app) fn add_fill(
        &mut self,
        i: usize,
        shape: Handle,
        spec: &FillSpec,
        mark: i16,
    ) -> Option<Handle> {
        let entity = self.tabs[i].scene.document.get_entity(shape)?.clone();
        if !is_closed_shape(&entity) || self.tabs[i].scene.is_layer_locked(shape) {
            return None;
        }
        let layer = entity.common().layer.clone();
        let plane = self.layer_fill_plane(i);
        let source = crate::scene::entity_boundary_source_on_plane(&entity, plane, 1.0e-6, None)?;
        let mut sources = rustc_hash::FxHashMap::default();
        sources.insert(shape, source);
        let mut command =
            crate::modules::draw::draw::hatch::HatchCommand::new(sources, vec![shape], None, plane)
                .with_origin(self.tabs[i].scene.document.hatch_origin())
                .with_pattern(spec.name.clone(), spec.pattern.clone());
        if let Some(mut settings) = command.hatch_settings() {
            settings.scale = spec.scale;
            settings.angle_deg = spec.angle_deg;
            command.set_hatch_settings(settings);
        }
        let crate::command::CmdResult::CommitHatch(model) = command.finish_selected() else {
            return None;
        };
        let hatch_handle = self.tabs[i].scene.add_hatch(model, Some(&layer), None);
        if hatch_handle.is_null() {
            return None;
        }
        let Some(EntityType::Hatch(mut hatch)) = self.tabs[i]
            .scene
            .document
            .get_entity(hatch_handle)
            .cloned()
        else {
            return None;
        };
        hatch.common.color = spec.color;
        hatch.common.transparency = spec.transparency;
        if let Some(background) = &spec.background {
            crate::entities::hatch::set_background_color(&mut hatch, background);
        }
        self.tabs[i].scene.ensure_app_id(FILL_APP);
        set_fill_mark(&mut hatch, mark);
        self.tabs[i].scene.update_entity(EntityType::Hatch(hatch));
        // HPDRAWORDER 3: behind its boundary.
        let _ = self.apply_draw_order(i, &[hatch_handle], None, Some((false, &[shape])));
        Some(hatch_handle)
    }
}

/// Give an existing fill the pattern, scale and angle of `fill` in place, so
/// it keeps its handle, its draw order and its link to the shape.
fn restyle(hatch: &mut document::entities::Hatch, fill: &LayerFill) -> bool {
    use document::entities::{HatchPattern as DxfPattern, HatchPatternType};
    match fill.kind {
        FillKind::None => return false,
        FillKind::Solid => {
            hatch.pattern = DxfPattern::new("SOLID");
            hatch.is_solid = true;
        }
        FillKind::Pattern => {
            let Some(entry) = crate::scene::model::hatch_patterns::catalog()
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(fill.pattern.trim()))
            else {
                return false;
            };
            hatch.pattern_scale = fill.scale;
            hatch.pattern_angle = fill.angle_deg.to_radians();
            let mut pattern = crate::scene::model::hatch_patterns::build_dxf_pattern(entry);
            crate::entities::hatch::scale_pattern_geometry(&mut pattern, fill.scale.max(1.0e-6));
            crate::entities::hatch::rotate_pattern_geometry(&mut pattern, hatch.pattern_angle);
            let origin = hatch.pattern_origin();
            crate::entities::hatch::translate_pattern_geometry(&mut pattern, origin.x, origin.y);
            hatch.pattern = pattern;
            hatch.is_solid = false;
        }
    }
    hatch.pattern_type = HatchPatternType::Predefined;
    hatch.gradient_color.enabled = false;
    apply_style(hatch, fill);
    true
}

impl OpenCADStudio {
    /// Bring every fill that follows `layer` (mark 1, shape on that layer) to
    /// the layer's style; a layer without a style erases them. Part of the
    /// caller's undo step. Returns how many fills changed.
    pub(in crate::app) fn restyle_layer_fills(&mut self, i: usize, layer: &str) -> usize {
        let document = &self.tabs[i].scene.document;
        if !document.app_ids.contains(FILL_APP) {
            return 0;
        }
        let style = store::read(document, layer).filter(LayerFill::fills);
        let on_layer = |handle: &Handle| {
            document
                .get_entity(*handle)
                .is_some_and(|shape| shape.common().layer.eq_ignore_ascii_case(layer))
        };
        let following: Vec<Box<document::entities::Hatch>> = document
            .entities()
            .filter(|entity| fill_mark(entity) == Some(FOLLOWS_LAYER))
            .filter_map(|entity| match entity {
                EntityType::Hatch(hatch) => Some(hatch),
                _ => None,
            })
            .filter(|hatch| {
                hatch
                    .paths
                    .iter()
                    .flat_map(|path| path.boundary_handles.iter())
                    .any(on_layer)
            })
            .cloned()
            .collect();
        let count = following.len();
        match style {
            None => {
                let handles: Vec<Handle> =
                    following.iter().map(|hatch| hatch.common.handle).collect();
                self.tabs[i].scene.erase_entities(&handles);
            }
            Some(style) => {
                for mut hatch in following {
                    if restyle(&mut hatch, &style) {
                        self.tabs[i].scene.update_entity(EntityType::Hatch(hatch));
                    }
                }
            }
        }
        count
    }

    /// FILLBYLAYERAPPLY: fill the closed shapes on `layers` in the current
    /// space that have no layer fill yet. One undo step. Returns how many.
    pub(in crate::app) fn apply_layer_fills(&mut self, i: usize, layers: &[String]) -> usize {
        let space = self.tabs[i].scene.current_layout_block_handle_pub();
        let document = &self.tabs[i].scene.document;
        let filled: rustc_hash::FxHashSet<Handle> = document
            .entities()
            .filter(|entity| fill_mark(entity).is_some())
            .filter_map(|entity| match entity {
                EntityType::Hatch(hatch) => Some(hatch),
                _ => None,
            })
            .flat_map(|hatch| {
                hatch
                    .paths
                    .iter()
                    .flat_map(|path| path.boundary_handles.iter().copied())
            })
            .collect();
        let shapes: Vec<Handle> = document
            .entities()
            .filter(|entity| {
                let common = entity.common();
                common.owner_handle == space
                    && is_closed_shape(entity)
                    && !filled.contains(&common.handle)
                    && layers
                        .iter()
                        .any(|layer| layer.eq_ignore_ascii_case(&common.layer))
            })
            .map(|entity| entity.common().handle)
            .collect();
        if shapes.is_empty() {
            return 0;
        }
        let pending = self.begin_undo(
            i,
            crate::app::tab_document::HistoryLabel::Message("FILLBYLAYERAPPLY"),
            shapes.len(),
            false,
        );
        let added = shapes
            .iter()
            .filter(|shape| self.add_layer_fill(i, **shape, FOLLOWS_LAYER).is_some())
            .count();
        if added > 0 {
            self.tabs[i].dirty = true;
        }
        if let Some(pending) = pending {
            self.commit_undo_delta(i, pending);
        }
        added
    }
}

/// Split a palette selection: a shape and its marked fill are selected
/// together, so the palette changes the fill itself (in place, keeping its
/// handle and draw order) and leaves the shape out, which would get a second
/// fill otherwise. Returns the handles to work on and the shapes left out.
pub(crate) fn split_layer_fills(
    doc: &document::Document,
    handles: &[Handle],
) -> (Vec<Handle>, Vec<Handle>) {
    if handles.is_empty() || !doc.app_ids.contains(FILL_APP) {
        return (handles.to_vec(), Vec::new());
    }
    let picked: rustc_hash::FxHashSet<Handle> = handles.iter().copied().collect();
    let mut shapes = rustc_hash::FxHashSet::default();
    for handle in handles {
        let Some(EntityType::Hatch(hatch)) = doc.get_entity(*handle) else {
            continue;
        };
        if hatch_mark(hatch).is_none() {
            continue;
        }
        shapes.extend(
            hatch
                .paths
                .iter()
                .flat_map(|path| path.boundary_handles.iter().copied())
                .filter(|shape| picked.contains(shape)),
        );
    }
    let kept = handles
        .iter()
        .copied()
        .filter(|handle| !shapes.contains(handle))
        .collect();
    let mut left_out: Vec<Handle> = shapes.into_iter().collect();
    left_out.sort_unstable_by_key(Handle::value);
    (kept, left_out)
}

impl OpenCADStudio {
    /// After the Graphic Attributes palette changed the fills of `handles`
    /// (shapes, or fills themselves), mark those fills as the objects' own
    /// (C), so a layer style no longer overwrites them.
    pub(in crate::app) fn mark_fills_overridden(&mut self, i: usize, shapes: &[Handle]) {
        if shapes.is_empty() {
            return;
        }
        let shape_set: rustc_hash::FxHashSet<Handle> = shapes.iter().copied().collect();
        let changed: Vec<Box<document::entities::Hatch>> = self.tabs[i]
            .scene
            .document
            .entities()
            .filter_map(|entity| match entity {
                EntityType::Hatch(hatch) => Some(hatch),
                _ => None,
            })
            .filter(|hatch| {
                shape_set.contains(&hatch.common.handle)
                    || hatch
                        .paths
                        .iter()
                        .flat_map(|path| path.boundary_handles.iter())
                        .any(|shape| shape_set.contains(shape))
            })
            .filter(|hatch| hatch_mark(hatch) != Some(OVERRIDDEN))
            .cloned()
            .collect();
        if changed.is_empty() {
            return;
        }
        self.tabs[i].scene.ensure_app_id(FILL_APP);
        for mut hatch in changed {
            set_fill_mark(&mut hatch, OVERRIDDEN);
            self.tabs[i].scene.update_entity(EntityType::Hatch(hatch));
        }
    }

    /// FILLBYLAYERRESET: give the selected shapes their layer's fill again,
    /// replacing what the user set. One undo step. Returns how many.
    pub(in crate::app) fn reset_layer_fills(&mut self, i: usize) -> usize {
        let selected = self.tabs[i].scene.selected_handles_in_order();
        let document = &self.tabs[i].scene.document;
        let shapes: Vec<Handle> = selected
            .iter()
            .copied()
            .filter(|handle| document.get_entity(*handle).is_some_and(is_closed_shape))
            .collect();
        let shape_set: rustc_hash::FxHashSet<Handle> = shapes.iter().copied().collect();
        // ByLayer: whatever fill the shapes have now gives way to the layer's.
        let old_fills: Vec<Handle> = document
            .entities()
            .filter_map(|entity| match entity {
                EntityType::Hatch(hatch) => Some(hatch),
                _ => None,
            })
            .filter(|hatch| {
                hatch
                    .paths
                    .iter()
                    .flat_map(|path| path.boundary_handles.iter())
                    .any(|shape| shape_set.contains(shape))
            })
            .map(|hatch| hatch.common.handle)
            .collect();
        if shapes.is_empty() {
            return 0;
        }
        let pending = self.begin_undo(
            i,
            crate::app::tab_document::HistoryLabel::Message("FILLBYLAYERRESET"),
            shapes.len(),
            false,
        );
        self.tabs[i].scene.erase_entities(&old_fills);
        let added = shapes
            .iter()
            .filter(|shape| self.add_layer_fill(i, **shape, FOLLOWS_LAYER).is_some())
            .count();
        self.tabs[i].dirty = true;
        if let Some(pending) = pending {
            self.commit_undo_delta(i, pending);
        }
        added
    }
}
