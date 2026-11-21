//! The planners on an in-memory drawing: the feature request's examples.

use super::*;
use document::entities::{
    BoundaryEdge, BoundaryPath, Circle, Ellipse, Hatch, Line, LwPolyline, LwVertex, PolylineEdge,
};
use document::tables::Layer;
use document::types::{Color, Vector2, Vector3};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64, color: i16) -> EntityType {
    let mut p = LwPolyline::new();
    for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
        p.vertices.push(LwVertex::new(Vector2::new(x, y)));
    }
    p.is_closed = true;
    p.common.color = Color::from_index(color);
    EntityType::LwPolyline(p)
}

fn circle(x: f64, y: f64, r: f64) -> EntityType {
    let mut c = Circle::new();
    c.center = Vector3::new(x, y, 0.0);
    c.radius = r;
    EntityType::Circle(c)
}

fn line(x0: f64, y0: f64, x1: f64, y1: f64) -> EntityType {
    let mut l = Line::new();
    l.start = Vector3::new(x0, y0, 0.);
    l.end = Vector3::new(x1, y1, 0.);
    EntityType::Line(l)
}

/// The zigzag of the reported example: it enters the rectangle at the
/// bottom, runs across and leaves at the top, sticking out at both ends.
fn zigzag(color: i16) -> EntityType {
    let mut p = LwPolyline::new();
    for (x, y) in [(2., -2.), (5., 4.), (8., 4.), (10., 12.)] {
        p.vertices.push(LwVertex::new(Vector2::new(x, y)));
    }
    p.common.color = Color::from_index(color);
    EntityType::LwPolyline(p)
}

fn add(doc: &mut Document, e: EntityType) -> Handle {
    doc.add_entity(e).expect("add entity")
}

fn entity(o: &Out) -> &EntityType {
    match o {
        Out::Entity(e) => e,
        Out::Region { .. } => panic!("unexpected REGION"),
    }
}

/// Enclosed area of a closed outline (polyline with bulges, or circle).
fn area(o: &Out) -> f64 {
    match entity(o) {
        EntityType::LwPolyline(p) => {
            let n = p.vertices.len();
            let mut a = 0.0;
            for k in 0..n {
                let (v, w) = (&p.vertices[k], &p.vertices[(k + 1) % n]);
                let (x0, y0, x1, y1) = (v.location.x, v.location.y, w.location.x, w.location.y);
                a += (x0 * y1 - x1 * y0) / 2.0;
                if v.bulge != 0.0 {
                    let c = (x1 - x0).hypot(y1 - y0);
                    let theta = 4.0 * v.bulge.atan();
                    let r = c / (2.0 * (theta / 2.0).sin());
                    a += r * r / 2.0 * (theta - theta.sin());
                }
            }
            a.abs()
        }
        EntityType::Circle(c) => std::f64::consts::PI * c.radius * c.radius,
        other => panic!("no area for {other:?}"),
    }
}

fn color(o: &Out) -> Color {
    entity(o).common().color
}

fn sorted_areas(outs: &[Out]) -> Vec<f64> {
    let mut v: Vec<f64> = outs.iter().map(|o| (area(o) * 1e6).round() / 1e6).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v
}

fn rect_path(x0: f64, y0: f64, x1: f64, y1: f64, bound_to: Option<Handle>) -> BoundaryPath {
    let mut path = BoundaryPath::new();
    let mut pe = PolylineEdge::new(Vec::new(), true);
    for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
        pe.add_vertex(Vector2::new(x, y), 0.0);
    }
    path.edges.push(BoundaryEdge::Polyline(pe));
    if let Some(h) = bound_to {
        path.boundary_handles = vec![h];
    }
    path
}

/// A rectangle with an associative solid hatch following it.
fn hatched_rect(
    doc: &mut Document,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    color: i16,
) -> (Handle, Handle) {
    let r = add(doc, rect(x0, y0, x1, y1, color));
    let mut h = Hatch::solid();
    h.common.color = Color::from_index(color);
    h.paths.push(rect_path(x0, y0, x1, y1, Some(r)));
    h.is_associative = true;
    let hh = add(doc, EntityType::Hatch(Box::new(h)));
    (r, hh)
}

fn solid_hatch(x0: f64, y0: f64, x1: f64, y1: f64) -> Hatch {
    let mut h = Hatch::solid();
    h.paths.push(rect_path(x0, y0, x1, y1, None));
    h
}

#[test]
fn clip_follows_drawing_order_not_selection_order() {
    let mut doc = Document::new();
    let a = add(&mut doc, rect(0., 0., 10., 10., 1));
    let b = add(&mut doc, rect(2., 0., 12., 10., 2));
    let c = add(&mut doc, rect(4., 0., 14., 10., 3));
    let d = add(&mut doc, rect(6., 0., 16., 10., 4));
    let plan = plan(Op::Clip, &doc, &[d, b, c, a]).unwrap();
    assert!(plan.removals.is_empty() && plan.additions.is_empty());
    let by: HashMap<_, _> = plan
        .replacements
        .iter()
        .map(|r| (r.handle, r.results.clone()))
        .collect();
    assert!(!by.contains_key(&d), "topmost stays unchanged");
    for h in [a, b, c] {
        let res = &by[&h];
        assert_eq!(res.len(), 1);
        assert!(
            (area(&res[0]) - 20.0).abs() < 1e-6,
            "area {}",
            area(&res[0])
        );
        assert_eq!(color(&res[0]), doc.get_entity(h).unwrap().common().color);
    }
}

