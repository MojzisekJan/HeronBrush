//! Synthetic workloads for the performance benchmarks (`docs/performance.md`).
//!
//! Everything here is generated deterministically from a seed, so a benchmark run on one
//! machine and on another measure the same map, and nothing big has to be committed:
//! maps of thousands of brushes, folders of thousands of textures and a map full of
//! animated entities are written into a temporary directory on demand.
//!
//! Only built with the `test-support` feature.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use geometry_math::bbox::Bbox3d;
use geometry_math::util::Axis;
use geometry_math::vec::Vec3d;
use map_model::io::map_reader::{Brush as MapBrush, Entity, MapFile};
use map_model::shapes::{create_arch, create_cuboid, create_cylinder, CircleShape};

/// A small, fast, deterministic generator (xorshift64*).
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    /// A generator started from `seed`.
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// A number in `0..n`.
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n.max(1) as u64) as usize
    }
    /// A number in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// What a synthetic map holds.
#[derive(Clone, Debug)]
pub struct MapSpec {
    /// About how many brushes (arches come in several pieces, so it may overshoot by a few).
    pub brushes: usize,
    /// How many distinct material names the faces cycle through.
    pub materials: usize,
    /// How many `func_animate` entities with keyframes, each owning a few of the brushes.
    pub animated: usize,
    /// The seed.
    pub seed: u64,
}

impl MapSpec {
    /// A map of about `brushes` brushes over 64 materials, nothing animated.
    pub fn brushes(brushes: usize) -> Self {
        Self {
            brushes,
            materials: 64,
            animated: 0,
            seed: 1,
        }
    }
}

/// The name of material `index`, as the generated texture folders name their files.
pub fn material_name(index: usize) -> String {
    format!("bench/m{index:05}")
}

const WORLD: f64 = 16384.0;

fn world() -> Bbox3d {
    Bbox3d::from_scalars(-WORLD, WORLD)
}

/// One structure's brushes: a box, an eight-sided cylinder or a five-piece arch.
fn structure(rng: &mut Rng, at: Vec3d, material: &str) -> Vec<map_model::brush::Brush> {
    let world = world();
    let size = Vec3d::new(
        32.0 + 16.0 * rng.below(5) as f64,
        32.0 + 16.0 * rng.below(5) as f64,
        32.0 + 32.0 * rng.below(4) as f64,
    );
    let bounds = Bbox3d::new(at, at + size);
    let roll = rng.below(100);
    if roll < 60 {
        create_cuboid(&world, &bounds, material)
            .map(|b| vec![b])
            .unwrap_or_default()
    } else if roll < 85 {
        create_cylinder(
            &world,
            &bounds,
            CircleShape::EdgeAligned { num_sides: 8 },
            Axis::Z,
            material,
        )
        .map(|b| vec![b])
        .unwrap_or_default()
    } else {
        let axis = if rng.below(2) == 0 { Axis::X } else { Axis::Y };
        create_arch(
            &world,
            &bounds,
            8.0,
            CircleShape::EdgeAligned { num_sides: 10 },
            axis,
            material,
        )
        .unwrap_or_default()
    }
}

fn map_brush(brush: &map_model::brush::Brush) -> MapBrush {
    MapBrush {
        faces: brush.faces().to_vec(),
        id: None,
        line: 0,
    }
}

fn entity(properties: &[(&str, String)]) -> Entity {
    Entity {
        properties: properties
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect(),
        ..Default::default()
    }
}

