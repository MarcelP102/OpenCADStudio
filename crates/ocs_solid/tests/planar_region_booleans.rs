//! What the surface commands (ADDSURFACE, CLIPSURFACE, INTERSECTSURFACE)
//! rely on from Solidcore's planar region Booleans and face splitting: the
//! topology of the awkward cases (shared edges, touching corners, holes,
//! identical shapes) and exact curves in the results.

use ocs_geom::geom2d::{Circle, Curve, Ellipse, EllipseArc, Line};
use ocs_geom::space::Plane;
use ocs_solid::model::{self, BooleanOp, Brep};
use std::f64::consts::PI;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Brep {
    let p = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
    let loop_: Vec<Curve> = (0..4)
        .map(|i| Curve::Line(Line { start: p[i], end: p[(i + 1) % 4] }))
        .collect();
    model::planar_region(Plane::XY, &[loop_]).expect("rect region")
}

fn circle(x: f64, y: f64, r: f64) -> Brep {
    model::planar_region(Plane::XY, &[vec![Curve::Circle(Circle { centre: [x, y], radius: r })]])
        .expect("circle region")
}

fn ellipse(x: f64, y: f64, a: f64, b: f64) -> Brep {
    let e = Ellipse { centre: [x, y], major_radius: a, minor_radius: b, major_axis: [1.0, 0.0] };
    model::planar_region(Plane::XY, &[vec![Curve::Ellipse(EllipseArc::full(e))]]).expect("ellipse region")
}

/// Area of a loop whose curves follow one another but not necessarily in one
/// direction: sample each curve, flip it when its start does not meet the
/// previous end, shoelace the points.
fn loop_area(l: &[Curve]) -> f64 {
    let d = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    let mut pts: Vec<[f64; 2]> = Vec::new();
    for (i, c) in l.iter().enumerate() {
        let mut s: Vec<[f64; 2]> = (0..=64).map(|k| c.point_at(k as f64 / 64.0)).collect();
        let prev_end = if i == 0 {
            // Orient the first curve towards the second one.
            let next = &l[(i + 1) % l.len()];
            let ends = [next.point_at(0.0), next.point_at(1.0)];
            let tail = ends.iter().map(|e| d(s[64], *e)).fold(f64::MAX, f64::min);
            let head = ends.iter().map(|e| d(s[0], *e)).fold(f64::MAX, f64::min);
            if head < tail { s.reverse(); }
            None
        } else {
            pts.last().copied()
        };
        if let Some(pe) = prev_end {
            if d(s[64], pe) < d(s[0], pe) { s.reverse(); }
        }
        pts.extend(s);
    }
    let n = pts.len();
    (0..n).map(|i| pts[i][0] * pts[(i + 1) % n][1] - pts[(i + 1) % n][0] * pts[i][1]).sum::<f64>() / 2.0
}

/// (faces, area, loops per face, curve kinds) of a planar sheet.
fn describe(b: &Brep) -> (usize, f64, Vec<usize>, Vec<&'static str>) {
    let mut area = 0.0;
    let mut loops = Vec::new();
    let mut kinds = Vec::new();
    for f in 0..b.faces.len() as u32 {
        let Some(p) = model::planar_face_profile(b, f) else { continue };
        loops.push(p.loops.len());
        for (i, l) in p.loops.iter().enumerate() {
            let a = loop_area(l);
            area += if i == 0 { a.abs() } else { -a.abs() };
            for c in l {
                kinds.push(match c {
                    Curve::Line(_) => "line",
                    Curve::Arc(_) => "arc",
                    Curve::Circle(_) => "circle",
                    Curve::Ellipse(_) => "ellipse",
                    Curve::Polyline(_) => "polyline",
                    Curve::Nurbs(_) => "nurbs",
                    _ => "other",
                });
            }
        }
    }
    kinds.sort();
    kinds.dedup();
    (b.faces.len(), area, loops, kinds)
}

fn op(a: &Brep, b: &Brep, o: BooleanOp) -> Option<Brep> {
    match model::region_boolean(a, b, o) {
        Ok(r) => r,
        Err(e) => {
            println!("  ERROR {e:?}");
            None
        }
    }
}

/// (faces, area, loops per face) of a result, `None` when it is empty.
fn shape(r: Option<Brep>) -> Option<(usize, f64, Vec<usize>)> {
    r.map(|b| {
        let (f, a, l, _) = describe(&b);
        (f, a, l)
    })
}

