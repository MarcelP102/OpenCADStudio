//! Timing probe for the per-placement delay seen when placing blocks
//! repeatedly from the Blocks palette, and when copying / moving a block.
//! Run with:
//!   PERF=1 cargo test --lib placement_perf_probe -- --nocapture --test-threads=1
//! Not an assertion test: it prints where the time goes per click.

use crate::app::OpenCADStudio;
use crate::command::StepInput;
use codec::entities::{Circle, Line, Text};
use codec::types::{Transform, Vector3};
use codec::EntityType;
use glam::DVec3;
use std::time::Instant;

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

/// A drawing with `n` loose lines plus a block "B" of 40 lines + 4 circles.
fn drawing(n: usize) -> OpenCADStudio {
    let mut app = OpenCADStudio::new_for_test();
    app.automation_op(r#"{"op":"new"}"#);
    let i = app.active_tab;
    let side = (n as f64).sqrt().ceil() as usize;
    let lines: Vec<EntityType> = (0..n)
        .map(|k| {
            let (x, y) = ((k % side) as f64 * 3.0, (k / side) as f64 * 3.0);
            let mut l = Line::new();
            l.start = Vector3::new(x, y, 0.0);
            l.end = Vector3::new(x + 2.0, y + 1.0, 0.0);
            EntityType::Line(l)
        })
        .collect();
    let _ = app.tabs[i].scene.add_entities(lines);
    let mut members = Vec::new();
    for k in 0..40 {
        let mut l = Line::new();
        l.start = Vector3::new(-500.0 + k as f64, -500.0, 0.0);
        l.end = Vector3::new(-500.0 + k as f64, -460.0, 0.0);
        members.push(app.tabs[i].scene.add_entity(EntityType::Line(l)));
    }
    for k in 0..4 {
        let mut c = Circle::new();
        c.center = Vector3::new(-480.0 + 5.0 * k as f64, -480.0, 0.0);
        c.radius = 2.0;
        members.push(app.tabs[i].scene.add_entity(EntityType::Circle(c)));
    }
    let id = Transform::identity();
    app.tabs[i]
        .scene
        .create_block_from_entities(&members, "B", &id, &id)
        .expect("block B");
    // "BT": the same plus a text label, like a typical architectural symbol.
    let mut members = Vec::new();
    for k in 0..40 {
        let mut l = Line::new();
        l.start = Vector3::new(-700.0 + k as f64, -500.0, 0.0);
        l.end = Vector3::new(-700.0 + k as f64, -460.0, 0.0);
        members.push(app.tabs[i].scene.add_entity(EntityType::Line(l)));
    }
    let mut t = Text::new();
    t.insertion_point = Vector3::new(-690.0, -470.0, 0.0);
    t.height = 2.5;
    t.value = "LABEL".to_string();
    members.push(app.tabs[i].scene.add_entity(EntityType::Text(t)));
    app.tabs[i]
        .scene
        .create_block_from_entities(&members, "BT", &id, &id)
        .expect("block BT");
    // Plus some loose text in the drawing, as real plans carry.
    let texts: Vec<EntityType> = (0..n / 20)
        .map(|k| {
            let mut t = Text::new();
            t.insertion_point = Vector3::new((k % 100) as f64 * 9.0, -900.0 - (k / 100) as f64 * 4.0, 0.0);
            t.height = 2.0;
            t.value = format!("T{k}");
            EntityType::Text(t)
        })
        .collect();
    let _ = app.tabs[i].scene.add_entities(texts);
    app.refresh_block_palette();
    // Warm the resident wire set, as the first frame would.
    let _ = app.tabs[i].scene.hit_test_wires();
    app
}

/// One click: the command step, then the "next frame" resident rebuild.
fn click(app: &mut OpenCADStudio, label: &str, p: DVec3) {
    let i = app.active_tab;
    let t = Instant::now();
    let _ = app.feed_command(StepInput::Point(p));
    let cmd_ms = ms(t);
    let t = Instant::now();
    let wires = app.tabs[i].scene.hit_test_wires();
    let frame_ms = ms(t);
    eprintln!(
        "[probe] {label:<14} command={cmd_ms:>8.2}ms frame-wires={frame_ms:>8.2}ms wires={} entities={}",
        wires.len(),
        app.tabs[i].scene.document.entities().count(),
    );
}

#[test]
fn placement_perf_probe() {
    for &n in &[2_000usize, 50_000] {
        eprintln!("\n===== drawing with {n} lines =====");

        // Baseline: a repeating LINE (no blocks involved).
        let mut app = drawing(n);
        let _ = app.dispatch_command("LINE");
        click(&mut app, "line start", DVec3::new(0.0, -50.0, 0.0));
        for k in 1..5 {
            click(&mut app, "line seg", DVec3::new(10.0 * k as f64, -50.0, 0.0));
        }

        for block in ["B", "BT"] {
        eprintln!("--- block {block} ---");
        // Blocks palette, repeat on.
        let mut app = drawing(n);
        app.block_palette.options.repeat = true;
        app.block_palette.options.insertion_point = true;
        app.block_palette.options.scale = false;
        app.block_palette.options.rotation = false;
        app.start_block_placement(block);
        for k in 0..5 {
            click(&mut app, "palette place", DVec3::new(20.0 * k as f64, -100.0, 0.0));
        }
        {
            let i = app.active_tab;
            let ins = app.tabs[i]
                .scene
                .document
                .entities()
                .filter(|e| matches!(e, EntityType::Insert(_)))
                .map(|e| e.common().handle)
                .last();
            if let Some(h) = ins {
                let w = app.tabs[i].scene.wire_models_for(&[h]);
                eprintln!(
                    "[probe] insert wires={} instanced={} with_text={} with_fill={} names={:?}",
                    w.len(),
                    w.iter().filter(|w| w.render_instance.is_some()).count(),
                    w.iter().filter(|w| !w.text_verts.is_empty()).count(),
                    w.iter().filter(|w| !w.fill_tris.is_empty()).count(),
                    w.iter().take(3).map(|w| w.name.clone()).collect::<Vec<_>>(),
                );
            }
        }
        let t = Instant::now();
        let cfg = app.current_config();
        let json = serde_json::to_string_pretty(&cfg).unwrap_or_default();
        eprintln!(
            "[probe] settings.json build+serialize={:.2}ms bytes={}",
            ms(t),
            json.len()
        );

        // COPY of one INSERT, multiple placements.
        let i = app.active_tab;
        let _ = app.feed_command(StepInput::Enter);
        let insert = app.tabs[i]
            .scene
            .document
            .entities()
            .find_map(|e| matches!(e, EntityType::Insert(_)).then(|| e.common().handle))
            .expect("an INSERT");
        app.tabs[i].scene.deselect_all();
        app.tabs[i].scene.select_entity(insert, true);
        let _ = app.dispatch_command("COPY");
        click(&mut app, "copy base", DVec3::new(0.0, -100.0, 0.0));
        for k in 1..5 {
            click(&mut app, "copy place", DVec3::new(15.0 * k as f64, -150.0, 0.0));
        }
        let _ = app.feed_command(StepInput::Enter);

        // MOVE of one INSERT.
        app.tabs[i].scene.deselect_all();
        app.tabs[i].scene.select_entity(insert, true);
        let _ = app.dispatch_command("MOVE");
        click(&mut app, "move base", DVec3::new(0.0, -100.0, 0.0));
        click(&mut app, "move place", DVec3::new(5.0, -300.0, 0.0));
        }
    }
}
