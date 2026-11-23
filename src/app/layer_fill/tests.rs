use super::apply::{self, FILL_APP, FOLLOWS_LAYER};
use super::store::{self, FillKind, LayerFill};
use crate::app::{Message, OpenCADStudio};
use document::objects::ObjectType;
use document::{EntityType, Handle};

fn fresh() -> OpenCADStudio {
    let mut app = OpenCADStudio::new_for_test();
    let _ = app.automation_op(r#"{"op":"new"}"#);
    app
}

/// A drawing with layer `Walls` current and its fill style solid.
fn walls() -> OpenCADStudio {
    let mut app = fresh();
    let _ = app.run_command_line("-LAYER M Walls ");
    let i = app.active_tab;
    assert_eq!(app.tabs[i].active_layer, "Walls");
    assert!(app.set_layer_fill(i, "Walls", Some(&LayerFill::default())));
    app
}

fn hatches(app: &OpenCADStudio) -> Vec<&document::entities::Hatch> {
    app.tabs[app.active_tab]
        .scene
        .document
        .entities()
        .filter_map(|entity| match entity {
            EntityType::Hatch(hatch) => Some(hatch.as_ref()),
            _ => None,
        })
        .collect()
}

fn shapes(app: &OpenCADStudio) -> Vec<Handle> {
    app.tabs[app.active_tab]
        .scene
        .document
        .entities()
        .filter(|entity| !matches!(entity, EntityType::Hatch(_)))
        .map(|entity| entity.common().handle)
        .collect()
}

/// The key an entity is drawn by: its sort-table entry, else its handle.
fn sort_key(app: &OpenCADStudio, handle: Handle) -> u64 {
    let i = app.active_tab;
    let block = app.tabs[i].scene.current_layout_block_handle_pub();
    app.tabs[i]
        .scene
        .document
        .objects
        .values()
        .find_map(|object| match object {
            ObjectType::SortEntitiesTable(table) if table.block_owner_handle == block => table
                .entries()
                .find(|entry| entry.entity_handle == handle)
                .map(|entry| entry.sort_handle.value()),
            _ => None,
        })
        .unwrap_or(handle.value())
}

#[test]
fn a_rectangle_on_a_layer_with_a_fill_style_is_filled_behind_its_lines() {
    let mut app = walls();
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let shapes = shapes(&app);
    assert_eq!(shapes.len(), 1);
    let rectangle = shapes[0];
    let fills = hatches(&app);
    assert_eq!(fills.len(), 1, "one fill");
    let fill = fills[0];
    assert!(fill.is_associative);
    assert!(fill.is_solid);
    assert_eq!(fill.common.layer, "Walls");
    let boundary: Vec<Handle> = fill
        .paths
        .iter()
        .flat_map(|path| path.boundary_handles.iter().copied())
        .collect();
    assert_eq!(boundary, vec![rectangle]);
    assert_eq!(
        apply::fill_mark(&EntityType::Hatch(Box::new(fill.clone()))),
        Some(FOLLOWS_LAYER)
    );
    let handle = fill.common.handle;
    assert!(
        sort_key(&app, handle) < sort_key(&app, rectangle),
        "the fill draws behind the rectangle"
    );
}

#[test]
fn one_undo_removes_shape_and_fill_and_redo_brings_both_back() {
    let mut app = walls();
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    assert_eq!(
        app.tabs[app.active_tab].scene.document.entities().count(),
        2
    );
    let _ = app.update(Message::Undo);
    assert_eq!(
        app.tabs[app.active_tab].scene.document.entities().count(),
        0
    );
    let _ = app.update(Message::Redo);
    assert_eq!(
        app.tabs[app.active_tab].scene.document.entities().count(),
        2
    );
    assert_eq!(hatches(&app).len(), 1);
}

#[test]
fn nothing_is_filled_on_a_layer_whose_fill_is_none() {
    let mut app = walls();
    let _ = app.dispatch_command("LAYERFILL Walls None");
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let _ = app.run_command_line("CIRCLE 20,0 3");
    assert_eq!(shapes(&app).len(), 2);
    assert!(hatches(&app).is_empty());
}

#[test]
fn circles_are_filled_and_open_shapes_and_other_layers_are_not() {
    let mut app = walls();
    let _ = app.run_command_line("CIRCLE 20,0 3");
    assert_eq!(hatches(&app).len(), 1);
    let _ = app.run_command_line("LINE 0,0 5,5 ");
    assert_eq!(hatches(&app).len(), 1, "a line gets no fill");
    let _ = app.run_command_line("-LAYER M Doors ");
    let _ = app.run_command_line("RECTANG 30,0 40,4");
    assert_eq!(hatches(&app).len(), 1, "Doors has no fill style");
}

#[test]
fn the_fill_takes_the_pattern_scale_and_transparency_of_the_layer() {
    let mut app = walls();
    let i = app.active_tab;
    let style = LayerFill {
        kind: FillKind::Pattern,
        pattern: "ANSI31".into(),
        transparency: 50,
        scale: 2.0,
        ..LayerFill::default()
    };
    assert!(app.set_layer_fill(i, "Walls", Some(&style)));
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let fills = hatches(&app);
    assert_eq!(fills.len(), 1);
    assert!(!fills[0].is_solid);
    assert!(fills[0].pattern.name.eq_ignore_ascii_case("ANSI31"));
    assert!((fills[0].pattern_scale - 2.0).abs() < 1.0e-9);
    assert_eq!(
        fills[0].common.transparency,
        document::types::Transparency::from_percent(0.5)
    );
}

#[test]
fn only_drawing_commands_with_closed_shapes_fill() {
    assert!(apply::is_draw_command("RECT"));
    assert!(apply::is_draw_command("PLINE"));
    assert!(!apply::is_draw_command("PASTECLIP"));
    assert!(!apply::is_draw_command("INSERT"));
    let mut open = document::entities::LwPolyline::from_points(vec![
        document::types::Vector2::new(0.0, 0.0),
        document::types::Vector2::new(1.0, 0.0),
        document::types::Vector2::new(1.0, 1.0),
    ]);
    assert!(!apply::is_closed_shape(&EntityType::LwPolyline(
        open.clone()
    )));
    open.is_closed = true;
    assert!(apply::is_closed_shape(&EntityType::LwPolyline(open)));
    assert!(apply::is_closed_shape(&EntityType::Circle(
        Default::default()
    )));
    assert!(!apply::is_closed_shape(&EntityType::Line(
        Default::default()
    )));
}

#[test]
fn layerfill_sets_a_layer_style_as_one_undo_step() {
    let mut app = fresh();
    let _ = app.run_command_line("-LAYER M Walls ");
    let i = app.active_tab;
    let _ = app.dispatch_command("LAYERFILL Walls ANSI31 2 45 30");
    let style = store::read(&app.tabs[i].scene.document, "Walls").expect("a style");
    assert_eq!(style.kind, FillKind::Pattern);
    assert_eq!(style.pattern, "ANSI31");
    assert_eq!(style.scale, 2.0);
    assert_eq!(style.angle_deg, 45.0);
    assert_eq!(style.transparency, 30);
    let _ = app.dispatch_command("LAYERFILL Walls Nopattern");
    assert_eq!(
        store::read(&app.tabs[i].scene.document, "Walls").map(|fill| fill.pattern),
        Some("ANSI31".into()),
        "an unknown pattern changes nothing"
    );
    let _ = app.update(Message::Undo);
    assert_eq!(store::read(&app.tabs[i].scene.document, "Walls"), None);
}

#[test]
fn a_hatch_the_user_made_carries_no_mark() {
    let hatch = document::entities::Hatch::default();
    assert_eq!(
        apply::fill_mark(&EntityType::Hatch(Box::new(hatch.clone()))),
        None
    );
    let mut marked = hatch;
    apply::set_fill_mark(&mut marked, apply::OVERRIDDEN);
    assert_eq!(
        apply::fill_mark(&EntityType::Hatch(Box::new(marked.clone()))),
        Some(apply::OVERRIDDEN)
    );
    assert!(marked.common.extended_data.get_record(FILL_APP).is_some());
}

#[test]
fn styles_and_marks_survive_saving_as_dxf_and_dwg() {
    let mut app = walls();
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let document = &app.tabs[app.active_tab].scene.document;
    for extension in ["dxf", "dwg"] {
        let dir = std::env::temp_dir().join(format!("ocs-layer-fill-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("fill.{extension}"));
        crate::io::save(document, &path).expect("saves");
        let loaded = crate::io::load_file(&path).expect("loads");
        assert_eq!(
            store::read(&loaded, "Walls").map(|fill| fill.kind),
            Some(FillKind::Solid),
            "{extension}: layer style"
        );
        let marks: Vec<_> = loaded.entities().filter_map(apply::fill_mark).collect();
        assert_eq!(marks, vec![FOLLOWS_LAYER], "{extension}: fill mark");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

fn rectangle(app: &OpenCADStudio) -> Handle {
    app.tabs[app.active_tab]
        .scene
        .document
        .entities()
        .find_map(|entity| {
            matches!(entity, EntityType::LwPolyline(_)).then(|| entity.common().handle)
        })
        .expect("a rectangle")
}

/// A user's own hatch beside the rectangle: it must never be touched.
fn user_hatch(app: &mut OpenCADStudio) -> Handle {
    let i = app.active_tab;
    let mut square = document::entities::LwPolyline::from_points(vec![
        document::types::Vector2::new(50.0, 0.0),
        document::types::Vector2::new(60.0, 0.0),
        document::types::Vector2::new(60.0, 10.0),
        document::types::Vector2::new(50.0, 10.0),
    ]);
    square.is_closed = true;
    let square = app.tabs[i].scene.add_entity(EntityType::LwPolyline(square));
    app.tabs[i].scene.select_entities(&[square]);
    let _ = app.update(Message::GraphicAttributes(
        crate::ui::window::graphic_attributes::GraphicAttributesMsg::Fill(
            crate::ui::window::graphic_attributes::GraphicAttribute::Solid,
        ),
    ));
    app.tabs[i].scene.deselect_all();
    // The palette marks its fill as custom; a hatch from HATCH or an older
    // drawing carries no mark at all.
    let mut hatch = hatches(app)
        .into_iter()
        .find(|hatch| {
            hatch
                .paths
                .iter()
                .any(|path| path.boundary_handles.contains(&square))
        })
        .cloned()
        .expect("the user's hatch");
    hatch.common.extended_data.remove_record(apply::FILL_APP);
    let handle = hatch.common.handle;
    app.tabs[i]
        .scene
        .update_entity(EntityType::Hatch(Box::new(hatch)));
    handle
}

#[test]
fn selecting_the_shape_selects_its_fill_and_the_other_way_round() {
    let mut app = walls();
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let users = user_hatch(&mut app);
    let i = app.active_tab;
    let rectangle = rectangle(&app);
    let fill = hatches(&app)
        .into_iter()
        .find(|hatch| hatch.common.handle != users)
        .map(|hatch| hatch.common.handle)
        .expect("the layer fill");
    app.tabs[i].scene.select_entity(rectangle, true);
    assert!(app.tabs[i].scene.selected.contains(&fill));
    app.tabs[i].scene.select_entity(fill, true);
    assert!(app.tabs[i].scene.selected.contains(&rectangle));
    assert!(!app.tabs[i].scene.selected.contains(&users));
}

#[test]
fn erasing_the_shape_erases_its_fill_and_leaves_the_users_hatch() {
    let mut app = walls();
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let users = user_hatch(&mut app);
    let i = app.active_tab;
    let rectangle = rectangle(&app);
    app.tabs[i].scene.select_entities(&[rectangle]);
    let _ = app.run_command_line("ERASE");
    let left: Vec<Handle> = hatches(&app)
        .iter()
        .map(|hatch| hatch.common.handle)
        .collect();
    assert_eq!(left, vec![users]);
    let _ = app.update(Message::Undo);
    assert_eq!(hatches(&app).len(), 2);
}

#[test]
fn a_copy_of_the_shape_gets_its_own_fill() {
    let mut app = walls();
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let i = app.active_tab;
    let rectangle = rectangle(&app);
    app.tabs[i].scene.select_entities(&[rectangle]);
    let _ = app.run_command_line("COPY");
    let _ = app.feed_script_line("0,0");
    let _ = app.feed_script_line("100,0");
    let _ = app.feed_script_line("");
    let shapes = shapes(&app);
    assert_eq!(shapes.len(), 2);
    let fills = hatches(&app);
    assert_eq!(fills.len(), 2);
    for shape in shapes {
        let bound: Vec<_> = fills
            .iter()
            .filter(|hatch| {
                hatch
                    .paths
                    .iter()
                    .any(|path| path.boundary_handles.contains(&shape))
            })
            .collect();
        assert_eq!(bound.len(), 1, "each shape has its own fill");
        assert_eq!(
            apply::fill_mark(&EntityType::Hatch(Box::new((*bound[0]).clone()))),
            Some(FOLLOWS_LAYER)
        );
    }
}

#[test]
fn a_new_layer_style_reaches_the_fills_that_follow_it_in_one_undo_step() {
    let mut app = walls();
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let users = user_hatch(&mut app);
    let i = app.active_tab;
    let before = hatches(&app)
        .into_iter()
        .find(|hatch| hatch.common.handle != users)
        .map(|hatch| hatch.common.handle)
        .unwrap();
    let _ = app.dispatch_command("LAYERFILL Walls ANSI31 3");
    let fill = match app.tabs[i].scene.document.get_entity(before) {
        Some(EntityType::Hatch(hatch)) => hatch.as_ref().clone(),
        _ => panic!("the fill keeps its handle"),
    };
    assert!(!fill.is_solid);
    assert!(fill.pattern.name.eq_ignore_ascii_case("ANSI31"));
    assert!((fill.pattern_scale - 3.0).abs() < 1.0e-9);
    let users_hatch = match app.tabs[i].scene.document.get_entity(users) {
        Some(EntityType::Hatch(hatch)) => hatch.as_ref().clone(),
        _ => panic!("the user's hatch is still there"),
    };
    assert!(users_hatch.is_solid, "the user's hatch is untouched");
    let _ = app.update(Message::Undo);
    match app.tabs[i].scene.document.get_entity(before) {
        Some(EntityType::Hatch(hatch)) => assert!(hatch.is_solid, "undo brings back solid"),
        _ => panic!("the fill is back"),
    }
    let _ = app.dispatch_command("LAYERFILL Walls None");
    assert!(app.tabs[i].scene.document.get_entity(before).is_none());
    assert!(app.tabs[i].scene.document.get_entity(users).is_some());
}

#[test]
fn fillbylayerapply_fills_shapes_drawn_before() {
    let mut app = walls();
    let _ = app.dispatch_command("LAYERFILL Walls None");
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let _ = app.run_command_line("CIRCLE 20,0 3");
    assert!(hatches(&app).is_empty());
    let _ = app.dispatch_command("LAYERFILL Walls Solid");
    let _ = app.dispatch_command("FILLBYLAYERAPPLY Walls");
    assert_eq!(hatches(&app).len(), 2);
    let _ = app.dispatch_command("FILLBYLAYERAPPLY Walls");
    assert_eq!(hatches(&app).len(), 2, "filled shapes are not filled twice");
    let _ = app.update(Message::Undo);
    assert!(hatches(&app).is_empty(), "one undo step");
}

#[test]
fn a_style_change_on_a_thousand_shapes_takes_under_a_second() {
    let mut app = walls();
    let i = app.active_tab;
    for n in 0..1000 {
        let x = f64::from(n % 40) * 20.0;
        let y = f64::from(n / 40) * 20.0;
        let mut square = document::entities::LwPolyline::from_points(vec![
            document::types::Vector2::new(x, y),
            document::types::Vector2::new(x + 10.0, y),
            document::types::Vector2::new(x + 10.0, y + 10.0),
            document::types::Vector2::new(x, y + 10.0),
        ]);
        square.is_closed = true;
        square.common.layer = "Walls".into();
        let _ = app.tabs[i].scene.add_entity(EntityType::LwPolyline(square));
    }
    let _ = app.dispatch_command("FILLBYLAYERAPPLY Walls");
    assert_eq!(hatches(&app).len(), 1000);
    let started = std::time::Instant::now();
    let _ = app.dispatch_command("LAYERFILL Walls ANSI31 2");
    let elapsed = started.elapsed();
    eprintln!("restyling 1000 fills took {elapsed:?}");
    assert!(hatches(&app).iter().all(|hatch| !hatch.is_solid));
    assert!(elapsed.as_secs_f64() < 1.0, "took {elapsed:?}");
}

#[test]
fn a_fill_changed_in_the_palette_keeps_its_own_style_until_reset() {
    let mut app = walls();
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let i = app.active_tab;
    let rectangle = rectangle(&app);
    app.tabs[i].scene.select_entities(&[rectangle]);
    let _ = app.update(Message::GraphicAttributes(
        crate::ui::window::graphic_attributes::GraphicAttributesMsg::Fill(
            crate::ui::window::graphic_attributes::GraphicAttribute::Hatch,
        ),
    ));
    let fills = hatches(&app);
    assert_eq!(
        fills.len(),
        1,
        "the palette replaces the fill, it does not add one"
    );
    assert!(!fills[0].is_solid);
    assert_eq!(apply::hatch_mark(fills[0]), Some(apply::OVERRIDDEN));
    // A new layer style leaves it alone.
    let _ = app.dispatch_command("LAYERFILL Walls Solid");
    assert!(!hatches(&app)[0].is_solid);
    // Reset makes it follow the layer again.
    app.tabs[i].scene.select_entities(&[rectangle]);
    let _ = app.dispatch_command("FILLBYLAYERRESET");
    let fills = hatches(&app);
    assert_eq!(fills.len(), 1);
    assert!(fills[0].is_solid);
    assert_eq!(apply::hatch_mark(fills[0]), Some(FOLLOWS_LAYER));
}

#[test]
fn the_fill_has_its_own_colour_apart_from_the_line_colour() {
    let mut app = walls();
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let i = app.active_tab;
    let _ = app.dispatch_command("LAYERFILL Walls Solid color=200,180,120");
    let tan = document::types::Color::Rgb {
        r: 200,
        g: 180,
        b: 120,
    };
    assert_eq!(
        store::read(&app.tabs[i].scene.document, "Walls").map(|fill| fill.color),
        Some(tan)
    );
    let fills = hatches(&app);
    assert_eq!(fills.len(), 1);
    assert_eq!(
        fills[0].common.color, tan,
        "the existing fill takes the colour"
    );
    let rectangle = rectangle(&app);
    let line_colour = app.tabs[i]
        .scene
        .document
        .get_entity(rectangle)
        .map(|shape| shape.common().color);
    assert_eq!(line_colour, Some(document::types::Color::ByLayer));
    // The Layer Manager column sets it too, as one undo step.
    let _ = app.update(Message::LayerFillColorSet(
        "Walls".into(),
        document::types::Color::Index(3),
    ));
    assert_eq!(
        hatches(&app)[0].common.color,
        document::types::Color::Index(3)
    );
    let _ = app.update(Message::Undo);
    assert_eq!(hatches(&app)[0].common.color, tan);
    // A shape drawn now gets the fill colour straight away.
    let _ = app.run_command_line("CIRCLE 30,0 3");
    assert!(hatches(&app).iter().all(|hatch| hatch.common.color == tan));
}

#[test]
fn a_fill_colour_that_is_not_a_colour_changes_nothing() {
    let mut app = walls();
    let i = app.active_tab;
    let _ = app.dispatch_command("LAYERFILL Walls Solid color=purple-ish");
    assert_eq!(
        store::read(&app.tabs[i].scene.document, "Walls").map(|fill| fill.color),
        Some(document::types::Color::ByLayer)
    );
}

#[test]
fn a_new_fill_style_starts_with_the_layer_colour_never_bylayer() {
    let mut app = fresh();
    let _ = app.run_command_line("-LAYER M Roof C 30 Roof ");
    let i = app.active_tab;
    let _ = app.update(Message::LayerFillSet("Roof".into(), "Solid".into()));
    assert_eq!(
        store::read(&app.tabs[i].scene.document, "Roof").map(|fill| fill.color),
        Some(document::types::Color::Index(30))
    );
    let _ = app.dispatch_command("LAYERFILL Roof Solid color=ByLayer");
    assert_eq!(
        store::read(&app.tabs[i].scene.document, "Roof").map(|fill| fill.color),
        Some(document::types::Color::Index(30)),
        "ByLayer is not a fill colour"
    );
}

fn palette(
    app: &mut OpenCADStudio,
    message: crate::ui::window::graphic_attributes::GraphicAttributesMsg,
) {
    let _ = app.update(Message::GraphicAttributes(message));
}

use crate::ui::window::graphic_attributes::{GraphicAttribute, GraphicAttributesMsg as Ga};

#[test]
fn with_nothing_selected_a_custom_fill_is_what_new_shapes_get() {
    let mut app = walls();
    let _ = app.run_command_line("-LAYER M Empty ");
    // L: Empty has no fill style, so nothing is filled.
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    assert!(hatches(&app).is_empty());
    // C: the palette's own fill, here solid red.
    palette(&mut app, Ga::Fill(GraphicAttribute::Solid));
    palette(&mut app, Ga::FillColor(document::types::Color::Index(1)));
    assert!(!app.graphic_attributes.creation_fill.by_layer);
    let _ = app.run_command_line("RECTANG 20,0 30,4");
    let fills = hatches(&app);
    assert_eq!(fills.len(), 1);
    assert!(fills[0].is_solid);
    assert_eq!(fills[0].common.color, document::types::Color::Index(1));
    assert_eq!(apply::hatch_mark(fills[0]), Some(apply::OVERRIDDEN));
    // Back to L: new shapes follow the layer again.
    palette(&mut app, Ga::FillByLayer);
    let _ = app.run_command_line("RECTANG 40,0 50,4");
    assert_eq!(hatches(&app).len(), 1);
}

#[test]
fn a_custom_fill_of_none_leaves_new_shapes_unfilled_on_a_filled_layer() {
    let mut app = walls();
    palette(&mut app, Ga::Fill(GraphicAttribute::None));
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    assert!(hatches(&app).is_empty());
}

#[test]
fn the_fill_mode_of_a_selection_is_l_c_or_v() {
    use apply::FillMode;
    let mut app = walls();
    let _ = app.run_command_line("RECTANG 0,0 10,4");
    let i = app.active_tab;
    let first = rectangle(&app);
    let document = |app: &OpenCADStudio| app.tabs[app.active_tab].scene.document.clone();
    assert_eq!(
        apply::fill_mode(&document(&app), &[first]),
        Some(FillMode::ByLayer)
    );
    let _ = app.run_command_line("RECTANG 20,0 30,4");
    let second = shapes(&app)
        .into_iter()
        .find(|shape| *shape != first)
        .unwrap();
    app.tabs[i].scene.select_entities(&[second]);
    palette(&mut app, Ga::Fill(GraphicAttribute::Hatch));
    assert_eq!(
        apply::fill_mode(&document(&app), &[second]),
        Some(FillMode::Custom)
    );
    assert_eq!(
        apply::fill_mode(&document(&app), &[first, second]),
        Some(FillMode::Varies)
    );
    // A line has no fill to judge.
    let _ = app.run_command_line("LINE 0,10 5,10 ");
    let line = app.tabs[i]
        .scene
        .document
        .entities()
        .find_map(|entity| matches!(entity, EntityType::Line(_)).then(|| entity.common().handle))
        .unwrap();
    assert_eq!(apply::fill_mode(&document(&app), &[line]), None);
    // L on the selection gives the custom one the layer's fill back.
    app.tabs[i].scene.deselect_all();
    app.tabs[i].scene.select_entities(&[second]);
    palette(&mut app, Ga::FillByLayer);
    assert_eq!(
        apply::fill_mode(&document(&app), &[second]),
        Some(FillMode::ByLayer)
    );
    assert!(hatches(&app).iter().all(|hatch| hatch.is_solid));
    // C keeps the fill and makes it the object's own.
    app.tabs[i].scene.select_entities(&[second]);
    palette(&mut app, Ga::FillCustom);
    assert_eq!(
        apply::fill_mode(&document(&app), &[second]),
        Some(FillMode::Custom)
    );
}

#[test]
fn a_shape_without_fill_is_l_on_a_layer_without_fill_and_c_on_one_with() {
    use apply::FillMode;
    let mut app = walls();
    let i = app.active_tab;
    let mut square = document::entities::LwPolyline::from_points(vec![
        document::types::Vector2::new(0.0, 0.0),
        document::types::Vector2::new(1.0, 0.0),
        document::types::Vector2::new(1.0, 1.0),
    ]);
    square.is_closed = true;
    square.common.layer = "Walls".into();
    let shape = app.tabs[i].scene.add_entity(EntityType::LwPolyline(square));
    let document = app.tabs[i].scene.document.clone();
    assert_eq!(
        apply::fill_mode(&document, &[shape]),
        Some(FillMode::Custom)
    );
    let _ = app.dispatch_command("LAYERFILL Walls None");
    let document = app.tabs[i].scene.document.clone();
    assert_eq!(
        apply::fill_mode(&document, &[shape]),
        Some(FillMode::ByLayer)
    );
}