#[test]
fn clip_removes_covered_and_keeps_untouched() {
    let mut doc = Document::new();
    let small = add(&mut doc, rect(1., 1., 2., 2., 1));
    let far = add(&mut doc, rect(50., 50., 60., 60., 2));
    let big = add(&mut doc, rect(0., 0., 10., 10., 3));
    let plan = plan(Op::Clip, &doc, &[small, far, big]).unwrap();
    assert_eq!(plan.removals, vec![small]);
    assert!(
        plan.replacements.is_empty(),
        "far shape does not overlap: unchanged"
    );
}

#[test]
fn clip_makes_a_hole_as_a_separate_outline() {
    let mut doc = Document::new();
    let base = add(&mut doc, rect(0., 0., 10., 10., 1));
    let hole = add(&mut doc, circle(5., 5., 2.));
    let plan = plan(Op::Clip, &doc, &[base, hole]).unwrap();
    let r = &plan.replacements[0];
    assert_eq!(r.handle, base);
    assert_eq!(r.results.len(), 2, "outer + hole");
    // The hole is an exact circle: a polyline of two half-circle bulges.
    let EntityType::LwPolyline(h) = entity(&r.results[1]) else {
        panic!()
    };
    assert_eq!(h.vertices.len(), 2);
    assert!(h
        .vertices
        .iter()
        .all(|v| (v.bulge.abs() - 1.0).abs() < 1e-9));
}

#[test]
fn clip_hatch_keeps_one_hatch_with_an_island() {
    let mut doc = Document::new();
    let base = add(
        &mut doc,
        EntityType::Hatch(Box::new(solid_hatch(0., 0., 10., 10.))),
    );
    let cutter = add(&mut doc, rect(4., 4., 6., 6., 2));
    let plan = plan(Op::Clip, &doc, &[base, cutter]).unwrap();
    let res = &plan.replacements[0].results;
    assert_eq!(res.len(), 1);
    let EntityType::Hatch(out) = entity(&res[0]) else {
        panic!()
    };
    assert_eq!(out.paths.len(), 2, "outer boundary and the island");
    assert!(out.is_solid);
}

#[test]
fn add_merges_connected_sets_and_leaves_isolated() {
    let mut doc = Document::new();
    let a = add(&mut doc, rect(0., 0., 4., 4., 1));
    let b = add(&mut doc, rect(2., 2., 6., 6., 2));
    let c = add(&mut doc, rect(20., 0., 24., 4., 3));
    // Edge-sharing pair.
    let d = add(&mut doc, rect(30., 0., 32., 2., 4));
    let e = add(&mut doc, rect(32., 0., 34., 2., 5));
    let plan = plan(Op::Add, &doc, &[e, c, b, d, a]).unwrap();
    assert_eq!(plan.replacements.len(), 2);
    let mut removed = plan.removals.clone();
    removed.sort();
    assert_eq!(removed, vec![b, e]);
    let ab = plan
        .replacements
        .iter()
        .find(|r| r.handle == a)
        .expect("A is the property source");
    assert_eq!(ab.results.len(), 1);
    assert_eq!(
        color(&ab.results[0]),
        Color::from_index(1),
        "lower object's properties"
    );
    assert!((area(&ab.results[0]) - 28.0).abs() < 1e-6);
    let de = plan.replacements.iter().find(|r| r.handle == d).unwrap();
    let EntityType::LwPolyline(p) = entity(&de.results[0]) else {
        panic!()
    };
    assert_eq!(p.vertices.len(), 4, "seam vertices dissolved");
    assert!(!plan.replacements.iter().any(|r| r.handle == c));
}

#[test]
fn add_with_nothing_connected_changes_nothing() {
    let mut doc = Document::new();
    let a = add(&mut doc, rect(0., 0., 1., 1., 1));
    let b = add(&mut doc, rect(1., 1., 2., 2., 2)); // touches at a point only
    let plan = plan(Op::Add, &doc, &[a, b]).unwrap();
    assert!(plan.is_empty());
}

/// A, B, C overlapping in a row: x 0–4, 2–6, 3–8 (height 4). Pieces covered
/// by two or more: A∩B only (x 2–3), A∩B∩C (x 3–4), B∩C only (x 4–6).
#[test]
fn intersect_splits_overlaps_into_pieces() {
    let mut doc = Document::new();
    let a = add(&mut doc, rect(0., 0., 4., 4., 1));
    let b = add(&mut doc, rect(2., 0., 6., 4., 2));
    let c = add(&mut doc, rect(3., 0., 8., 4., 3));
    let plan = plan(Op::Intersect, &doc, &[a, b, c]).unwrap();
    assert!(plan.replacements.is_empty() && plan.removals.is_empty());
    assert_eq!(sorted_areas(&plan.additions), vec![4.0, 4.0, 8.0]);
    let colors: Vec<Color> = plan.additions.iter().map(color).collect();
    assert_eq!(
        colors
            .iter()
            .filter(|c| **c == Color::from_index(1))
            .count(),
        2
    );
    assert_eq!(
        colors
            .iter()
            .filter(|c| **c == Color::from_index(2))
            .count(),
        1
    );
}

