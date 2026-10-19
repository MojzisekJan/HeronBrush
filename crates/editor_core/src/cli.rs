//! The command line every frontend shares: the headless batch commands that run before
//! any window exists, and the positional `map [fgd] [materials]` arguments.
//!
//! The batch commands return an exit code instead of exiting, so a frontend decides when to leave and a
//! test can run them.

use std::path::PathBuf;

use crate::document::Document;
use crate::{plugin, plugin_manifest, world};

/// What the command line asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Print the version and leave (`--version`, `-V`).
    Version,
    /// Print the usage and leave (`--help`, `-h`).
    Help,
    /// Serve the MCP protocol over stdio (`--mcp`).
    Mcp,
    /// A three-way `.map` merge for git (`--merge-map`).
    MergeMap(Vec<String>),
    /// A mesh turned into brushes (`--obj-to-map`).
    ObjToMap(Vec<String>),
    /// A PNG thumbnail of a map (`--render-preview`).
    RenderPreview(Vec<String>),
    /// A plugin script run over a map (`--run-script`).
    RunScript(Vec<String>),
    /// Open the editor on these files.
    Open(Positional),
}

/// The files the editor opens with, in the order they are most often wanted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Positional {
    /// The map; none shows the welcome screen.
    pub map: Option<PathBuf>,
    /// The entity definitions.
    pub fgd: Option<PathBuf>,
    /// The materials.
    pub materials: Option<PathBuf>,
}

/// Reads the arguments after the program name.
///
/// An empty argument is a placeholder for one that was skipped, not a path to nothing:
/// `heron map.map "" materials/` asks for materials and no entity definitions.
pub fn parse(args: &[String]) -> Command {
    let rest = || args.get(1..).unwrap_or_default().to_vec();
    match args.first().map(String::as_str) {
        Some("--version" | "-V") => Command::Version,
        Some("--help" | "-h") => Command::Help,
        Some("--mcp") => Command::Mcp,
        Some("--merge-map") => Command::MergeMap(rest()),
        Some("--obj-to-map") => Command::ObjToMap(rest()),
        Some("--render-preview") => Command::RenderPreview(rest()),
        Some("--run-script") => Command::RunScript(rest()),
        _ => {
            let path = |at: usize| args.get(at).filter(|a| !a.is_empty()).map(PathBuf::from);
            Command::Open(Positional { map: path(0), fgd: path(1), materials: path(2) })
        }
    }
}

/// The usage text, naming the program as it was installed.
pub fn usage(program: &str) -> String {
    format!(
        "Usage: {program} [map.map] [entities.fgd] [materials/]\n\
         \x20      an empty argument skips one: {program} map.map \"\" materials/\n\
         \x20      {program} --version | -V\n\
         \x20      {program} --mcp\n\
         \x20      {program} --merge-map <base> <ours> <theirs> [<output>]\n\
         \x20      {program} --obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]\n\
         \x20      {program} --render-preview <in.map> <out.png> [size]\n\
         \x20      {program} --run-script <script.rhai> <in.map> [out.map] [key=value ...]"
    )
}

/// Runs a headless command. Returns the exit code, or `None` when the command opens the
/// editor (or serves MCP, which the frontend starts itself because it never returns).
pub fn run(command: &Command, program: &str, version: &str) -> Option<i32> {
    Some(match command {
        Command::Version => {
            println!("{version}");
            0
        }
        Command::Help => {
            println!("{version}\n\n{}", usage(program));
            0
        }
        Command::MergeMap(args) => merge_map(args, program),
        Command::ObjToMap(args) => obj_to_map(args, program),
        Command::RenderPreview(args) => render_preview(args, program),
        Command::RunScript(args) => run_script(args, program),
        Command::Mcp | Command::Open(_) => return None,
    })
}

/// Writes a PNG thumbnail of a map without opening an editor window: the same picture
/// the asset library shows, drawn in software.
pub fn render_preview(args: &[String], program: &str) -> i32 {
    let (input, output, size) = match args {
        [input, output] => (input, output, 512),
        [input, output, size] => match size.parse() {
            Ok(size) => (input, output, size),
            Err(_) => {
                eprintln!("heron-brush preview: {size} is not a size");
                return 2;
            }
        },
        _ => {
            eprintln!("usage: {program} --render-preview <in.map> <out.png> [size]");
            return 2;
        }
    };
    if let Err(error) = std::fs::read_to_string(input) {
        eprintln!("heron-brush preview: cannot read {input}: {error}");
        return 2;
    }
    let Some(surface) = crate::assets::preview_file(std::path::Path::new(input), size) else {
        eprintln!("heron-brush preview: {input} has no brushes to draw");
        return 1;
    };
    match surface.save_png(std::path::Path::new(output)) {
        Ok(()) => {
            println!("wrote {output} ({size}x{size})");
            0
        }
        Err(error) => {
            eprintln!("heron-brush preview: cannot write {output}: {error}");
            2
        }
    }
}

