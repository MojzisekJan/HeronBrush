//! The egui side of the performance benchmarks (`docs/performance.md`): a flat view of a
//! generated map drawn through [`crate::egui_canvas::EguiCanvas`] into egui shapes, and
//! those shapes tessellated, headless. Ignored by default:
//!
//! ```text
//! cargo test --release -p heron-brush perf_ -- --ignored --nocapture --test-threads=1
//! ```

use std::time::Instant;

use eframe::egui;
use editor_core::bench_support::{self as bench, MapSpec, Scratch, Stats};

use crate::editor::Editor;

fn sizes() -> Vec<usize> {
    std::env::var("HERON_BENCH_SIZES")
        .ok()
        .map(|s| s.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![1_000, 10_000, 50_000])
}

#[test]
#[ignore]
fn perf_egui_flat_view() {
    editor_core::preferences::set_current(Default::default());
    let scratch = Scratch::new("egui");
    let mut rows: Vec<Stats> = Vec::new();
    for n in sizes() {
        let runs = if n >= 50_000 { 3 } else { 5 };
        let path = scratch.0.join(format!("m{n}.map"));
        std::fs::write(&path, bench::synthetic_map(&MapSpec::brushes(n))).unwrap();
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let editor = Editor::open(&ctx, None);
        *editor.document.borrow_mut() = editor_core::document::Document::open(&path, Default::default()).unwrap();
        let mut pane = crate::view2d::Pane2d::new(&editor, editor_core::camera2d::ViewPlane::Top);
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0));
        let input = || egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))), ..Default::default() };
        // A first frame frames the map and warms the caches.
        let _ = ctx.run_ui(input(), |ui| pane.show(ui, rect));
        let mut shapes = 0;
        rows.push(bench::measure(&format!("{n}: Top view, whole map: draw into egui shapes (run_ui)"), runs, || (), |_| {
            let output = ctx.run_ui(input(), |ui| pane.show(ui, rect));
            shapes = output.shapes.len();
            output
        }));
        let output = ctx.run_ui(input(), |ui| pane.show(ui, rect));
        let primitives = std::cell::Cell::new(0);
        rows.push(bench::measure(&format!("{n}: Top view, whole map: tessellate ({shapes} shapes)"), runs, || output.shapes.clone(), |shapes| {
            let clipped = ctx.tessellate(shapes, 2.0);
            primitives.set(clipped.iter().map(|c| match &c.primitive {
                egui::epaint::Primitive::Mesh(mesh) => mesh.indices.len() / 3,
                _ => 0,
            }).sum::<usize>());
            clipped
        }));
        eprintln!("{n}: {shapes} shapes, {} triangles after tessellation", primitives.get());
        // Zoomed in on a corner, as one works: most of the map is off the view.
        pane.flat.state.borrow_mut().camera.zoom_at(16.0, geometry_math::vec::Vec2d::new(600.0, 400.0), (1200.0, 800.0));
        let start = Instant::now();
        let output = ctx.run_ui(input(), |ui| pane.show(ui, rect));
        let zoomed_shapes = output.shapes.len();
        drop(output);
        let _ = start;
        rows.push(bench::measure(&format!("{n}: Top view, zoomed 16x: draw + tessellate ({zoomed_shapes} shapes)"), runs, || (), |_| {
            let output = ctx.run_ui(input(), |ui| pane.show(ui, rect));
            ctx.tessellate(output.shapes, 2.0)
        }));
    }
    bench::print_table("egui flat view", &rows);
}

