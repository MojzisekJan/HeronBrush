//! Renders small maps on whatever adapter the machine has, without a window: the
//! shaders validate (wgpu panics on a validation error), every display mode and every
//! effect draws something sensible, the caches hold, and a selection click is a
//! state-only update.

use editor_core::document::Document;
use editor_core::render_scene::{Pose, SceneTracker};
use editor_core::view_filters::ViewFilters;
use render::camera3d::Camera3d;
use render_wgpu::{DisplayMode, Renderer, Tonemapper, ViewParams};

fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok()?;
    if std::env::var("HERON_REQUIRE_SOFTWARE_GPU").as_deref() == Ok("1") {
        let info = adapter.get_info();
        assert_eq!(
            info.device_type,
            wgpu::DeviceType::Cpu,
            "Software material checks require a CPU adapter: {info:?}"
        );
        eprintln!("Software GPU: {} ({:?})", info.name, info.backend);
    }
    if std::env::var_os("RENDER_WGPU_BIG").is_some() {
        let info = adapter.get_info();
        eprintln!(
            "Benchmark GPU: {} ({:?}, {:?})",
            info.name, info.backend, info.device_type
        );
    }
    // Timestamps when the adapter has them, so the profiling path is exercised too.
    let required_features = adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features,
        ..Default::default()
    }))
    .ok()
}

fn open(name: &str) -> Document {
    let path = format!("{}/../../{name}", env!("CARGO_MANIFEST_DIR"));
    Document::open(&path, editor_core::world::WorldBounds::default())
        .unwrap_or_else(|e| panic!("{name} opens: {e:?}"))
}

fn save(name: &str, (w, h, pixels): &(u32, u32, Vec<u8>)) {
    if let Some(dir) = std::env::var_os("RENDER_WGPU_SHOTS") {
        let path = std::path::Path::new(&dir).join(format!("{name}.png"));
        image::save_buffer(&path, pixels, *w, *h, image::ColorType::Rgba8).expect("saved");
    }
}

/// The preferences' render settings without the temporal filter, for tests that compare
/// pictures frame by frame: each frame then shows only its own settings.
fn single_frame_settings() -> render::RenderSettings {
    let mut s = editor_core::render_settings::from_preferences();
    s.antialiasing = false;
    s.accumulate = false;
    s
}

/// Draws `params` from a fresh history until the temporal filter converges, as a still
/// view in the app does; returns the frames it took.
fn settle(renderer: &mut Renderer, params: &ViewParams<'_>) -> u32 {
    renderer.reset_history();
    let mut frames = 0;
    loop {
        renderer.render(&ViewParams {
            continuing: frames > 0,
            ..*params
        });
        frames += 1;
        if renderer.converged() || frames > 256 {
            return frames;
        }
    }
}

/// Mean luminance (0..255) of the picture.
fn mean(pixels: &[u8]) -> f64 {
    let sum: f64 = pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| 0.2126 * f64::from(p[0]) + 0.7152 * f64::from(p[1]) + 0.0722 * f64::from(p[2]))
        .sum();
    sum / (pixels.len() / 4) as f64
}

fn view<'a>(
    camera: &'a Camera3d,
    settings: &'a render::RenderSettings,
    mode: DisplayMode,
    tonemapper: Tonemapper,
    size: (u32, u32),
) -> ViewParams<'a> {
    ViewParams {
        camera,
        size,
        settings,
        mode,
        tonemapper,
        points: &[],
        guides: &[],
        world_bounds: None,
        outline_width: 2,
        sample: None,
        lights: true,
        continuing: false,
    }
}

#[test]
fn draws_every_mode_and_selects_by_state() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let mut document = open("assets/crate.map");
    let mut renderer = Renderer::new(&device, &queue);
    renderer.set_profiling(true);
    let mut tracker = SceneTracker::new();
    let filters = ViewFilters::default();
    tracker.sync(&document, &filters, None, Pose::Rest, &mut renderer.scene);
    assert!(tracker.stats.full && tracker.stats.upserted > 0);

    let mut camera = Camera3d::default();
    camera.frame(&document.bounds().expect("the map has bounds"));
    let mut settings = single_frame_settings();
    settings.procedural_sky = false;
    settings.background_color = [0.0, 0.0, 0.0];
    settings.shadows = true;
    settings.ambient_occlusion = true;
    settings.bloom = true;
    for mode in [
        DisplayMode::Material,
        DisplayMode::Solid,
        DisplayMode::Wireframe,
    ] {
        renderer.render(&view(&camera, &settings, mode, Tonemapper::Agx, (320, 200)));
        let shot = renderer.read_output().expect("output");
        save(&format!("headless-{mode:?}"), &shot);
        let (w, h, pixels) = &shot;
        let lit = pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[0] > 8 || p[1] > 8 || p[2] > 8)
            .count();
        assert!(
            lit > (w * h / 50) as usize,
            "{mode:?}: only {lit} of {} pixels drawn",
            w * h
        );
        if mode == DisplayMode::Material {
            // The first material frame builds the environment and every cascade.
            assert!(renderer.stats.environment_rebuilt);
            assert_eq!(renderer.stats.cascades_drawn, 4);
            assert!(renderer.stats.sun_shadow_draws > 0);
            assert!(renderer.stats.prepass_draws > 0);
            assert!(
                renderer.stats.draws
                    >= renderer.stats.sun_shadow_draws
                        + renderer.stats.light_shadow_draws
                        + renderer.stats.prepass_draws
            );
            let times = renderer.gpu_times().to_vec();
            if device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
                for group in ["shadows", "prepass", "ao", "forward", "bloom", "post"] {
                    assert!(
                        times.iter().any(|(label, _)| *label == group),
                        "no GPU time for {group}: {times:?}"
                    );
                }
            }
        }
    }

    // The same view again: environment and shadows are cached.
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        (320, 200),
    ));
    assert!(!renderer.stats.environment_rebuilt);
    assert_eq!(
        renderer.stats.cascades_drawn, 0,
        "an unchanged view redrew shadow cascades"
    );
    assert_eq!(renderer.stats.sun_shadow_draws, 0);

    let id = document.selectable_brushes()[0].0;
    document.select_only(id);
    tracker.sync(&document, &filters, None, Pose::Rest, &mut renderer.scene);
    assert!(!tracker.stats.full);
    assert_eq!(tracker.stats.upserted, 0, "{:?}", tracker.stats);
    assert_eq!(tracker.stats.states, 1);
    assert_eq!(
        renderer.scene.stats.chunks, 0,
        "a selection click rebuilt chunks"
    );
    // Nor does a selection click redraw a shadow cascade.
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        (320, 200),
    ));
    assert_eq!(
        renderer.stats.cascades_drawn, 0,
        "a selection click redrew shadow cascades"
    );
    // Nor the prepass and the occlusion: only the shading changed.
    assert!(
        renderer.stats.prepass_cached,
        "a selection click redrew the prepass"
    );
    assert_eq!(renderer.stats.prepass_draws, 0);

    // Moving the sun rebuilds the environment and every cascade.
    settings.sun_azimuth += 40.0;
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        (320, 200),
    ));
    assert!(renderer.stats.environment_rebuilt);
    assert_eq!(renderer.stats.cascades_drawn, 4);
}

/// Each effect changes the picture, and each tone map gives a sensible exposure.
#[test]
fn effects_and_tone_maps_change_the_picture() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let document = open("assets/crate.map");
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d::default();
    camera.turn(0.6, -0.35);
    camera.frame(&document.bounds().expect("bounds"));
    let base = {
        let mut s = single_frame_settings();
        s.procedural_sky = true;
        s.shadows = true;
        s.ambient_occlusion = true;
        s.bloom = true;
        s
    };
    let size = (256, 192);
    let mut draw = |settings: &render::RenderSettings, tonemapper: Tonemapper, name: &str| {
        renderer.render(&view(
            &camera,
            settings,
            DisplayMode::Material,
            tonemapper,
            size,
        ));
        let shot = renderer.read_output().expect("output");
        save(name, &shot);
        shot.2
    };
    let all = draw(&base, Tonemapper::Agx, "effects-all");
    let difference =
        |a: &[u8], b: &[u8]| a.iter().zip(b).filter(|(x, y)| x.abs_diff(**y) > 2).count();
    for (name, change) in [
        (
            "effects-no-shadows",
            Box::new(|s: &mut render::RenderSettings| s.shadows = false)
                as Box<dyn Fn(&mut render::RenderSettings)>,
        ),
        (
            "effects-no-ao",
            Box::new(|s: &mut render::RenderSettings| s.ambient_occlusion = false),
        ),
        (
            "effects-no-contact-shadows",
            Box::new(|s: &mut render::RenderSettings| s.contact_shadows = false),
        ),
    ] {
        let mut settings = base.clone();
        change(&mut settings);
        let other = draw(&settings, Tonemapper::Agx, name);
        assert!(
            difference(&all, &other) > 50,
            "{name} looks the same as with every effect"
        );
    }
    // Nothing in this scene passes the default threshold; with a low one, bloom shows.
    let mut low = base.clone();
    low.bloom_threshold = 0.1;
    let glowing = draw(&low, Tonemapper::Agx, "effects-low-threshold");
    low.bloom = false;
    let plain = draw(&low, Tonemapper::Agx, "effects-no-bloom");
    assert!(difference(&glowing, &plain) > 50, "bloom changes nothing");
    for tonemapper in [
        Tonemapper::Aces,
        Tonemapper::Agx,
        Tonemapper::KhronosNeutral,
        Tonemapper::TonyMcMapface,
    ] {
        let pixels = draw(&base, tonemapper, &format!("tonemap-{tonemapper:?}"));
        let m = mean(&pixels);
        assert!(
            (20.0..235.0).contains(&m),
            "{tonemapper:?}: mean luminance {m}"
        );
    }
}

/// An offline render: jittered samples average to a smoother picture than one frame, and
/// depth of field blurs what is out of focus.
#[test]
fn accumulates_samples_with_depth_of_field() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let document = open("assets/crate.map");
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d::default();
    camera.frame(&document.bounds().expect("bounds"));
    let mut settings = editor_core::render_settings::from_preferences();
    settings.antialiasing = false;
    let size = (200, 150);
    let mut sharp = view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    );
    sharp.sample = Some(0);
    renderer.render(&sharp);
    let one = renderer.read_output().expect("output").2;
    for sample in 0..16 {
        let mut params = view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Agx,
            size,
        );
        params.sample = Some(sample);
        renderer.accumulate(&params);
    }
    let averaged = renderer.read_output().expect("output");
    save("accumulated", &averaged);
    // Edges soften: a step between neighbours becomes a ramp, which the sum of squared
    // differences (unlike the sum of differences) sees as smaller.
    let roughness = |pixels: &[u8]| -> u64 {
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(pixels.as_chunks::<4>().0.iter().skip(1))
            .map(|(a, b)| u64::from(a[1].abs_diff(b[1])).pow(2))
            .sum()
    };
    assert!(
        roughness(&averaged.2) < roughness(&one),
        "averaging did not smooth the edges"
    );
    let mean_difference = one
        .iter()
        .zip(&averaged.2)
        .map(|(a, b)| f64::from(a.abs_diff(*b)))
        .sum::<f64>()
        / one.len() as f64;
    assert!(
        mean_difference < 8.0,
        "the average drifted from a single sample by {mean_difference}"
    );

    settings.depth_of_field = Some(render::DepthOfField {
        focus_distance: 1.0,
        focus_range: 0.0,
        radius: 24.0,
    });
    for sample in 0..4 {
        let mut params = view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Agx,
            size,
        );
        params.sample = Some(sample);
        renderer.accumulate(&params);
    }
    let blurred = renderer.read_output().expect("output");
    save("depth-of-field", &blurred);
    assert!(
        roughness(&blurred.2) * 10 < roughness(&averaged.2) * 9,
        "depth of field did not blur"
    );
}

/// A point entity whose class names an `iconsprite` is drawn as a camera-facing sprite.
#[test]
fn draws_point_entity_sprites() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let game = std::env::temp_dir().join(format!("render-wgpu-sprite-{}", std::process::id()));
    std::fs::create_dir_all(game.join("id1/sprites")).expect("a game directory");
    // A Half-Life sprite (IDSP v2) with its own palette: 4x2 texels of a light grey.
    let mut spr = Vec::new();
    spr.extend_from_slice(b"IDSP");
    for v in [2i32, 2, 3] {
        spr.extend_from_slice(&v.to_le_bytes());
    }
    spr.extend_from_slice(&1.0f32.to_le_bytes());
    for v in [4i32, 2, 1] {
        spr.extend_from_slice(&v.to_le_bytes());
    }
    spr.extend_from_slice(&0.0f32.to_le_bytes());
    spr.extend_from_slice(&0i32.to_le_bytes());
    spr.extend_from_slice(&256i16.to_le_bytes());
    spr.extend_from_slice(&[200u8; 768]);
    for v in [0i32, 0, 0, 4, 2] {
        spr.extend_from_slice(&v.to_le_bytes());
    }
    spr.extend_from_slice(&[1u8; 8]);
    std::fs::write(game.join("id1/sprites/bulb.spr"), &spr).expect("a written sprite");
    let previous_game = editor_core::preferences::pref(|p| {
        (p.game.clone(), p.game_path.clone(), p.game_mods.clone())
    });
    editor_core::preferences::edit(|p| {
        p.game = "Quake".into();
        p.game_path = game.display().to_string();
        p.game_mods.clear();
    });
    let fgd = game.join("test.fgd");
    std::fs::write(
        &fgd,
        "@PointClass iconsprite(\"sprites/bulb.spr\") = light : \"L\" []\n",
    )
    .expect("a written fgd");
    let mut document = Document::new(editor_core::world::WorldBounds::default());
    document.load_definitions(&fgd).expect("definitions load");
    document.create_point_entity("light", geometry_math::vec::Vec3d::new(0.0, 0.0, 0.0));
    editor_core::preferences::edit(|p| {
        (p.game, p.game_path, p.game_mods) = previous_game;
    });
    let _ = std::fs::remove_dir_all(&game);
    assert_eq!(document.sprites().len(), 1);

    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d {
        position: geometry_math::vec::Vec3d::new(-40.0, 0.0, 0.0),
        ..Default::default()
    };
    camera.look_along(geometry_math::vec::Vec3d::new(1.0, 0.0, 0.0));
    let mut settings = editor_core::render_settings::from_preferences();
    settings.procedural_sky = false;
    settings.background_color = [0.0, 0.0, 0.0];
    // The sprite's texture is streamed in: frames are drawn until it is resident.
    let start = std::time::Instant::now();
    loop {
        renderer.render(&view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Agx,
            (200, 200),
        ));
        renderer.wait_idle();
        if !renderer.streaming() {
            break;
        }
        assert!(
            start.elapsed().as_secs() < 20,
            "the sprite's texture never arrived"
        );
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(renderer.scene.texture_stats.resident >= 1);
    let shot = renderer.read_output().expect("output");
    save("sprite", &shot);
    let lit = shot
        .2
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 100)
        .count();
    assert!(lit > 400, "the sprite covers only {lit} pixels");
}

