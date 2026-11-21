//! Objects as exact planar regions and paths, for the surface commands.
//!
//! Every selected object is brought into one common plane (the OCS of the
//! bottommost object) as exact curves: closed shapes become a Solidcore
//! planar region (a B-rep sheet of faces), open shapes keep their path and,
//! for ADDSURFACE, also a region closed by the chord from end to start.
//! Results are read back from the regions' faces as loops of exact curves.

use document::entities::{BoundaryEdge, HatchStyleType};
use document::EntityType;
use ocs_geom::geom2d::vec::Vec2;
use ocs_geom::geom2d::{
    contains, inside_spans, Arc, Circle, Curve, EllipseArc, Line, Polyline, PolylineVertex,
    Tolerance, Transform,
};
use ocs_geom::space::Plane;
use ocs_solid::model::{self, BooleanOp, Brep};

/// The common plane of a selection, with its tolerances.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Frame {
    /// OCS plane of the bottommost object: 2D coordinates here are OCS
    /// coordinates, so polylines and circles are written without conversion.
    pub plane: Plane,
    pub normal: [f64; 3],
    pub elevation: f64,
    /// Coincidence distance in drawing units.
    pub linear: f64,
}

impl Frame {
    pub fn new(normal: [f64; 3], elevation: f64, extent: f64) -> Self {
        let plane = crate::entities::curve::ocs_plane(
            document::types::Vector3::new(normal[0], normal[1], normal[2]),
            elevation,
        );
        let linear = (extent * 1e-9).max(1e-9);
        Self {
            plane,
            normal,
            elevation,
            linear,
        }
    }

    pub fn tolerance(&self) -> Tolerance {
        Tolerance::new(self.linear.max(1e-9))
    }

    /// Distance within which a point counts as on this plane.
    fn plane_slack(&self) -> f64 {
        (self.linear * 100.0).max(1e-7)
    }
}

/// Why an object cannot take part.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Reject {
    Unsupported,
    Degenerate,
    NotPlanar,
}

/// One object in the common plane.
#[derive(Debug, Clone)]
pub(crate) struct Shape {
    /// Its area; for an open path, the area closed by the chord (if any).
    pub area: Option<Brep>,
    /// The path of an open object, as lines, arcs, elliptical arcs and NURBS
    /// pieces in order.
    pub path: Option<Vec<Curve>>,
}

/// The affine map from `from`'s 2D coordinates into `to`'s.
fn plane_map(from: &Plane, to: &Plane) -> Option<Transform> {
    let o = to.project(from.origin)?;
    let x = to.project_vector(from.x_axis)?;
    let y = to.project_vector(from.y_axis)?;
    Some(Transform {
        x_axis: Vec2::new(x[0], x[1]),
        y_axis: Vec2::new(y[0], y[1]),
        origin: Vec2::new(o[0], o[1]),
    })
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The normal of a plane, unit length.
pub(crate) fn plane_normal(p: &Plane) -> Option<[f64; 3]> {
    p.normal()
}

/// Curves on `plane` brought into the frame, after checking the plane is the
/// frame's.
fn into_frame(plane: &Plane, curves: Vec<Curve>, frame: &Frame) -> Result<Vec<Curve>, Reject> {
    let n = plane_normal(plane).ok_or(Reject::Degenerate)?;
    if dot(n, frame.normal).abs() < 1.0 - 1e-9 {
        return Err(Reject::NotPlanar);
    }
    let d = frame
        .plane
        .distance_to(plane.origin)
        .ok_or(Reject::Degenerate)?;
    if d.abs() > frame.plane_slack() {
        return Err(Reject::NotPlanar);
    }
    let map = plane_map(plane, &frame.plane).ok_or(Reject::Degenerate)?;
    curves
        .iter()
        .map(|c| c.transformed(&map).ok_or(Reject::Degenerate))
        .collect()
}

/// The plane an object lies in, to define the common plane; `None` for a
/// line, which lies in many.
pub(crate) fn object_plane(entity: &EntityType) -> Option<Plane> {
    match entity {
        EntityType::Line(_) => None,
        EntityType::Hatch(h) => Some(crate::entities::curve::ocs_plane(h.normal, h.elevation)),
        EntityType::Solid(s) => Some(crate::entities::curve::ocs_plane(
            s.normal,
            s.first_corner.z,
        )),
        EntityType::Region(_) => crate::scene::model::presspull_model::profile_geometry(entity)
            .map(|(plane, _, _)| plane),
        _ => crate::entities::curve::entity_curve(entity).map(|c| c.plane),
    }
}

fn start_of(c: &Curve) -> [f64; 2] {
    c.point_at(0.0)
}

fn end_of(c: &Curve) -> [f64; 2] {
    c.point_at(1.0)
}

fn close(a: [f64; 2], b: [f64; 2], tol: f64) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) <= tol
}