/// The map text for `spec`: brushes laid out on a grid across the world, spread over the
/// default layer and three more, about a tenth of them in groups of twenty, some in
/// `func_wall`/`func_door` entities, a light for every fifty brushes, and `spec.animated`
/// keyframed `func_animate` entities.
pub fn synthetic_map(spec: &MapSpec) -> String {
    let mut rng = Rng::new(spec.seed);
    let cell = 160.0;
    let across = ((WORLD * 2.0 - 512.0) / cell) as usize;
    let mut slot = 0usize;
    let mut next_at = |rng: &mut Rng| {
        let x = slot % across;
        let y = (slot / across) % across;
        let z = slot / (across * across);
        slot += 1;
        let jitter = 8.0 * rng.below(4) as f64;
        Vec3d::new(
            -WORLD + 256.0 + x as f64 * cell + jitter,
            -WORLD + 256.0 + y as f64 * cell,
            -2048.0 + z as f64 * 384.0,
        )
    };
    let mut material = 0usize;
    let mut next_material = || {
        material = (material + 1) % spec.materials.max(1);
        material_name(material)
    };

    let mut worldspawn = entity(&[("classname", "worldspawn".into()), ("wad", String::new())]);
    let layer_ids = ["2000001", "2000002", "2000003"];
    let mut entities = Vec::new();
    for (i, id) in layer_ids.iter().enumerate() {
        entities.push(entity(&[
            ("classname", "func_group".into()),
            ("_tb_type", "_tb_layer".into()),
            ("_tb_name", format!("Layer {}", i + 1)),
            ("_tb_id", id.to_string()),
            ("_tb_layer_sort_index", i.to_string()),
        ]));
    }

    let mut made = 0usize;
    // The animated entities first, three or so brushes each.
    for a in 0..spec.animated {
        let at = next_at(&mut rng);
        let mut e = entity(&[
            ("classname", "func_animate".into()),
            (
                "animation_keys",
                format!(
                    "0 0 0 0 0 0 0; 1 {} 0 32 0 0 90; 2 0 {} 0 0 0 180; 4 0 0 0 0 0 360",
                    64 + a % 64,
                    32 + a % 32
                ),
            ),
            ("animation_mode", "loop".into()),
            ("start_on", "1".into()),
        ]);
        let m = next_material();
        for k in 0..3 {
            let brushes = structure(&mut rng, at + Vec3d::new(0.0, 0.0, 128.0 * k as f64), &m);
            made += brushes.len();
            e.brushes.extend(brushes.iter().map(map_brush));
        }
        entities.push(e);
    }

    let mut group_id = 3_000_000u64;
    while made < spec.brushes {
        let at = next_at(&mut rng);
        let m = next_material();
        let roll = rng.below(1000);
        if roll < 5 {
            // A group of twenty structures, on a layer.
            group_id += 1;
            let mut g = entity(&[
                ("classname", "func_group".into()),
                ("_tb_type", "_tb_group".into()),
                ("_tb_name", format!("Group {group_id}")),
                ("_tb_id", group_id.to_string()),
                (
                    "_tb_layer",
                    layer_ids[rng.below(layer_ids.len())].to_string(),
                ),
            ]);
            for _ in 0..20 {
                let at = next_at(&mut rng);
                let brushes = structure(&mut rng, at, &m);
                made += brushes.len();
                g.brushes.extend(brushes.iter().map(map_brush));
            }
            entities.push(g);
        } else if roll < 15 {
            // A brush entity of two structures.
            let class = if roll.is_multiple_of(2) {
                "func_wall"
            } else {
                "func_door"
            };
            let mut e = entity(&[("classname", class.into())]);
            for _ in 0..2 {
                let brushes = structure(&mut rng, at, &m);
                made += brushes.len();
                e.brushes.extend(brushes.iter().map(map_brush));
            }
            entities.push(e);
        } else {
            let brushes = structure(&mut rng, at, &m);
            made += brushes.len();
            let layer = rng.below(4);
            if layer == 0 {
                worldspawn.brushes.extend(brushes.iter().map(map_brush));
            } else {
                // A layer's loose brushes are written inside the layer's own entity.
                entities[layer - 1]
                    .brushes
                    .extend(brushes.iter().map(map_brush));
            }
        }
        if made % 50 < 2 && rng.below(2) == 0 {
            let at = at + Vec3d::new(16.0, 16.0, 200.0);
            entities.push(entity(&[
                ("classname", "light".into()),
                ("origin", format!("{} {} {}", at.x(), at.y(), at.z())),
                ("light", "300".into()),
            ]));
        }
    }

    let mut all = vec![worldspawn];
    all.extend(entities);
    let map = MapFile {
        entities: all,
        ..Default::default()
    };
    map_model::io::map_writer::write_map(&map)
}

/// Lays structures out on a grid across the world, one cell after another.
struct Placer {
    slot: usize,
    across: usize,
}

impl Placer {
    fn new() -> Self {
        Self {
            slot: 0,
            across: ((WORLD * 2.0 - 512.0) / 160.0) as usize,
        }
    }
    fn next(&mut self) -> Vec3d {
        let (x, y, z) = (
            self.slot % self.across,
            (self.slot / self.across) % self.across,
            self.slot / (self.across * self.across),
        );
        self.slot += 1;
        Vec3d::new(
            -WORLD + 256.0 + x as f64 * 160.0,
            -WORLD + 256.0 + y as f64 * 160.0,
            -2048.0 + z as f64 * 384.0,
        )
    }
}

fn boxed(at: Vec3d, size: f64, material: &str) -> MapBrush {
    let brush = create_cuboid(
        &world(),
        &Bbox3d::new(at, at + Vec3d::new(size, size, size)),
        material,
    )
    .expect("a box");
    map_brush(&brush)
}

