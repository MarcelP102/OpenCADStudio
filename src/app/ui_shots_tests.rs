//! Screenshots of the real view, for checking UI changes against the design
//! at several scale factors and themes. Not a regression test: ignored by
//! default, run by hand.
//!
//! ```text
//! UI_SHOTS_DIR=/tmp/shots cargo test --lib ui_shots -- --ignored --nocapture
//! ```
//!
//! Each shot is drawn by the same headless renderer the dock tests use, at
//! 100 %, 125 % and 150 %, in Studio Dark and the light theme. The drawing
//! canvas itself (GPU) is not rendered there; the chrome around it is.

use crate::app::{Message, OpenCADStudio};

/// Logical size of the window the shots are laid out at.
const SIZE: iced::Size = iced::Size::new(1600.0, 900.0);
const SCALES: [f32; 3] = [1.0, 1.25, 1.5];

fn app() -> OpenCADStudio {
    let mut app = OpenCADStudio::new_for_test();
    app.automation_op(r#"{"op":"new"}"#);
    // Software snapshots have no scene GPU adapter: no hardware warning.
    app.gpu_warning_silenced = crate::scene::pipeline::gpu_status()
        .identity()
        .or_else(|| crate::scene::pipeline::GpuStatus::NoRenderer.identity())
        .unwrap();
    app.pending_startup_modals
        .retain(|modal| *modal != crate::app::ModalKind::GpuWarning);
    if app.active_modal == Some(crate::app::ModalKind::GpuWarning) {
        app.active_modal = None;
    }
    app.dock = Default::default();
    app.dock.ensure_settings();
    app.show_properties = true;
    app.show_block_palette = true;
    let i = app.active_tab;
    app.tabs[i].scene.selection.borrow_mut().view.vp_size = (SIZE.width, SIZE.height);
    app.win_size = (SIZE.width, SIZE.height).into();
    app
}

fn themes() -> [(&'static str, iced::Theme); 2] {
    [
        ("dark", crate::ui::style::studio_theme::studio_dark()),
        ("light", iced::Theme::Light),
    ]
}

/// Draw the app's view at `scale` in `theme` with the pointer at `cursor`
/// and save it as `<dir>/<name>-<theme>-<pct>.png`.
fn shot(app: &mut OpenCADStudio, dir: &std::path::Path, name: &str, cursor: Option<iced::Point>) {
    use iced_test::core::renderer::Headless as _;
    use iced_test::core::{mouse, renderer, shell, window};
    use iced_test::runtime::user_interface::{Cache, UserInterface};

    for (theme_name, theme) in themes() {
        app.active_theme = theme.clone();
        for scale in SCALES {
            crate::ui::icons::set_scale_factor(scale);
            let mut renderer = iced_test::futures::futures::executor::block_on(
                <iced::Renderer as iced_test::core::renderer::Headless>::new(
                    renderer::Settings::default(),
                    std::env::var("ICED_TEST_BACKEND").ok().as_deref(),
                ),
            )
            .expect("headless renderer");
            let cursor = cursor.map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available);
            let mut cache = Cache::default();
            // A first pass lets widgets that size themselves on redraw
            // (scrollables, measured strips) settle.
            for _ in 0..2 {
                let mut messages: Vec<Message> = Vec::new();
                let mut ui = UserInterface::build(
                    app.view_main(),
                    SIZE,
                    std::mem::take(&mut cache),
                    &mut renderer,
                );
                let _ = ui.update(
                    &window::Headless,
                    &shell::Waker::noop(),
                    &[iced::Event::Window(iced::window::Event::RedrawRequested(
                        std::time::Instant::now(),
                    ))],
                    cursor,
                    &mut renderer,
                    &mut messages,
                );
                cache = ui.into_cache();
                for message in messages {
                    let _ = app.update(message);
                }
            }
            let mut ui = UserInterface::build(app.view_main(), SIZE, cache, &mut renderer);
            let base = iced::theme::Base::base(&theme);
            ui.draw(
                &mut renderer,
                &theme,
                &renderer::Style {
                    text_color: base.text_color,
                },
                cursor,
            );
            let physical = iced::Size::new(
                (SIZE.width * scale).round() as u32,
                (SIZE.height * scale).round() as u32,
            );
            let rgba = renderer.screenshot(physical, scale, base.background_color);
            let path = dir.join(format!(
                "{name}-{theme_name}-{}.png",
                (scale * 100.0).round() as u32
            ));
            image::save_buffer(
                &path,
                &rgba,
                physical.width,
                physical.height,
                image::ExtendedColorType::Rgba8,
            )
            .expect("save screenshot");
            println!("{}", path.display());
        }
    }
    crate::ui::icons::set_scale_factor(1.0);
}

fn out_dir() -> std::path::PathBuf {
    let dir = std::env::var_os("UI_SHOTS_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("target/ui-shots"));
    std::fs::create_dir_all(&dir).expect("screenshot dir");
    dir
}

/// Run `f` on a thread with the application's stack, one headless renderer
/// at a time (see `dock::tests::with_stack`).
fn with_stack(f: impl FnOnce() + Send + 'static) {
    let _one = crate::headless_ui_lock();
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(f)
        .expect("spawn")
        .join()
        .unwrap_or_else(|e| std::panic::resume_unwind(e));
}

#[test]
#[ignore = "writes screenshots; run by hand"]
fn ui_shots() {
    with_stack(|| {
        let dir = out_dir();
        let only = std::env::var("UI_SHOTS_ONLY").ok();
        let wanted = |name: &str| {
            only.as_deref()
                .is_none_or(|o| o.split(',').any(|o| o == name))
        };

        if wanted("main") {
            let mut app = app();
            shot(&mut app, &dir, "main", None);
        }
        if wanted("properties") {
            let mut app = app();
            let _ = app.run_command_line("LINE 0,0 100,50 ");
            let _ = app.run_command_line("SELECT ALL ");
            shot(&mut app, &dir, "properties", None);
        }
        if wanted("layers") {
            let mut app = app();
            let _ = app.run_command_line("-LAYER N WALLS ");
            let _ = app.dispatch_command("LAYER");
            shot(&mut app, &dir, "layers", None);
        }
        if wanted("quickcalc") {
            let mut app = app();
            let _ = app.dispatch_command("QUICKCALC");
            shot(&mut app, &dir, "quickcalc", None);
        }
        if wanted("tabs") {
            let mut app = app();
            app.automation_op(r#"{"op":"new"}"#);
            app.automation_op(r#"{"op":"new"}"#);
            shot(&mut app, &dir, "tabs", None);
        }
    });
}