/// The reported case: three rectangles in a chain, the first and last not
/// touching: two overlaps.
#[test]
fn intersect_of_a_chain_gives_each_overlap() {
    let mut doc = Document::new();
    let top = add(&mut doc, rect(0., 6., 6., 12., 1));
    let mid = add(&mut doc, rect(4., 2., 9., 8., 2));
    let low = add(&mut doc, rect(7., -2., 11., 4., 3));
    let plan = plan(Op::Intersect, &doc, &[low, top, mid]).unwrap();
    assert_eq!(sorted_areas(&plan.additions), vec![4.0, 4.0]);
    let colors: Vec<Color> = plan.additions.iter().map(color).collect();
    assert!(colors.contains(&Color::from_index(1)) && colors.contains(&Color::from_index(2)));
}

/// The pieces never overlap and together are exactly the area covered by
/// two or more shapes.
#[test]
fn intersect_pieces_tile_the_overlap_area() {
    let mut doc = Document::new();
    let hs = [
        add(&mut doc, rect(0., 0., 6., 6., 1)),
        add(&mut doc, rect(3., 3., 9., 9., 2)),
        add(&mut doc, rect(2., 4., 8., 5., 3)),
        add(&mut doc, circle(5., 5., 2.)),
    ];
    let sel = validate(&doc, &hs).unwrap();
    let frame = sel.frame;
    let pieces: Vec<Vec<ocs_geom::geom2d::Curve>> = arrangement(&sel)
        .unwrap()
        .into_iter()
        .filter(|p| p.cover.len() >= 2)
        .map(|p| shape::boundary(&p.region, &frame))
        .collect();
    let shapes: Vec<Vec<ocs_geom::geom2d::Curve>> = sel
        .items
        .iter()
        .map(|i| shape::boundary(i.shape.area.as_ref().unwrap(), &frame))
        .collect();
    for gx in 0..36 {
        for gy in 0..36 {
            let p = [0.13 + gx as f64 * 0.25, 0.11 + gy as f64 * 0.25];
            let n = shapes
                .iter()
                .filter(|s| shape::inside(s, p, &frame))
                .count();
            let k = pieces
                .iter()
                .filter(|s| shape::inside(s, p, &frame))
                .count();
            assert_eq!(
                k,
                usize::from(n >= 2),
                "point {p:?}: {n} shapes, {k} pieces"
            );
        }
    }
}

/// The result takes the bottom object's settings, not the drawing's current
/// ones, whatever order the objects were picked in.
#[test]
fn intersect_takes_the_bottom_objects_properties() {
    use document::types::{LineWeight, Transparency};
    use document::xdata::{ExtendedDataRecord, XDataValue};
    let mut doc = Document::new();
    doc.header.current_entity_color = Color::from_index(6);
    doc.layers.add(Layer::new("WALLS")).unwrap();
    let mut bottom = rect(0., 0., 4., 4., 1);
    {
        let c = bottom.common_mut();
        c.layer = "WALLS".into();
        c.linetype = "DASHED".into();
        c.linetype_scale = 2.5;
        c.line_weight = LineWeight::from_value(50);
        c.transparency = Transparency::from_alpha_value(40);
        let mut rec = ExtendedDataRecord::new("MYAPP");
        rec.add_value(XDataValue::String("room 1".into()));
        c.extended_data.add_record(rec);
    }
    let a = add(&mut doc, bottom);
    let b = add(&mut doc, rect(2., 2., 6., 6., 3));
    for picked in [[a, b], [b, a]] {
        let plan = plan(Op::Intersect, &doc, &picked).unwrap();
        let src = doc.get_entity(a).unwrap().common();
        let e = entity(&plan.additions[0]).common();
        assert_eq!(e.color, Color::from_index(1));
        assert_eq!(e.layer, "WALLS");
        assert_eq!(e.linetype, "DASHED");
        assert_eq!(e.linetype_scale, 2.5);
        assert_eq!(e.line_weight, src.line_weight);
        assert_eq!(e.transparency, src.transparency);
        assert_eq!(e.extended_data, src.extended_data, "XDATA copied");
        assert!(e.handle.is_null(), "a new object, not the source's handle");
        assert!(e.xdictionary_handle.is_none() && e.reactors.is_empty());
    }
}

#[test]
fn intersect_with_a_hatch_at_the_bottom_gives_a_hatch() {
    let mut doc = Document::new();
    let mut h = solid_hatch(0., 0., 4., 4.);
    h.common.color = Color::from_index(5);
    let a = add(&mut doc, EntityType::Hatch(Box::new(h)));
    let b = add(&mut doc, rect(2., 2., 6., 6., 2));
    let plan = plan(Op::Intersect, &doc, &[b, a]).unwrap();
    assert_eq!(plan.additions.len(), 1);
    let EntityType::Hatch(out) = entity(&plan.additions[0]) else {
        panic!()
    };
    assert!(out.is_solid, "same fill");
    assert_eq!(out.common.color, Color::from_index(5));
    assert_eq!(out.paths.len(), 1);
}

#[test]
fn intersect_without_common_area_reports() {
    let mut doc = Document::new();
    let a = add(&mut doc, rect(0., 0., 1., 1., 1));
    let b = add(&mut doc, rect(1., 0., 2., 1., 2)); // shared edge only
    let err = plan(Op::Intersect, &doc, &[a, b]).unwrap_err();
    assert_eq!(err, NO_COMMON_SURFACE);
}