/// A region bounded by `loops` (outer first) in the frame.
pub(crate) fn region(frame: &Frame, loops: &[Vec<Curve>]) -> Option<Brep> {
    model::planar_region(frame.plane, loops)
}

/// `a` op `b`; `Ok(None)` when nothing is left.
pub(crate) fn boolean(a: &Brep, b: &Brep, op: BooleanOp) -> Result<Option<Brep>, String> {
    model::region_boolean(a, b, op).map_err(|e| format!("{e:?}"))
}

/// The union of all regions.
pub(crate) fn union_all(regions: &[&Brep]) -> Result<Option<Brep>, String> {
    let mut iter = regions.iter();
    let Some(first) = iter.next() else {
        return Ok(None);
    };
    let mut acc = (*first).clone();
    for r in iter {
        acc = match boolean(&acc, r, BooleanOp::Union)? {
            Some(u) => u,
            None => acc,
        };
    }
    Ok(Some(acc))
}

/// The symmetric difference, for hatch islands (even-odd).
fn xor(a: &Brep, b: &Brep) -> Result<Option<Brep>, String> {
    let left = boolean(a, b, BooleanOp::Subtract)?;
    let right = boolean(b, a, BooleanOp::Subtract)?;
    match (left, right) {
        (Some(l), Some(r)) => boolean(&l, &r, BooleanOp::Union),
        (l, r) => Ok(l.or(r)),
    }
}

