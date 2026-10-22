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
            println!("  GPU {label}: mean {mean:.3} ms, median {:.3} ms, p95 {:.3} ms over {n} frames (cached frames included as zero)",
                values[values.len() / 2], values[values.len() * 95 / 100]);
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
    eprintln!("animation: flicker {flicker:.2}, error against converged {error:.2}, pixels off by more than 32: {trail}");
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
    assert!(shadow_changes[1]>shadow_changes[0]+50,
        "Double-sided caster must add a receiver shadow that its back-culled variant lacks: {shadow_changes:?}");
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