/// A larger picture of any map, for looking at: `RENDER_WGPU_MAP=demos/x/x.map
/// RENDER_WGPU_SHOTS=dir cargo test -p render_wgpu --test headless gallery -- --ignored`.
#[test]
#[ignore]
fn gallery() {
    let Some((device, queue)) = device() else {
        return;
    };
    let map = std::env::var("RENDER_WGPU_MAP")
        .unwrap_or_else(|_| "demos/lantern_lane/lantern_lane.map".into());
    let document = open(&map);
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d::default();
    let yaw: f64 = std::env::var("RENDER_WGPU_YAW")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.8);
    let pitch: f64 = std::env::var("RENDER_WGPU_PITCH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(-0.3);
    camera.turn(yaw, pitch);
    let bounds = document.bounds().expect("bounds");
    camera.frame(&bounds);
    // A fraction of the way to the map's centre.
    let zoom: f64 = std::env::var("RENDER_WGPU_ZOOM")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    camera.fly(
        geometry_math::vec::length(bounds.center() - camera.position) * zoom,
        0.0,
        0.0,
    );
    let settings = editor_core::render_settings::from_preferences();
    let size = (1280, 800);
    for tonemapper in [
        Tonemapper::Agx,
        Tonemapper::Aces,
        Tonemapper::KhronosNeutral,
        Tonemapper::TonyMcMapface,
    ] {
        settle(
            &mut renderer,
            &view(&camera, &settings, DisplayMode::Material, tonemapper, size),
        );
        save(
            &format!("gallery-{tonemapper:?}"),
            &renderer.read_output().expect("output"),
        );
    }
    for (name, change) in [
        (
            "gallery-no-shadows",
            Box::new(|s: &mut render::RenderSettings| s.shadows = false)
                as Box<dyn Fn(&mut render::RenderSettings)>,
        ),
        (
            "gallery-no-ao",
            Box::new(|s: &mut render::RenderSettings| s.ambient_occlusion = false),
        ),
        (
            "gallery-no-bloom",
            Box::new(|s: &mut render::RenderSettings| s.bloom = false),
        ),
        (
            "gallery-sunset",
            Box::new(|s: &mut render::RenderSettings| s.sun_elevation = 8.0),
        ),
        // Night: the sun under the horizon, so light entities carry the picture.
        (
            "gallery-night",
            Box::new(|s: &mut render::RenderSettings| {
                s.sun_elevation = -10.0;
                s.sun_intensity = 0.0;
                s.ambient_intensity *= 0.25;
            }),
        ),
    ] {
        let mut s = settings.clone();
        change(&mut s);
        settle(
            &mut renderer,
            &view(&camera, &s, DisplayMode::Material, Tonemapper::Agx, size),
        );
        save(name, &renderer.read_output().expect("output"));
    }
    settle(
        &mut renderer,
        &view(
            &camera,
            &settings,
            DisplayMode::Solid,
            Tonemapper::Agx,
            size,
        ),
    );
    save("gallery-solid", &renderer.read_output().expect("output"));
}

/// Frame cost per effect, GPU included (each frame waits for the GPU), with the camera
/// turning so the shadow cascades are redrawn as in a real orbit; then a still camera,
/// and selection clicks (an object's state changing under a still camera). Prints the
/// GPU time per pass where the device has timestamps:
/// `cargo test --release -p render_wgpu --test headless bench -- --ignored --nocapture`.
#[test]
#[ignore]
fn bench() {
    let Some((device, queue)) = device() else {
        return;
    };
    let map = std::env::var("RENDER_WGPU_MAP")
        .unwrap_or_else(|_| "demos/lantern_lane/lantern_lane.map".into());
    let document = open(&map);
    let mut renderer = Renderer::new(&device, &queue);
    renderer.set_profiling(true);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let clicked = document.selectable_brushes()[0].0 .0;
    // Every effect on, whatever the preferences say.
    let mut base = editor_core::render_settings::from_preferences();
    base.shadows = true;
    base.ambient_occlusion = true;
    base.bloom = true;
    base.contact_shadows = true;
    base.reflections = true;
    base.antialiasing = true;
    base.accumulate = true;
    let size = (1058, 808);
    /// Turns the base settings into one configuration to time.
    type Configure = fn(&mut render::RenderSettings);
    let configurations: [(&str, Configure); 13] = [
        ("everything", |_| {}),
        ("no light entities", |_| {}),
        ("no bloom", |s| s.bloom = false),
        ("no ao", |s| s.ambient_occlusion = false),
        ("no shadows", |s| s.shadows = false),
        ("no contact shadows", |s| s.contact_shadows = false),
        ("no reflections", |s| s.reflections = false),
        ("no temporal filter", |s| {
            s.antialiasing = false;
            s.accumulate = false;
        }),
        ("phase 2's effects", |s| {
            s.contact_shadows = false;
            s.reflections = false;
            s.antialiasing = false;
            s.accumulate = false;
        }),
        ("nothing", |s| {
            s.bloom = false;
            s.ambient_occlusion = false;
            s.shadows = false;
            s.reflections = false;
            s.antialiasing = false;
            s.accumulate = false;
        }),
        ("still (cached shadows)", |_| {}),
        ("selection clicks", |_| {}),
        ("selection clicks, phase 2's effects", |s| {
            s.contact_shadows = false;
            s.reflections = false;
            s.antialiasing = false;
            s.accumulate = false;
        }),
    ];
    for (name, change) in configurations {
        let mut settings = base.clone();
        change(&mut settings);
        let still = name.starts_with("still") || name.starts_with("selection");
        let mut times = Vec::new();
        let mut passes: Vec<(&'static str, f64)> = Vec::new();
        for frame in 0..120 {
            let mut camera = Camera3d::default();
            if !still {
                camera.turn(f64::from(frame) * 0.05, 0.0);
            }
            camera.frame(&document.bounds().expect("bounds"));
            let started = std::time::Instant::now();
            if name.starts_with("selection") {
                use editor_core::render_scene::SceneSink;
                renderer.scene.set_state(
                    clicked,
                    editor_core::render_scene::ObjectState {
                        selected: frame % 2 == 0,
                        ..Default::default()
                    },
                );
                renderer.scene.finish();
            }
            let mut params = view(
                &camera,
                &settings,
                DisplayMode::Material,
                Tonemapper::Agx,
                size,
            );
            // A still camera with nothing changing refines the temporal filter's picture.
            params.continuing = name.starts_with("still");
            params.lights = name != "no light entities";
            renderer.render(&params);
            renderer.wait_idle();
            if frame >= 10 {
                times.push(started.elapsed().as_secs_f64() * 1000.0);
                for (label, ms) in renderer.gpu_times() {
                    match passes.iter_mut().find(|(l, _)| l == label) {
                        Some((_, total)) => *total += ms,
                        None => passes.push((label, *ms)),
                    }
                }
            }
        }
        times.sort_by(f64::total_cmp);
        let gpu: Vec<String> = passes
            .iter()
            .map(|(label, total)| format!("{label} {:.2}", total / times.len() as f64))
            .collect();
        println!(
            "{name:>36}: median {:.2} ms, p95 {:.2} ms; gpu ms: {}",
            times[times.len() / 2],
            times[times.len() * 95 / 100],
            gpu.join(", ")
        );
    }
}

/// The temporal filter: a still view converges after the accumulation count and comes
/// out smoother than one frame; moving the camera keeps filtering without converging.
#[test]
fn temporal_filter_converges_for_a_still_view() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let document = open("assets/crate.map");
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d::default();
    camera.turn(0.6, -0.35);
    camera.frame(&document.bounds().expect("bounds"));
    let size = (240, 160);
    let mut plain = single_frame_settings();
    plain.procedural_sky = false;
    renderer.render(&view(
        &camera,
        &plain,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    ));
    assert!(
        renderer.converged(),
        "without the filter there is nothing to converge"
    );
    let one = renderer.read_output().expect("output");

    let mut settings = plain.clone();
    settings.antialiasing = true;
    settings.accumulate = true;
    settings.accumulation_samples = 16;
    let frames = settle(
        &mut renderer,
        &view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Agx,
            size,
        ),
    );
    assert_eq!(frames, 16);
    let settled = renderer.read_output().expect("output");
    save("taa-settled", &settled);
    let roughness = |pixels: &[u8]| -> u64 {
        pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip(pixels.as_chunks::<4>().0.iter().skip(1))
            .map(|(a, b)| u64::from(a[1].abs_diff(b[1])).pow(2))
            .sum()
    };
    assert!(
        roughness(&settled.2) < roughness(&one.2),
        "the settled picture is no smoother than one frame"
    );
    let drift = one
        .2
        .iter()
        .zip(&settled.2)
        .map(|(a, b)| f64::from(a.abs_diff(*b)))
        .sum::<f64>()
        / one.2.len() as f64;
    assert!(
        drift < 8.0,
        "the settled picture drifted from one frame by {drift}"
    );

    // A turning camera filters every frame and never counts as converged.
    for step in 1..6 {
        let mut turned = camera;
        turned.turn(0.01 * f64::from(step), 0.0);
        renderer.render(&view(
            &turned,
            &settings,
            DisplayMode::Material,
            Tonemapper::Agx,
            size,
        ));
        assert!(!renderer.converged());
    }
    // A change with a still camera keeps filtering too, and settles again. Its first
    // frame repeats the last jitter, so the prepass is kept when nothing moved.
    let mut params = view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    );
    renderer.render(&params);
    renderer.render(&params);
    assert!(
        renderer.stats.prepass_cached,
        "a change under a still camera redrew the prepass"
    );
    params.continuing = true;
    let mut frames = 1;
    while !renderer.converged() {
        renderer.render(&params);
        frames += 1;
        assert!(frames < 64);
    }
}

/// A floor and a pillar lit only by light entities: a point light that casts a shadow and
/// a spot pointing down.
fn lit_room() -> (Document, map_model::node::NodeId) {
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;
    let mut document = Document::new(editor_core::world::WorldBounds::default());
    let world = document.world_bounds();
    let cuboid = |min: [f64; 3], max: [f64; 3]| {
        map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(
                Vec3d::new(min[0], min[1], min[2]),
                Vec3d::new(max[0], max[1], max[2]),
            ),
            "floor",
        )
        .expect("a box")
    };
    document.add_brushes(vec![
        cuboid([-320.0, -320.0, -16.0], [320.0, 320.0, 0.0]),
        cuboid([-16.0, -16.0, 0.0], [16.0, 16.0, 128.0]),
    ]);
    let point = document.create_point_entity("light", Vec3d::new(96.0, 0.0, 48.0));
    for (key, value) in [
        ("light_color", "255 200 150"),
        ("light_energy", "4"),
        ("omni_range", "10"),
    ] {
        document.set_entity_property_on(&[point], key, value);
    }
    let spot = document.create_point_entity("light_spot", Vec3d::new(-160.0, 96.0, 160.0));
    for (key, value) in [
        ("light_color", "120 160 255"),
        ("light_energy", "6"),
        ("spot_range", "10"),
        ("spot_angle", "30"),
        ("angles", "90 0 0"),
        ("shadow_enabled", "0"),
    ] {
        document.set_entity_property_on(&[spot], key, value);
    }
    document.clear_selection();
    (document, point)
}

/// A smooth floor reflects the pillar standing on it when screen-space reflections are on.
#[test]
fn reflections_show_the_pillar_in_a_smooth_floor() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let (document, _) = lit_room();
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d {
        position: geometry_math::vec::Vec3d::new(-300.0, -20.0, 60.0),
        ..Default::default()
    };
    camera.look_along(geometry_math::vec::Vec3d::new(300.0, 20.0, -20.0));
    let mut settings = single_frame_settings();
    settings.procedural_sky = true;
    settings.default_roughness = 0.05;
    settings.default_metalness = 1.0;
    settings.reflections = true;
    settings.reflection_strength = 1.0;
    let size = (320, 200);
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    ));
    let reflected = renderer.read_output().expect("output");
    save("reflections-on", &reflected);
    settings.reflections = false;
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    ));
    let plain = renderer.read_output().expect("output");
    save("reflections-off", &plain);
    // Only the floor below the horizon line changes: the pillar replaces the sky there.
    let changed = reflected
        .2
        .as_chunks::<4>()
        .0
        .iter()
        .zip(plain.2.as_chunks::<4>().0)
        .filter(|(a, b)| a[0].abs_diff(b[0]) > 12)
        .count();
    assert!(changed > 300, "reflections changed only {changed} pixels");
}

/// Light entities light the scene through the clusters; a shadowed light's cube is drawn
/// once and then kept until the light or its casters change.
#[test]
fn light_entities_light_the_scene_and_cache_their_shadows() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let (mut document, point) = lit_room();
    assert_eq!(document.scene_lights().len(), 2);
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    let filters = ViewFilters::default();
    tracker.sync(&document, &filters, None, Pose::Rest, &mut renderer.scene);
    let mut camera = Camera3d {
        position: geometry_math::vec::Vec3d::new(-260.0, -380.0, 300.0),
        ..Default::default()
    };
    camera.look_along(geometry_math::vec::Vec3d::new(260.0, 380.0, -300.0));
    let mut settings = single_frame_settings();
    settings.procedural_sky = false;
    settings.background_color = [0.0, 0.0, 0.0];
    settings.sun_intensity = 0.0;
    settings.ambient_intensity = 0.02;
    settings.ambient_occlusion = false;
    settings.bloom = false;
    let size = (320, 240);
    let mut params = view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    );
    params.lights = false;
    renderer.render(&params);
    let dark = renderer.read_output().expect("output");
    save("lights-off", &dark);
    assert_eq!(renderer.stats.lights, 0);
    assert_eq!(renderer.stats.light_faces_drawn, 0);

    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    ));
    let lit = renderer.read_output().expect("output");
    save("lights-on", &lit);
    assert_eq!(renderer.stats.lights, 2);
    // The point light's six faces; the spot casts none.
    assert_eq!(renderer.stats.light_faces_drawn, 6);
    assert!(renderer.stats.light_shadow_draws > 0);
    assert!(
        mean(&lit.2) > mean(&dark.2) + 3.0,
        "lights add little: {} vs {}",
        mean(&lit.2),
        mean(&dark.2)
    );
    // Both colours show: warm near the point light, cool under the spot.
    let warm = lit
        .2
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] > 60 && p[0] > p[2] + 20)
        .count();
    let cool = lit
        .2
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[2] > 60 && p[2] > p[0] + 20)
        .count();
    assert!(warm > 500 && cool > 200, "warm {warm}, cool {cool}");

    // Unchanged, and after a selection click: the cube is cached.
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    ));
    assert_eq!(
        renderer.stats.light_faces_drawn, 0,
        "an unchanged frame redrew light shadows"
    );
    assert_eq!(renderer.stats.light_shadow_draws, 0);
    let id = document.selectable_brushes()[0].0;
    document.select_only(id);
    tracker.sync(&document, &filters, None, Pose::Rest, &mut renderer.scene);
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    ));
    assert_eq!(
        renderer.stats.light_faces_drawn, 0,
        "a selection click redrew light shadows"
    );
    // Nor does looking away and back: a light out of view keeps its cube.
    let mut away = camera;
    away.position = geometry_math::vec::Vec3d::new(-4000.0, -4000.0, 300.0);
    away.look_along(geometry_math::vec::Vec3d::new(-1.0, -1.0, 0.0));
    renderer.render(&view(
        &away,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    ));
    assert_eq!(
        renderer.stats.lights, 0,
        "the lights are behind this camera"
    );
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    ));
    assert_eq!(
        renderer.stats.light_faces_drawn, 0,
        "looking back redrew light shadows"
    );

    // The pillar shadows the floor behind it: without the shadow the picture brightens.
    document.set_entity_property_on(&[point], "shadow_enabled", "0");
    tracker.sync(&document, &filters, None, Pose::Rest, &mut renderer.scene);
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    ));
    let unshadowed = renderer.read_output().expect("output");
    save("lights-unshadowed", &unshadowed);
    let brighter = lit
        .2
        .as_chunks::<4>()
        .0
        .iter()
        .zip(unshadowed.2.as_chunks::<4>().0)
        .filter(|(a, b)| b[0] > a[0].saturating_add(20))
        .count();
    assert!(
        brighter > 300,
        "the pillar's shadow covers only {brighter} pixels"
    );

    // Moving a shadowed light redraws its cube.
    document.set_entity_property_on(&[point], "shadow_enabled", "1");
    document.select_only(point);
    document.translate_selection(geometry_math::vec::Vec3d::new(0.0, 32.0, 0.0));
    tracker.sync(&document, &filters, None, Pose::Rest, &mut renderer.scene);
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    ));
    assert_eq!(renderer.stats.light_faces_drawn, 6);
}

