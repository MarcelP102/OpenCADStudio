//! ADDSURFACE, CLIPSURFACE and INTERSECTSURFACE: Booleans on closed 2D
//! shapes (polylines, circles, ellipses, splines, hatches, solids and
//! regions), done on exact Solidcore planar regions.
//!
//! The planners here are pure: a document and a selection in, an
//! [`EditPlan`] out. Nothing is written until the whole plan exists, so a
//! failure leaves the drawing unchanged; the app applies a plan as one undo
//! step.
//!
//! Drawing order decides the roles: CLIPSURFACE cuts every shape by the
//! shapes above it, and every result takes the settings of the bottommost
//! shape it comes from (layer, colour, linetype, lineweight, transparency,
//! XDATA, and its kind, so a hatch gives a hatch).

pub(crate) mod output;
pub(crate) mod shape;
#[cfg(test)]
mod tests;

pub use output::Out;
use shape::{Face, Frame, Oriented, Reject, Shape};

use document::entities::{BoundaryEdge, BoundaryPath, Hatch};
use document::objects::ObjectType;
use document::{Document, EntityType, Handle};
use ocs_solid::model::{BooleanOp, Brep};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Clip,
    Intersect,
}

impl Op {
    pub fn command(self) -> &'static str {
        match self {
            Op::Add => "ADDSURFACE",
            Op::Clip => "CLIPSURFACE",
            Op::Intersect => "INTERSECTSURFACE",
        }
    }

    /// The command for `name` (a command name or its alias).
    pub fn parse(name: &str) -> Option<Op> {
        match name
            .trim()
            .trim_start_matches(['_', '.'])
            .to_ascii_uppercase()
            .as_str()
        {
            "ADDSURFACE" | "AS" => Some(Op::Add),
            "CLIPSURFACE" | "CS" => Some(Op::Clip),
            "INTERSECTSURFACE" | "IS" => Some(Op::Intersect),
            _ => None,
        }
    }
}

pub const SELECT_PROMPT: &str = "Select at least two closed 2D shapes, then press Enter to apply.";
pub const SELECT_MORE_PROMPT: &str =
    "Select at least one more closed 2D shape, then press Enter to apply.";
pub const NO_COMMON_SURFACE: &str = "Selected shapes have no common surface.";

/// One object replaced by its results. The first result keeps the object's
/// handle (and so its drawing-order slot, extension dictionary and reactors)
/// where its kind allows; the rest go right above it.
#[derive(Debug, Clone)]
pub struct Replacement {
    pub handle: Handle,
    pub results: Vec<Out>,
}

/// A result object before it has a handle: result `index` of replacement
/// `rep`, or addition `n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultRef {
    Replacement { rep: usize, index: usize },
    Addition(usize),
}

/// An associative hatch that follows result objects: an existing hatch
/// rewritten in place, or a new one (null handle).
#[derive(Debug, Clone)]
pub struct HatchUpdate {
    /// The hatch with every property it keeps; its paths are replaced.
    pub hatch: Box<Hatch>,
    /// Paths that follow a result outline, with the object that bounds each.
    pub bound: Vec<(BoundaryPath, ResultRef)>,
    /// Paths the operation does not touch, kept as they are.
    pub kept: Vec<BoundaryPath>,
}

#[derive(Debug, Clone, Default)]
pub struct EditPlan {
    pub replacements: Vec<Replacement>,
    pub removals: Vec<Handle>,
    /// New objects (INTERSECTSURFACE), on top of the drawing order.
    pub additions: Vec<Out>,
    pub hatches: Vec<HatchUpdate>,
    /// Summary line for the command line.
    pub summary: String,
}

impl EditPlan {
    pub fn is_empty(&self) -> bool {
        self.replacements.is_empty()
            && self.removals.is_empty()
            && self.additions.is_empty()
            && self.hatches.is_empty()
    }
}

/// A validated selection, bottom → top in drawing order.
pub(crate) struct Selection {
    pub items: Vec<Item>,
    pub frame: Frame,
}