/// Runs a plugin script over a `.map`, headless, with everything selected, and writes
/// the result to `<out.map>` (default `<in.map>`). `key=value` arguments answer the
/// settings the plugin's manifest declares.
pub fn run_script(args: &[String], program: &str) -> i32 {
    let (paths, overrides): (Vec<&String>, Vec<&String>) = args.iter().partition(|arg| !arg.contains('='));
    let (script_path, input, output) = match paths.as_slice() {
        [script, input] => (*script, *input, (*input).clone()),
        [script, input, output] => (*script, *input, (*output).clone()),
        _ => {
            eprintln!("usage: {program} --run-script <script.rhai> <in.map> [out.map] [key=value ...]");
            return 2;
        }
    };
    let script = match std::fs::read_to_string(script_path) {
        Ok(script) => script,
        Err(error) => {
            eprintln!("heron-brush script: cannot read {script_path}: {error}");
            return 2;
        }
    };
    let manifest = plugin_manifest::Manifest::beside(std::path::Path::new(script_path)).unwrap_or_default();
    if let Err(error) = crate::plugins::check_path_enabled(std::path::Path::new(script_path)) {
        eprintln!("heron-brush script: {error}");
        return 2;
    }
    if !manifest.api_matches() {
        eprintln!(
            "heron-brush script: {script_path} asks for plugin API {}; this editor offers {}",
            manifest.api.unwrap_or_default(),
            plugin_manifest::API_VERSION,
        );
        return 2;
    }
    let mut document = match Document::open(input, world::WorldBounds::default()) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("heron-brush script: cannot open {input}: {error}");
            return 2;
        }
    };
    document.select_all();
    let mut settings = manifest.defaults();
    for over in overrides {
        let Some((key, value)) = over.split_once('=') else { continue };
        let Some(slot) = settings.iter_mut().find(|(declared, _)| declared == key) else {
            eprintln!("heron-brush script: this plugin has no setting called {key}");
            return 2;
        };
        slot.1 = match &slot.1 {
            plugin_manifest::SettingValue::Number(_) => match value.parse() {
                Ok(number) => plugin_manifest::SettingValue::Number(number),
                Err(_) => {
                    eprintln!("heron-brush script: {key} wants a number, not {value}");
                    return 2;
                }
            },
            plugin_manifest::SettingValue::Check(_) => {
                plugin_manifest::SettingValue::Check(matches!(value, "true" | "yes" | "1" | "on"))
            }
            plugin_manifest::SettingValue::Text(_) => plugin_manifest::SettingValue::Text(value.to_string()),
        };
    }
    match plugin::run_with(&mut document, &script, &settings) {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            if let Err(error) = std::fs::write(&output, document.to_map_text()) {
                eprintln!("heron-brush script: cannot write {output}: {error}");
                return 2;
            }
            0
        }
        Err(error) => {
            eprintln!("heron-brush script: {error}");
            1
        }
    }
}

/// A `git` merge driver for `.map` files: `--merge-map <base> <ours> <theirs> [<output>]`.
/// The output defaults to `<ours>`, git's `%A`. Conflicts kept as *ours* are printed and
/// make the exit code non-zero.
pub fn merge_map(args: &[String], program: &str) -> i32 {
    let (base, ours, theirs, output) = match args {
        [base, ours, theirs] => (base, ours, theirs, ours.clone()),
        [base, ours, theirs, output] => (base, ours, theirs, output.clone()),
        _ => {
            eprintln!("usage: {program} --merge-map <base> <ours> <theirs> [<output>]");
            eprintln!("       the output path defaults to <ours>, which is git's %A");
            return 2;
        }
    };
    let read = |path: &str| {
        std::fs::read_to_string(path).map_err(|error| eprintln!("heron-brush merge: cannot read {path}: {error}"))
    };
    let (Ok(base), Ok(ours), Ok(theirs)) = (read(base), read(ours), read(theirs)) else { return 2 };
    match map_model::merge::merge_text(&base, &ours, &theirs) {
        Ok((merged, conflicts)) => {
            if let Err(error) = std::fs::write(&output, merged) {
                eprintln!("heron-brush merge: cannot write {output}: {error}");
                return 2;
            }
            for conflict in &conflicts {
                eprintln!("heron-brush merge: {conflict}");
            }
            i32::from(!conflicts.is_empty())
        }
        Err(error) => {
            eprintln!("heron-brush merge: {error}");
            2
        }
    }
}