#[test]
fn rejects_locked_layers_and_shapes_in_other_planes() {
    let mut doc = Document::new();
    let a = add(&mut doc, rect(0., 0., 4., 4., 1));
    let EntityType::LwPolyline(mut high) = rect(1., 1., 3., 3., 2) else {
        unreachable!()
    };
    high.elevation = 5.0;
    let h = add(&mut doc, EntityType::LwPolyline(high));
    let err = plan(Op::Clip, &doc, &[a, h]).unwrap_err();
    assert!(err.contains("one plane"), "{err}");

    let mut layer = Layer::new("LOCKED");
    layer.lock();
    doc.layers.add(layer).unwrap();
    let mut l = rect(1., 1., 3., 3., 2);
    l.common_mut().layer = "LOCKED".into();
    let l = add(&mut doc, l);
    let err = plan(Op::Clip, &doc, &[a, l]).unwrap_err();
    assert!(err.contains("locked"), "{err}");
}

#[test]
fn fewer_than_two_shapes_asks_for_more() {
    let mut doc = Document::new();
    let a = add(&mut doc, rect(0., 0., 4., 4., 1));
    assert_eq!(plan(Op::Add, &doc, &[a]).unwrap_err(), SELECT_PROMPT);
}

#[test]
fn geometrically_closed_polyline_counts_as_closed() {
    let mut doc = Document::new();
    let mut p = LwPolyline::new();
    for (x, y) in [(0., 0.), (4., 0.), (4., 4.), (0., 4.), (0., 0.)] {
        p.vertices.push(LwVertex::new(Vector2::new(x, y)));
    }
    let a = add(&mut doc, EntityType::LwPolyline(p));
    let b = add(&mut doc, rect(2., 2., 6., 6., 2));
    let plan = plan(Op::Intersect, &doc, &[a, b]).unwrap();
    assert!((area(&plan.additions[0]) - 4.0).abs() < 1e-6);
}

#[test]
fn circle_union_is_a_circle() {
    let mut doc = Document::new();
    let a = add(&mut doc, circle(0., 0., 5.));
    let b = add(&mut doc, circle(1., 0., 1.)); // inside a
    let plan = plan(Op::Add, &doc, &[a, b]).unwrap();
    let r = &plan.replacements[0];
    assert!(
        matches!(entity(&r.results[0]), EntityType::Circle(c) if (c.radius - 5.0).abs() < 1e-9)
    );
    assert_eq!(r.results.len(), 1);
}

/// Curves stay exact: the overlap of two circles is two arcs, not a chain of
/// short segments.
#[test]
fn circle_overlap_is_two_arcs() {
    let mut doc = Document::new();
    let a = add(&mut doc, circle(0., 0., 5.));
    let b = add(&mut doc, circle(6., 0., 5.));
    let plan = plan(Op::Intersect, &doc, &[a, b]).unwrap();
    let EntityType::LwPolyline(p) = entity(&plan.additions[0]) else {
        panic!()
    };
    assert_eq!(p.vertices.len(), 2);
    // Lens area of two r = 5 circles 6 apart.
    let (r, d) = (5.0f64, 6.0f64);
    let want = 2.0 * r * r * (d / (2.0 * r)).acos() - d / 2.0 * (4.0 * r * r - d * d).sqrt();
    assert!(
        (area(&plan.additions[0]) - want).abs() < 1e-6,
        "{} vs {want}",
        area(&plan.additions[0])
    );
}

/// An ellipse keeps its exact edge: the result is a REGION where a polyline
/// cannot follow it.
#[test]
fn an_elliptical_edge_gives_a_region() {
    let mut doc = Document::new();
    let mut e = Ellipse::new();
    e.center = Vector3::new(0., 0., 0.);
    e.major_axis = Vector3::new(10., 0., 0.);
    e.minor_axis_ratio = 0.5;
    e.start_parameter = 0.0;
    e.end_parameter = std::f64::consts::TAU;
    let a = add(&mut doc, EntityType::Ellipse(e));
    let b = add(&mut doc, rect(0., -10., 20., 10., 2));
    let plan = plan(Op::Intersect, &doc, &[a, b]).unwrap();
    assert_eq!(plan.additions.len(), 1);
    assert!(matches!(plan.additions[0], Out::Region { .. }));
}

#[test]
fn command_names_parse() {
    assert_eq!(Op::parse("ADDSURFACE"), Some(Op::Add));
    assert_eq!(Op::parse("CLIPSURFACE"), Some(Op::Clip));
    assert_eq!(Op::parse("INTERSECTSURFACE"), Some(Op::Intersect));
    assert_eq!(Op::parse("CIRCLE"), None);
}

/// A sliver over the top of a unioned circle: the circle's edge is an arc
/// whose samples miss its top, and the boxes must still overlap.
#[test]
fn an_overlap_near_an_arcs_extreme_is_found() {
    let mut doc = Document::new();
    let sliver = add(&mut doc, rect(-0.5, 9.99, 0.5, 10.5, 1));
    let a = add(&mut doc, circle(0., 0., 10.));
    let b = add(&mut doc, circle(15., 0., 10.));
    let p = plan(Op::Clip, &doc, &[sliver, a, b]).unwrap();
    assert!(
        p.replacements.iter().any(|r| r.handle == sliver),
        "the sliver is clipped"
    );
}