pub(crate) struct Item {
    pub handle: Handle,
    pub entity: EntityType,
    pub shape: Shape,
    /// Associative hatches that follow this object (and no lower selected
    /// object): they follow its result.
    pub hatches: Vec<Hatch>,
}

impl Item {
    /// The path of an open object; `None` for a closed shape.
    fn path(&self) -> Option<&Vec<ocs_geom::geom2d::Curve>> {
        self.shape.path.as_ref()
    }

    fn closed_area(&self) -> Option<&Brep> {
        if self.path().is_some() {
            None
        } else {
            self.shape.area.as_ref()
        }
    }
}

fn t(source: &str) -> String {
    crate::t!(source).into_owned()
}

/// Every associative hatch, keyed by the objects its paths follow.
fn associative_hatches(doc: &Document) -> HashMap<Handle, Vec<Handle>> {
    let mut index: HashMap<Handle, Vec<Handle>> = HashMap::new();
    for e in doc.entities() {
        let EntityType::Hatch(h) = e else { continue };
        if !h.is_associative {
            continue;
        }
        for b in h.paths.iter().flat_map(|p| p.boundary_handles.iter()) {
            let list = index.entry(*b).or_default();
            if !list.contains(&h.common.handle) {
                list.push(h.common.handle);
            }
        }
    }
    index
}

fn follows_any(h: &Hatch, objects: &[Handle]) -> bool {
    h.paths
        .iter()
        .flat_map(|p| p.boundary_handles.iter())
        .any(|b| objects.contains(b))
}

/// Drawing-order key of every entity in `block` that a sort table moves.
fn draw_order_keys(doc: &Document, block: Handle) -> HashMap<u64, u64> {
    let mut keys = HashMap::new();
    for obj in doc.objects.values() {
        if let ObjectType::SortEntitiesTable(t) = obj {
            if t.block_owner_handle == block {
                for e in t.entries() {
                    keys.insert(e.entity_handle.value(), e.sort_handle.value());
                }
            }
        }
    }
    keys
}

fn layer_locked(doc: &Document, layer: &str) -> bool {
    doc.layers.get(layer).is_some_and(|l| l.is_locked())
}

fn reject_text(r: &Reject, e: &EntityType) -> String {
    let kind = crate::entities::traits::entity_type_name(e).to_string();
    match r {
        Reject::Unsupported => crate::t!(
            "Cannot apply: a %{kind} is not a supported 2D shape.",
            kind = kind
        )
        .into_owned(),
        Reject::Degenerate => {
            crate::t!("Cannot apply: the %{kind} has no area.", kind = kind).into_owned()
        }
        Reject::NotPlanar => t("Cannot apply: the shapes do not lie in one plane."),
    }
}

/// The selection's size, for the tolerances.
fn extent(entities: &[EntityType]) -> f64 {
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for e in entities {
        let b = e.as_entity().bounding_box();
        for (k, (l, h)) in [(b.min.x, b.max.x), (b.min.y, b.max.y), (b.min.z, b.max.z)]
            .into_iter()
            .enumerate()
        {
            if l.is_finite() && h.is_finite() {
                lo[k] = lo[k].min(l);
                hi[k] = hi[k].max(h);
            }
        }
    }
    if lo.iter().chain(&hi).all(|v| v.is_finite()) {
        ((hi[0] - lo[0]).powi(2) + (hi[1] - lo[1]).powi(2) + (hi[2] - lo[2]).powi(2))
            .sqrt()
            .max(1e-6)
    } else {
        1.0
    }
}

/// The common plane: that of the bottommost object that defines one (a line
/// lies in many), else world XY through the first line.
fn frame_of(entities: &[EntityType], extent: f64) -> Frame {
    for e in entities {
        if let Some(plane) = shape::object_plane(e) {
            if let Some(n) = shape::plane_normal(&plane) {
                let o = plane.origin;
                let elevation = o[0] * n[0] + o[1] * n[1] + o[2] * n[2];
                return Frame::new(n, elevation, extent);
            }
        }
    }
    let z = match entities.first() {
        Some(EntityType::Line(l)) => l.start.z,
        _ => 0.0,
    };
    Frame::new([0.0, 0.0, 1.0], z, extent)
}