/// Textures stream in: a first frame shows each material's placeholder (its average
/// colour once known, grey before), later frames the decoded texture; textures nothing
/// draws any more are evicted once the budget is exceeded.
#[test]
fn textures_stream_in_and_are_evicted_over_budget() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let dir = std::env::temp_dir().join(format!("render-wgpu-streaming-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let colours = [[220u8, 30, 30, 255], [30, 200, 40, 255], [40, 60, 230, 255]];
    for (i, colour) in colours.iter().enumerate() {
        image::RgbaImage::from_pixel(64, 64, image::Rgba(*colour))
            .save(dir.join(format!("m{i}.png")))
            .unwrap();
    }
    let mut document = Document::new(editor_core::world::WorldBounds::default());
    document.load_materials(&dir).unwrap();
    let world = document.world_bounds();
    for i in 0..3 {
        let x = f64::from(i) * 96.0 - 96.0;
        let bounds = geometry_math::bbox::Bbox3d::new(
            geometry_math::vec::Vec3d::new(x - 32.0, -32.0, -32.0),
            geometry_math::vec::Vec3d::new(x + 32.0, 32.0, 32.0),
        );
        document.add_brushes(vec![map_model::shapes::create_cuboid(
            &world,
            &bounds,
            &format!("m{i}"),
        )
        .unwrap()]);
    }
    document.clear_selection();
    assert!(
        document
            .materials()
            .get("m0")
            .unwrap()
            .resident_pixels()
            .is_none(),
        "the folder was read from its headers"
    );

    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d::default();
    camera.frame(&document.bounds().unwrap());
    let settings = single_frame_settings();
    let params = view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        (240, 160),
    );
    renderer.render(&params);
    assert!(
        renderer.streaming(),
        "the first frame asks for its textures"
    );
    assert_eq!(renderer.scene.texture_stats.resident, 0);
    let start = std::time::Instant::now();
    while renderer.streaming() {
        assert!(start.elapsed().as_secs() < 20, "the textures never arrived");
        std::thread::sleep(std::time::Duration::from_millis(2));
        renderer.render(&params);
    }
    assert_eq!(renderer.scene.texture_stats.resident, 3);
    assert!(renderer.scene.texture_stats.resident_bytes > 3 * 64 * 64 * 4);
    // The decode settled each material's average colour, the next placeholder.
    assert_eq!(
        document.materials().get("m1").unwrap().average_color(),
        Some([30, 200, 40, 255])
    );

    // Looking away under a tiny budget: nothing drawn, everything evicted.
    renderer.scene.texture_budget = 1;
    let mut away = camera;
    away.position = geometry_math::vec::Vec3d::new(0.0, 0.0, 4000.0);
    away.look_along(geometry_math::vec::Vec3d::new(0.0, 0.0, 1.0));
    let params = view(
        &away,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        (240, 160),
    );
    renderer.render(&params);
    renderer.render(&params);
    renderer.render(&params);
    assert_eq!(
        renderer.scene.texture_stats.resident, 0,
        "{:?}",
        renderer.scene.texture_stats
    );
    assert_eq!(renderer.scene.texture_stats.resident_bytes, 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// CPU submission on big maps -- whether frustum culling and draw
/// submission on the CPU are a bottleneck worth moving to the GPU. Ignored; run with
/// `cargo test --release -p render_wgpu --test headless bench_big_maps -- --ignored --nocapture`.
/// `RENDER_WGPU_BIG=50000` sets the brush count, `RENDER_WGPU_TEXTURES=2000` the textures.
#[test]
#[ignore]
fn bench_big_maps() {
    use editor_core::bench_support::{self as bench, MapSpec};
    let (device, queue) = device().expect("Large-map benchmark requires a GPU adapter");
    let profiling = std::env::var("RENDER_WGPU_PROFILE").as_deref() == Ok("1");
    assert!(
        !profiling || device.features().contains(wgpu::Features::TIMESTAMP_QUERY),
        "RENDER_WGPU_PROFILE requires GPU timestamp queries"
    );
    let brushes: usize = std::env::var("RENDER_WGPU_BIG")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(50_000);
    let textures: usize = std::env::var("RENDER_WGPU_TEXTURES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(2_000);
    let dir = std::env::temp_dir().join(format!("render-wgpu-big-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    bench::write_textures(&dir.join("textures"), textures, 0, 3).unwrap();
    let mut size = (1400, 900);
    if let Ok(s) = std::env::var("RENDER_WGPU_SIZE") {
        let parts: Vec<u32> = s.split('x').filter_map(|n| n.parse().ok()).collect();
        if parts.len() == 2 {
            size = (parts[0], parts[1]);
        }
    }
    for (name, spec, textured) in [
        (
            format!("{brushes} brushes, 64 materials, untextured"),
            MapSpec {
                brushes,
                materials: 64,
                animated: 0,
                seed: 5,
            },
            false,
        ),
        (
            format!("{brushes} brushes over {textures} textures"),
            MapSpec {
                brushes,
                materials: textures,
                animated: 0,
                seed: 5,
            },
            true,
        ),
    ] {
        let path = dir.join("big.map");
        std::fs::write(&path, bench::synthetic_map(&spec)).unwrap();
        let mut document =
            Document::open(&path, editor_core::world::WorldBounds::default()).unwrap();
        if textured {
            document.load_materials(dir.join("textures")).unwrap();
        }
        let mut renderer = Renderer::new(&device, &queue);
        renderer.set_profiling(profiling);
        let mut tracker = SceneTracker::new();
        let started = std::time::Instant::now();
        tracker.sync(
            &document,
            &ViewFilters::default(),
            None,
            Pose::Rest,
            &mut renderer.scene,
        );
        let sync = started.elapsed();
        let mut settings = editor_core::render_settings::from_preferences();
        settings.shadows = true;
        settings.ambient_occlusion = true;
        settings.antialiasing = true;
        let bounds = document.bounds().unwrap();
        // Streaming: frames until every texture in view is resident.
        let mut camera = Camera3d::default();
        camera.frame(&bounds);
        let started = std::time::Instant::now();
        let mut streamed = 0;
        loop {
            renderer.render(&view(
                &camera,
                &settings,
                DisplayMode::Material,
                Tonemapper::Agx,
                size,
            ));
            renderer.wait_idle();
            streamed += 1;
            if !renderer.streaming() {
                break;
            }
            assert!(
                streamed < 5000,
                "Texture streaming did not settle within 5000 frames: {name}"
            );
        }
        let streaming = started.elapsed();
        let t = renderer.scene.texture_stats;
        let mut frames = Vec::new();
        let mut encodes = Vec::new();
        let mut gpu_passes: std::collections::BTreeMap<&str, Vec<f64>> =
            std::collections::BTreeMap::new();
        let (mut draws, mut chunks, mut culled) = (0, 0, 0);
        let (mut sun_draws, mut light_draws, mut prepass_draws) = (0, 0, 0);
        let orbit_frames: u32 = std::env::var("RENDER_WGPU_FRAMES")
            .ok()
            .and_then(|n| n.parse().ok())
            .unwrap_or(90);
        assert!(
            orbit_frames > 10,
            "RENDER_WGPU_FRAMES must exceed the ten warmup frames"
        );
        for frame in 0..orbit_frames {
            // Orbiting inside the map, so culling has work to do.
            let mut camera = Camera3d::default();
            camera.frame(&bounds);
            camera.turn(f64::from(frame) * 0.07, -0.25);
            camera.position = bounds.center() + geometry_math::vec::Vec3d::new(0.0, 0.0, 256.0);
            let started = std::time::Instant::now();
            renderer.render(&view(
                &camera,
                &settings,
                DisplayMode::Material,
                Tonemapper::Agx,
                size,
            ));
            renderer.wait_idle();
            if frame >= 10 {
                frames.push(started.elapsed().as_secs_f64() * 1000.0);
                encodes.push(renderer.stats.encode.as_secs_f64() * 1000.0);
                draws += renderer.stats.draws;
                sun_draws += renderer.stats.sun_shadow_draws;
                light_draws += renderer.stats.light_shadow_draws;
                prepass_draws += renderer.stats.prepass_draws;
                chunks += renderer.stats.chunks_drawn;
                culled += renderer.stats.chunks_culled;
                if profiling {
                    let measured = frames.len();
                    for (label, ms) in renderer.gpu_times() {
                        let values = gpu_passes
                            .entry(label)
                            .or_insert_with(|| vec![0.0; measured - 1]);
                        values.push(*ms);
                    }
                    // Cached or disabled passes cost zero in this frame.
                    for values in gpu_passes.values_mut() {
                        values.resize(measured, 0.0);
                    }
                }
            }
        }
        frames.sort_by(f64::total_cmp);
        encodes.sort_by(f64::total_cmp);
        let n = frames.len() as u32;
        println!(
            "{name} at {}x{}, {n} measured frames: first sync {:.0} ms; streamed {} textures ({:.0} MB) in {:.0} ms over {streamed} frames; frame median {:.2} ms (p95 {:.2}), CPU encode median {:.2} ms (p95 {:.2}); {} draws ({} sun shadow, {} light shadow, {} prepass, {} scene/sprites), {} chunks drawn, {} culled per frame",
            size.0,
            size.1,
            sync.as_secs_f64() * 1000.0,
            t.resident,
            t.resident_bytes as f64 / 1e6,
            streaming.as_secs_f64() * 1000.0,
            frames[frames.len() / 2],
            frames[frames.len() * 95 / 100],
            encodes[encodes.len() / 2],
            encodes[encodes.len() * 95 / 100],
            draws / n,
            sun_draws / n,
            light_draws / n,
            prepass_draws / n,
            (draws - sun_draws - light_draws - prepass_draws) / n,
            chunks / n,
            culled / n,
        );
        for (label, mut values) in gpu_passes {
            assert_eq!(values.len(), n as usize);
            let mean = values.iter().sum::<f64>() / f64::from(n);
            values.sort_by(f64::total_cmp);
            println!(
                "  GPU {label}: mean {mean:.3} ms, median {:.3} ms, p95 {:.3} ms over {n} frames (cached frames included as zero)",
                values[values.len() / 2],
                values[values.len() * 95 / 100]
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A floor with a wall and two pillars, and a box (a `func_shuttle`) sliding across the
/// floor along x at 128 units a second, without waiting at the ends.
fn shuttle_yard() -> Document {
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;
    let mut document = Document::new(editor_core::world::WorldBounds::default());
    let world = document.world_bounds();
    let cuboid = |min: [f64; 3], max: [f64; 3]| {
        map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(
                Vec3d::new(min[0], min[1], min[2]),
                Vec3d::new(max[0], max[1], max[2]),
            ),
            "floor",
        )
        .expect("a box")
    };
    document.add_brushes(vec![
        cuboid([-384.0, -384.0, -16.0], [384.0, 384.0, 0.0]),
        cuboid([-384.0, 256.0, 0.0], [384.0, 288.0, 160.0]),
        cuboid([-224.0, 128.0, 0.0], [-192.0, 160.0, 128.0]),
        cuboid([192.0, 128.0, 0.0], [224.0, 160.0, 128.0]),
    ]);
    document.clear_selection();
    document.add_brushes(vec![cuboid([-200.0, -24.0, 0.0], [-152.0, 24.0, 48.0])]);
    let shuttle = document
        .create_brush_entity("func_shuttle")
        .expect("a brush entity");
    for (key, value) in [
        ("direction", "1 0 0"),
        ("travel", "320"),
        ("speed", "128"),
        ("wait", "0"),
    ] {
        document.set_entity_property_on(&[shuttle], key, value);
    }
    document.clear_selection();
    document
}

/// Luminance (0..255) of an RGBA picture, row by row.
fn luminance(pixels: &[u8]) -> Vec<f64> {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .map(|p| 0.2126 * f64::from(p[0]) + 0.7152 * f64::from(p[1]) + 0.0722 * f64::from(p[2]))
        .collect()
}

/// The pixel rectangle a world box covers in a view.
fn screen_rect(
    camera: &Camera3d,
    size: (u32, u32),
    min: [f64; 3],
    max: [f64; 3],
) -> (usize, usize, usize, usize) {
    let m = render_wgpu::view_projection(camera, size.0, size.1);
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for i in 0..8 {
        let p = [
            if i & 1 == 0 { min[0] } else { max[0] },
            if i & 2 == 0 { min[1] } else { max[1] },
            if i & 4 == 0 { min[2] } else { max[2] },
        ];
        let clip: Vec<f64> = (0..4)
            .map(|r| m[0][r] * p[0] + m[1][r] * p[1] + m[2][r] * p[2] + m[3][r])
            .collect();
        let (x, y) = (
            (clip[0] / clip[3] * 0.5 + 0.5) * f64::from(size.0),
            (0.5 - clip[1] / clip[3] * 0.5) * f64::from(size.1),
        );
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
    }
    let clamp = |v: f64, hi: u32| v.clamp(0.0, f64::from(hi)) as usize;
    (
        clamp(x0, size.0),
        clamp(y0, size.1),
        clamp(x1, size.0),
        clamp(y1, size.1),
    )
}

/// The animation preview with every effect on: frame to frame, a part of the scene that
/// does not move keeps still (no frozen or flickering noise of the effects that the
/// temporal filter should average, and close to the converged picture), and the moving
/// box leaves no trail: each frame matches a converged picture of the same pose except
/// for a few pixels. `RENDER_WGPU_SHOTS` saves the frames.
#[test]
fn an_animation_stays_quiet_and_leaves_no_trail() {
    let Some((device, queue)) = device() else {
        eprintln!("no GPU adapter; skipping");
        return;
    };
    let document = shuttle_yard();
    let filters = ViewFilters::default();
    let mut camera = Camera3d {
        position: geometry_math::vec::Vec3d::new(0.0, -260.0, 150.0),
        ..Default::default()
    };
    camera.look_along(geometry_math::vec::Vec3d::new(0.0, 400.0, -190.0));
    let size = (320, 200);
    let mut settings = editor_core::render_settings::from_preferences();
    settings.antialiasing = true;
    settings.accumulate = true;
    settings.accumulation_samples = 64;
    settings.shadows = true;
    settings.ambient_occlusion = true;
    settings.contact_shadows = true;
    settings.reflections = true;
    settings.bloom = false;
    let params = view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        size,
    );

    // The animation as the egui view draws it: a sync per frame at the preview's time,
    // and nothing else changed (the host's picture key stays put).
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    let mut frames = Vec::new();
    let mut moving = 0;
    for frame in 0..48 {
        let time = 0.25 + f64::from(frame) / 60.0;
        tracker.sync(
            &document,
            &filters,
            None,
            Pose::Playing(time),
            &mut renderer.scene,
        );
        renderer.render(&ViewParams {
            continuing: frame > 0,
            ..params
        });
        moving += usize::from(renderer.scene.moving());
        if frame >= 32 {
            let shot = renderer.read_output().expect("output");
            save(&format!("animation-{frame}"), &shot);
            frames.push((time, luminance(&shot.2)));
        }
    }
    assert!(moving >= 46, "the box moved in only {moving} frames");

    // A part of the floor, wall and pillar far from the box and its shadow.
    let (x0, y0, x1, y1) = screen_rect(&camera, size, [176.0, 96.0, 0.0], [300.0, 250.0, 100.0]);
    assert!(
        x1 > x0 + 8 && y1 > y0 + 8,
        "the still region is on screen: {:?}",
        (x0, y0, x1, y1)
    );
    let region = |picture: &[f64]| -> Vec<f64> {
        (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .map(|(x, y)| picture[y * size.0 as usize + x])
            .collect()
    };
    let regions: Vec<Vec<f64>> = frames.iter().map(|(_, p)| region(p)).collect();
    let count = regions[0].len() as f64;
    let flicker = (0..regions[0].len())
        .map(|i| {
            let mean = regions.iter().map(|r| r[i]).sum::<f64>() / regions.len() as f64;
            (regions.iter().map(|r| (r[i] - mean).powi(2)).sum::<f64>() / regions.len() as f64)
                .sqrt()
        })
        .sum::<f64>()
        / count;

    // Converged pictures of the last frames' poses, each from a renderer of its own.
    let mut error = 0.0;
    let mut trail = 0usize;
    for (index, (time, picture)) in frames.iter().enumerate().skip(frames.len() - 4) {
        let mut still = Renderer::new(&device, &queue);
        let mut still_tracker = SceneTracker::new();
        still_tracker.sync(
            &document,
            &filters,
            None,
            Pose::Playing(*time),
            &mut still.scene,
        );
        settle(&mut still, &params);
        let reference = still.read_output().expect("output");
        if index == frames.len() - 1 {
            save("animation-reference", &reference);
        }
        let reference = luminance(&reference.2);
        let r = region(&reference);
        error += (regions[index]
            .iter()
            .zip(&r)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            / count)
            .sqrt()
            / 4.0;
        trail += picture
            .iter()
            .zip(&reference)
            .filter(|(a, b)| (*a - *b).abs() > 32.0)
            .count();
    }
    let trail = trail / 4;
    eprintln!(
        "animation: flicker {flicker:.2}, error against converged {error:.2}, pixels off by more than 32: {trail}"
    );
    assert!(
        flicker < 1.0,
        "the still part flickers by {flicker:.2} levels frame to frame"
    );
    assert!(
        error < 1.4,
        "the still part is {error:.2} levels from the converged picture"
    );
    assert!(
        trail < 25,
        "{trail} pixels differ from the converged picture of the same pose"
    );
}

#[test]
fn imported_pbr_maps_change_the_rendered_material() {
    use editor_core::render_scene::SceneSink;
    let Some((device, queue)) = device() else {
        panic!("PBR verification requires a GPU adapter")
    };
    let mut document = Document::new(editor_core::world::WorldBounds::default());
    let source = format!(
        "{}/../../examples/editor-checks/fixtures/pbr.glb",
        env!("CARGO_MANIFEST_DIR")
    );
    document
        .import_model(
            std::path::Path::new(&source),
            geometry_math::vec::Vec3d::zero(),
        )
        .unwrap();
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d::default();
    camera.frame_in_viewport(&document.selection_bounds().unwrap(), 320., 240.);
    let settings = single_frame_settings();
    renderer.scene.load_textures_now();
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        (320, 240),
    ));
    let full = renderer.read_output().unwrap();
    save("pbr-imported", &full);
    let originals = document.materials().materials().to_vec();
    let draw_sun = |renderer: &mut Renderer, shadows| {
        let mut settings = settings.clone();
        settings.ambient_intensity = 0.0;
        settings.ambient_occlusion = false;
        settings.contact_shadows = false;
        settings.shadows = shadows;
        settings.sun_elevation = 12.0;
        settings.reflections = false;
        settings.bloom = false;
        let mut params = view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Agx,
            (320, 240),
        );
        params.lights = false;
        renderer.render(&params);
        renderer.read_output().unwrap()
    };
    let occluded_sun = draw_sun(&mut renderer, true);
    let unshadowed_sun = draw_sun(&mut renderer, false);
    for slot in ["normal", "roughness_metallic", "ao", "emissive"] {
        let mut materials = originals.clone();
        for material in &mut materials {
            if let Some(source) = &material.model_pbr {
                let mut pbr = (**source).clone();
                match slot {
                    "normal" => {
                        pbr.normal_scale = 0.;
                    }
                    "roughness_metallic" => {
                        pbr.metallic_roughness = None;
                    }
                    "ao" => {
                        pbr.occlusion_strength = 0.;
                    }
                    _ => pbr.emissive_factor = [0.; 3],
                }
                material.model_pbr = Some(std::sync::Arc::new(pbr));
            }
        }
        renderer.scene.set_materials(&materials);
        renderer.scene.finish();
        renderer.scene.load_textures_now();
        renderer.render(&view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Agx,
            (320, 240),
        ));
        let changed = renderer.read_output().unwrap();
        let difference = full
            .2
            .iter()
            .zip(&changed.2)
            .filter(|(a, b)| a.abs_diff(**b) > 1)
            .count();
        assert!(
            difference > 50,
            "{slot} had no visible effect: {difference} changed channels"
        );
        if slot == "ao" {
            let open_sun = draw_sun(&mut renderer, true);
            assert!(
                mean(&open_sun.2) > mean(&occluded_sun.2) + 0.05,
                "material occlusion should reduce grazing direct sunlight"
            );
            assert_eq!(draw_sun(&mut renderer, false), unshadowed_sun);
            save("microshadows-on", &occluded_sun);
            save("microshadows-off", &open_sun);
        }
    }
}

#[test]
fn imported_mask_cutoff_changes_rendered_coverage() {
    use editor_core::render_scene::SceneSink;
    let (device, queue) = device().expect("Mask cutoff verification requires a GPU adapter");
    let mut document = Document::new(editor_core::world::WorldBounds::default());
    let source = format!(
        "{}/../../examples/editor-checks/fixtures/material_modes.glb",
        env!("CARGO_MANIFEST_DIR")
    );
    document
        .import_model(
            std::path::Path::new(&source),
            geometry_math::vec::Vec3d::zero(),
        )
        .unwrap();
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d::default();
    camera.frame_in_viewport(&document.selection_bounds().unwrap(), 320., 240.);
    let settings = single_frame_settings();
    let originals = document.materials().materials().to_vec();
    let mut outputs = Vec::new();
    for cutoff in [0.0, 1.0] {
        let mut materials = originals.clone();
        let mut changed = 0;
        for material in &mut materials {
            if let Some(source) = &material.model_pbr {
                if source.alpha_mode != map_model::material::Alpha::Masked {
                    continue;
                }
                let mut pbr = (**source).clone();
                pbr.alpha_cutoff = cutoff;
                material.model_pbr = Some(std::sync::Arc::new(pbr));
                changed += 1;
            }
        }
        assert_eq!(changed, 2);
        renderer.scene.set_materials(&materials);
        renderer.scene.finish();
        renderer.scene.load_textures_now();
        renderer.render(&view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Agx,
            (320, 240),
        ));
        outputs.push(renderer.read_output().unwrap());
    }
    let difference = outputs[0]
        .2
        .iter()
        .zip(&outputs[1].2)
        .filter(|(a, b)| a.abs_diff(**b) > 1)
        .count();
    save("mask-cutoff-zero", &outputs[0]);
    save("mask-cutoff-one", &outputs[1]);
    assert!(
        difference > 50,
        "Source cutoff had no visible effect: {difference} channels"
    );
}

#[test]
fn imported_face_rules_control_front_and_back_coverage() {
    use editor_core::render_scene::SceneSink;
    let (device, queue) = device().expect("Face-rule verification requires a GPU adapter");
    let mut document = Document::new(editor_core::world::WorldBounds::default());
    let source = format!(
        "{}/../../examples/editor-checks/fixtures/pbr.glb",
        env!("CARGO_MANIFEST_DIR")
    );
    document
        .import_model(
            std::path::Path::new(&source),
            geometry_math::vec::Vec3d::zero(),
        )
        .unwrap();
    let bounds = document.selection_bounds().unwrap();
    document.clear_selection();
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut settings = single_frame_settings();
    settings.procedural_sky = false;
    settings.background_color = [0.; 3];
    settings.show_axes = false;
    settings.shadows = false;
    settings.ambient_occlusion = false;
    settings.reflections = false;
    settings.bloom = false;
    let originals = document.materials().materials().to_vec();
    let mut coverage = Vec::new();
    for double in [false, true] {
        let mut materials = originals.clone();
        for material in &mut materials {
            if let Some(source) = &material.model_pbr {
                let mut pbr = (**source).clone();
                pbr.double_sided = double;
                pbr.emissive_factor = [2.; 3];
                pbr.emissive = None;
                pbr.normal = None;
                pbr.normal_scale = 0.;
                material.normal_map = None;
                material.model_pbr = Some(std::sync::Arc::new(pbr));
            }
        }
        renderer.scene.set_materials(&materials);
        renderer.scene.finish();
        renderer.scene.load_textures_now();
        for back in [false, true] {
            let mut camera = Camera3d {
                pitch: if back { 1.56 } else { -1.56 },
                yaw: 0.,
                ..Default::default()
            };
            camera.frame_in_viewport(&bounds, 320., 240.);
            renderer.render(&view(
                &camera,
                &settings,
                DisplayMode::Material,
                Tonemapper::Agx,
                (320, 240),
            ));
            let image = renderer.read_output().unwrap();
            coverage.push(
                image
                    .2
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|pixel| pixel[0] > 100)
                    .count(),
            );
            save(&format!("face-rule-double-{double}-back-{back}"), &image);
        }
    }
    assert!(coverage[0] > 50, "Single-sided front missing: {coverage:?}");
    assert_eq!(coverage[1], 0, "Single-sided back not culled: {coverage:?}");
    assert!(
        coverage[2] > 50 && coverage[3] > 50,
        "Double-sided face missing: {coverage:?}"
    );
    assert!(
        coverage[2].abs_diff(coverage[3]) < coverage[2] / 5,
        "Double-sided coverage differs: {coverage:?}"
    );
}

#[test]
fn imported_single_sided_back_does_not_cast_a_sun_shadow() {
    use editor_core::render_scene::SceneSink;
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    let (device, queue) = device().expect("Shadow sidedness verification requires a GPU adapter");
    let mut document = Document::new(editor_core::world::WorldBounds::default());
    let floor = map_model::shapes::create_cuboid(
        &document.world_bounds(),
        &Bbox3d::new(Vec3d::new(-320., -320., -16.), Vec3d::new(320., 320., 0.)),
        "__TB_empty",
    )
    .unwrap();
    document.add_brushes(vec![floor]);
    let source = format!(
        "{}/../../examples/editor-checks/fixtures/pbr.glb",
        env!("CARGO_MANIFEST_DIR")
    );
    let model = document
        .import_model(std::path::Path::new(&source), Vec3d::new(0., 0., 80.))
        .unwrap();
    document.set_entity_property_on(&[model], "angles", "180 0 0");
    document.set_entity_property_on(&[model], "modelscale", "96");
    document.clear_selection();
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d::default();
    camera.frame(&document.bounds().unwrap());
    let mut settings = single_frame_settings();
    settings.ambient_occlusion = false;
    settings.reflections = false;
    settings.bloom = false;
    settings.sun_elevation = 45.;
    settings.sun_azimuth = 45.;
    settings.sun_intensity = 3.;
    settings.shadow_map_size = 1024;
    settings.contact_shadows = false;
    let originals = document.materials().materials().to_vec();
    let mut shadow_changes = Vec::new();
    for double in [false, true] {
        let mut materials = originals.clone();
        for material in &mut materials {
            if let Some(source) = &material.model_pbr {
                let mut pbr = (**source).clone();
                pbr.double_sided = double;
                material.model_pbr = Some(std::sync::Arc::new(pbr));
            }
        }
        renderer.scene.set_materials(&materials);
        renderer.scene.finish();
        renderer.scene.load_textures_now();
        let mut images = Vec::new();
        for shadows in [false, true] {
            settings.shadows = shadows;
            renderer.render(&view(
                &camera,
                &settings,
                DisplayMode::Material,
                Tonemapper::Agx,
                (320, 240),
            ));
            let image = renderer.read_output().unwrap();
            save(
                &format!("back-shadow-double-{double}-shadows-{shadows}"),
                &image,
            );
            images.push(image.2);
        }
        shadow_changes.push(
            images[0]
                .as_chunks::<4>()
                .0
                .iter()
                .zip(images[1].as_chunks::<4>().0)
                .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(x, y)| x.abs_diff(*y) > 4))
                .count(),
        );
    }
    assert!(
        shadow_changes[1] > shadow_changes[0] + 50,
        "Double-sided caster must add a receiver shadow that its back-culled variant lacks: {shadow_changes:?}"
    );
}