/// `--obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]`: every triangle
/// of a mesh becomes a thin wedge brush (or `solid` grid boxes, or `layered` slabs).
pub fn obj_to_map(args: &[String], program: &str) -> i32 {
    let [source, output, rest @ ..] = args else {
        eprintln!("usage: {program} --obj-to-map <in.obj> <out.map> [size] [thickness] [snap] [material]");
        eprintln!("       size is what the model's longest side becomes, in map units;");
        eprintln!("       snap is the grid the corners are rounded onto, 0 for none;");
        eprintln!("       `layered` builds it as slabs of convex pieces, `solid` as grid boxes");
        return 2;
    };
    let text = match std::fs::read_to_string(source) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("{source}: {error}");
            return 2;
        }
    };
    let number = |at: usize| rest.get(at).and_then(|word| word.parse::<f64>().ok());
    let options = map_model::obj_to_brushes::Options {
        size: number(0).or(Some(256.0)),
        thickness: number(1).unwrap_or(4.0),
        snap: number(2).filter(|grid| *grid > 0.0),
        solid: rest.iter().any(|word| word == "solid"),
        layered: rest.iter().any(|word| word == "layered"),
        y_up: !rest.iter().any(|word| word == "z-up"),
        material: rest
            .get(3)
            .filter(|word| !matches!(word.as_str(), "solid" | "layered" | "z-up"))
            .cloned()
            .unwrap_or_else(|| "stone".to_string()),
        ..map_model::obj_to_brushes::Options::default()
    };
    let bounds = world::WorldBounds::default();
    let world = geometry_math::bbox::Bbox3d::from_scalars(-bounds.half_extent(), bounds.half_extent());
    let (brushes, report) = map_model::obj_to_brushes::brushes_from_obj(&text, &world, &options);
    if brushes.is_empty() {
        eprintln!("{source}: no brushes came out of it");
        return 1;
    }
    let mut document = Document::new(bounds);
    document.add_brushes(brushes);
    match document.export_map_to(output, map_model::map_format::MapFormat::LOSSLESS) {
        Ok(()) => {
            let part = |n: usize, what: &str| if n > 0 { format!(", {n} {what}") } else { String::new() };
            println!(
                "wrote {output}: {} brushes from {} triangles{}{}{}",
                report.brushes,
                report.triangles,
                part(report.joined, "joined into a neighbour"),
                part(report.collapsed, "collapsed onto the grid"),
                part(report.dropped, "dropped"),
            );
            0
        }
        Err(error) => {
            eprintln!("{output}: {error}");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn flags_pick_their_command() {
        assert_eq!(parse(&args(&["-V"])), Command::Version);
        assert_eq!(parse(&args(&["--version"])), Command::Version);
        assert_eq!(parse(&args(&["-h"])), Command::Help);
        assert_eq!(parse(&args(&["--mcp"])), Command::Mcp);
        assert_eq!(parse(&args(&["--merge-map", "a", "b", "c"])), Command::MergeMap(args(&["a", "b", "c"])));
        assert_eq!(parse(&args(&["--obj-to-map", "x.obj", "y.map"])), Command::ObjToMap(args(&["x.obj", "y.map"])));
        assert_eq!(parse(&args(&["--render-preview", "m.map", "o.png", "64"])), Command::RenderPreview(args(&["m.map", "o.png", "64"])));
        assert_eq!(parse(&args(&["--run-script", "s.rhai", "m.map"])), Command::RunScript(args(&["s.rhai", "m.map"])));
    }

    #[test]
    fn positional_arguments_skip_empty_ones() {
        assert_eq!(parse(&[]), Command::Open(Positional::default()));
        assert_eq!(parse(&args(&[""])), Command::Open(Positional::default()));
        let Command::Open(open) = parse(&args(&["m.map", "", "textures"])) else { panic!() };
        assert_eq!(open.map, Some(PathBuf::from("m.map")));
        assert_eq!(open.fgd, None);
        assert_eq!(open.materials, Some(PathBuf::from("textures")));
        let Command::Open(open) = parse(&args(&["m.map", "q.fgd"])) else { panic!() };
        assert_eq!(open.fgd, Some(PathBuf::from("q.fgd")));
        assert_eq!(open.materials, None);
    }

    #[test]
    fn a_bad_batch_command_says_how_to_use_it() {
        assert_eq!(run(&Command::MergeMap(vec![]), "heron", "v"), Some(2));
        assert_eq!(run(&Command::ObjToMap(vec![]), "heron", "v"), Some(2));
        assert_eq!(run(&Command::RenderPreview(vec![]), "heron", "v"), Some(2));
        assert_eq!(run(&Command::RunScript(vec![]), "heron", "v"), Some(2));
        assert_eq!(run(&Command::Open(Positional::default()), "heron", "v"), None);
    }

    #[test]
    fn render_preview_writes_a_png() {
        let map = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/crate.map");
        let out = std::env::temp_dir().join(format!("heron-cli-preview-{}.png", std::process::id()));
        let code = render_preview(&[map.display().to_string(), out.display().to_string(), "32".into()], "heron");
        assert_eq!(code, 0);
        assert!(std::fs::metadata(&out).map(|m| m.len() > 0).unwrap_or(false));
        let _ = std::fs::remove_file(out);
    }
}