/// Validate the whole selection before anything changes. Errors are
/// user-facing sentences.
pub(crate) fn validate(doc: &Document, handles: &[Handle]) -> Result<Selection, String> {
    let mut unique: Vec<Handle> = Vec::new();
    for h in handles {
        if !unique.contains(h) {
            unique.push(*h);
        }
    }
    // A selected associative hatch whose boundary is selected too (a window
    // selection picks both) is not a shape of its own: it follows the result.
    let snapshot = unique.clone();
    unique.retain(|h| match doc.get_entity(*h) {
        Some(EntityType::Hatch(hatch)) => !(hatch.is_associative && follows_any(hatch, &snapshot)),
        _ => true,
    });
    let mut entities = Vec::new();
    for h in &unique {
        let Some(e) = doc.get_entity(*h) else {
            continue;
        };
        if layer_locked(doc, &e.common().layer) {
            return Err(t("Cannot apply: a shape is on a locked layer."));
        }
        entities.push(e.clone());
    }
    if entities.len() < 2 {
        return Err(t(SELECT_PROMPT));
    }
    let owner = entities[0].common().owner_handle;
    if entities.iter().any(|e| e.common().owner_handle != owner) {
        return Err(t(
            "Cannot apply: the shapes are in different spaces or blocks.",
        ));
    }
    // Bottom → top; the bottommost object defines the plane.
    let keys = draw_order_keys(doc, owner);
    let key = |h: Handle| {
        (
            keys.get(&h.value()).copied().unwrap_or(h.value()),
            h.value(),
        )
    };
    entities.sort_by_key(|e| key(e.common().handle));
    let frame = frame_of(&entities, extent(&entities));

    let mut items = Vec::with_capacity(entities.len());
    for e in entities {
        match shape::shape_of(&e, &frame) {
            Ok(shape) => items.push(Item {
                handle: e.common().handle,
                entity: e,
                shape,
                hatches: Vec::new(),
            }),
            Err(r) => return Err(reject_text(&r, &e)),
        }
    }

    // Each associative hatch follows the bottommost selected object among
    // its boundaries, and must be editable like the objects themselves.
    let index = associative_hatches(doc);
    let mut owned: Vec<Handle> = Vec::new();
    for item in &mut items {
        for hh in index.get(&item.handle).into_iter().flatten() {
            if owned.contains(hh) {
                continue;
            }
            let Some(EntityType::Hatch(hatch)) = doc.get_entity(*hh) else {
                continue;
            };
            if layer_locked(doc, &hatch.common.layer) {
                return Err(t("Cannot apply: a hatch of a shape is on a locked layer."));
            }
            owned.push(*hh);
            item.hatches.push((**hatch).clone());
        }
    }
    Ok(Selection { items, frame })
}

/// The paths of `hatch` the operation does not touch: those that follow
/// objects outside the selection, and those that follow no object at all.
fn untouched_paths(hatch: &Hatch, sel: &Selection) -> Vec<BoundaryPath> {
    let selected: Vec<Handle> = sel.items.iter().map(|i| i.handle).collect();
    hatch
        .paths
        .iter()
        .filter(|p| !p.boundary_handles.iter().any(|b| selected.contains(b)))
        .cloned()
        .collect()
}

/// A hatch whose selected boundary object disappears or becomes a hatch
/// itself. Its other paths still bound a surface, so it keeps them; only a
/// hatch with nothing left is erased.
fn release(hatch: &Hatch, sel: &Selection, plan: &mut EditPlan) {
    let kept = untouched_paths(hatch, sel);
    if kept.is_empty() {
        plan.removals.push(hatch.common.handle);
    } else {
        plan.hatches.push(HatchUpdate {
            hatch: Box::new(hatch.clone()),
            bound: Vec::new(),
            kept,
        });
    }
}