/// A culled model back must leave the receiver behind it visible in the depth prepass.
#[test]
fn imported_opaque_back_does_not_occlude_the_receiver() {
    use editor_core::render_scene::SceneSink;
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    let (device, queue) = device().expect("Prepass sidedness verification requires a GPU adapter");
    let mut document = Document::new(editor_core::world::WorldBounds::default());
    let receiver = map_model::shapes::create_cuboid(
        &document.world_bounds(),
        &Bbox3d::new(Vec3d::new(-320., -320., 80.), Vec3d::new(320., 320., 96.)),
        "__TB_empty",
    )
    .unwrap();
    document.add_brushes(vec![receiver]);
    document.clear_selection();
    let mut camera = Camera3d {
        pitch: 1.56,
        yaw: 0.,
        ..Default::default()
    };
    camera.frame_in_viewport(&document.bounds().unwrap(), 320., 240.);
    let mut settings = single_frame_settings();
    settings.procedural_sky = false;
    settings.background_color = [0.; 3];
    settings.show_axes = false;
    settings.shadows = false;
    settings.ambient_occlusion = false;
    settings.reflections = false;
    settings.bloom = false;
    let mut renderer = Renderer::new(&device, &queue);
    let mut tracker = SceneTracker::new();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Agx,
        (320, 240),
    ));
    let baseline = renderer.read_output().unwrap();
    let source = format!(
        "{}/../../examples/editor-checks/fixtures/pbr.glb",
        env!("CARGO_MANIFEST_DIR")
    );
    let model = document
        .import_model(std::path::Path::new(&source), Vec3d::zero())
        .unwrap();
    document.set_entity_property_on(&[model], "modelscale", "96");
    document.clear_selection();
    tracker.sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let originals = document.materials().materials().to_vec();
    let mut differences = Vec::new();
    for double in [false, true] {
        let mut materials = originals.clone();
        for material in &mut materials {
            if let Some(source) = &material.model_pbr {
                let mut pbr = (**source).clone();
                pbr.alpha_mode = map_model::material::Alpha::Opaque;
                pbr.double_sided = double;
                pbr.emissive_factor = [2.; 3];
                pbr.emissive = None;
                pbr.normal = None;
                material.normal_map = None;
                material.model_pbr = Some(std::sync::Arc::new(pbr));
            }
        }
        renderer.scene.set_materials(&materials);
        renderer.scene.finish();
        renderer.scene.load_textures_now();
        renderer.render(&view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Agx,
            (320, 240),
        ));
        let image = renderer.read_output().unwrap();
        save(&format!("prepass-receiver-double-{double}"), &image);
        differences.push(
            image
                .2
                .as_chunks::<4>()
                .0
                .iter()
                .zip(baseline.2.as_chunks::<4>().0)
                .filter(|(a, b)| a[..3].iter().zip(&b[..3]).any(|(a, b)| a.abs_diff(*b) > 4))
                .count(),
        );
    }
    assert!(
        differences[1] > 100,
        "Double-sided control must cover the receiver: {differences:?}"
    );
    assert!(
        differences[0] < differences[1] / 20,
        "Culled opaque back left a hole in the receiver: {differences:?}"
    );
}