/// Whole frames of one flat view (draw into shapes, then tessellate, as eframe does) in
/// the situations a big map is used in: nothing changed (the 3D camera turning, the
/// pointer moving), a drag of 100 selected brushes, and a pan (the picture changes every
/// frame).
#[test]
#[ignore]
fn perf_egui_flat_view_frames() {
    editor_core::preferences::set_current(Default::default());
    let scratch = Scratch::new("egui-frames");
    let mut rows: Vec<Stats> = Vec::new();
    for n in sizes() {
        let runs = std::env::var("HERON_BENCH_RUNS").ok().and_then(|r| r.parse().ok()).unwrap_or(if n >= 50_000 { 10 } else { 20 });
        let path = scratch.0.join(format!("m{n}.map"));
        std::fs::write(&path, bench::synthetic_map(&MapSpec::brushes(n))).unwrap();
        let ctx = egui::Context::default();
        ctx.set_embed_viewports(true);
        crate::theme::install(&ctx);
        let editor = Editor::open(&ctx, None);
        *editor.document.borrow_mut() = editor_core::document::Document::open(&path, Default::default()).unwrap();
        let mut pane = crate::view2d::Pane2d::new(&editor, editor_core::camera2d::ViewPlane::Top);
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0));
        let input = || egui::RawInput { screen_rect: Some(rect), ..Default::default() };
        let frame = |pane: &mut crate::view2d::Pane2d| {
            let mut output = ctx.run_ui(input(), |ui| pane.show(ui, rect));
            output.textures_delta.clear();
            ctx.tessellate(output.shapes, 2.0).len()
        };
        frame(&mut pane);
        frame(&mut pane);
        rows.push(bench::measure(&format!("{n}: flat view frame, nothing changed"), runs, || (), |_| frame(&mut pane)));
        // 100 brushes in the middle selected and dragged a grid step per frame.
        {
            let mut document = editor.document.borrow_mut();
            let centre = document.bounds().unwrap().center();
            let mut brushes: Vec<(f64, map_model::node::NodeId)> = document
                .selectable_brushes()
                .into_iter()
                .map(|(id, b)| {
                    let d = b.bounds().center() - centre;
                    (geometry_math::vec::dot(d, d), id)
                })
                .collect();
            brushes.sort_by(|a, b| a.0.total_cmp(&b.0));
            let caught: Vec<_> = brushes.into_iter().take(100).map(|(_, id)| id).collect();
            let empty = document.selection_snapshot();
            document.preview_marquee(&empty, &caught, false);
            document.begin_transaction();
        }
        frame(&mut pane);
        rows.push(bench::measure(&format!("{n}: flat view frame, dragging 100 selected"), runs, || (), |_| {
            editor.document.borrow_mut().translate_selection(geometry_math::vec::Vec3d::new(8.0, 0.0, 0.0));
            frame(&mut pane)
        }));
        editor.document.borrow_mut().commit_transaction();
        rows.push(bench::measure(&format!("{n}: flat view frame, panning"), runs, || (), |_| {
            pane.flat.pan(3.0, 0.0);
            frame(&mut pane)
        }));
        let mut paired=[Vec::new(),Vec::new()];
        for sample in 0..runs+2 {
            pane.flat.pan(3.0,0.0);
            for path in if sample%2==0 {[0,1]}else{[1,0]} {
                pane.base.invalidate();
                let before=pane.base.rebuilds;
                let started=Instant::now();
                bench::with_legacy_line_paths(path==0,||frame(&mut pane));
                if sample>=2 {paired[path].push(started.elapsed());}
                assert_eq!(pane.base.rebuilds,before+1,"Both paths must rebuild the same base layer");
            }
        }
        for (path,mut times) in paired.into_iter().enumerate() {
            times.sort();rows.push(Stats{name:format!("{n}: paired full pan frame {}",if path==0 {"legacy paths"}else{"streamed segments"}),runs:times});
        }
    }
    bench::print_table("egui flat view, whole frames", &rows);
}

/// The Face tab's material browser over 2,000 / 5,000 / 10,000 materials
/// (`HERON_BENCH_TEXTURES`): the first frame, a frame with nothing changing, frames
/// scrolling through the grid, and a frame after typing a filter. Prints how much texture
/// data each uploaded.
#[test]
#[ignore]
fn perf_egui_material_browser() {
    editor_core::preferences::set_current(Default::default());
    let counts: Vec<usize> = std::env::var("HERON_BENCH_TEXTURES")
        .ok()
        .map(|s| s.split(',').filter_map(|n| n.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![2_000, 5_000, 10_000]);
    let mut rows: Vec<Stats> = Vec::new();
    for count in counts {
        let scratch = Scratch::new(&format!("egui-materials{count}"));
        bench::write_textures(&scratch.0, count, 0, 3).unwrap();
        let document = std::rc::Rc::new(std::cell::RefCell::new(editor_core::document::Document::new(Default::default())));
        let start = Instant::now();
        document.borrow_mut().load_materials(&scratch.0).unwrap();
        eprintln!("{count}: loaded in {:.0} ms", bench::ms(start.elapsed()));
        let uploaded = std::cell::Cell::new(0usize);
        let time = std::cell::Cell::new(0.0f64);
        let run = |ctx: &egui::Context, page: &mut crate::material_page::MaterialPage, events: Vec<egui::Event>| {
            time.set(time.get() + 1.0 / 60.0);
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420.0, 900.0))),
                time: Some(time.get()),
                events,
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| page.show(ui, &document));
            uploaded.set(uploaded.get() + output.textures_delta.set.iter().flat_map(|(_, deltas)| deltas.iter()).map(|delta| delta.image.width() * delta.image.height() * 4).sum::<usize>());
            output.textures_delta.clear();
            ctx.tessellate(output.shapes, 2.0).len()
        };
        let fresh = || {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx);
            ctx.all_styles_mut(|s| s.scroll_animation = egui::style::ScrollAnimation::none());
            (ctx, crate::material_page::MaterialPage::default())
        };
        rows.push(bench::measure(&format!("{count} materials: first frame of the browser"), 3, fresh, |(ctx, mut page)| {
            uploaded.set(0);
            run(&ctx, &mut page, Vec::new());
            (ctx, page)
        }));
        eprintln!("{count}: the first frame uploaded {:.1} MB of textures", uploaded.get() as f64 / 1e6);
        let (ctx, mut page) = fresh();
        let start = Instant::now();
        let mut frames = 0;
        for _ in 0..200 {
            run(&ctx, &mut page, Vec::new());
            frames += 1;
            if !ctx.has_requested_repaint() {
                break;
            }
        }
        eprintln!("{count}: {frames} frames until every thumbnail in view was made ({:.0} ms)", bench::ms(start.elapsed()));
        rows.push(bench::measure(&format!("{count} materials: frame, nothing changing"), 20, || (), |_| run(&ctx, &mut page, Vec::new())));
        // Scrolling: the pointer over the grid, the wheel turned a few lines a frame.
        let hover = egui::Event::PointerMoved(egui::pos2(200.0, 300.0));
        run(&ctx, &mut page, vec![hover.clone()]);
        uploaded.set(0);
        let wheel = || egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -240.0), phase: egui::TouchPhase::Move, modifiers: egui::Modifiers::NONE };
        rows.push(bench::measure(&format!("{count} materials: frame while scrolling the grid"), 60, || (), |_| run(&ctx, &mut page, vec![hover.clone(), wheel()])));
        eprintln!("{count}: 60 scrolling frames uploaded {:.1} MB of textures", uploaded.get() as f64 / 1e6);
        drop(page);
        drop(ctx);
    }
    bench::print_table("egui material browser", &rows);
}