/// The loops of `faces` in the order outline results are written: each outer
/// loop, then its holes.
fn ordered_loops(faces: &[Face]) -> Vec<(&[Oriented], bool)> {
    faces
        .iter()
        .flat_map(|f| {
            f.loops
                .iter()
                .enumerate()
                .map(|(k, l)| (l.as_slice(), k == 0))
        })
        .collect()
}

/// A polyline path written in `from`'s coordinates, rewritten in `to`'s.
fn reframe(path: &mut BoundaryPath, from: &Frame, to: &ocs_geom::space::Plane, mirrored: bool) {
    for edge in &mut path.edges {
        if let BoundaryEdge::Polyline(pe) = edge {
            for v in &mut pe.vertices {
                if let Some(p) = to.project(from.plane.point_at([v.x, v.y])) {
                    v.x = p[0];
                    v.y = p[1];
                }
                if mirrored {
                    v.z = -v.z; // bulge
                }
            }
        }
    }
}

/// Rewrite `hatch` to follow the outline results of `faces`, whose objects
/// are `refs` (one per loop). With `keep`, the paths the operation does not
/// touch stay; a new hatch for an INTERSECTSURFACE piece keeps none.
fn follow(
    hatch: &Hatch,
    faces: &[Face],
    refs: Vec<ResultRef>,
    sel: &Selection,
    keep: bool,
) -> HatchUpdate {
    let kept = if keep {
        untouched_paths(hatch, sel)
    } else {
        Vec::new()
    };
    // The result loops are the outermost boundary only when no other path
    // (e.g. one of an unselected object) stays.
    let outermost = kept.is_empty();
    // A hatch facing the other way keeps its own normal, so its kept paths
    // stay valid: the new paths are written in its coordinates.
    let n = sel.frame.normal;
    let flipped = n[0] * hatch.normal.x + n[1] * hatch.normal.y + n[2] * hatch.normal.z < 0.0;
    let target = crate::entities::curve::ocs_plane(hatch.normal, hatch.elevation);
    let bound = ordered_loops(faces)
        .into_iter()
        .zip(refs)
        .map(|((l, outer), r)| {
            let mut path = output::hatch_path(l, outer && outermost);
            if flipped {
                reframe(&mut path, &sel.frame, &target, true);
            }
            (path, r)
        })
        .collect();
    let mut hatch = hatch.clone();
    if !flipped {
        hatch.normal = document::types::Vector3::new(n[0], n[1], n[2]);
        hatch.elevation = sel.frame.elevation;
    }
    hatch.seed_points.clear();
    HatchUpdate {
        hatch: Box::new(hatch),
        bound,
        kept,
    }
}

/// Replace `item` by `faces` (in `item`'s style). The `hatches` follow the
/// result; when the result is not a set of outlines (a hatch or a REGION),
/// they are released instead.
fn push_replacement(
    plan: &mut EditPlan,
    sel: &Selection,
    item: &Item,
    faces: &[Face],
    hatches: &[&Hatch],
) {
    let results = output::results(&item.entity, faces, &sel.frame);
    let rep = plan.replacements.len();
    let outlines = results.iter().all(Out::is_outline);
    for h in hatches {
        if outlines {
            let refs = (0..results.len())
                .map(|index| ResultRef::Replacement { rep, index })
                .collect();
            plan.hatches.push(follow(h, faces, refs, sel, true));
        } else {
            release(h, sel, plan);
        }
    }
    plan.replacements.push(Replacement {
        handle: item.handle,
        results,
    });
}

fn op_err(e: String) -> String {
    crate::t!(
        "Cannot apply: the geometry operation failed (%{error}).",
        error = e
    )
    .into_owned()
}

fn overlaps_bounds(a: &Brep, b: &Brep, frame: &Frame) -> bool {
    match (shape::bounds(a, frame), shape::bounds(b, frame)) {
        (Some(x), Some(y)) => shape::bounds_overlap(x, y, frame.linear),
        _ => false,
    }
}