/// An object as a shape in the frame.
pub(crate) fn shape_of(entity: &EntityType, frame: &Frame) -> Result<Shape, Reject> {
    match entity {
        EntityType::Hatch(h) => {
            if h.is_mpolygon {
                return Err(Reject::Unsupported);
            }
            let plane = crate::entities::curve::ocs_plane(h.normal, h.elevation);
            let mut loops: Vec<Vec<Curve>> = Vec::with_capacity(h.paths.len());
            for path in &h.paths {
                let curves: Vec<Curve> = path
                    .edges
                    .iter()
                    .map(|e| match e {
                        // A polyline edge stores its bulges in z; closed by nature.
                        BoundaryEdge::Polyline(pe) => Some(Curve::Polyline(Polyline {
                            vertices: pe
                                .vertices
                                .iter()
                                .map(|v| PolylineVertex {
                                    position: [v.x, v.y],
                                    bulge: v.z,
                                })
                                .collect(),
                            closed: true,
                        })),
                        other => crate::entities::hatch::edge_curve(other),
                    })
                    .collect::<Option<_>>()
                    .ok_or(Reject::Degenerate)?;
                loops.push(into_frame(&plane, curves, frame)?);
            }
            // The area as drawn: nesting depth decides which loops count
            // (Normal: all, alternating; Outer: the two outermost levels;
            // Ignore: the outermost only).
            let depth = |k: usize| {
                let p = loops[k].first().map(start_of);
                p.map_or(0, |p| {
                    (0..loops.len())
                        .filter(|&m| m != k && inside(&loops[m], p, frame))
                        .count()
                })
            };
            let max_depth = match h.style {
                HatchStyleType::Ignore => 0,
                HatchStyleType::Outer => 1,
                _ => usize::MAX,
            };
            let mut acc: Option<Brep> = None;
            for (k, l) in loops.iter().enumerate() {
                if depth(k) > max_depth {
                    continue;
                }
                let Some(loop_region) = region(frame, std::slice::from_ref(l)) else {
                    continue;
                };
                acc = Some(match acc {
                    None => loop_region,
                    Some(a) => xor(&a, &loop_region)
                        .map_err(|_| Reject::Degenerate)?
                        .ok_or(Reject::Degenerate)?,
                });
            }
            Ok(Shape {
                area: Some(acc.ok_or(Reject::Degenerate)?),
                path: None,
            })
        }
        EntityType::Solid(s) => {
            // SOLID corners are drawn 1-2-4-3.
            let c = [
                &s.first_corner,
                &s.second_corner,
                &s.fourth_corner,
                &s.third_corner,
            ];
            let mut vertices: Vec<PolylineVertex> = c
                .iter()
                .map(|p| PolylineVertex {
                    position: [p.x, p.y],
                    bulge: 0.0,
                })
                .collect();
            vertices.dedup_by(|a, b| close(a.position, b.position, frame.linear));
            let plane = crate::entities::curve::ocs_plane(s.normal, s.first_corner.z);
            let curve = Curve::Polyline(Polyline {
                vertices,
                closed: true,
            });
            let curves = into_frame(&plane, vec![curve], frame)?;
            Ok(Shape {
                area: Some(region(frame, &[curves]).ok_or(Reject::Degenerate)?),
                path: None,
            })
        }
        EntityType::Region(_) => {
            let (plane, loops, closed) =
                crate::scene::model::presspull_model::profile_geometry(entity)
                    .ok_or(Reject::Unsupported)?;
            if !closed {
                return Err(Reject::Degenerate);
            }
            let loops: Vec<Vec<Curve>> = loops
                .into_iter()
                .map(|l| into_frame(&plane, l, frame))
                .collect::<Result<_, _>>()?;
            Ok(Shape {
                area: Some(region(frame, &loops).ok_or(Reject::Degenerate)?),
                path: None,
            })
        }
        EntityType::Line(l) => {
            // A line lies in many planes: check its end points instead.
            let point = |v: &document::types::Vector3| {
                let w = [v.x, v.y, v.z];
                let d = frame.plane.distance_to(w).ok_or(Reject::Degenerate)?;
                if d.abs() > frame.plane_slack() {
                    return Err(Reject::NotPlanar);
                }
                frame.plane.project(w).ok_or(Reject::Degenerate)
            };
            let (a, b) = (point(&l.start)?, point(&l.end)?);
            if close(a, b, frame.linear) {
                return Err(Reject::Degenerate);
            }
            Ok(Shape {
                area: None,
                path: Some(vec![Curve::Line(Line { start: a, end: b })]),
            })
        }
        EntityType::Arc(_)
        | EntityType::Circle(_)
        | EntityType::Ellipse(_)
        | EntityType::LwPolyline(_)
        | EntityType::Polyline2D(_)
        | EntityType::Spline(_) => {
            if let EntityType::Polyline2D(p) = entity {
                if p.flags.bits() & 0b110 != 0 {
                    return Err(Reject::Unsupported); // curve- or spline-fit
                }
            }
            let planar = crate::entities::curve::entity_curve(entity).ok_or(Reject::NotPlanar)?;
            let mut curve = into_frame(&planar.plane, vec![planar.curve], frame)?.remove(0);
            // A polyline whose last vertex lies on the first is closed.
            if let Curve::Polyline(p) = &mut curve {
                let n = p.vertices.len();
                if !p.closed
                    && n >= 3
                    && close(
                        p.vertices[0].position,
                        p.vertices[n - 1].position,
                        frame.linear,
                    )
                {
                    p.vertices.pop();
                    p.closed = true;
                }
            }
            if curve.is_closed() {
                let area = region(frame, &[vec![curve]]).ok_or(Reject::Degenerate)?;
                return Ok(Shape {
                    area: Some(area),
                    path: None,
                });
            }
            let pieces = pieces_of(&curve);
            if pieces.is_empty() {
                return Err(Reject::Degenerate);
            }
            // Closed by the chord, for ADDSURFACE; a straight line has no area.
            let (s, e) = (start_of(&pieces[0]), end_of(&pieces[pieces.len() - 1]));
            let mut closed_loop = pieces.clone();
            if !close(s, e, frame.linear) {
                closed_loop.push(Curve::Line(Line { start: e, end: s }));
            }
            let area = region(frame, &[closed_loop]);
            Ok(Shape {
                area,
                path: Some(pieces),
            })
        }
        _ => Err(Reject::Unsupported),
    }
}

/// An open curve as pieces in travel order: a polyline taken apart into its
/// lines and arcs (each in the polyline's direction), anything else whole.
pub(crate) fn pieces_of(curve: &Curve) -> Vec<Curve> {
    let Curve::Polyline(p) = curve else {
        return vec![curve.clone()];
    };
    let n = p.vertices.len();
    let segments = if p.closed { n } else { n.saturating_sub(1) };
    (0..segments)
        .filter_map(|i| {
            let a = p.vertices[i];
            let b = p.vertices[(i + 1) % n].position;
            if close(a.position, b, 1e-12) {
                return None;
            }
            Some(if a.bulge.abs() < 1e-12 {
                Curve::Line(Line {
                    start: a.position,
                    end: b,
                })
            } else {
                bulge_arc(a.position, b, a.bulge)
            })
        })
        .collect()
}

