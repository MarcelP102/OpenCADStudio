//! Results written back as objects, styled like a template object.
//!
//! - Loops of lines and arcs become closed polylines (a circle stays a
//!   circle where the source was one). Holes are polylines of their own.
//! - A face with an elliptical or spline edge becomes one REGION: exact, with
//!   its holes, where a polyline cannot follow the curve.
//! - A hatch template gives one hatch holding every loop, with exact edges.
//! - Open pieces become a line (a straight piece of a line), an arc (a piece
//!   of an arc) or an open polyline, and an ellipse or spline piece its own
//!   ellipse or spline.

use super::shape::{oriented_end, oriented_start, Face, Frame, Oriented};
use document::entities::{
    Arc as ArcEnt, BoundaryEdge, BoundaryPath, BoundaryPathFlags, Circle as CircleEnt,
    Ellipse as EllipseEnt, EntityCommon, Hatch, Line as LineEnt, LwPolyline, LwVertex,
    PolylineEdge, Region, Spline as SplineEnt,
};
use document::types::{Vector2, Vector3};
use document::{EntityType, Handle};
use ocs_geom::geom2d::Curve;
use ocs_solid::model::Brep;

/// A result: an object ready to add, or a REGION that the app encodes.
#[derive(Debug, Clone)]
pub enum Out {
    Entity(EntityType),
    Region { region: Region, body: Brep },
}

impl Out {
    pub fn common_mut(&mut self) -> &mut EntityCommon {
        match self {
            Out::Entity(e) => e.common_mut(),
            Out::Region { region, .. } => &mut region.common,
        }
    }

    /// One closed outline (a polyline or circle), which an associative hatch
    /// can follow.
    pub fn is_outline(&self) -> bool {
        matches!(
            self,
            Out::Entity(EntityType::LwPolyline(_) | EntityType::Circle(_))
        )
    }
}

/// The template's common data for a new object: appearance, layer, owner and
/// XDATA; never its handle, extension dictionary or reactors.
pub fn fresh_common(template: &EntityType) -> EntityCommon {
    fresh(template.common())
}

/// `common` for a new object: no handle, extension dictionary or reactors.
pub fn fresh(common: &EntityCommon) -> EntityCommon {
    let mut c = common.clone();
    c.handle = Handle::NULL;
    c.xdictionary_handle = None;
    c.reactors.clear();
    c
}

fn v3(p: [f64; 3]) -> Vector3 {
    Vector3::new(p[0], p[1], p[2])
}

/// Bulge of an oriented line or arc piece; `None` for anything else.
fn bulge(o: &Oriented) -> Option<f64> {
    match &o.0 {
        Curve::Line(_) => Some(0.0),
        Curve::Arc(a) => {
            let b = (a.sweep() / 4.0).tan();
            Some(if o.1 { b } else { -b })
        }
        _ => None,
    }
}

/// A loop or chain of lines and arcs as polyline vertices (each vertex with
/// the bulge of the segment it starts); `None` when it has other curves.
fn bulge_vertices(chain: &[Oriented], closed: bool) -> Option<Vec<(Vector2, f64)>> {
    // A full circle as a loop: two half arcs.
    if let [(Curve::Circle(c), forward)] = chain {
        let b = if *forward { 1.0 } else { -1.0 };
        return Some(vec![
            (Vector2::new(c.centre[0] + c.radius, c.centre[1]), b),
            (Vector2::new(c.centre[0] - c.radius, c.centre[1]), b),
        ]);
    }
    let mut out = Vec::with_capacity(chain.len() + 1);
    for o in chain {
        let s = oriented_start(o);
        out.push((Vector2::new(s[0], s[1]), bulge(o)?));
    }
    if !closed {
        let e = oriented_end(chain.last()?);
        out.push((Vector2::new(e[0], e[1]), 0.0));
    }
    Some(out)
}

/// A polyline like `base` (its width) in the style of `style`.
fn polyline(
    base: &EntityType,
    style: &EntityType,
    vertices: Vec<(Vector2, f64)>,
    closed: bool,
    frame: &Frame,
) -> EntityType {
    let mut pl = match base {
        EntityType::LwPolyline(src) => src.clone(),
        _ => LwPolyline::new(),
    };
    pl.common = fresh_common(style);
    let width = pl.constant_width;
    pl.vertices = vertices
        .into_iter()
        .map(|(p, b)| {
            let mut v = LwVertex::new(p);
            v.bulge = b;
            v.start_width = width;
            v.end_width = width;
            v
        })
        .collect();
    pl.is_closed = closed;
    pl.elevation = frame.elevation;
    pl.normal = v3(frame.normal);
    EntityType::LwPolyline(pl)
}