/// A REGION stays a REGION and a filled 2D SOLID stays filled.
#[test]
fn regions_and_solids_keep_their_kind() {
    use document::entities::{Region, Solid};
    let mut doc = Document::new();
    let s1 = add(
        &mut doc,
        EntityType::Solid(Solid::new(
            Vector3::new(0., 0., 0.),
            Vector3::new(4., 0., 0.),
            Vector3::new(0., 4., 0.),
            Vector3::new(4., 4., 0.),
        )),
    );
    let s2 = add(&mut doc, rect(2., 2., 6., 6., 2));
    let p = plan(Op::Add, &doc, &[s1, s2]).unwrap();
    let EntityType::Hatch(h) = entity(&p.replacements[0].results[0]) else {
        panic!("a filled solid gives a fill")
    };
    assert!(h.is_solid);

    let sel = validate(&doc, &[s1, s2]).unwrap();
    let faces = shape::faces(sel.items[0].shape.area.as_ref().unwrap(), &sel.frame);
    let out = output::results(&EntityType::Region(Region::new()), &faces, &sel.frame);
    assert!(matches!(out.as_slice(), [Out::Region { .. }]));
}

/// An Ignore-style hatch fills over its islands: its area is the outer
/// loop, and a hole the clip makes is drawn as a hole.
#[test]
fn an_ignore_style_hatch_is_clipped_as_drawn() {
    use document::entities::HatchStyleType;
    let mut doc = Document::new();
    let mut h = solid_hatch(0., 0., 10., 10.);
    h.paths.push(rect_path(1., 1., 2., 2., None)); // island, filled over
    h.style = HatchStyleType::Ignore;
    let base = add(&mut doc, EntityType::Hatch(Box::new(h)));
    let cutter = add(&mut doc, rect(4., 4., 6., 6., 2));
    let p = plan(Op::Clip, &doc, &[base, cutter]).unwrap();
    let EntityType::Hatch(out) = entity(&p.replacements[0].results[0]) else {
        panic!()
    };
    assert_eq!(out.paths.len(), 2, "outer loop and the new hole only");
    assert_eq!(out.style, HatchStyleType::Normal, "the hole must show");
}

/// A hatch filling C (topmost, unchanged) with island A: covering A keeps
/// the hatch and C's path.
#[test]
fn a_hatch_keeps_the_path_of_a_selected_object_left_unchanged() {
    let mut doc = Document::new();
    let a = add(&mut doc, rect(5., 5., 8., 8., 1));
    let mut h = Hatch::solid();
    h.is_associative = true;
    let hatch = add(&mut doc, EntityType::Hatch(Box::new(h)));
    let c = add(&mut doc, rect(0., 0., 20., 20., 2));
    if let Some(EntityType::Hatch(h)) = doc.get_entity_mut(hatch) {
        h.paths.push(rect_path(0., 0., 20., 20., Some(c)));
        h.paths.push(rect_path(5., 5., 8., 8., Some(a)));
    }
    let p = plan(Op::Clip, &doc, &[a, c]).unwrap();
    assert_eq!(p.removals, vec![a], "only A goes, not the hatch");
    let u = &p.hatches[0];
    assert_eq!(u.hatch.common.handle, hatch);
    assert_eq!(u.kept.len(), 1);
    assert_eq!(u.kept[0].boundary_handles, vec![c]);
}

// ── Associative hatches follow their boundary's result ────────────────────

#[test]
fn clip_rewrites_the_hatch_to_follow_outline_and_hole() {
    let mut doc = Document::new();
    let (p, hatch) = hatched_rect(&mut doc, 0., 0., 10., 10., 1);
    let c = add(&mut doc, circle(5., 5., 2.));
    // Picked without and with the hatch (a window selection takes both).
    for picked in [vec![p, c], vec![hatch, c, p]] {
        let plan = plan(Op::Clip, &doc, &picked).unwrap();
        assert_eq!(
            plan.replacements.len(),
            1,
            "the hatch is not clipped as a shape of its own"
        );
        assert_eq!(plan.replacements[0].results.len(), 2, "outline + hole");
        assert_eq!(plan.hatches.len(), 1);
        let u = &plan.hatches[0];
        assert_eq!(
            u.hatch.common.handle, hatch,
            "the same hatch, rewritten in place"
        );
        let refs: Vec<ResultRef> = u.bound.iter().map(|(_, r)| *r).collect();
        assert_eq!(
            refs,
            vec![
                ResultRef::Replacement { rep: 0, index: 0 },
                ResultRef::Replacement { rep: 0, index: 1 }
            ]
        );
        assert!(u.kept.is_empty());
        assert!(u.hatch.is_solid);
        assert!(!plan.removals.contains(&hatch));
    }
}

#[test]
fn clip_removes_the_hatch_of_a_covered_object() {
    let mut doc = Document::new();
    let (p, hatch) = hatched_rect(&mut doc, 1., 1., 2., 2., 1);
    let big = add(&mut doc, rect(0., 0., 10., 10., 2));
    let plan = plan(Op::Clip, &doc, &[p, big]).unwrap();
    let mut removed = plan.removals.clone();
    removed.sort();
    assert_eq!(removed, vec![p, hatch]);
    assert!(plan.hatches.is_empty());
}