/// The arc from `a` to `b` with `bulge`. Arcs in ocs_geom run
/// counter-clockwise, so a clockwise one comes back reversed; callers that
/// need the travel direction orient pieces by their end points.
fn bulge_arc(a: [f64; 2], b: [f64; 2], bulge: f64) -> Curve {
    match ocs_geom::geom2d::BulgeArc::from_bulge(a, b, bulge) {
        Some(arc) => {
            let (from, to) = if arc.sweep >= 0.0 {
                (arc.start_angle, arc.start_angle + arc.sweep)
            } else {
                (arc.end_angle, arc.end_angle - arc.sweep)
            };
            Curve::Arc(Arc {
                centre: arc.center,
                radius: arc.radius,
                start_angle: from,
                end_angle: to,
            })
        }
        None => Curve::Line(Line { start: a, end: b }),
    }
}

/// A curve and whether it runs forward (start → end) in its loop or path.
pub(crate) type Oriented = (Curve, bool);

pub(crate) fn oriented_start(o: &Oriented) -> [f64; 2] {
    if o.1 {
        start_of(&o.0)
    } else {
        end_of(&o.0)
    }
}

pub(crate) fn oriented_end(o: &Oriented) -> [f64; 2] {
    if o.1 {
        end_of(&o.0)
    } else {
        start_of(&o.0)
    }
}

/// Give a chain of curves that follow one another (in order, any direction)
/// a consistent travel direction.
pub(crate) fn orient(chain: Vec<Curve>) -> Vec<Oriented> {
    let d = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    let n = chain.len();
    if n == 1 {
        return vec![(chain.into_iter().next().unwrap(), true)];
    }
    let mut out: Vec<Oriented> = Vec::with_capacity(n);
    for (i, c) in chain.iter().enumerate() {
        let forward = if i == 0 {
            // Towards whichever end of the next curve is nearer.
            let next = &chain[1];
            let ends = [start_of(next), end_of(next)];
            let tail = ends
                .iter()
                .map(|e| d(end_of(c), *e))
                .fold(f64::MAX, f64::min);
            let head = ends
                .iter()
                .map(|e| d(start_of(c), *e))
                .fold(f64::MAX, f64::min);
            tail <= head
        } else {
            let prev = oriented_end(&out[i - 1]);
            d(start_of(c), prev) <= d(end_of(c), prev)
        };
        out.push((c.clone(), forward));
    }
    out
}

/// `b` continuing `a` along the same circle or line, as one curve. A kernel
/// edge can end at a seam (a circle's parameter start) where the outline
/// has no corner; joining them keeps a circle a two-bulge polyline.
fn merge_pair(a: &Oriented, b: &Oriented, tol: f64) -> Option<Oriented> {
    if a.1 != b.1 || !close(oriented_end(a), oriented_start(b), tol) {
        return None;
    }
    match (&a.0, &b.0) {
        (Curve::Arc(x), Curve::Arc(y))
            if close(x.centre, y.centre, tol) && (x.radius - y.radius).abs() <= tol =>
        {
            let sweep = x.sweep() + y.sweep();
            if sweep >= std::f64::consts::TAU - 1e-9 {
                return None;
            }
            // Arcs run counter-clockwise: the merged one starts where the
            // first of the two (in angle) starts.
            let first = if a.1 { x } else { y };
            Some((
                Curve::Arc(Arc {
                    centre: first.centre,
                    radius: first.radius,
                    start_angle: first.start_angle,
                    end_angle: first.start_angle + sweep,
                }),
                a.1,
            ))
        }
        (Curve::Line(_), Curve::Line(_)) => {
            let (s, m, e) = (oriented_start(a), oriented_end(a), oriented_end(b));
            let cross = (m[0] - s[0]) * (e[1] - s[1]) - (m[1] - s[1]) * (e[0] - s[0]);
            let len = (e[0] - s[0]).hypot(e[1] - s[1]);
            let ahead = (m[0] - s[0]) * (e[0] - m[0]) + (m[1] - s[1]) * (e[1] - m[1]) > 0.0;
            (len > tol && cross.abs() <= tol * len && ahead)
                .then_some((Curve::Line(Line { start: s, end: e }), true))
        }
        _ => None,
    }
}