/// CLIPSURFACE: every object loses the area of all selected closed shapes
/// above it, and closed shapes are split along every open path above them.
/// An open path loses its parts inside the closed shapes above it. All from
/// the original geometry; the topmost object stays as it is.
pub(crate) fn plan_clip(sel: &Selection) -> Result<EditPlan, String> {
    let n = sel.items.len();
    let frame = &sel.frame;
    // Above item i: the union of the closed shapes, and the open paths.
    let mut area_above: Vec<Option<Brep>> = vec![None; n];
    let mut knives_above: Vec<Vec<Vec<ocs_geom::geom2d::Curve>>> = vec![Vec::new(); n];
    for i in (0..n.saturating_sub(1)).rev() {
        let above = &sel.items[i + 1];
        knives_above[i] = knives_above[i + 1].clone();
        area_above[i] = area_above[i + 1].clone();
        if let Some(path) = above.path() {
            knives_above[i].push(path.clone());
        } else if let Some(a) = above.closed_area() {
            area_above[i] = Some(match area_above[i].take() {
                None => a.clone(),
                Some(acc) => shape::boolean(&acc, a, BooleanOp::Union)
                    .map_err(op_err)?
                    .unwrap_or(acc),
            });
        }
    }
    let mut plan = EditPlan::default();
    let (mut changed, mut removed) = (0, 0);
    for (i, item) in sel.items.iter().enumerate().take(n.saturating_sub(1)) {
        if let Some(path) = item.path() {
            // An open path keeps its parts outside the closed shapes above.
            let Some(above) = &area_above[i] else {
                continue;
            };
            let boundary = shape::boundary(above, frame);
            if shape::clip_path(path, &boundary, true, frame).is_empty() {
                continue; // Nothing of it is covered.
            }
            let outside = shape::clip_path(path, &boundary, false, frame);
            if outside.is_empty() {
                plan.removals.push(item.handle);
                removed += 1;
                continue;
            }
            let results = output::open_results(&item.entity, &item.entity, &outside, frame);
            plan.replacements.push(Replacement {
                handle: item.handle,
                results,
            });
            changed += 1;
            continue;
        }
        let Some(area) = &item.shape.area else {
            continue;
        };
        let (rest, covered) = match &area_above[i] {
            Some(above) if overlaps_bounds(area, above, frame) => {
                let covered = shape::boolean(area, above, BooleanOp::Intersect)
                    .map_err(op_err)?
                    .is_some();
                if covered {
                    (
                        shape::boolean(area, above, BooleanOp::Subtract).map_err(op_err)?,
                        true,
                    )
                } else {
                    (Some(area.clone()), false)
                }
            }
            _ => (Some(area.clone()), false),
        };
        let Some(rest) = rest else {
            // Fully covered: the object goes, and with it every hatch that has
            // no other boundary left.
            plan.removals.push(item.handle);
            for h in &item.hatches {
                release(h, sel, &mut plan);
            }
            removed += 1;
            continue;
        };
        let cut = shape::split(&rest, &knives_above[i], frame);
        if !covered && cut.is_none() {
            continue; // No overlap and no cut through it.
        }
        let result = cut.unwrap_or(rest);
        let faces = shape::faces(&result, frame);
        let hatches: Vec<&Hatch> = item.hatches.iter().collect();
        push_replacement(&mut plan, sel, item, &faces, &hatches);
        changed += 1;
    }
    plan.summary = if changed + removed == 0 {
        t("Clip Surface: the selected shapes do not overlap; nothing changed.")
    } else {
        crate::t!(
            "Clip Surface: %{changed} shape(s) clipped, %{removed} fully covered shape(s) removed.",
            changed = changed,
            removed = removed
        )
        .into_owned()
    };
    Ok(plan)
}