/// Exposure and the display transform must not be baked into accumulated samples.
#[test]
fn hdr_accumulation_applies_the_latest_display_transform_to_all_samples() {
    let (device, queue) = device().expect("HDR validation requires a GPU adapter");
    let mut renderer = Renderer::new(&device, &queue);
    let camera = Camera3d::default();
    let mut settings = single_frame_settings();
    settings.procedural_sky = true;
    settings.shadows = false;
    settings.bloom = false;
    settings.sun_color = [0.0; 3];
    settings.sun_elevation = 60.0;
    settings.sky_color = [0.8, 0.4, 0.15];
    settings.horizon_color = settings.sky_color;
    settings.ground_color = settings.sky_color;
    settings.exposure = 0.1;
    let mut first = view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Aces,
        (64, 48),
    );
    first.sample = Some(0);
    renderer.accumulate(&first);
    let dark = renderer.read_output().expect("dark sample");
    settings.exposure = 3.0;
    let mut latest = view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::TonyMcMapface,
        (64, 48),
    );
    latest.sample = Some(1);
    renderer.accumulate(&latest);
    let accumulated = renderer.read_output().expect("HDR average");
    latest.sample = Some(0);
    renderer.accumulate(&latest);
    let reference = renderer.read_output().expect("fresh display transform");
    let difference = accumulated
        .2
        .iter()
        .zip(&reference.2)
        .map(|(a, b)| f64::from(a.abs_diff(*b)))
        .sum::<f64>()
        / reference.2.len() as f64;
    assert!(
        difference < 1.0,
        "old exposure or tone mapping leaked into HDR history: {difference}"
    );
    assert!(
        mean(&reference.2) > mean(&dark.2) + 30.0,
        "the test must exercise a visible exposure change"
    );
    save("hdr-latest-exposure", &accumulated);
    save("hdr-fresh-reference", &reference);

    // An explicit first sample and a resized output must both discard earlier history.
    settings.procedural_sky = false;
    settings.background_color = [0.2, 0.35, 0.6];
    let mut background = view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Aces,
        (80, 40),
    );
    background.sample = Some(0);
    renderer.accumulate(&background);
    let plain = renderer.read_output().expect("resized background");
    assert_eq!((plain.0, plain.1), (80, 40));
    for pixel in plain.2.as_chunks::<4>().0 {
        for (channel, &value) in pixel[..3].iter().enumerate() {
            assert!(
                (f32::from(value) - settings.background_color[channel] * 255.0).abs() <= 2.0,
                "flat background must bypass exposure and tone mapping"
            );
        }
        assert_eq!(pixel[3], 255);
    }
}

/// Minified, opposing normal details must broaden the highlight as a rough surface
/// would, rather than normalizing the averaged vector into a polished mirror.
#[test]
fn minified_normal_details_match_a_broader_specular_reference() {
    use editor_core::render_scene::SceneSink;
    use geometry_math::bbox::Bbox3d;
    use geometry_math::vec::Vec3d;
    use map_model::material::{Material, NormalMap};
    let (device, queue) = device().expect("specular validation requires a GPU adapter");
    let mut document = Document::new(Default::default());
    document.add_brushes(vec![map_model::shapes::create_cuboid(
        &document.world_bounds(),
        &Bbox3d::from_scalars(-32.0, 32.0),
        "filter-test",
    )
    .expect("cube")]);
    let mut triangles = document.render_triangles();
    for triangle in &mut triangles {
        triangle.material = Some(0);
        triangle.surface_material = None;
        triangle.color = [1.0; 3];
        triangle.selected = false;
        triangle.uv = [
            geometry_math::vec::Vec2f::new(0.0, 0.0),
            geometry_math::vec::Vec2f::new(512.0, 0.0),
            geometry_math::vec::Vec2f::new(0.0, 512.0),
        ];
    }
    let mut camera = Camera3d {
        position: Vec3d::new(0.0, -160.0, 0.0),
        ..Default::default()
    };
    camera.look_along(Vec3d::new(0.0, 1.0, 0.0));
    let mut settings = single_frame_settings();
    settings.procedural_sky = false;
    settings.shadows = false;
    settings.ambient_occlusion = false;
    settings.reflections = false;
    settings.bloom = false;
    settings.ambient_intensity = 0.0;
    settings.sun_elevation = 0.0;
    settings.sun_azimuth = 270.0;
    settings.sun_intensity = 3.0;
    settings.default_metalness = 1.0;
    let mut pictures = Vec::new();
    let mut motion_errors = Vec::new();
    for (name, details, roughness) in [
        ("specular-filtered", true, 0.05),
        ("specular-rough-reference", false, 0.2_f32.sqrt().sqrt()),
        ("specular-gloss-reference", false, 0.05),
    ] {
        let mut material =
            Material::rgba("filter-test".into(), Default::default(), 1, 1, vec![255; 4]);
        let pixels = (0..32 * 32)
            .flat_map(|index| {
                if details {
                    if (index % 32 + index / 32) % 2 == 0 {
                        [230, 128, 204, 255]
                    } else {
                        [26, 128, 204, 255]
                    }
                } else {
                    [128, 128, 255, 255]
                }
            })
            .collect();
        material.normal_map = Some(NormalMap::rgba(Default::default(), 32, 32, pixels, false));
        let mut renderer = Renderer::new(&device, &queue);
        renderer.scene.set_materials(&[material]);
        renderer.scene.upsert(0, &triangles, Default::default());
        renderer.scene.finish();
        renderer.scene.load_textures_now();
        settings.default_roughness = roughness;
        let mut previous = None::<Vec<u8>>;
        let mut squared_change = 0.0;
        let mut comparisons = 0;
        for step in 0..4 {
            let moved = Camera3d {
                position: camera.position + Vec3d::new(f64::from(step) * 0.5, 0.0, 0.0),
                ..camera
            };
            renderer.render(&view(
                &moved,
                &settings,
                DisplayMode::Material,
                Tonemapper::Agx,
                (256, 256),
            ));
            let picture = renderer.read_output().expect("specular test image");
            if step == 0 {
                save(name, &picture);
                pictures.push(picture.2.clone());
            }
            // Interior only: avoid measuring silhouette motion instead of highlights.
            let mut region = Vec::new();
            for y in 96..160 {
                for x in 96..160 {
                    region.extend_from_slice(&picture.2[(y * 256 + x) * 4..(y * 256 + x) * 4 + 3]);
                }
            }
            if let Some(before) = previous {
                squared_change += before
                    .iter()
                    .zip(&region)
                    .map(|(a, b)| (f64::from(*a) - f64::from(*b)).powi(2))
                    .sum::<f64>();
                comparisons += region.len();
            }
            previous = Some(region);
        }
        motion_errors.push((squared_change / comparisons as f64).sqrt());
    }
    let difference = |a: &[u8], b: &[u8]| {
        a.iter()
            .zip(b)
            .map(|(a, b)| f64::from(a.abs_diff(*b)))
            .sum::<f64>()
            / a.len() as f64
    };
    eprintln!(
        "specular motion RMS: filtered {:.3}, rough reference {:.3}, polished reference {:.3}",
        motion_errors[0], motion_errors[1], motion_errors[2]
    );
    assert!(
        motion_errors[2] > 0.1,
        "the polished highlight must move between frames"
    );
    assert!(
        motion_errors[0] < motion_errors[2] * 0.8,
        "filtering must reduce highlight shimmer under subpixel camera motion"
    );
    let reference_error = difference(&pictures[0], &pictures[1]);
    let unfiltered_error = difference(&pictures[0], &pictures[2]);
    eprintln!(
        "specular filter: rough-reference error {reference_error:.3}, polished-reference difference {unfiltered_error:.3}"
    );
    assert!(
        reference_error < 2.0,
        "filtered normal details must follow a rough reference: {reference_error}"
    );
    assert!(
        unfiltered_error > reference_error + 1.0,
        "the scene must distinguish filtered highlights from polished ones: {unfiltered_error}"
    );
}

#[test]
fn hdr_panorama_controls_background_rotation_intensity_and_cache() {
    use render::environment::{Environment, EnvironmentImage};
    use std::sync::Arc;
    let (device, queue) = device().expect("environment validation requires a GPU");
    let mut renderer = Renderer::new(&device, &queue);
    let camera = Camera3d::default();
    let mut settings = single_frame_settings();
    settings.bloom = false;
    settings.shadows = false;
    settings.sun_color = [0.0; 3];
    settings.background_color = [0.01, 0.02, 0.03];
    let pixels = (0..32)
        .flat_map(|_| {
            (0..64).map(|x| {
                if x < 32 {
                    [3.0, 0.1, 0.05, 1.0]
                } else {
                    [0.05, 0.2, 3.0, 1.0]
                }
            })
        })
        .collect();
    settings.environment = Some(Environment {
        image: Arc::new(EnvironmentImage::new(64, 32, pixels).unwrap()),
        rotation_degrees: 0.0,
        intensity: 1.0,
        show_background: true,
    });
    let draw = |renderer: &mut Renderer, settings: &render::RenderSettings| {
        renderer.render(&view(
            &camera,
            settings,
            DisplayMode::Material,
            Tonemapper::Aces,
            (96, 64),
        ));
        renderer.read_output().unwrap()
    };
    let first = draw(&mut renderer, &settings);
    assert!(renderer.stats.environment_rebuilt);
    let repeated = draw(&mut renderer, &settings);
    assert!(!renderer.stats.environment_rebuilt);
    assert_eq!(first.2, repeated.2);
    settings.environment.as_mut().unwrap().rotation_degrees = 180.0;
    let rotated = draw(&mut renderer, &settings);
    assert!(renderer.stats.environment_rebuilt);
    let error = |a: &[u8], b: &[u8]| {
        a.iter()
            .zip(b)
            .map(|(a, b)| f64::from(a.abs_diff(*b)))
            .sum::<f64>()
            / a.len() as f64
    };
    assert!(
        error(&first.2, &rotated.2) > 40.0,
        "rotation must move the panorama"
    );
    settings.environment.as_mut().unwrap().rotation_degrees = 360.0;
    let wrapped = draw(&mut renderer, &settings);
    assert!(
        error(&first.2, &wrapped.2) < 1.0,
        "rotation wraps a complete turn"
    );
    settings.environment.as_mut().unwrap().intensity = 0.1;
    let dim = draw(&mut renderer, &settings);
    assert!(mean(&dim.2) < mean(&first.2) - 15.0);
    settings.environment.as_mut().unwrap().show_background = false;
    let hidden = draw(&mut renderer, &settings);
    settings.environment = None;
    settings.procedural_sky = false;
    let flat = draw(&mut renderer, &settings);
    assert_eq!(
        hidden.2, flat.2,
        "hidden panorama uses the flat camera background"
    );
    save("environment-original", &first);
    save("environment-rotated", &rotated);
    save("environment-dim", &dim);
    save("environment-hidden", &hidden);
}

#[test]
fn hidden_hdr_environment_still_lights_materials_and_offline_samples() {
    use render::environment::{Environment, EnvironmentImage};
    use std::sync::Arc;
    let (device, queue) = device().expect("environment lighting requires a GPU");
    let mut renderer = Renderer::new(&device, &queue);
    let document = open("assets/crate.map");
    SceneTracker::new().sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d::default();
    camera.frame(&document.bounds().unwrap());
    let mut settings = single_frame_settings();
    settings.sun_color = [0.0; 3];
    settings.bloom = false;
    settings.shadows = false;
    settings.ambient_occlusion = false;
    settings.background_color = [0.02; 3];
    let environment = |color| Environment {
        image: Arc::new(EnvironmentImage::new(8, 4, vec![color; 32]).unwrap()),
        rotation_degrees: 0.0,
        intensity: 1.0,
        show_background: false,
    };
    settings.environment = Some(environment([4.0, 0.02, 0.02, 1.0]));
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Aces,
        (160, 120),
    ));
    let red = renderer.read_output().unwrap();
    settings.environment = Some(environment([0.02, 0.02, 4.0, 1.0]));
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Aces,
        (160, 120),
    ));
    let blue = renderer.read_output().unwrap();
    let sum = |pixels: &[u8], channel: usize| {
        pixels
            .chunks_exact(4)
            .map(|p| u64::from(p[channel]))
            .sum::<u64>()
    };
    assert!(
        sum(&red.2, 0) > sum(&blue.2, 0) + 10000,
        "diffuse IBL must receive red light"
    );
    assert!(
        sum(&blue.2, 2) > sum(&red.2, 2) + 10000,
        "diffuse IBL must receive blue light"
    );
    let mut offline = view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Aces,
        (160, 120),
    );
    offline.sample = Some(0);
    renderer.accumulate(&offline);
    let still = renderer.read_output().unwrap();
    let error = still
        .2
        .iter()
        .zip(&blue.2)
        .map(|(a, b)| f64::from(a.abs_diff(*b)))
        .sum::<f64>()
        / blue.2.len() as f64;
    assert!(
        error < 2.0,
        "viewport and offline use the same environment: {error}"
    );
    save("environment-light-red", &red);
    save("environment-light-blue", &blue);
    save("environment-offline", &still);
}