/// A closed loop with contiguous pieces of one circle or line joined,
/// across the loop's start too.
fn merge_loop(mut chain: Vec<Oriented>, tol: f64) -> Vec<Oriented> {
    let mut i = 0;
    while chain.len() > 1 && i < chain.len() {
        let j = (i + 1) % chain.len();
        if let Some(m) = merge_pair(&chain[i], &chain[j], tol) {
            chain[i] = m;
            chain.remove(j);
            if j < i {
                i -= 1;
            }
        } else {
            i += 1;
        }
    }
    chain
}

/// One face of a region in the frame: its outer loop, then its holes.
#[derive(Debug, Clone)]
pub(crate) struct Face {
    pub loops: Vec<Vec<Oriented>>,
}

impl Face {
    /// All boundary curves, for containment and clipping.
    pub fn curves(&self) -> Vec<Curve> {
        self.loops
            .iter()
            .flatten()
            .map(|(c, _)| c.clone())
            .collect()
    }

    /// The face as its own region.
    pub fn region(&self, frame: &Frame) -> Option<Brep> {
        let loops: Vec<Vec<Curve>> = self
            .loops
            .iter()
            .map(|l| l.iter().map(|(c, _)| c.clone()).collect())
            .collect();
        region(frame, &loops)
    }
}

/// The faces of `b` with their loops in frame coordinates.
pub(crate) fn faces(b: &Brep, frame: &Frame) -> Vec<Face> {
    (0..b.faces.len() as u32)
        .filter_map(|f| {
            let profile = model::planar_face_profile(b, f)?;
            let map = plane_map(&profile.plane, &frame.plane)?;
            let tol = frame.linear * 1e3;
            let loops = profile
                .loops
                .into_iter()
                .map(|l| {
                    l.iter()
                        .map(|c| c.transformed(&map))
                        .collect::<Option<Vec<_>>>()
                        .map(|chain| merge_loop(orient(chain), tol))
                })
                .collect::<Option<Vec<_>>>()?;
            Some(Face { loops })
        })
        .collect()
}

/// All boundary curves of `b`, for containment and clipping.
pub(crate) fn boundary(b: &Brep, frame: &Frame) -> Vec<Curve> {
    faces(b, frame).iter().flat_map(Face::curves).collect()
}

/// Whether `p` lies inside the region bounded by `curves`.
pub(crate) fn inside(curves: &[Curve], p: [f64; 2], frame: &Frame) -> bool {
    !curves.is_empty() && contains(curves, p, frame.tolerance())
}

/// The part of `c` between normalised parameters `a` and `b` (a < b).
pub(crate) fn between(c: &Curve, a: f64, b: f64) -> Option<Curve> {
    if b - a <= 1e-12 {
        return None;
    }
    Some(match c {
        Curve::Line(l) => {
            let at = |t: f64| {
                [
                    l.start[0] + (l.end[0] - l.start[0]) * t,
                    l.start[1] + (l.end[1] - l.start[1]) * t,
                ]
            };
            Curve::Line(Line {
                start: at(a),
                end: at(b),
            })
        }
        Curve::Arc(arc) => {
            let s = arc.sweep();
            Curve::Arc(Arc {
                centre: arc.centre,
                radius: arc.radius,
                start_angle: arc.start_angle + s * a,
                end_angle: arc.start_angle + s * b,
            })
        }
        Curve::Circle(Circle { centre, radius }) => Curve::Arc(Arc {
            centre: *centre,
            radius: *radius,
            start_angle: std::f64::consts::TAU * a,
            end_angle: std::f64::consts::TAU * b,
        }),
        Curve::Ellipse(e) => {
            let s = e.sweep();
            Curve::Ellipse(EllipseArc {
                ellipse: e.ellipse,
                start_parameter: e.start_parameter + s * a,
                end_parameter: e.start_parameter + s * b,
            })
        }
        Curve::Nurbs(n) => Curve::Nurbs(n.trimmed(a, b)?),
        _ => return None,
    })
}