/// A REGION in the template's style, for a face a polyline cannot follow.
fn region_out(template: &EntityType, face: &Face, frame: &Frame) -> Option<Out> {
    let body = face.region(frame)?;
    let mut region = Region::new();
    region.common = fresh_common(template);
    region.point_of_reference = v3(frame.plane.origin);
    Some(Out::Region { region, body })
}

/// Outline results for `faces`, styled like `template`: per face, one object
/// per loop (outer first, then its holes), or one REGION when a loop has an
/// elliptical or spline edge.
pub fn outline_results(template: &EntityType, faces: &[Face], frame: &Frame) -> Vec<Out> {
    let mut out = Vec::new();
    for face in faces {
        let loops: Option<Vec<Vec<(Vector2, f64)>>> =
            face.loops.iter().map(|l| bulge_vertices(l, true)).collect();
        match loops {
            Some(loops) => {
                for (l, vertices) in face.loops.iter().zip(loops) {
                    // A circle stays a circle where the source was one.
                    if let (EntityType::Circle(src), [(Curve::Circle(c), _)]) =
                        (template, l.as_slice())
                    {
                        let mut circle: CircleEnt = src.clone();
                        circle.common = fresh_common(template);
                        circle.center = Vector3::new(c.centre[0], c.centre[1], frame.elevation);
                        circle.radius = c.radius;
                        circle.normal = v3(frame.normal);
                        out.push(Out::Entity(EntityType::Circle(circle)));
                    } else {
                        out.push(Out::Entity(polyline(
                            template, template, vertices, true, frame,
                        )));
                    }
                }
            }
            None => out.extend(region_out(template, face, frame)),
        }
    }
    out
}

/// A hatch boundary path for one loop: a polyline edge (bulges in z) for
/// lines and arcs, exact edges otherwise.
pub fn hatch_path(l: &[Oriented], outer: bool) -> BoundaryPath {
    let mut path = BoundaryPath::new();
    let mut bits = BoundaryPathFlags::EXTERNAL.bits();
    if outer {
        bits |= BoundaryPathFlags::OUTERMOST.bits();
    }
    match bulge_vertices(l, true) {
        Some(vertices) => {
            path.flags = BoundaryPathFlags::from_bits(bits | BoundaryPathFlags::POLYLINE.bits());
            let mut pe = PolylineEdge::new(Vec::new(), true);
            for (p, b) in at_least_three(vertices) {
                pe.add_vertex(p, b);
            }
            path.edges = vec![BoundaryEdge::Polyline(pe)];
        }
        None => {
            path.flags = BoundaryPathFlags::from_bits(bits);
            path.edges = l
                .iter()
                .map(|o| {
                    let (s, e) = (oriented_start(o), oriented_end(o));
                    // A point just along the travel direction tells which way.
                    let next = o.0.point_at(if o.1 { 0.01 } else { 0.99 });
                    crate::scene::exact_boundary_edge(Some(&o.0), s, e, next, o.0.is_closed())
                })
                .collect();
        }
    }
    path
}

/// A hatch polyline path needs three vertices or more; a circle has two.
/// Split the most curved segment at its midpoint until there are three.
fn at_least_three(mut v: Vec<(Vector2, f64)>) -> Vec<(Vector2, f64)> {
    while v.len() < 3 && !v.is_empty() {
        let n = v.len();
        let Some(i) = (0..n).max_by(|&a, &b| v[a].1.abs().total_cmp(&v[b].1.abs())) else {
            break;
        };
        let (a, bulge) = v[i];
        let b = v[(i + 1) % n].0;
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = dx.hypot(dy);
        if bulge.abs() < 1e-12 || len < 1e-15 {
            break;
        }
        let sagitta = bulge * len / 2.0;
        let mid = Vector2::new(
            (a.x + b.x) / 2.0 + sagitta * dy / len,
            (a.y + b.y) / 2.0 - sagitta * dx / len,
        );
        let half = (bulge.atan() / 2.0).tan();
        v[i].1 = half;
        v.insert(i + 1, (mid, half));
    }
    v
}

/// One hatch holding every loop of `faces`, styled like the hatch `src`.
pub fn hatch_result(src: &Hatch, template: &EntityType, faces: &[Face], frame: &Frame) -> Out {
    let mut h = src.clone();
    h.common = fresh_common(template);
    h.paths = faces
        .iter()
        .flat_map(|f| {
            f.loops
                .iter()
                .enumerate()
                .map(|(k, l)| hatch_path(l, k == 0))
        })
        .collect();
    h.normal = v3(frame.normal);
    h.elevation = frame.elevation;
    h.is_associative = false;
    h.seed_points.clear();
    normal_style_for_holes(&mut h, faces);
    Out::Entity(EntityType::Hatch(Box::new(h)))
}