#[test]
fn camera_grading_changes_display_without_baking_it_into_linear_hdr() {
    use render::environment::{Environment, EnvironmentImage};
    let (device, queue) = device().expect("HDR and grading validation requires a GPU");
    let mut renderer = Renderer::new(&device, &queue);
    assert!(renderer.read_hdr().is_none());
    let camera = Camera3d::default();
    let mut settings = single_frame_settings();
    settings.bloom = false;
    settings.environment = Some(Environment {
        image: std::sync::Arc::new(
            EnvironmentImage::new(8, 4, vec![[4.0, 2.0, 0.5, 1.0]; 32]).unwrap(),
        ),
        rotation_degrees: 0.0,
        intensity: 1.0,
        show_background: true,
    });
    let draw = |renderer: &mut Renderer, settings: &render::RenderSettings| {
        let mut params = view(
            &camera,
            settings,
            DisplayMode::Material,
            Tonemapper::Aces,
            (37, 19),
        );
        params.sample = Some(0);
        renderer.accumulate(&params);
    };
    draw(&mut renderer, &settings);
    let neutral = renderer.read_output().unwrap();
    let linear = renderer.read_hdr().unwrap();
    assert_eq!((linear.0, linear.1), (37, 19));
    for pixel in linear.2.as_chunks::<4>().0 {
        assert!((pixel[0] - 4.0).abs() < 0.01);
        assert!((pixel[1] - 2.0).abs() < 0.01);
        assert!((pixel[2] - 0.5).abs() < 0.01);
        assert_eq!(pixel[3], 1.0);
    }
    settings.exposure = 0.1;
    settings.color_grading.temperature = 3000.0;
    settings.color_grading.tint = 0.2;
    settings.color_grading.saturation = 0.0;
    settings.color_grading.contrast = 1.4;
    settings.color_grading.tone_map = Some(Tonemapper::KhronosNeutral);
    draw(&mut renderer, &settings);
    let graded = renderer.read_output().unwrap();
    assert_eq!(
        linear,
        renderer.read_hdr().unwrap(),
        "EXR source must exclude every display transform"
    );
    assert!(mean(&graded.2) < mean(&neutral.2) - 20.0);
    for p in graded.2.as_chunks::<4>().0 {
        assert!(
            p[0].abs_diff(p[1]) <= 1 && p[1].abs_diff(p[2]) <= 1,
            "zero saturation must be neutral"
        );
    }
    save("camera-neutral", &neutral);
    save("camera-graded", &graded);
    settings.environment.as_mut().unwrap().show_background = false;
    draw(&mut renderer, &settings);
    let transparent = renderer.read_hdr().unwrap();
    assert!(
        transparent.2.iter().all(|value| *value == 0.0),
        "display background must stay out of scene-linear EXR"
    );
    let display = renderer.read_output().unwrap();
    renderer.release_accumulation();
    assert!(renderer.read_hdr().is_none());
    assert_eq!(renderer.read_output().unwrap(), display);
}
#[test]
fn asynchronous_frame_timing_does_not_stall_or_break_rendering() {
    let (device, queue) = device().expect("timing validation requires a GPU");
    let timestamps = device.features().contains(wgpu::Features::TIMESTAMP_QUERY);
    let mut renderer = Renderer::new(&device, &queue);
    renderer.set_frame_timing(true);
    let camera = Camera3d::default();
    let settings = single_frame_settings();
    for _ in 0..6 {
        renderer.render(&view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Aces,
            (64, 48),
        ));
    }
    renderer.wait_idle();
    let duration = renderer.take_frame_time();
    if timestamps {
        assert!(
            duration.is_some_and(|ms| ms.is_finite() && ms > 0.0),
            "completed frames must provide actual GPU time: {duration:?}"
        );
    }
    assert!(
        renderer.take_frame_time().is_none(),
        "each timing result is consumed once"
    );
    renderer.set_frame_timing(false);
    assert!(renderer.take_frame_time().is_none());
}
#[test]
fn directional_indirect_occlusion_reduces_environment_light_in_recesses() {
    use geometry_math::vec::Vec3d;
    let (device, queue) = device().expect("occlusion validation requires a GPU");
    let (document, _) = lit_room();
    let mut renderer = Renderer::new(&device, &queue);
    SceneTracker::new().sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d {
        position: Vec3d::new(170.0, -220.0, 150.0),
        ..Default::default()
    };
    camera.look_along(Vec3d::new(-170.0, 220.0, -120.0));
    let mut settings = single_frame_settings();
    settings.sun_intensity = 0.0;
    settings.shadows = false;
    settings.bloom = false;
    settings.reflections = false;
    settings.ambient_intensity = 1.0;
    settings.default_metalness = 0.9;
    settings.default_roughness = 0.2;
    settings.ambient_occlusion = true;
    settings.ao_radius = 96.0;
    settings.ao_strength = 1.0;
    settings.indirect_occlusion = false;
    let draw = |renderer: &mut Renderer, settings: &render::RenderSettings| {
        let mut params = view(
            &camera,
            settings,
            DisplayMode::Material,
            Tonemapper::Aces,
            (320, 240),
        );
        params.lights = false;
        renderer.render(&params);
        renderer.read_output().unwrap()
    };
    let before = draw(&mut renderer, &settings);
    settings.indirect_occlusion = true;
    let after = draw(&mut renderer, &settings);
    let changed = before
        .2
        .iter()
        .zip(&after.2)
        .filter(|(a, b)| a.abs_diff(**b) > 3)
        .count();
    assert!(
        changed > 100,
        "directional occlusion must affect recessed surfaces: {changed}"
    );
    assert!(
        mean(&after.2) < mean(&before.2),
        "occluded directions must not add environment light"
    );
    save("occlusion-before", &before);
    save("occlusion-after", &after);
}
#[test]
fn pcss_softness_changes_shadow_edges_without_redrawing_cached_cascades() {
    use geometry_math::vec::Vec3d;
    let (device, queue) = device().expect("soft shadow validation requires a GPU");
    let (document, _) = lit_room();
    let mut renderer = Renderer::new(&device, &queue);
    SceneTracker::new().sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d {
        position: Vec3d::new(170.0, -220.0, 180.0),
        ..Default::default()
    };
    camera.look_along(Vec3d::new(-170.0, 220.0, -150.0));
    let mut settings = single_frame_settings();
    settings.shadows = true;
    settings.bloom = false;
    settings.reflections = false;
    settings.ambient_occlusion = false;
    settings.contact_shadows = false;
    settings.ambient_intensity = 0.1;
    settings.sun_intensity = 3.0;
    settings.sun_elevation = 35.0;
    settings.shadow_map_size = 1024;
    settings.shadow_softness = 0.0;
    let draw = |renderer: &mut Renderer, settings: &render::RenderSettings| {
        let mut params = view(
            &camera,
            settings,
            DisplayMode::Material,
            Tonemapper::Aces,
            (320, 240),
        );
        params.lights = false;
        renderer.render(&params);
        renderer.read_output().unwrap()
    };
    let hard = draw(&mut renderer, &settings);
    assert!(renderer.stats.cascades_drawn > 0);
    settings.shadow_softness = 4.0;
    let soft = draw(&mut renderer, &settings);
    assert_eq!(
        renderer.stats.cascades_drawn, 0,
        "PCSS must reuse the stable cascade maps"
    );
    let changed = hard
        .2
        .iter()
        .zip(&soft.2)
        .filter(|(a, b)| a.abs_diff(**b) > 4)
        .count();
    assert!(changed > 100, "softness must change penumbrae: {changed}");
    save("shadow-contact-hard", &hard);
    save("shadow-contact-soft", &soft);
}

#[test]
fn shadow_penumbra_grows_with_blocker_distance_and_planes_stay_lit() {
    use editor_core::render_scene::SceneSink;
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    let (device, queue) = device().expect("penumbra validation requires a GPU");
    let mut document = Document::new(Default::default());
    let world = document.world_bounds();
    let cube = |min, max| {
        map_model::shapes::create_cuboid(&world, &Bbox3d::new(min, max), "pcss").unwrap()
    };
    let floor = cube(
        Vec3d::new(-220.0, -220.0, -16.0),
        Vec3d::new(220.0, 220.0, 0.0),
    );
    document.add_brushes(vec![floor]);
    let mut renderer = Renderer::new(&device, &queue);
    let camera = Camera3d {
        position: Vec3d::new(0.0, 0.0, 3000.0),
        fov_degrees: 20.0,
        pitch: -std::f64::consts::FRAC_PI_2,
        yaw: 0.0,
        ..Default::default()
    };
    let mut settings = single_frame_settings();
    settings.sun_elevation = 90.0;
    settings.sun_intensity = 2.0;
    settings.ambient_intensity = 0.0;
    settings.bloom = false;
    settings.reflections = false;
    settings.ambient_occlusion = false;
    settings.contact_shadows = false;
    settings.shadows = true;
    settings.shadow_map_size = 2048;
    let upload = |renderer: &mut Renderer, document: &Document| {
        let mut triangles = document.render_triangles();
        for triangle in &mut triangles {
            triangle.color = [0.5; 3];
            triangle.material = None;
            triangle.surface_material = None;
            triangle.selected = false;
        }
        renderer.scene.upsert(0, &triangles, Default::default());
        renderer.scene.finish();
    };
    let draw = |renderer: &mut Renderer, settings: &render::RenderSettings| {
        let mut params = view(
            &camera,
            settings,
            DisplayMode::Material,
            Tonemapper::Aces,
            (512, 512),
        );
        params.lights = false;
        renderer.render(&params);
        renderer.read_output().unwrap()
    };
    upload(&mut renderer, &document);
    settings.shadow_softness = 0.0;
    let plane_hard = draw(&mut renderer, &settings);
    settings.shadow_softness = 4.0;
    let plane_soft = draw(&mut renderer, &settings);
    assert_eq!(
        plane_hard.2, plane_soft.2,
        "a lone receiver plane must not become a blocker"
    );
    document.add_brushes(vec![
        cube(
            Vec3d::new(-20.0, -100.0, 4.0),
            Vec3d::new(20.0, -60.0, 12.0),
        ),
        cube(
            Vec3d::new(-20.0, 60.0, 120.0),
            Vec3d::new(20.0, 100.0, 128.0),
        ),
    ]);
    upload(&mut renderer, &document);
    settings.shadow_softness = 0.0;
    let hard = draw(&mut renderer, &settings);
    settings.shadow_softness = 4.0;
    let soft = draw(&mut renderer, &settings);
    let halo = |y: f64| {
        let (x0, y0, x1, y1) = screen_rect(
            &camera,
            (512, 512),
            [-40.0, y - 40.0, 0.0],
            [40.0, y + 40.0, 0.0],
        );
        (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (y * 512 + x) * 4))
            .filter(|&offset| hard.2[offset] > soft.2[offset].saturating_add(3))
            .count()
    };
    let near = halo(-80.0);
    let far = halo(80.0);
    assert!(
        far > near * 2 + 10,
        "a distant blocker must make a wider visible penumbra: near {near}, far {far}"
    );
    save("shadow-distance-hard", &hard);
    save("shadow-distance-soft", &soft);
}

#[test]
fn perturbed_normals_do_not_reflect_environment_through_the_geometric_face() {
    use editor_core::render_scene::SceneSink;
    use geometry_math::{
        bbox::Bbox3d,
        vec::{Vec2f, Vec3d},
    };
    use map_model::material::{Material, NormalMap};
    use render::environment::{Environment, EnvironmentImage};
    let (device, queue) = device().expect("normal horizon validation requires a GPU");
    let mut document = Document::new(Default::default());
    document.add_brushes(vec![map_model::shapes::create_cuboid(
        &document.world_bounds(),
        &Bbox3d::from_scalars(-32.0, 32.0),
        "horizon",
    )
    .unwrap()]);
    let mut triangles = document.render_triangles();
    for triangle in &mut triangles {
        triangle.material = Some(0);
        triangle.surface_material = None;
        triangle.color = [1.0; 3];
        triangle.selected = false;
        triangle.uv = [
            Vec2f::new(0.0, 0.0),
            Vec2f::new(1.0, 0.0),
            Vec2f::new(0.0, 1.0),
        ];
    }
    let mut material = Material::rgba("horizon".into(), Default::default(), 1, 1, vec![255; 4]);
    material.normal_map = Some(NormalMap::rgba(
        Default::default(),
        1,
        1,
        vec![245, 128, 180, 255],
        false,
    ));
    let mut renderer = Renderer::new(&device, &queue);
    renderer.scene.set_materials(&[material]);
    renderer.scene.upsert(0, &triangles, Default::default());
    renderer.scene.finish();
    renderer.scene.load_textures_now();
    let mut camera = Camera3d {
        position: Vec3d::new(-70.0, -160.0, 0.0),
        ..Default::default()
    };
    camera.look_along(-camera.position);
    let mut settings = single_frame_settings();
    settings.sun_intensity = 0.0;
    settings.bloom = false;
    settings.shadows = false;
    settings.ambient_occlusion = false;
    settings.reflections = false;
    settings.default_metalness = 1.0;
    settings.default_roughness = 0.1;
    settings.ambient_intensity = 1.0;
    settings.environment = Some(Environment {
        image: std::sync::Arc::new(
            EnvironmentImage::new(8, 4, vec![[2.0, 2.0, 2.0, 1.0]; 32]).unwrap(),
        ),
        rotation_degrees: 0.0,
        intensity: 1.0,
        show_background: false,
    });
    settings.indirect_occlusion = false;
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Aces,
        (240, 180),
    ));
    let before = renderer.read_output().unwrap();
    settings.indirect_occlusion = true;
    renderer.render(&view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::Aces,
        (240, 180),
    ));
    let after = renderer.read_output().unwrap();
    assert!(
        mean(&after.2) < mean(&before.2) - 1.0,
        "normal-map horizon must reject light below the geometric surface"
    );
    save("normal-horizon-before", &before);
    save("normal-horizon-after", &after);
}

#[test]
#[ignore = "measures navigation GPU cost at native and reduced resolution"]
fn navigation_resolution_benchmark() {
    let (device, queue) = device().expect("navigation benchmark requires a GPU");
    assert!(device.features().contains(wgpu::Features::TIMESTAMP_QUERY));
    let document = open("demos/lantern_lane/lantern_lane.map");
    let mut renderer = Renderer::new(&device, &queue);
    renderer.set_frame_timing(true);
    SceneTracker::new().sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    renderer.scene.load_textures_now();
    let mut camera = Camera3d::default();
    camera.frame_in_viewport(&document.bounds().unwrap(), 1280.0, 800.0);
    let mut settings = single_frame_settings();
    settings.antialiasing = true;
    settings.accumulate = false;
    settings.shadows = true;
    settings.ambient_occlusion = true;
    settings.contact_shadows = true;
    settings.reflections = true;
    settings.bloom = true;
    for (name, size) in [
        ("navigation-native", (1280, 800)),
        ("navigation-reduced", (896, 560)),
    ] {
        let mut times = Vec::new();
        for frame in 0..24 {
            let mut moving = camera;
            moving.turn(f64::from(frame) * 0.002, 0.0);
            renderer.render(&view(
                &moving,
                &settings,
                DisplayMode::Material,
                Tonemapper::Agx,
                size,
            ));
            renderer.wait_idle();
            if let Some(ms) = renderer.take_frame_time() {
                if frame >= 4 {
                    times.push(ms);
                }
            }
        }
        times.sort_by(f64::total_cmp);
        assert!(!times.is_empty());
        eprintln!(
            "{name}: {}x{}, median GPU {:.3} ms",
            size.0,
            size.1,
            times[times.len() / 2]
        );
        save(name, &renderer.read_output().unwrap());
    }
}