/// ADDSURFACE: merge each connected set of overlapping or edge-sharing
/// shapes into the bottommost one. An open shape takes part closed by the
/// line from its end to its start.
pub(crate) fn plan_add(sel: &Selection) -> Result<EditPlan, String> {
    let frame = &sel.frame;
    let members: Vec<usize> = (0..sel.items.len())
        .filter(|i| sel.items[*i].shape.area.is_some())
        .collect();
    let areas: Vec<&Brep> = members
        .iter()
        .map(|i| sel.items[*i].shape.area.as_ref().unwrap())
        .collect();
    let Some(merged) = shape::union_all(&areas).map_err(op_err)? else {
        return Ok(EditPlan {
            summary: nothing_added(),
            ..Default::default()
        });
    };
    // Each face of the union is one connected surface.
    let parts = shape::faces(&merged, frame);
    let part_regions: Vec<Option<Brep>> = parts.iter().map(|f| f.region(frame)).collect();
    let part_bounds: Vec<Option<[f64; 4]>> = part_regions
        .iter()
        .map(|r| r.as_ref().and_then(|r| shape::bounds(r, frame)))
        .collect();
    let n = sel.items.len();
    let mut uf: Vec<usize> = (0..n).collect();
    fn find(uf: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while uf[r] != r {
            r = uf[r];
        }
        uf[x] = r;
        r
    }
    let mut owner_of_part: Vec<Option<usize>> = vec![None; parts.len()];
    let mut parts_of: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (&i, area) in members.iter().zip(&areas) {
        let Some(b) = shape::bounds(area, frame) else {
            continue;
        };
        let near: Vec<usize> = (0..parts.len())
            .filter(|j| part_bounds[*j].is_some_and(|c| shape::bounds_overlap(b, c, frame.linear)))
            .collect();
        // Every shape lies inside the union: with one part near, it belongs
        // to that part, no Boolean needed.
        let single = near.len() == 1;
        for j in near {
            let inside = single
                || match &part_regions[j] {
                    Some(r) => shape::boolean(area, r, BooleanOp::Intersect)
                        .map_err(op_err)?
                        .is_some(),
                    None => false,
                };
            if !inside {
                continue;
            }
            parts_of[i].push(j);
            match owner_of_part[j] {
                None => owner_of_part[j] = Some(i),
                Some(o) => {
                    let (ra, rb) = (find(&mut uf, o), find(&mut uf, i));
                    uf[ra.max(rb)] = ra.min(rb);
                }
            }
        }
    }
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for &i in &members {
        let r = find(&mut uf, i);
        groups.entry(r).or_default().push(i);
    }
    let mut groups: Vec<Vec<usize>> = groups.into_values().filter(|g| g.len() >= 2).collect();
    for g in &mut groups {
        g.sort();
    }
    groups.sort();
    let mut plan = EditPlan::default();
    for g in &groups {
        // Items are bottom → top: the first is the bottommost.
        let source = g[0];
        let mut ids: Vec<usize> = g
            .iter()
            .flat_map(|i| parts_of[*i].iter().copied())
            .collect();
        ids.sort();
        ids.dedup();
        let faces: Vec<Face> = ids.iter().map(|j| parts[*j].clone()).collect();
        // The merged surface keeps the hatch of the bottommost member that
        // has one; the others lose that boundary.
        let keeper = g
            .iter()
            .copied()
            .find(|i| !sel.items[*i].hatches.is_empty());
        for &i in g.iter().filter(|i| Some(**i) != keeper) {
            for h in &sel.items[i].hatches {
                release(h, sel, &mut plan);
            }
        }
        let kept: Vec<&Hatch> = keeper
            .map(|k| sel.items[k].hatches.iter().collect())
            .unwrap_or_default();
        push_replacement(&mut plan, sel, &sel.items[source], &faces, &kept);
        plan.removals
            .extend(g[1..].iter().map(|i| sel.items[*i].handle));
    }
    plan.summary = if groups.is_empty() {
        nothing_added()
    } else {
        let merged: usize = groups.iter().map(Vec::len).sum();
        crate::t!(
            "Add Surface: %{merged} shapes merged into %{count} surface(s).",
            merged = merged,
            count = groups.len()
        )
        .into_owned()
    };
    Ok(plan)
}

fn nothing_added() -> String {
    t("Add Surface: the selected shapes do not overlap or share an edge; nothing changed.")
}