fn group_entity(id: u64, parent: Option<u64>) -> Entity {
    let mut properties = vec![
        ("classname", "func_group".to_string()),
        ("_tb_type", "_tb_group".into()),
        ("_tb_name", format!("Group {id}")),
        ("_tb_id", id.to_string()),
    ];
    if let Some(parent) = parent {
        properties.push(("_tb_group", parent.to_string()));
    }
    entity(&properties)
}

/// The map text for these entities, the first being the worldspawn.
fn write(entities: Vec<Entity>) -> String {
    map_model::io::map_writer::write_map(&MapFile {
        entities,
        ..Default::default()
    })
}

/// What a map for the group benchmarks holds.
#[derive(Clone, Debug)]
pub struct GroupSpec {
    /// Groups of `per_group` boxes each, at the top level.
    pub small_groups: usize,
    /// How many boxes each small group holds.
    pub per_group: usize,
    /// Chains of nested groups, each `depth` levels deep with two boxes per level.
    pub chains: usize,
    /// How deep each chain goes.
    pub depth: usize,
    /// The boxes in one big group.
    pub big_group: usize,
    /// Boxes in no group.
    pub loose: usize,
}

impl Default for GroupSpec {
    fn default() -> Self {
        Self {
            small_groups: 5_000,
            per_group: 4,
            chains: 100,
            depth: 10,
            big_group: 5_000,
            loose: 2_000,
        }
    }
}

/// The `_tb_id` of the group map's big group.
pub const BIG_GROUP_ID: u64 = 1;
/// The `_tb_id` of the outermost group of the group map's first chain.
pub const FIRST_CHAIN_ID: u64 = 100;
/// The `_tb_id` of the group map's first small group.
pub const FIRST_SMALL_GROUP_ID: u64 = 1_000_000;

/// A map of many groups: `small_groups` top-level groups, `chains` chains of groups
/// nested `depth` deep, one group of `big_group` boxes and `loose` boxes in no group.
pub fn grouped_map(spec: &GroupSpec) -> String {
    let mut placer = Placer::new();
    let mut worldspawn = entity(&[("classname", "worldspawn".into())]);
    for i in 0..spec.loose {
        worldspawn
            .brushes
            .push(boxed(placer.next(), 64.0, &material_name(i % 64)));
    }
    let mut entities = vec![];
    let mut big = group_entity(BIG_GROUP_ID, None);
    for i in 0..spec.big_group {
        big.brushes
            .push(boxed(placer.next(), 64.0, &material_name(i % 64)));
    }
    entities.push(big);
    let mut next_chain_id = FIRST_CHAIN_ID;
    for _ in 0..spec.chains {
        let mut parent = None;
        for level in 0..spec.depth {
            let id = next_chain_id;
            next_chain_id += 1;
            let mut g = group_entity(id, parent);
            for _ in 0..2 {
                g.brushes.push(boxed(
                    placer.next(),
                    48.0 - level as f64,
                    &material_name(level),
                ));
            }
            entities.push(g);
            parent = Some(id);
        }
    }
    for i in 0..spec.small_groups {
        let mut g = group_entity(FIRST_SMALL_GROUP_ID + i as u64, None);
        for _ in 0..spec.per_group {
            g.brushes
                .push(boxed(placer.next(), 64.0, &material_name(i % 64)));
        }
        entities.push(g);
    }
    let mut all = vec![worldspawn];
    all.extend(entities);
    write(all)
}

/// A map of `copies` linked copies of one group of `per_copy` boxes, side by side, plus
/// `loose` boxes in no group.
pub fn linked_map(copies: usize, per_copy: usize, loose: usize) -> String {
    let mut placer = Placer::new();
    let mut worldspawn = entity(&[("classname", "worldspawn".into())]);
    for i in 0..loose {
        worldspawn
            .brushes
            .push(boxed(placer.next(), 64.0, &material_name(i % 64)));
    }
    // The source's boxes, in a block above the loose ones; each copy is shifted.
    let source: Vec<Vec3d> = (0..per_copy)
        .map(|i| Vec3d::new((i % 10) as f64 * 80.0, (i / 10) as f64 * 80.0, 4096.0))
        .collect();
    let mut all = vec![worldspawn];
    let across = 30usize;
    for c in 0..copies {
        let delta = Vec3d::new(
            (c % across) as f64 * 1024.0 - 15.0 * 1024.0,
            (c / across) as f64 * 1024.0 - 8.0 * 1024.0,
            0.0,
        );
        let mut g = group_entity(10_000 + c as u64, None);
        g.properties
            .push(("_tb_linked_group_id".into(), "bench_link".into()));
        g.properties.push((
            "_tb_transformation".into(),
            format!(
                "1 0 0 {} 0 1 0 {} 0 0 1 {} 0 0 0 1",
                delta.x(),
                delta.y(),
                delta.z()
            ),
        ));
        for (i, at) in source.iter().enumerate() {
            g.brushes
                .push(boxed(*at + delta, 64.0, &material_name(i % 64)));
        }
        all.push(g);
    }
    write(all)
}