fn assert_shape(r: Option<Brep>, faces: usize, area: f64, loops: &[usize]) {
    let (f, a, l) = shape(r).expect("a result");
    assert_eq!(f, faces, "faces");
    // Areas are measured by sampling the curves: allow for the chords.
    assert!((a - area).abs() <= 1e-3 * area.max(1.0), "area {a} vs {area}");
    assert_eq!(l, loops, "loops per face");
}

#[test]
fn awkward_cases_have_the_right_topology() {
    use BooleanOp::*;
    assert_shape(op(&rect(0., 0., 2., 2.), &rect(1., 1., 3., 3.), Union), 1, 7.0, &[1]);
    assert_shape(op(&rect(0., 0., 1., 1.), &rect(1., 0., 2., 1.), Union), 1, 2.0, &[1]);
    // Touching at a corner only: two faces, never one self-touching face.
    assert_shape(op(&rect(0., 0., 1., 1.), &rect(1., 1., 2., 2.), Union), 2, 2.0, &[1, 1]);
    assert_shape(op(&rect(0., 0., 10., 10.), &rect(4., 4., 6., 6.), Subtract), 1, 96.0, &[2]);
    assert_shape(op(&rect(0., 0., 10., 2.), &rect(4., -1., 6., 3.), Subtract), 2, 16.0, &[1, 1]);
    assert_shape(op(&rect(0., 0., 2., 2.), &rect(2., 0.5, 4., 1.5), Union), 1, 6.0, &[1]);
    // A shared edge or identical shapes leave no area.
    assert!(op(&rect(0., 0., 1., 1.), &rect(1., 0., 2., 1.), Intersect).is_none());
    assert!(op(&rect(0., 0., 4., 4.), &rect(0., 0., 4., 4.), Subtract).is_none());
    assert!(op(&circle(0., 0., 3.), &circle(0., 0., 3.), Subtract).is_none());
}

#[test]
fn curves_stay_exact() {
    use BooleanOp::*;
    let r = op(&rect(0., -10., 10., 10.), &circle(10., 0., 5.), Subtract).expect("rect - circle");
    let (_, area, _, kinds) = describe(&r);
    assert!((area - (200.0 - 12.5 * PI)).abs() < 0.05, "{area}");
    assert_eq!(kinds, vec!["arc", "line"]);
    let ring = op(&circle(0., 0., 5.), &circle(0., 0., 1.), Subtract).expect("ring");
    assert_eq!(describe(&ring).3, vec!["circle"]);
    let half = op(&ellipse(0., 0., 10., 5.), &rect(0., -10., 20., 10.), Intersect).expect("half ellipse");
    assert_eq!(describe(&half).3, vec!["ellipse", "line"]);
}

#[test]
fn many_shapes_in_reasonable_time() {
    let t = std::time::Instant::now();
    let mut acc = circle(0., 0., 1.);
    for k in 1..40 {
        acc = op(&acc, &circle(k as f64 * 1.5, 0., 1.), BooleanOp::Union).expect("chain union");
    }
    assert_eq!(describe(&acc).0, 1);
    assert!(t.elapsed().as_secs_f64() < 10.0, "{:?}", t.elapsed());
}

#[test]
fn faces_split_along_open_curves() {
    let r = rect(0., 0., 12., 10.);
    // The zigzag, trimmed to the rectangle: (3,0) → (5,4) → (8,4) → (9.5,10).
    let seg = |a: [f64; 2], b: [f64; 2]| {
        ocs_solid::sweep::profile_loops(&Plane::XY, &[vec![Curve::Line(Line { start: a, end: b })]])
            .unwrap()
            .remove(0)
            .0
            .remove(0)
    };
    let knife = [seg([3., 0.], [5., 4.]), seg([5., 4.], [8., 4.]), seg([8., 4.], [9.5, 10.])];
    let (b, regions) = model::split_face(&r, 0, &knife, 1e-6).expect("split");
    assert_eq!(regions.len(), 2);
    assert!((describe(&b).1 - 120.0).abs() < 1e-6);
    // Untrimmed: the zigzag sticks out at both ends.
    let long = [seg([2., -2.], [5., 4.]), seg([5., 4.], [8., 4.]), seg([8., 4.], [10., 12.])];
    assert_eq!(model::split_face(&r, 0, &long, 1e-6).expect("untrimmed split").1.len(), 2);
    // A line ending inside.
    // A line that ends inside cuts nothing: the command leaves the shape as it is.
    assert!(model::split_face(&r, 0, &[seg([0., 5.], [6., 5.])], 1e-6).is_err());
}