/// A piece of the selection's arrangement: an area and the closed shapes
/// covering it (indices into `Selection::items`, ascending = bottom to top).
struct Piece {
    region: Brep,
    bounds: [f64; 4],
    cover: Vec<usize>,
}

/// Split the closed shapes into pieces that do not overlap one another, each
/// covered by exactly the same set of shapes. Built shape by shape: every
/// piece splits into its part inside the new shape and its part outside, and
/// the new shape's area outside all earlier shapes is a piece of its own.
fn arrangement(sel: &Selection) -> Result<Vec<Piece>, String> {
    let frame = &sel.frame;
    let mut pieces: Vec<Piece> = Vec::new();
    let mut covered: Option<Brep> = None;
    for (i, item) in sel.items.iter().enumerate() {
        let Some(shape) = item.closed_area() else {
            continue;
        };
        let Some(sb) = shape::bounds(shape, frame) else {
            continue;
        };
        let mut next = Vec::with_capacity(pieces.len() * 2 + 1);
        for p in pieces {
            if !shape::bounds_overlap(p.bounds, sb, frame.linear) {
                next.push(p);
                continue;
            }
            let Some(inside) =
                shape::boolean(&p.region, shape, BooleanOp::Intersect).map_err(op_err)?
            else {
                next.push(p);
                continue;
            };
            if let Some(outside) =
                shape::boolean(&p.region, shape, BooleanOp::Subtract).map_err(op_err)?
            {
                let bounds = shape::bounds(&outside, frame).unwrap_or(p.bounds);
                next.push(Piece {
                    region: outside,
                    bounds,
                    cover: p.cover.clone(),
                });
            }
            let mut cover = p.cover;
            cover.push(i);
            let bounds = shape::bounds(&inside, frame).unwrap_or(sb);
            next.push(Piece {
                region: inside,
                bounds,
                cover,
            });
        }
        let only = match &covered {
            None => Some(shape.clone()),
            Some(c) => shape::boolean(shape, c, BooleanOp::Subtract).map_err(op_err)?,
        };
        if let Some(only) = only {
            let bounds = shape::bounds(&only, frame).unwrap_or(sb);
            next.push(Piece {
                region: only,
                bounds,
                cover: vec![i],
            });
        }
        covered = Some(match covered {
            None => shape.clone(),
            Some(c) => shape::boolean(&c, shape, BooleanOp::Union)
                .map_err(op_err)?
                .unwrap_or(c),
        });
        pieces = next;
    }
    Ok(pieces)
}

/// The parts of every selected open path inside the selected closed shapes,
/// as (path item, style item, piece). The style item is the bottommost of
/// the path itself and the closed shapes the piece lies in.
fn line_pieces(sel: &Selection) -> Result<Vec<(usize, usize, Vec<Oriented>)>, String> {
    let frame = &sel.frame;
    let closed: Vec<(usize, Vec<ocs_geom::geom2d::Curve>)> = sel
        .items
        .iter()
        .enumerate()
        .filter_map(|(i, it)| it.closed_area().map(|a| (i, shape::boundary(a, frame))))
        .collect();
    if closed.is_empty() {
        return Ok(Vec::new());
    }
    let areas: Vec<&Brep> = sel.items.iter().filter_map(Item::closed_area).collect();
    let Some(area) = shape::union_all(&areas).map_err(op_err)? else {
        return Ok(Vec::new());
    };
    let boundary = shape::boundary(&area, frame);
    let mut out = Vec::new();
    for (k, item) in sel.items.iter().enumerate() {
        let Some(path) = item.path() else { continue };
        for piece in shape::clip_path(path, &boundary, true, frame) {
            let mid = shape::chain_middle(&piece);
            let style = closed
                .iter()
                .filter(|(_, b)| shape::inside(b, mid, frame))
                .map(|(i, _)| *i)
                .chain(std::iter::once(k))
                .min()
                .unwrap_or(k);
            out.push((k, style, piece));
        }
    }
    Ok(out)
}