fn constant_local_probe(
    color: [f32; 3],
    reflections: bool,
    diffuse: bool,
    position: [f32; 3],
) -> std::sync::Arc<render::probes::Lighting> {
    let size = 16u32;
    let volume = render::probes::Volume {
        position,
        half_size: [96.0; 3],
        blend: 0.2,
        intensity: 1.0,
        reflections,
        diffuse,
    };
    let mips = (0..=size.ilog2())
        .map(|level| {
            vec![[color[0], color[1], color[2], 1.0]; ((size >> level).pow(2) * 6) as usize]
        })
        .collect();
    let mut sh = [[0.0; 3]; 9];
    sh[0] = color.map(|v| v * 3.54491);
    std::sync::Arc::new(
        render::probes::Lighting::new(vec![render::probes::BakedProbe {
            volume,
            size,
            mips,
            sh,
        }])
        .unwrap(),
    )
}
#[test]
fn local_reflections_and_diffuse_light_work_without_screen_space_hits_and_fade_outside_the_volume()
{
    use editor_core::render_scene::SceneSink;
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    use render::environment::{Environment, EnvironmentImage};
    let (device, queue) = device().expect("local lighting validation requires a GPU");
    let mut document = Document::new(Default::default());
    document.add_brushes(vec![map_model::shapes::create_cuboid(
        &document.world_bounds(),
        &Bbox3d::from_scalars(-32.0, 32.0),
        "probe-receiver",
    )
    .unwrap()]);
    let mut triangles = document.render_triangles();
    for triangle in &mut triangles {
        triangle.color = [1.0; 3];
        triangle.material = None;
        triangle.selected = false;
        triangle.surface_material = None;
    }
    let mut renderer = Renderer::new(&device, &queue);
    renderer.scene.upsert(0, &triangles, Default::default());
    renderer.scene.finish();
    let mut camera = Camera3d {
        position: Vec3d::new(0.0, -160.0, 40.0),
        ..Default::default()
    };
    camera.look_along(-camera.position);
    let mut settings = single_frame_settings();
    settings.sun_intensity = 0.0;
    settings.ambient_intensity = 1.0;
    settings.bloom = false;
    settings.shadows = false;
    settings.ambient_occlusion = false;
    settings.reflections = false;
    settings.indirect_occlusion = false;
    settings.default_metalness = 1.0;
    settings.default_roughness = 0.15;
    settings.environment = Some(Environment {
        image: std::sync::Arc::new(
            EnvironmentImage::new(8, 4, vec![[0.02, 0.02, 2.0, 1.0]; 32]).unwrap(),
        ),
        rotation_degrees: 0.0,
        intensity: 1.0,
        show_background: false,
    });
    let draw = |renderer: &mut Renderer, settings: &render::RenderSettings| {
        renderer.render(&view(
            &camera,
            settings,
            DisplayMode::Material,
            Tonemapper::Aces,
            (240, 180),
        ));
        renderer.read_output().unwrap()
    };
    let global = draw(&mut renderer, &settings);
    settings.probes = Some(constant_local_probe(
        [4.0, 0.02, 0.02],
        true,
        false,
        [0.0; 3],
    ));
    let reflection = draw(&mut renderer, &settings);
    let sum = |image: &[u8], channel: usize| {
        image
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| u64::from(p[channel]))
            .sum::<u64>()
    };
    assert!(
        sum(&reflection.2, 0) > sum(&global.2, 0) + 10000,
        "local captured geometry must reflect even with SSR disabled"
    );
    settings.probes = Some(constant_local_probe(
        [4.0, 0.02, 0.02],
        true,
        false,
        [10000.0; 3],
    ));
    let outside = draw(&mut renderer, &settings);
    assert_eq!(
        outside.2, global.2,
        "outside the volume the global environment is retained"
    );
    settings.default_metalness = 0.0;
    settings.probes = None;
    let before = draw(&mut renderer, &settings);
    settings.probes = Some(constant_local_probe([2.0, 0.0, 0.0], false, true, [0.0; 3]));
    let bounce = draw(&mut renderer, &settings);
    assert!(
        sum(&bounce.2, 0) > sum(&before.2, 0) + 10000,
        "local SH must transfer colored light to diffuse surfaces"
    );
    assert!(
        sum(&bounce.2, 2) < sum(&before.2, 2),
        "the local volume replaces occluded outside diffuse light"
    );
    save("probe-global-environment", &global);
    save("probe-local-reflections", &reflection);
    save("probe-global-diffuse", &before);
    save("probe-local-diffuse", &bounce);
}
#[test]
fn clearcoat_and_anisotropy_change_the_brdf_and_rotation_changes_its_direction() {
    use editor_core::render_scene::SceneSink;
    use geometry_math::{
        bbox::Bbox3d,
        vec::{Vec2f, Vec3d},
    };
    use map_model::brush_face::SurfaceMaterial;
    let (device, queue) = device().expect("layered material validation requires a GPU");
    let mut renderer = Renderer::new(&device, &queue);
    let mut document = Document::new(Default::default());
    document.add_brushes(vec![map_model::shapes::create_cuboid(
        &document.world_bounds(),
        &Bbox3d::from_scalars(-32.0, 32.0),
        "layers",
    )
    .unwrap()]);
    let mut triangles = document.render_triangles();
    for triangle in &mut triangles {
        triangle.color = [0.4, 0.5, 0.7];
        triangle.material = None;
        triangle.selected = false;
        triangle.uv = [
            Vec2f::new(0.0, 0.0),
            Vec2f::new(64.0, 0.0),
            Vec2f::new(0.0, 64.0),
        ];
    }
    let mut camera = Camera3d {
        position: Vec3d::new(0.0, -160.0, 45.0),
        ..Default::default()
    };
    camera.look_along(-camera.position);
    let mut settings = single_frame_settings();
    settings.bloom = false;
    settings.shadows = false;
    settings.ambient_occlusion = false;
    settings.reflections = false;
    settings.sun_elevation = 15.0;
    settings.sun_azimuth = 270.0;
    settings.sun_intensity = 4.0;
    settings.ambient_intensity = 0.0;
    let mut images = Vec::new();
    for (name, surface) in [
        (
            "material-base",
            SurfaceMaterial {
                roughness: 55,
                metallic: 70,
                ..Default::default()
            },
        ),
        (
            "material-clearcoat",
            SurfaceMaterial {
                roughness: 55,
                metallic: 70,
                clearcoat: 100,
                coat_roughness: 10,
                ..Default::default()
            },
        ),
        (
            "material-anisotropy-0",
            SurfaceMaterial {
                roughness: 55,
                metallic: 70,
                anisotropy: 90,
                ..Default::default()
            },
        ),
        (
            "material-anisotropy-90",
            SurfaceMaterial {
                roughness: 55,
                metallic: 70,
                anisotropy: 90,
                anisotropy_rotation: 90,
                ..Default::default()
            },
        ),
    ] {
        for triangle in &mut triangles {
            triangle.surface_material = Some(surface);
        }
        renderer.scene.upsert(0, &triangles, Default::default());
        renderer.scene.finish();
        renderer.render(&view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Aces,
            (320, 240),
        ));
        let image = renderer.read_output().unwrap();
        save(name, &image);
        images.push(image);
    }
    let difference = |a: usize, b: usize| {
        images[a]
            .2
            .iter()
            .zip(&images[b].2)
            .map(|(a, b)| u64::from(a.abs_diff(*b)))
            .sum::<u64>()
    };
    assert!(
        difference(0, 1) > 1000,
        "clear coat must add a separate specular response"
    );
    assert!(
        difference(0, 2) > 1000,
        "anisotropy must change the highlight"
    );
    assert!(
        difference(2, 3) > 1000,
        "rotating the UV tangent must rotate the anisotropic highlight"
    );
}
#[test]
fn progressive_hdr_keeps_small_contributions_at_4096_samples() {
    use render::environment::{Environment, EnvironmentImage};
    let (device, queue) = device().expect("progressive precision validation requires a GPU");
    let mut renderer = Renderer::new(&device, &queue);
    let camera = Camera3d::default();
    let mut settings = single_frame_settings();
    settings.bloom = false;
    settings.reflections = false;
    settings.shadows = false;
    settings.ambient_occlusion = false;
    let environment = |pixel| Environment {
        image: std::sync::Arc::new(EnvironmentImage::new(2, 1, vec![pixel; 2]).unwrap()),
        rotation_degrees: 0.0,
        intensity: 1.0,
        show_background: true,
    };
    settings.environment = Some(environment([1.0, 0.0, 0.0, 1.0]));
    for sample in 0..4096 {
        if sample == 4095 {
            settings.environment = Some(environment([0.0, 0.0, 4.0, 1.0]));
        }
        let mut params = view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Aces,
            (2, 2),
        );
        params.sample = Some(sample);
        renderer.accumulate(&params);
    }
    let output = renderer.read_hdr().unwrap();
    for pixel in output.2.as_chunks::<4>().0 {
        assert!(
            (pixel[0] - 4095.0 / 4096.0).abs() < 0.00001,
            "large histories must not freeze in half precision: {pixel:?}"
        );
        assert!((pixel[2] - 4.0 / 4096.0).abs() < 0.00001);
        assert_eq!(pixel[3], 1.0);
    }
}

#[test]
fn baked_emissive_geometry_lights_a_receiver_without_being_visible_in_the_camera() {
    use editor_core::light_probes::{self, Probe, Settings};
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    use map_model::brush_face::{FaceColor, Rgb, SurfaceMaterial};
    let (device, queue) = device().expect("baked diffuse transport requires a GPU");
    let mut document = Document::new(Default::default());
    let world = document.world_bounds();
    let cuboid = |lo: [f64; 3], hi: [f64; 3]| {
        map_model::shapes::create_cuboid(
            &world,
            &Bbox3d::new(
                Vec3d::new(lo[0], lo[1], lo[2]),
                Vec3d::new(hi[0], hi[1], hi[2]),
            ),
            "bounce-room",
        )
        .unwrap()
    };
    document.add_brushes(vec![cuboid([-72.0, -72.0, -8.0], [72.0, 72.0, 0.0])]);
    document.select_all();
    document.set_face_color(Some(FaceColor::solid(Rgb::new(255, 0, 0))));
    document.set_surface_material(Some(SurfaceMaterial {
        emission: 400,
        ..Default::default()
    }));
    document.clear_selection();
    document.add_brushes(vec![
        cuboid([-72.0, -72.0, 64.0], [72.0, 72.0, 72.0]),
        cuboid([-72.0, -72.0, 0.0], [-64.0, 72.0, 64.0]),
        cuboid([64.0, -72.0, 0.0], [72.0, 72.0, 64.0]),
        cuboid([-64.0, -72.0, 0.0], [64.0, -64.0, 64.0]),
        cuboid([-64.0, 64.0, 0.0], [64.0, 72.0, 64.0]),
    ]);
    document.clear_selection();
    let mut settings = single_frame_settings();
    settings.sun_intensity = 0.0;
    settings.sun_color = [0.0; 3];
    settings.ambient_intensity = 1.0;
    settings.sky_color = [0.0; 3];
    settings.horizon_color = [0.0; 3];
    settings.ground_color = [0.0; 3];
    settings.shadows = false;
    settings.reflections = false;
    settings.ambient_occlusion = false;
    settings.bloom = false;
    let config = Settings {
        probes: vec![Probe {
            position: [0.0, 0.0, 32.0],
            half_size: [80.0, 80.0, 48.0],
            ..Default::default()
        }],
        resolution: 32,
        ..Default::default()
    };
    let lighting =
        std::sync::Arc::new(light_probes::bake_blocking(&document, &config, &settings).unwrap());
    let mut renderer = Renderer::new(&device, &queue);
    SceneTracker::new().sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d {
        position: Vec3d::new(0.0, -48.0, 32.0),
        fov_degrees: 25.0,
        ..Default::default()
    };
    camera.look_along(Vec3d::new(0.0, 1.0, 0.0));
    let draw = |renderer: &mut Renderer, settings: &render::RenderSettings| {
        renderer.render(&view(
            &camera,
            settings,
            DisplayMode::Material,
            Tonemapper::Aces,
            (320, 240),
        ));
        renderer.read_output().unwrap()
    };
    let direct = draw(&mut renderer, &settings);
    save("baked-gi-direct-only", &direct);
    assert!(
        mean(&direct.2) < 1.0,
        "the emissive floor must be outside the camera image: {}",
        mean(&direct.2)
    );
    settings.probes = Some(lighting);
    let bounce = draw(&mut renderer, &settings);
    assert!(
        mean(&bounce.2) > mean(&direct.2) + 5.0,
        "off-screen emission must light the visible receiver"
    );
    assert!(
        bounce
            .2
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] > p[1].saturating_add(40)),
        "diffuse transport must preserve the emitter's red color"
    );
    save("baked-gi-direct-only", &direct);
    save("baked-gi-red-bounce", &bounce);
}

#[test]
fn imported_sheen_renders_in_viewport_and_offline() {
    use editor_core::render_scene::SceneSink;
    let (device, queue) = device().expect("sheen validation requires a GPU");
    let source = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/editor-checks/fixtures/pbr.glb"
    );
    let mut document = Document::new(editor_core::world::WorldBounds::default());
    document
        .import_model(
            std::path::Path::new(source),
            geometry_math::vec::Vec3d::zero(),
        )
        .unwrap();
    let masked_source = std::path::Path::new(source).with_file_name("sheen_masks.glb");
    let mut masked_document = Document::new(Default::default());
    masked_document
        .import_model(&masked_source, geometry_math::vec::Vec3d::zero())
        .unwrap();
    let masked_materials = masked_document.materials().materials();
    use editor_core::export::{export_files, Contents, Options, Target};
    let (_, files) = export_files(
        &masked_document,
        std::path::Path::new("."),
        &Options {
            target: Target::Glb,
            name: "sheen-round-trip".into(),
            collision: false,
            cull_hidden: false,
            ..Default::default()
        },
    )
    .unwrap();
    let Contents::Bytes(bytes) = &files[0].1 else {
        panic!("expected GLB bytes");
    };
    let exported = map_model::glb_model::read(bytes).unwrap();
    assert_eq!(exported.pbr.len(), 2);
    for pbr in &exported.pbr {
        assert_eq!(pbr.sheen_color, [0., 1., 0.]);
        assert_eq!(pbr.sheen_roughness, 0.75);
        assert_eq!(
            pbr.sheen_color_texture.as_ref().unwrap().pixels,
            [0, 0, 0, 255, 255, 200, 64, 255],
        );
        assert_eq!(
            pbr.sheen_roughness_texture.as_ref().unwrap().pixels,
            [255, 255, 255, 64, 255, 255, 255, 192],
        );
    }
    let mut renderer = Renderer::new(&device, &queue);
    SceneTracker::new().sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    let mut camera = Camera3d::default();
    camera.frame_in_viewport(&document.selection_bounds().unwrap(), 320., 240.);
    let mut settings = single_frame_settings();
    settings.shadows = false;
    settings.contact_shadows = false;
    settings.ambient_intensity = 0.;
    settings.bloom = false;
    settings.reflections = false;
    let original = document.materials().materials().to_vec();
    let mut coated = original.clone();
    for material in &mut coated {
        if let Some(pbr) = &material.model_pbr {
            let mut pbr = (**pbr).clone();
            pbr.sheen_color = [0.0, 1.0, 0.0];
            pbr.sheen_roughness = 0.75;
            material.model_pbr = Some(std::sync::Arc::new(pbr));
        }
    }
    for (angle, elevation) in [("front", 80.), ("grazing", 12.), ("hdr-only", 0.)] {
        settings.sun_elevation = elevation;
        if angle == "hdr-only" {
            use render::environment::{Environment, EnvironmentImage};
            settings.sun_color = [0.0; 3];
            settings.ambient_intensity = 1.0;
            settings.environment = Some(Environment {
                image: std::sync::Arc::new(
                    EnvironmentImage::new(8, 4, vec![[2.0, 2.0, 2.0, 1.0]; 32]).unwrap(),
                ),
                rotation_degrees: 0.0,
                intensity: 1.0,
                show_background: false,
            });
        }
        let mut render_pair = |materials: &[map_model::material::Material], label: &str| {
            renderer.scene.set_materials(materials);
            renderer.scene.finish();
            renderer.scene.load_textures_now();
            let mut params = view(
                &camera,
                &settings,
                DisplayMode::Material,
                Tonemapper::Agx,
                (320, 240),
            );
            params.lights = false;
            renderer.render(&params);
            let viewport = renderer.read_output().unwrap();
            save(&format!("sheen-{angle}-{label}-viewport"), &viewport);
            for sample in 0..8 {
                params.sample = Some(sample);
                renderer.accumulate(&params);
            }
            save(
                &format!("sheen-{angle}-{label}-still"),
                &renderer.read_output().unwrap(),
            );
            let hdr = renderer.read_hdr().unwrap();
            renderer.release_accumulation();
            (viewport.2, hdr.2)
        };
        let (viewport_off, hdr_off) = render_pair(&original, "off");
        let (viewport_on, hdr_on) = render_pair(&coated, "on");
        if angle == "front" {
            let (imported_viewport, imported_hdr) = render_pair(masked_materials, "glb-masks");
            assert_ne!(imported_viewport, viewport_off);
            assert_ne!(imported_viewport, viewport_on);
            assert_ne!(imported_hdr, hdr_off);
            assert_ne!(imported_hdr, hdr_on);
            let with_mask = |color: Option<[u8; 4]>, roughness: Option<[u8; 4]>| {
                let mut materials = coated.clone();
                for material in &mut materials {
                    if let Some(pbr) = &mut material.model_pbr {
                        let pbr = std::sync::Arc::make_mut(pbr);
                        let texture = |pixels: [u8; 4]| {
                            std::sync::Arc::new(map_model::entity_model::Skin {
                                width: 1,
                                height: 1,
                                pixels: pixels.to_vec(),
                            })
                        };
                        pbr.sheen_color_texture = color.map(texture);
                        pbr.sheen_roughness_texture = roughness.map(texture);
                    }
                }
                materials
            };
            let black = with_mask(Some([0, 0, 0, 255]), None);
            let (masked_viewport, masked_hdr) = render_pair(&black, "color-mask");
            assert_eq!(masked_viewport, viewport_off);
            assert_eq!(masked_hdr, hdr_off);
            let rough = with_mask(None, Some([255, 255, 255, 64]));
            let (rough_viewport, rough_hdr) = render_pair(&rough, "roughness-mask");
            assert_ne!(rough_viewport, viewport_on);
            assert_ne!(rough_hdr, hdr_on);
        }
        let changed = viewport_off
            .chunks_exact(4)
            .zip(viewport_on.chunks_exact(4))
            .filter(|(a, b)| a[1].abs_diff(b[1]) > 1)
            .count();
        let added_green = hdr_off
            .chunks_exact(4)
            .zip(hdr_on.chunks_exact(4))
            .filter(|(a, b)| b[1] > a[1] + 0.001)
            .count();
        assert!(hdr_on.iter().all(|v| v.is_finite()));
        println!("{angle}: viewport changed {changed} pixels, HDR gained green in {added_green}");
        assert!(changed > 20 && added_green > 20);
    }
}