/// The result's paths are exactly its area, read even-odd: an Ignore or
/// Outer style would fill a hole the operation made, so a result with holes
/// draws in Normal style.
pub fn normal_style_for_holes(h: &mut Hatch, faces: &[Face]) {
    if faces.iter().any(|f| f.loops.len() > 1) {
        h.style = document::entities::HatchStyleType::Normal;
    }
}

/// Results for `faces` of the template's kind: a hatch stays one hatch, a
/// REGION one REGION per face, a filled 2D SOLID (which holds four corners
/// at most) a solid-fill hatch; anything else gives outlines.
pub fn results(template: &EntityType, faces: &[Face], frame: &Frame) -> Vec<Out> {
    match template {
        EntityType::Hatch(h) => vec![hatch_result(h, template, faces, frame)],
        EntityType::Region(_) => faces
            .iter()
            .filter_map(|f| region_out(template, f, frame))
            .collect(),
        EntityType::Solid(_) => vec![hatch_result(&Hatch::solid(), template, faces, frame)],
        _ => outline_results(template, faces, frame),
    }
}

/// Open results for `chains`: their kind follows `kind` (the open source),
/// their style follows `style`.
pub fn open_results(
    kind: &EntityType,
    style: &EntityType,
    chains: &[Vec<Oriented>],
    frame: &Frame,
) -> Vec<Out> {
    let world = |p: [f64; 2]| v3(frame.plane.point_at(p));
    let mut out = Vec::new();
    for chain in chains {
        // A straight piece of a line stays a line; a piece of an arc an arc.
        if let ([(Curve::Line(_), _)], EntityType::Line(src)) = (chain.as_slice(), kind) {
            let mut l: LineEnt = src.clone();
            l.common = fresh_common(style);
            l.start = world(oriented_start(&chain[0]));
            l.end = world(oriented_end(&chain[0]));
            out.push(Out::Entity(EntityType::Line(l)));
            continue;
        }
        if let ([(Curve::Arc(a), _)], EntityType::Arc(src)) = (chain.as_slice(), kind) {
            let mut arc: ArcEnt = src.clone();
            arc.common = fresh_common(style);
            arc.center = Vector3::new(a.centre[0], a.centre[1], frame.elevation);
            arc.radius = a.radius;
            arc.start_angle = a.start_angle.rem_euclid(std::f64::consts::TAU);
            arc.end_angle = a.end_angle.rem_euclid(std::f64::consts::TAU);
            arc.normal = v3(frame.normal);
            out.push(Out::Entity(EntityType::Arc(arc)));
            continue;
        }
        if let Some(vertices) = bulge_vertices(chain, false) {
            out.push(Out::Entity(polyline(kind, style, vertices, false, frame)));
            continue;
        }
        // Ellipse or spline pieces: each its own exact object, lines and
        // arcs between them as a polyline.
        let mut run: Vec<Oriented> = Vec::new();
        let flush = |run: &mut Vec<Oriented>, out: &mut Vec<Out>| {
            if let Some(v) = bulge_vertices(run, false) {
                if !run.is_empty() {
                    out.push(Out::Entity(polyline(kind, style, v, false, frame)));
                }
            }
            run.clear();
        };
        for o in chain {
            match &o.0 {
                Curve::Ellipse(e) => {
                    flush(&mut run, &mut out);
                    let mut ent = EllipseEnt::new();
                    ent.common = fresh_common(style);
                    ent.center = world(e.ellipse.centre);
                    ent.major_axis = v3(frame.plane.vector_at([
                        e.ellipse.major_axis[0] * e.ellipse.major_radius,
                        e.ellipse.major_axis[1] * e.ellipse.major_radius,
                    ]));
                    ent.minor_axis_ratio = e.ellipse.minor_radius / e.ellipse.major_radius;
                    ent.start_parameter = e.start_parameter;
                    ent.end_parameter = e.end_parameter;
                    ent.normal = v3(frame.normal);
                    out.push(Out::Entity(EntityType::Ellipse(ent)));
                }
                Curve::Nurbs(n) => {
                    flush(&mut run, &mut out);
                    let mut sp = SplineEnt::new();
                    sp.common = fresh_common(style);
                    sp.degree = n.degree() as i32;
                    sp.knots = n.knots().to_vec();
                    sp.control_points = n.control_points().iter().map(|p| world(*p)).collect();
                    if n.is_rational() {
                        sp.weights = n.weights().to_vec();
                    }
                    sp.flags.rational = n.is_rational();
                    sp.normal = v3(frame.normal);
                    out.push(Out::Entity(EntityType::Spline(sp)));
                }
                _ => run.push(o.clone()),
            }
        }
        flush(&mut run, &mut out);
    }
    out
}