/// The parts of the open `path` inside (`want_inside`) or outside the region
/// bounded by `boundary`, as chains of oriented pieces.
pub(crate) fn clip_path(
    path: &[Curve],
    boundary: &[Curve],
    want_inside: bool,
    frame: &Frame,
) -> Vec<Vec<Oriented>> {
    let tol = frame.tolerance();
    let oriented = orient(path.to_vec());
    let mut chains: Vec<Vec<Oriented>> = Vec::new();
    let mut current: Vec<Oriented> = Vec::new();
    for (piece, forward) in oriented {
        let inner = if boundary.is_empty() {
            Vec::new()
        } else {
            inside_spans(boundary, &piece, tol)
        };
        // Spans of the piece with whether they are inside, in parameter order.
        let mut spans: Vec<([f64; 2], bool)> = Vec::new();
        let mut t = 0.0;
        for [a, b] in inner {
            if a > t + 1e-12 {
                spans.push(([t, a], false));
            }
            spans.push(([a, b], true));
            t = b;
        }
        if t < 1.0 - 1e-12 {
            spans.push(([t, 1.0], false));
        }
        if !forward {
            spans.reverse();
        }
        for ([a, b], is_inside) in spans {
            let keep = is_inside == want_inside;
            match (keep, between(&piece, a, b)) {
                (true, Some(part)) => {
                    let o = (part, forward);
                    let joins = current.last().is_some_and(|last| {
                        close(oriented_end(last), oriented_start(&o), frame.linear * 1e3)
                    });
                    if !joins && !current.is_empty() {
                        chains.push(std::mem::take(&mut current));
                    }
                    current.push(o);
                }
                _ => {
                    if !current.is_empty() {
                        chains.push(std::mem::take(&mut current));
                    }
                }
            }
        }
    }
    if !current.is_empty() {
        chains.push(current);
    }
    // Drop numerical crumbs.
    chains.retain(|c| c.iter().map(|(p, _)| p.length()).sum::<f64>() > frame.linear * 1e3);
    chains
}

/// A point in the middle of a chain, for "which shapes is this inside".
pub(crate) fn chain_middle(chain: &[Oriented]) -> [f64; 2] {
    let (c, _) = &chain[chain.len() / 2];
    c.point_at(0.5)
}

/// Split `area` along the open `knives`. Returns `None` when no knife cuts
/// through any face (the area stays as it is).
pub(crate) fn split(area: &Brep, knives: &[Vec<Curve>], frame: &Frame) -> Option<Brep> {
    if knives.is_empty() {
        return None;
    }
    let sweep: Vec<_> = knives
        .iter()
        .filter_map(|k| ocs_solid::sweep::profile_loops(&frame.plane, std::slice::from_ref(k)))
        .flat_map(|chains| chains.into_iter().flat_map(|(curves, _)| curves))
        .collect();
    if sweep.is_empty() {
        return None;
    }
    let tolerance = (frame.linear * 1e3).clamp(1e-9, 1.0);
    let mut acc = area.clone();
    let mut cut = false;
    // Each successful split adds faces; a face a knife does not cross (or
    // already cut) answers with an error and is skipped.
    let mut face = 0u32;
    let mut guard = 0;
    while (face as usize) < acc.faces.len() && guard < 1000 {
        guard += 1;
        match model::split_face(&acc, face, &sweep, tolerance) {
            Ok((next, regions)) if regions.len() > 1 && next.faces.len() > acc.faces.len() => {
                acc = next;
                cut = true;
            }
            _ => face += 1,
        }
    }
    cut.then_some(acc)
}

/// Bounding box of a region in the frame, `[x0, y0, x1, y1]`.
pub(crate) fn bounds(b: &Brep, frame: &Frame) -> Option<[f64; 4]> {
    let mut out = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for c in boundary(b, frame) {
        for k in 0..=16 {
            let p = c.point_at(k as f64 / 16.0);
            out[0] = out[0].min(p[0]);
            out[1] = out[1].min(p[1]);
            out[2] = out[2].max(p[0]);
            out[3] = out[3].max(p[1]);
        }
    }
    if !out[0].is_finite() {
        return None;
    }
    // Samples can fall short of a curve's true extreme (by up to ~2% of an
    // arc's radius at this density): pad so the box always contains it. A
    // box too large only costs a Boolean; one too small would miss an overlap.
    let pad = 0.03 * (out[2] - out[0]).max(out[3] - out[1]) + frame.linear;
    Some([out[0] - pad, out[1] - pad, out[2] + pad, out[3] + pad])
}

pub(crate) fn bounds_overlap(a: [f64; 4], b: [f64; 4], eps: f64) -> bool {
    a[0] <= b[2] + eps && b[0] <= a[2] + eps && a[1] <= b[3] + eps && b[1] <= a[3] + eps
}