/// A map of `entities` point entities (lights and others) and `brushes` boxes.
pub fn point_entity_map(entities: usize, brushes: usize) -> String {
    let mut placer = Placer::new();
    let mut worldspawn = entity(&[("classname", "worldspawn".into())]);
    for i in 0..brushes {
        worldspawn
            .brushes
            .push(boxed(placer.next(), 64.0, &material_name(i % 64)));
    }
    let mut all = vec![worldspawn];
    let classes = ["light", "info_player_start", "info_null", "light_spot"];
    for i in 0..entities {
        let at = placer.next() + Vec3d::new(32.0, 32.0, 32.0);
        all.push(entity(&[
            ("classname", classes[i % 4].into()),
            ("origin", format!("{} {} {}", at.x(), at.y(), at.z())),
            ("targetname", format!("e{i}")),
        ]));
    }
    write(all)
}

/// A map of `brushes` boxes spread over `layers` layers (the default one included).
pub fn layered_map(layers: usize, brushes: usize) -> String {
    let mut placer = Placer::new();
    let mut all = vec![entity(&[("classname", "worldspawn".into())])];
    for l in 1..layers.max(1) {
        all.push(entity(&[
            ("classname", "func_group".into()),
            ("_tb_type", "_tb_layer".into()),
            ("_tb_name", format!("Layer {l}")),
            ("_tb_id", (2_000_000 + l).to_string()),
            ("_tb_layer_sort_index", l.to_string()),
        ]));
    }
    for i in 0..brushes {
        let layer = i % layers.max(1);
        all[layer]
            .brushes
            .push(boxed(placer.next(), 64.0, &material_name(i % 64)));
    }
    write(all)
}

/// Writes `count` PNG textures named after [`material_name`] into `dir`, sized 64 to 512
/// pixels, plus `big` more of 2048 pixels. Returns the bytes written.
pub fn write_textures(dir: &Path, count: usize, big: usize, seed: u64) -> std::io::Result<u64> {
    let folder = dir.join("bench");
    std::fs::create_dir_all(&folder)?;
    let mut rng = Rng::new(seed);
    let mut total = 0u64;
    for index in 0..count + big {
        let size: u32 = if index >= count {
            2048
        } else {
            [64, 128, 128, 256, 256, 512][rng.below(6)]
        };
        let name = if index >= count {
            format!("big{:02}", index - count)
        } else {
            format!("m{index:05}")
        };
        let path = folder.join(format!("{name}.png"));
        let base = [
            rng.below(256) as u8,
            rng.below(256) as u8,
            rng.below(256) as u8,
        ];
        let mut pixels = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                let check = (((x / 16) + (y / 16)) % 2) as u8 * 40;
                let noise = ((x.wrapping_mul(31) ^ y.wrapping_mul(17)) & 15) as u8;
                pixels.extend([
                    base[0].wrapping_add(check + noise),
                    base[1].wrapping_add(check),
                    base[2].wrapping_add(noise),
                    255,
                ]);
            }
        }
        let bytes =
            crate::assets::encode_png(size, size, &pixels).map_err(std::io::Error::other)?;
        total += bytes.len() as u64;
        std::fs::write(path, bytes)?;
    }
    Ok(total)
}

/// A directory under the system's temporary directory, emptied first and removed when
/// dropped (unless `HERON_BENCH_KEEP` is set).
pub struct Scratch(pub PathBuf);