#[test]
fn add_keeps_the_bottom_hatch_and_drops_the_others() {
    let mut doc = Document::new();
    let (a, ha) = hatched_rect(&mut doc, 0., 0., 4., 4., 1);
    let (b, hb) = hatched_rect(&mut doc, 2., 2., 6., 6., 2);
    let plan = plan(Op::Add, &doc, &[b, a]).unwrap();
    assert_eq!(plan.hatches.len(), 1);
    assert_eq!(
        plan.hatches[0].hatch.common.handle, ha,
        "the bottom object's hatch follows the merge"
    );
    assert_eq!(plan.hatches[0].bound.len(), 1);
    assert!(plan.removals.contains(&hb) && plan.removals.contains(&b));
    assert!(!plan.removals.contains(&ha));
}

#[test]
fn add_takes_the_hatch_from_the_lowest_member_that_has_one() {
    let mut doc = Document::new();
    let a = add(&mut doc, rect(0., 0., 4., 4., 1)); // no hatch, bottom
    let (b, hb) = hatched_rect(&mut doc, 2., 2., 6., 6., 2);
    let plan = plan(Op::Add, &doc, &[a, b]).unwrap();
    assert_eq!(
        plan.replacements[0].handle, a,
        "properties still from the bottom object"
    );
    assert_eq!(plan.hatches.len(), 1);
    assert_eq!(plan.hatches[0].hatch.common.handle, hb);
    assert!(!plan.removals.contains(&hb));
}

#[test]
fn intersect_gives_each_piece_a_new_hatch() {
    let mut doc = Document::new();
    let (a, ha) = hatched_rect(&mut doc, 0., 0., 4., 4., 1);
    let b = add(&mut doc, rect(2., 2., 6., 6., 3));
    let plan = plan(Op::Intersect, &doc, &[b, a]).unwrap();
    assert_eq!(plan.additions.len(), 1);
    assert_eq!(plan.hatches.len(), 1);
    let u = &plan.hatches[0];
    assert!(
        u.hatch.common.handle.is_null(),
        "a new hatch, not the source's"
    );
    assert_eq!(u.hatch.common.color, Color::from_index(1));
    assert_eq!(
        u.bound.iter().map(|(_, r)| *r).collect::<Vec<_>>(),
        vec![ResultRef::Addition(0)]
    );
    assert!(!plan.removals.contains(&ha) && plan.removals.is_empty());
}

#[test]
fn a_hatch_on_a_locked_layer_refuses() {
    let mut doc = Document::new();
    let (p, hatch) = hatched_rect(&mut doc, 0., 0., 10., 10., 1);
    let mut layer = Layer::new("LOCKED");
    layer.lock();
    doc.layers.add(layer).unwrap();
    if let Some(EntityType::Hatch(h)) = doc.get_entity_mut(hatch) {
        h.common.layer = "LOCKED".into();
    }
    let c = add(&mut doc, circle(5., 5., 2.));
    let err = plan(Op::Clip, &doc, &[p, c]).unwrap_err();
    assert!(err.contains("hatch") && err.contains("locked"), "{err}");
}

#[test]
fn island_paths_of_unselected_objects_are_kept() {
    let mut doc = Document::new();
    let p = add(&mut doc, rect(0., 0., 10., 10., 1));
    let island = add(&mut doc, rect(1., 1., 2., 2., 4)); // not selected
    let mut h = Hatch::solid();
    h.paths.push(rect_path(0., 0., 10., 10., Some(p)));
    h.paths.push(rect_path(1., 1., 2., 2., Some(island)));
    h.is_associative = true;
    add(&mut doc, EntityType::Hatch(Box::new(h)));
    let c = add(&mut doc, rect(8., 8., 12., 12., 2));
    let plan = plan(Op::Clip, &doc, &[p, c]).unwrap();
    let u = &plan.hatches[0];
    assert_eq!(u.kept.len(), 1);
    assert_eq!(u.kept[0].boundary_handles, vec![island]);
}

/// A real overlap stays a surface however large the drawing is.
#[test]
fn small_overlaps_count_in_large_drawings() {
    let mut doc = Document::new();
    let site = add(&mut doc, rect(0., 0., 10_000., 10_000., 1));
    let a = add(&mut doc, rect(5_000., 5_000., 5_004., 5_004., 2));
    let b = add(&mut doc, rect(5_002., 5_002., 5_006., 5_006., 3));
    let p = plan(Op::Intersect, &doc, &[a, b, site]).unwrap();
    assert!(
        sorted_areas(&p.additions).contains(&4.0),
        "{:?}",
        sorted_areas(&p.additions)
    );
    let p = plan(Op::Clip, &doc, &[a, b]).unwrap();
    assert_eq!(p.replacements.len(), 1, "the 2 x 2 overlap is clipped");
}