/// INTERSECTSURFACE: every area where two or more selected shapes overlap
/// becomes a new object, and so does every part of an open path inside a
/// closed shape. Overlap areas never stack: where A, B and C all overlap,
/// that spot is one piece (A∩B∩C), and the parts covered only by A and B,
/// or only by B and C, are pieces of their own.
///
/// Each result takes everything from the bottommost shape covering it:
/// layer, colour, linetype, lineweight, transparency, XDATA and its kind. It
/// is still a new object: no handle, extension dictionary or reactors.
pub(crate) fn plan_intersect(sel: &Selection) -> Result<EditPlan, String> {
    let mut pieces: Vec<Piece> = arrangement(sel)?
        .into_iter()
        .filter(|p| p.cover.len() >= 2)
        .collect();
    let lines = line_pieces(sel)?;
    if pieces.is_empty() && lines.is_empty() {
        return Err(t(NO_COMMON_SURFACE));
    }
    pieces.sort_by(|a, b| a.cover.cmp(&b.cover));
    let frame = &sel.frame;
    let mut plan = EditPlan::default();
    for p in &pieces {
        let template = &sel.items[p.cover[0]].entity;
        let faces = shape::faces(&p.region, frame);
        let results = output::results(template, &faces, frame);
        // A piece gets a new hatch like the bottommost covering object that
        // has one, following the piece's own outlines.
        let source = p.cover.iter().find_map(|i| sel.items[*i].hatches.first());
        if let (Some(h), true) = (source, results.iter().all(Out::is_outline)) {
            let first = plan.additions.len();
            let refs = (0..results.len())
                .map(|k| ResultRef::Addition(first + k))
                .collect();
            let mut update = follow(h, &faces, refs, sel, false);
            update.hatch.common = output::fresh_common(&EntityType::Hatch(update.hatch.clone()));
            plan.hatches.push(update);
        }
        plan.additions.extend(results);
    }
    for (source, style, piece) in &lines {
        let (kind, style) = (&sel.items[*source].entity, &sel.items[*style].entity);
        plan.additions.extend(output::open_results(
            kind,
            style,
            std::slice::from_ref(piece),
            frame,
        ));
    }
    plan.summary = crate::t!(
        "Intersect Surface: %{count} object(s) created from %{shapes} shapes.",
        count = plan.additions.len(),
        shapes = sel.items.len()
    )
    .into_owned();
    Ok(plan)
}

/// Plan `op` for `handles`. An `Err` is a user-facing message; the drawing
/// must then stay unchanged.
pub fn plan(op: Op, doc: &Document, handles: &[Handle]) -> Result<EditPlan, String> {
    let sel = validate(doc, handles)?;
    match op {
        Op::Add => plan_add(&sel),
        Op::Clip => plan_clip(&sel),
        Op::Intersect => plan_intersect(&sel),
    }
}

// ── Autocomplete registry ─────────────────────────────────
inventory::submit!(crate::command::CommandRegistration {
    names: &["ADDSURFACE", "CLIPSURFACE", "INTERSECTSURFACE"]
});

// ── Ribbon: Home → Modify ─────────────────────────────────
use crate::modules::IconKind;

pub const DROPDOWN_ID: &str = "surface_boolean";
pub const ICON: IconKind = IconKind::Svg(include_bytes!(
    "../../../../../assets/icons/model/union.svg"
));

pub const DROPDOWN_ITEMS: &[(&str, &str, IconKind)] = &[
    (
        "ADDSURFACE",
        "Add Surface",
        IconKind::Svg(include_bytes!(
            "../../../../../assets/icons/model/union.svg"
        )),
    ),
    (
        "CLIPSURFACE",
        "Clip Surface",
        IconKind::Svg(include_bytes!(
            "../../../../../assets/icons/model/subtract.svg"
        )),
    ),
    (
        "INTERSECTSURFACE",
        "Intersect Surface",
        IconKind::Svg(include_bytes!(
            "../../../../../assets/icons/model/intersect.svg"
        )),
    ),
];