#[test]
fn glass_refracts_grid_in_viewport_and_still() {
    use editor_core::render_scene::SceneSink;
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    use map_model::{material::Material, model_pbr::ModelPbr};
    use std::sync::Arc;

    let (device, queue) = device().expect("glass validation requires a GPU");
    let mut doc = Document::new(Default::default());
    let mut add_box = |min, max| {
        doc.add_brushes(vec![map_model::shapes::create_cuboid(
            &doc.world_bounds(),
            &Bbox3d::new(min, max),
            "glass-test",
        )
        .unwrap()]);
    };
    add_box(Vec3d::new(-42., -1., -42.), Vec3d::new(42., 1., 42.));
    add_box(Vec3d::new(-34., 12., -34.), Vec3d::new(34., 14., 34.));
    for x in (-80..=80).step_by(10).map(f64::from) {
        add_box(Vec3d::new(x, 32., -90.), Vec3d::new(x + 3., 33., 90.));
        add_box(Vec3d::new(-90., 33., x), Vec3d::new(90., 34., x + 3.));
    }
    let (mut rear, mut triangles): (Vec<_>, Vec<_>) = doc
        .render_triangles()
        .into_iter()
        .partition(|triangle| triangle.positions.iter().all(|p| p.y() > 8. && p.y() < 20.));
    assert_eq!(rear.len(), 12);
    for triangle in &mut rear {
        triangle.material = Some(0);
        triangle.color = [1.; 3];
        triangle.selected = false;
    }
    for triangle in &mut triangles {
        let is_glass = triangle.positions.iter().all(|p| p.y() < 16.);
        triangle.material = Some(usize::from(!is_glass));
        triangle.color = [1.; 3];
        triangle.selected = false;
    }
    assert_eq!(
        triangles
            .iter()
            .filter(|triangle| triangle.material == Some(0))
            .count(),
        12,
        "all six pane faces must use the glass material"
    );
    let white = || Material::rgba("white".into(), Default::default(), 1, 1, vec![255; 4]);
    let mut grid = white();
    grid.model_pbr = Some(Arc::new(ModelPbr {
        emissive_factor: [1.5; 3],
        metallic: 0.,
        ..Default::default()
    }));
    let mut renderer = Renderer::new(&device, &queue);
    let mut camera = Camera3d {
        position: Vec3d::new(35., -140., 10.),
        ..Default::default()
    };
    camera.look_along(-camera.position);
    let mut settings = single_frame_settings();
    settings.procedural_sky = false;
    settings.shadows = false;
    settings.ambient_occlusion = false;
    settings.reflections = false;
    settings.bloom = false;
    settings.sun_intensity = 0.;
    settings.ambient_intensity = 0.;
    settings.background_color = [0.05; 3];
    let mut draw = |ior, tint: [u8; 3], name: &str, pane_count: u64, joined, camera: &Camera3d| {
        let mut glass = Material::rgba(
            "glass".into(),
            Default::default(),
            1,
            1,
            [tint[0], tint[1], tint[2], 255].to_vec(),
        );
        glass.model_pbr = Some(Arc::new(ModelPbr {
            transmission: 1.,
            ior,
            metallic: 0.,
            ..Default::default()
        }));
        renderer.scene.set_materials(&[glass, grid.clone()]);
        if joined {
            let merged: Vec<_> = triangles.iter().chain(&rear).cloned().collect();
            renderer.scene.upsert(0, &merged, Default::default());
        } else {
            renderer.scene.upsert(0, &triangles, Default::default());
        }
        for key in 1..=9 {
            if key < pane_count && !joined {
                let mut pane = rear.clone();
                for triangle in &mut pane {
                    for point in &mut triangle.positions {
                        *point = *point + Vec3d::new(0., (key - 1) as f64 * 3., 0.);
                    }
                }
                renderer.scene.upsert(key, &pane, Default::default());
            } else {
                renderer.scene.remove(key);
            }
        }
        renderer.scene.finish();
        renderer.scene.load_textures_now();
        let mut params = view(
            camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Agx,
            (320, 240),
        );
        renderer.render(&params);
        let viewport = renderer.read_output().unwrap();
        save(&format!("glass-{name}-viewport"), &viewport);
        for sample in 0..8 {
            params.sample = Some(sample);
            renderer.accumulate(&params);
        }
        let still = renderer.read_output().unwrap();
        save(&format!("glass-{name}-still"), &still);
        renderer.release_accumulation();
        if name == "clear" && device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            renderer.set_profiling(true);
            let params = view(
                camera,
                &settings,
                DisplayMode::Material,
                Tonemapper::Agx,
                (1280, 720),
            );
            let mut durations = Vec::new();
            for frame in 0..24 {
                renderer.render(&params);
                for &(pass, ms) in renderer.gpu_times() {
                    if frame >= 4 && pass == "refraction" && ms > 0.0 {
                        durations.push(ms);
                    }
                }
            }
            durations.sort_by(f64::total_cmp);
            println!(
                "1280×720 refraction pass median ms: {:?}",
                durations.get(durations.len() / 2)
            );
            renderer.set_profiling(false);
        }
        [viewport.2, still.2]
    };
    let straight = draw(1.0, [255; 3], "straight", 1, false, &camera);
    let clear = draw(1.5, [255; 3], "clear", 1, false, &camera);
    let tinted = draw(1.5, [255, 55, 55], "red", 1, false, &camera);
    for ((straight, clear), tinted) in straight.iter().zip(&clear).zip(&tinted) {
        let displaced = straight
            .as_chunks::<4>()
            .0
            .iter()
            .zip(clear.as_chunks::<4>().0)
            .filter(|(a, b)| a[0].abs_diff(b[0]) > 60)
            .count();
        assert!(displaced > 100, "expected the grid to move: {displaced}");
        assert!(tinted
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] > p[1].saturating_add(60)));
        println!("glass displaced {displaced} grid pixels");
    }
    let single = draw(1.0, [255, 160, 160], "one-pane", 1, false, &camera);
    let double = draw(1.0, [255, 160, 160], "two-panes", 2, false, &camera);
    for (single, double) in single.iter().zip(&double) {
        let doubly_tinted = single
            .as_chunks::<4>()
            .0
            .iter()
            .zip(double.as_chunks::<4>().0)
            .filter(|(a, b)| a[1] > b[1].saturating_add(12) && a[0].abs_diff(b[0]) < 20)
            .count();
        assert!(
            doubly_tinted > 100,
            "expected stacked absorption: {doubly_tinted}"
        );
        println!("second pane tinted {doubly_tinted} grid pixels");
    }
    let joined = draw(1.0, [255, 160, 160], "joined-panes", 2, true, &camera);
    assert_ne!(
        joined[0], double[0],
        "shared object/material uses one snapshot"
    );
    let crowded = draw(1.0, [255, 160, 160], "ten-panes", 10, false, &camera);
    assert_ne!(
        crowded[0], double[0],
        "crowded scenes retain nearest layers"
    );
    assert_ne!(crowded[1], double[1]);

    // View from behind the glass but in front of the grid, reversing pane depth.
    let mut reversed = Camera3d {
        position: Vec3d::new(8., 30., 2.),
        ..Default::default()
    };
    reversed.look_along(-reversed.position);
    let rear_single = draw(
        1.5,
        [255, 160, 160],
        "reverse-one-pane",
        1,
        false,
        &reversed,
    );
    let rear_double = draw(
        1.5,
        [255, 160, 160],
        "reverse-two-panes",
        2,
        false,
        &reversed,
    );
    assert_ne!(rear_double[0], rear_single[0]);
    assert_ne!(rear_double[1], rear_single[1]);
}

#[test]
fn sun_shadows_transmit_glass_tint() {
    use editor_core::render_scene::SceneSink;
    use geometry_math::{bbox::Bbox3d, vec::Vec3d};
    use map_model::{material::Material, model_pbr::ModelPbr};

    let (device, queue) = device().expect("colored shadows require a GPU");
    let mut document = Document::new(Default::default());
    let world = document.world_bounds();
    let cube = |min, max| {
        map_model::shapes::create_cuboid(&world, &Bbox3d::new(min, max), "shadow-test").unwrap()
    };
    document.add_brushes(vec![
        cube(Vec3d::new(-210., -210., -8.), Vec3d::new(210., 210., 0.)),
        cube(Vec3d::new(-34., -34., 70.), Vec3d::new(34., 34., 72.)),
    ]);
    let mut triangles = document.render_triangles();
    for triangle in &mut triangles {
        triangle.material = triangle.positions.iter().all(|p| p.z() > 50.).then_some(0);
        triangle.color = [1.; 3];
    }
    let camera = Camera3d {
        position: Vec3d::new(0., 0., 420.),
        pitch: -std::f64::consts::FRAC_PI_2,
        ..Default::default()
    };
    let mut settings = single_frame_settings();
    settings.sun_elevation = 35.;
    settings.sun_intensity = 3.;
    settings.sun_color = [1.; 3];
    settings.ambient_intensity = 0.;
    settings.shadows = true;
    settings.contact_shadows = false;
    settings.bloom = false;
    settings.reflections = false;
    settings.ambient_occlusion = false;
    let mut renderer = Renderer::new(&device, &queue);
    let mut draw = |tint: [u8; 3], transmission, name: &str, geometry: Option<&[_]>| {
        let mut glass = Material::rgba(
            "glass".into(),
            Default::default(),
            1,
            1,
            [tint[0], tint[1], tint[2], 255].to_vec(),
        );
        glass.model_pbr = Some(std::sync::Arc::new(ModelPbr {
            transmission,
            metallic: 0.,
            double_sided: true,
            ..Default::default()
        }));
        if !matches!(name, "shadow-red-moved" | "shadow-red-returned") {
            renderer.scene.set_materials(&[glass]);
        }
        if let Some(geometry) = geometry {
            renderer.scene.upsert(0, geometry, Default::default());
        }
        renderer.scene.finish();
        renderer.scene.load_textures_now();
        renderer.render(&view(
            &camera,
            &settings,
            DisplayMode::Material,
            Tonemapper::Agx,
            (320, 240),
        ));
        let image = renderer.read_output().unwrap();
        save(name, &image);
        image.2
    };
    let clear = draw([255; 3], 1., "shadow-clear-glass", Some(&triangles));
    let red = draw([255, 45, 45], 1., "shadow-red-glass", None);
    let opaque = draw([255; 3], 0., "shadow-opaque", None);
    let tinted = clear
        .as_chunks::<4>()
        .0
        .iter()
        .zip(red.as_chunks::<4>().0)
        .zip(opaque.as_chunks::<4>().0)
        .enumerate()
        .filter(|(index, ((clear, red), opaque))| {
            let (x, y) = (index % 320, index / 320);
            let outside_pane = x.abs_diff(160) > 35 || y.abs_diff(120) > 35;
            outside_pane
                && clear[1] > red[1].saturating_add(35)
                && red[0] > red[1].saturating_add(35)
                && red[0] > opaque[0].saturating_add(35)
        })
        .count();
    assert!(
        tinted > 100,
        "expected a colored shadow on the floor: {tinted}"
    );
    println!("colored sun shadow on {tinted} floor pixels");

    let recolored = draw([255, 45, 45], 1., "shadow-red-restored", None);
    assert_eq!(recolored, red, "changing material must refresh cached tint");
    let mut moved = triangles.clone();
    for triangle in moved
        .iter_mut()
        .filter(|triangle| triangle.material == Some(0))
    {
        for point in &mut triangle.positions {
            *point = *point + Vec3d::new(100., 0., 0.);
        }
    }
    let shifted = draw([255, 45, 45], 1., "shadow-red-moved", Some(&moved));
    let cleared = red
        .as_chunks::<4>()
        .0
        .iter()
        .zip(clear.as_chunks::<4>().0)
        .zip(shifted.as_chunks::<4>().0)
        .filter(|((red, clear), shifted)| {
            clear[1] > red[1].saturating_add(35) && shifted[1].abs_diff(clear[1]) < 5
        })
        .count();
    assert!(
        cleared > 100,
        "moving glass must clear the old shadow: {cleared}"
    );
    let restored = draw([255, 45, 45], 1., "shadow-red-returned", Some(&triangles));
    assert_eq!(restored, red, "moving glass back must restore the shadow");
}

/// Render one shared glTF scene per process so RSS is attributable to that scene.
#[test]
#[ignore = "set RENDER_WGPU_COMPARE_SCENE and RENDER_WGPU_SHOTS to benchmark a shared GLB"]
fn shared_glb_benchmark() {
    use geometry_math::vec::Vec3d;
    let source = std::env::var_os("RENDER_WGPU_COMPARE_SCENE").expect("shared GLB path");
    let mut document = Document::new(Default::default());
    document
        .import_model(std::path::Path::new(&source), Vec3d::zero())
        .unwrap();
    document.clear_selection();
    let (device, queue) = device().expect("benchmark requires a GPU");
    let mut renderer = Renderer::new(&device, &queue);
    SceneTracker::new().sync(
        &document,
        &ViewFilters::default(),
        None,
        Pose::Rest,
        &mut renderer.scene,
    );
    renderer.scene.load_textures_now();
    let mut camera = Camera3d {
        position: Vec3d::new(2., -3., 1.4) * 32.,
        near: 0.32,
        far: 3200.,
        fov_degrees: 60.,
        ..Default::default()
    };
    camera.look_along(-camera.position);
    let mut settings = single_frame_settings();
    settings.procedural_sky = false;
    settings.background_color = [0.; 3];
    settings.show_axes = false;
    settings.environment = None;
    settings.ambient_intensity = 0.;
    settings.ambient_occlusion = false;
    settings.contact_shadows = false;
    settings.reflections = false;
    settings.bloom = false;
    settings.shadows = true;
    settings.sun_color = [1.; 3];
    settings.sun_intensity = 100000. / (1.2 * 16. * 16. * 125.);
    settings.sun_elevation = 54.735610317;
    settings.sun_azimuth = -45.;
    settings.exposure = 1.;
    let mut params = view(
        &camera,
        &settings,
        DisplayMode::Material,
        Tonemapper::KhronosNeutral,
        (320, 240),
    );
    params.lights = false;
    for _ in 0..64 {
        renderer.render(&params);
        renderer.wait_idle();
    }
    save("heron", &renderer.read_output().unwrap());
}