/// An ellipse with ratio 1 and a circle of the same size: fully covered,
/// not a leftover sliver.
#[test]
fn equal_circle_and_ellipse_cover_each_other() {
    let mut doc = Document::new();
    let mut e = Ellipse::new();
    e.center = Vector3::new(0., 0., 0.);
    e.major_axis = Vector3::new(500., 0., 0.);
    e.minor_axis_ratio = 1.0;
    e.start_parameter = 0.0;
    e.end_parameter = std::f64::consts::TAU;
    let low = add(&mut doc, EntityType::Ellipse(e));
    let top = add(&mut doc, circle(0., 0., 500.));
    let plan = plan(Op::Clip, &doc, &[low, top]).unwrap();
    assert_eq!(
        plan.removals,
        vec![low],
        "covered: {:?}",
        plan.replacements.len()
    );
    assert!(plan.replacements.is_empty());
}

/// A hatch whose outer boundary is an unselected object U and whose island
/// is a selected object C: covering C keeps U's fill.
#[test]
fn covering_an_island_keeps_the_hatch_of_its_unselected_outline() {
    let mut doc = Document::new();
    let u = add(&mut doc, rect(0., 0., 20., 20., 1)); // not selected
    let c = add(&mut doc, rect(5., 5., 8., 8., 2));
    let mut h = Hatch::solid();
    h.paths.push(rect_path(0., 0., 20., 20., Some(u)));
    h.paths.push(rect_path(5., 5., 8., 8., Some(c)));
    h.is_associative = true;
    let hatch = add(&mut doc, EntityType::Hatch(Box::new(h)));
    let cover = add(&mut doc, rect(4., 4., 9., 9., 3));
    let plan = plan(Op::Clip, &doc, &[c, cover]).unwrap();
    assert_eq!(
        plan.removals,
        vec![c],
        "only the covered island object goes"
    );
    assert_eq!(plan.hatches.len(), 1);
    let up = &plan.hatches[0];
    assert_eq!(up.hatch.common.handle, hatch);
    assert!(up.bound.is_empty());
    assert_eq!(up.kept[0].boundary_handles, vec![u]);
}

#[test]
fn clipped_island_loops_are_not_outermost() {
    let mut doc = Document::new();
    let u = add(&mut doc, rect(0., 0., 20., 20., 1));
    let c = add(&mut doc, rect(5., 5., 10., 10., 2));
    let mut h = Hatch::solid();
    h.paths.push(rect_path(0., 0., 20., 20., Some(u)));
    h.paths.push(rect_path(5., 5., 10., 10., Some(c)));
    h.is_associative = true;
    add(&mut doc, EntityType::Hatch(Box::new(h)));
    let cutter = add(&mut doc, rect(8., 0., 12., 20., 3));
    let plan = plan(Op::Clip, &doc, &[c, cutter]).unwrap();
    let up = &plan.hatches[0];
    assert_eq!(up.kept.len(), 1);
    assert!(!up.bound.is_empty());
    for (p, _) in &up.bound {
        assert_eq!(p.flags.bits() & 16, 0, "OUTERMOST must not be set");
    }
}

#[test]
fn unbound_paths_of_an_associative_hatch_stay() {
    let mut doc = Document::new();
    let p = add(&mut doc, rect(0., 0., 10., 10., 1));
    let mut h = Hatch::solid();
    h.paths.push(rect_path(0., 0., 10., 10., Some(p)));
    h.paths.push(rect_path(2., 2., 3., 3., None));
    h.is_associative = true;
    add(&mut doc, EntityType::Hatch(Box::new(h)));
    let c = add(&mut doc, rect(8., 8., 12., 12., 2));
    let plan = plan(Op::Clip, &doc, &[p, c]).unwrap();
    let up = &plan.hatches[0];
    assert_eq!(up.kept.len(), 1);
    assert!(up.kept[0].boundary_handles.is_empty());
}

/// A hatch facing −Z keeps its normal and its kept paths; the new paths are
/// written in its own (mirrored) coordinates.
#[test]
fn a_hatch_facing_down_keeps_its_normal() {
    let mut doc = Document::new();
    let p = add(&mut doc, rect(0., 0., 10., 10., 1));
    let island = add(&mut doc, rect(1., 1., 2., 2., 4)); // not selected
    let mut h = Hatch::solid();
    h.normal = Vector3::new(0., 0., -1.);
    // In the −Z frame x is mirrored: world x ↦ −x.
    h.paths.push(rect_path(-10., 0., 0., 10., Some(p)));
    h.paths.push(rect_path(-2., 1., -1., 2., Some(island)));
    h.is_associative = true;
    add(&mut doc, EntityType::Hatch(Box::new(h)));
    let c = add(&mut doc, rect(8., 0., 12., 10., 2));
    let plan = plan(Op::Clip, &doc, &[p, c]).unwrap();
    let up = &plan.hatches[0];
    assert_eq!(up.hatch.normal.z, -1.0);
    assert_eq!(up.kept[0].boundary_handles, vec![island]);
    let BoundaryEdge::Polyline(pe) = &up.bound[0].0.edges[0] else {
        panic!()
    };
    let xs: Vec<f64> = pe
        .vertices
        .iter()
        .map(|v| (v.x * 1e9).round() / 1e9)
        .collect();
    assert!(xs.iter().all(|x| (-8.0..=0.0).contains(x)), "{xs:?}");
}

// ── Open shapes (lines, arcs, open polylines) ─────────────────────────────