impl Scratch {
    /// A fresh `heron-bench-<name>` directory.
    pub fn new(name: &str) -> Self {
        let root = std::env::var_os("HERON_BENCH_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let path = root.join(format!("heron-bench-{name}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the scratch directory can be made");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if std::env::var_os("HERON_BENCH_KEEP").is_none() {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

/// The timings of one benchmark.
pub struct Stats {
    /// What was measured.
    pub name: String,
    /// Each run, sorted.
    pub runs: Vec<Duration>,
}

impl Stats {
    /// The median run.
    pub fn median(&self) -> Duration {
        self.runs[self.runs.len() / 2]
    }
    /// The 95th percentile run (the slowest, for fewer than twenty runs).
    pub fn p95(&self) -> Duration {
        let index = ((self.runs.len() as f64 * 0.95).ceil() as usize).clamp(1, self.runs.len()) - 1;
        self.runs[index]
    }
}

/// Runs `body` `runs` times after `setup` (untimed) and returns the sorted timings.
pub fn measure<S, T>(
    name: &str,
    runs: usize,
    mut setup: impl FnMut() -> S,
    mut body: impl FnMut(S) -> T,
) -> Stats {
    // `HERON_BENCH_ONLY=<text>` runs only the benchmarks whose name contains it: what a
    // profiler is attached to.
    if std::env::var("HERON_BENCH_ONLY").is_ok_and(|only| !name.contains(&only)) {
        return Stats {
            name: name.to_string(),
            runs: Vec::new(),
        };
    }
    let mut times = Vec::with_capacity(runs);
    for _ in 0..runs.max(1) {
        let input = setup();
        let start = Instant::now();
        let output = body(input);
        times.push(start.elapsed());
        drop(std::hint::black_box(output));
    }
    times.sort();
    let stats = Stats {
        name: name.to_string(),
        runs: times,
    };
    eprintln!(
        "  {:<58} median {:>10.3} ms   p95 {:>10.3} ms   (n={})",
        stats.name,
        ms(stats.median()),
        ms(stats.p95()),
        stats.runs.len()
    );
    stats
}

/// [`measure`] where the untimed setup and the timed body both work on `target`.
pub fn measure_on<S, T>(
    name: &str,
    runs: usize,
    target: &mut S,
    mut setup: impl FnMut(&mut S),
    mut body: impl FnMut(&mut S) -> T,
) -> Stats {
    let cell = std::cell::RefCell::new(target);
    measure(
        name,
        runs,
        || setup(&mut cell.borrow_mut()),
        |()| body(&mut cell.borrow_mut()),
    )
}

/// Milliseconds, as a float.
pub fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Prints the results as a Markdown table.
pub fn print_table(title: &str, rows: &[Stats]) {
    println!("\n### {title}\n");
    println!("| benchmark | median ms | p95 ms | n |");
    println!("|---|---:|---:|---:|");
    for row in rows.iter().filter(|row| !row.runs.is_empty()) {
        println!(
            "| {} | {:.3} | {:.3} | {} |",
            row.name,
            ms(row.median()),
            ms(row.p95()),
            row.runs.len()
        );
    }
}

/// The peak resident set size of this process so far, in bytes.
pub fn peak_rss() -> u64 {
    #[cfg(unix)]
    {
        #[repr(C)]
        struct Timeval {
            sec: i64,
            usec: i64,
        }
        #[repr(C)]
        struct Rusage {
            utime: Timeval,
            stime: Timeval,
            maxrss: i64,
            rest: [i64; 13],
        }
        extern "C" {
            fn getrusage(who: i32, usage: *mut Rusage) -> i32;
        }
        let mut usage = std::mem::MaybeUninit::<Rusage>::zeroed();
        // SAFETY: getrusage fills the struct, whose layout matches the C one on 64-bit Unix.
        let ok = unsafe { getrusage(0, usage.as_mut_ptr()) } == 0;
        if !ok {
            return 0;
        }
        // SAFETY: `usage` was zeroed and every field is an integer, so it holds a valid `Rusage`
        // whatever `getrusage` wrote over it.
        let maxrss = unsafe { usage.assume_init() }.maxrss.max(0) as u64;
        // macOS reports bytes, Linux kilobytes.
        if cfg!(target_os = "macos") {
            maxrss
        } else {
            maxrss * 1024
        }
    }
    #[cfg(not(unix))]
    {
        0
    }
}

/// The current resident set size, in bytes, as `ps` reports it (0 where it cannot).
pub fn current_rss() -> u64 {
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map_or(0, |kb| kb * 1024)
}

thread_local! {
    static LEGACY_LINE_PATHS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Scoped benchmark control; absent from builds without `test-support`.
/// Restores the normal path even when the benchmark panics.
pub fn with_legacy_line_paths<R>(legacy: bool, body: impl FnOnce() -> R) -> R {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            LEGACY_LINE_PATHS.with(|flag| flag.set(self.0));
        }
    }
    let _restore = Restore(LEGACY_LINE_PATHS.with(|flag| flag.replace(legacy)));
    body()
}

pub(crate) fn legacy_line_paths() -> bool {
    LEGACY_LINE_PATHS.with(std::cell::Cell::get)
}