/// The inspector's Map tab (layers, groups, issues, view filters) on a map of 5,000 groups
/// and on one of 200 layers, frame by frame with nothing changing.
#[test]
#[ignore]
fn perf_egui_map_tab() {
    editor_core::preferences::set_current(Default::default());
    let scratch = Scratch::new("egui-map-tab");
    let mut rows: Vec<Stats> = Vec::new();
    for (name, text) in [
        ("5k groups", bench::grouped_map(&bench::GroupSpec::default())),
        ("200 layers", bench::layered_map(200, 20_000)),
        ("20k entities", bench::point_entity_map(20_000, 5_000)),
    ] {
        let path = scratch.0.join("map.map");
        std::fs::write(&path, text).unwrap();
        let document = std::rc::Rc::new(std::cell::RefCell::new(editor_core::document::Document::open(&path, Default::default()).unwrap()));
        let filters = std::rc::Rc::new(std::cell::RefCell::new(editor_core::view_filters::ViewFilters::default()));
        let ctx = egui::Context::default();
        crate::theme::install(&ctx);
        let mut page = crate::map_page::MapPage::default();
        let frame = |page: &mut crate::map_page::MapPage| {
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420.0, 1000.0))), ..Default::default() };
            let mut output = ctx.run_ui(input, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| page.show(ui, &document, &filters));
            });
            output.textures_delta.clear();
            ctx.tessellate(output.shapes, 2.0).len()
        };
        frame(&mut page);
        frame(&mut page);
        rows.push(bench::measure(&format!("{name}: Map tab frame"), 20, || (), |_| frame(&mut page)));
    }
    bench::print_table("egui Map tab", &rows);
}

/// The inspector's Entity tab over a map of 20,000 point entities: nothing selected, one
/// selected, and 5,000 lights selected together.
#[test]
#[ignore]
fn perf_egui_entity_tab() {
    editor_core::preferences::set_current(Default::default());
    let scratch = Scratch::new("egui-entity-tab");
    let path = scratch.0.join("map.map");
    std::fs::write(&path, bench::point_entity_map(20_000, 5_000)).unwrap();
    let ctx = egui::Context::default();
    ctx.set_embed_viewports(true);
    crate::theme::install(&ctx);
    let editor = Editor::open(&ctx, None);
    *editor.document.borrow_mut() = editor_core::document::Document::open(&path, Default::default()).unwrap();
    let mut page = crate::entity_page::EntityPage::default();
    let frame = |page: &mut crate::entity_page::EntityPage| {
        let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(420.0, 1000.0))), ..Default::default() };
        let mut output = ctx.run_ui(input, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| page.show(ui, &editor.document, &editor.grid));
        });
        output.textures_delta.clear();
        ctx.tessellate(output.shapes, 2.0).len()
    };
    let lights: Vec<map_model::node::NodeId> = map_model::node::point_entities(editor.document.borrow().tree_world())
        .into_iter()
        .filter(|(_, e)| e.classname() == Some("light"))
        .map(|(id, _)| id)
        .collect();
    let mut rows: Vec<Stats> = Vec::new();
    for (what, count) in [("nothing selected", 0usize), ("one light selected", 1), ("5,000 lights selected", 5_000)] {
        {
            let mut document = editor.document.borrow_mut();
            let empty = document.selection_snapshot();
            document.preview_marquee(&empty, &lights[..count.min(lights.len())], false);
        }
        frame(&mut page);
        rows.push(bench::measure(&format!("20k entities: Entity tab frame, {what}"), 10, || (), |_| frame(&mut page)));
    }
    bench::print_table("egui Entity tab", &rows);
}