#[test]
fn add_closes_an_open_path_and_merges_it() {
    let mut doc = Document::new();
    let r = add(&mut doc, rect(0., 0., 12., 10., 1));
    let z = add(&mut doc, zigzag(2));
    let plan = plan(Op::Add, &doc, &[z, r]).unwrap();
    assert_eq!(plan.replacements.len(), 1);
    assert_eq!(
        plan.replacements[0].handle, r,
        "the rectangle (bottom) takes the result"
    );
    assert_eq!(plan.removals, vec![z], "the path is absorbed");
    let res = &plan.replacements[0].results;
    assert_eq!(res.len(), 1);
    // The closed zigzag adds two small corners outside the rectangle.
    assert!(area(&res[0]) > 120.0 + 1e-6, "{}", area(&res[0]));
}

#[test]
fn clip_splits_a_shape_along_an_open_path_above_it() {
    let mut doc = Document::new();
    let r = add(&mut doc, rect(0., 0., 12., 10., 1));
    let z = add(&mut doc, zigzag(2));
    let plan = plan(Op::Clip, &doc, &[z, r]).unwrap();
    assert_eq!(plan.replacements.len(), 1, "the path on top stays as it is");
    let rep = &plan.replacements[0];
    assert_eq!(rep.handle, r);
    assert_eq!(rep.results.len(), 2, "two halves");
    let total: f64 = rep.results.iter().map(area).sum();
    assert!((total - 120.0).abs() < 1e-6, "{total}");
    assert!(rep.results.iter().all(|e| color(e) == Color::from_index(1)));
    assert!(plan.removals.is_empty());
}

#[test]
fn clip_splits_with_a_line_and_keeps_the_rest_of_the_clip() {
    let mut doc = Document::new();
    let r = add(&mut doc, rect(0., 0., 12., 10., 1));
    let cover = add(&mut doc, rect(10., 0., 14., 10., 3));
    let knife = add(&mut doc, line(-1., 5., 15., 5.));
    let plan = plan(Op::Clip, &doc, &[r, cover, knife]).unwrap();
    let rep = plan.replacements.iter().find(|p| p.handle == r).unwrap();
    assert_eq!(rep.results.len(), 2);
    let total: f64 = rep.results.iter().map(area).sum();
    assert!(
        (total - 100.0).abs() < 1e-6,
        "12 x 10 minus the covered 2 x 10: {total}"
    );
    let cov = plan
        .replacements
        .iter()
        .find(|p| p.handle == cover)
        .unwrap();
    assert_eq!(
        cov.results.len(),
        2,
        "the rectangle above the line is split too"
    );
}

#[test]
fn a_line_ending_inside_does_not_split() {
    let mut doc = Document::new();
    let r = add(&mut doc, rect(0., 0., 12., 10., 1));
    let l = add(&mut doc, line(-1., 5., 6., 5.));
    let plan = plan(Op::Clip, &doc, &[r, l]).unwrap();
    assert!(plan.is_empty(), "nothing is cut through");
}

#[test]
fn an_open_path_below_a_shape_loses_its_covered_part() {
    let mut doc = Document::new();
    let z = add(&mut doc, zigzag(2));
    let r = add(&mut doc, rect(0., 0., 12., 10., 1));
    let plan = plan(Op::Clip, &doc, &[r, z]).unwrap();
    let rep = &plan.replacements[0];
    assert_eq!(rep.handle, z);
    assert_eq!(rep.results.len(), 2, "both ends outside the rectangle stay");
    assert!(rep
        .results
        .iter()
        .all(|e| matches!(entity(e), EntityType::LwPolyline(p) if !p.is_closed)));
}

#[test]
fn intersect_keeps_the_part_of_an_open_path_inside() {
    let mut doc = Document::new();
    let r = add(&mut doc, rect(0., 0., 12., 10., 1));
    let z = add(&mut doc, zigzag(2));
    let plan = plan(Op::Intersect, &doc, &[z, r]).unwrap();
    assert_eq!(plan.additions.len(), 1);
    let EntityType::LwPolyline(p) = entity(&plan.additions[0]) else {
        panic!()
    };
    assert!(!p.is_closed, "an open path stays open");
    // Inside: from (3, 0) via (5, 4), (8, 4) to (9.5, 10).
    let len: f64 = p
        .vertices
        .windows(2)
        .map(|w| (w[1].location.x - w[0].location.x).hypot(w[1].location.y - w[0].location.y))
        .sum();
    let want = (2f64 * 2. + 4. * 4.).sqrt() + 3.0 + (1.5f64 * 1.5 + 6. * 6.).sqrt();
    assert!((len - want).abs() < 1e-6, "{len} vs {want}");
    assert_eq!(
        p.common.color,
        Color::from_index(1),
        "styled after the rectangle below"
    );
}

#[test]
fn intersect_with_a_line_gives_a_line() {
    let mut doc = Document::new();
    let r = add(&mut doc, rect(0., 0., 12., 10., 1));
    let l = add(&mut doc, line(-1., 5., 13., 5.));
    let plan = plan(Op::Intersect, &doc, &[l, r]).unwrap();
    assert_eq!(plan.additions.len(), 1);
    let EntityType::Line(out) = entity(&plan.additions[0]) else {
        panic!()
    };
    let xs = [out.start.x.min(out.end.x), out.start.x.max(out.end.x)];
    assert!(
        (xs[0] - 0.).abs() < 1e-6 && (xs[1] - 12.).abs() < 1e-6,
        "{xs:?}"
    );
}
