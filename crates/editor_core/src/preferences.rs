//! The settings the editor keeps between runs.
//!
//! Stores typed preferences under stable path-like keys in a configuration file.
//!
//! Defaults include application, input, colour and renderer settings.
//!
//! The store is a text file of `key=value` lines rather than a platform settings
//! database, so it can be read, edited and diffed like anything else in a project.

use crate::i18n::translation_key;
use std::cell::RefCell;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Fresh installations discover the asset folder shipped beside the executable.
/// Explicit user preferences still override this default, including an empty list.
///
/// A build run from the repository (`cargo run`) has nothing beside its executable, so
/// it falls back to the repository's own `assets`.
fn bundled_asset_directory() -> String {
    install_directory()
        .map(|dir| dir.join("assets"))
        .filter(|assets| crate::files::is_dir(assets))
        .or_else(|| {
            Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"))
                .filter(|assets| assets.is_dir())
        })
        // The web build's folders are on no disk, so there is nothing to canonicalize.
        .and_then(|assets| {
            assets
                .canonicalize()
                .ok()
                .or(cfg!(target_arch = "wasm32").then_some(assets))
        })
        .map(|assets| assets.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Where a platform without a usable home directory or executable folder keeps things,
/// set once at startup before anything reads the preferences.
#[derive(Default)]
struct PlatformDirectories {
    /// Stands in for the user's configuration directory (`~/.config`).
    config: Option<PathBuf>,
    /// Stands in for the executable's folder: where `assets` and `games` are shipped.
    install: Option<PathBuf>,
}

static PLATFORM_DIRECTORIES: std::sync::OnceLock<PlatformDirectories> = std::sync::OnceLock::new();

/// Overrides where the profile and the shipped data are, for a platform whose apps get
/// their storage from the system rather than from `$HOME` and the executable's folder
/// (Android hands the activity a files directory). `config` replaces
/// [`user_config_dir`]; `install` replaces the executable's folder, where `assets` and
/// `games` are looked for. Call it before anything else here; later calls are ignored.
pub fn set_platform_directories(config: PathBuf, install: PathBuf) {
    let _ = PLATFORM_DIRECTORIES.set(PlatformDirectories {
        config: Some(config),
        install: Some(install),
    });
}

/// The folder a package ships its data in (`assets`, `games`): the one
/// [`set_platform_directories`] named, else the executable's.
pub fn install_directory() -> Option<PathBuf> {
    if let Some(dir) = PLATFORM_DIRECTORIES
        .get()
        .and_then(|dirs| dirs.install.clone())
    {
        return Some(dir);
    }
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
}

/// A colour, as four channels from zero to one.
///
/// Stores colours as four floating-point channels, accepting either byte-range or normalised input.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// The red channel.
    pub r: f32,
    /// The green channel.
    pub g: f32,
    /// The blue channel.
    pub b: f32,
    /// The alpha channel, where zero is invisible.
    pub a: f32,
}

impl Color {
    /// An opaque colour from three channels.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// A colour from four channels.
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// An opaque colour from three byte channels.
    pub const fn bytes(r: u8, g: u8, b: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
    }

    /// The three colour channels, for a shader uniform.
    pub fn rgb_f32(self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }

    /// All four channels, for a shader uniform.
    pub fn rgba_f32(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// The four channels as doubles, which is what cairo takes.
    pub fn rgba_f64(self) -> (f64, f64, f64, f64) {
        (self.r as f64, self.g as f64, self.b as f64, self.a as f64)
    }

    /// The same colour at a different opacity.
    pub fn with_alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }

    /// Whether the colour is dark enough to want light ink over it.
    ///
    /// The weights are the ones a display uses to turn a colour into a brightness, so a
    /// saturated green counts as light and a saturated blue does not.
    pub fn is_dark(self) -> bool {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b < 0.5
    }
}

/// A value that can go into the store and come back out.
trait Stored: Sized {
    /// Writes the value in the form [`Stored::parse`] reads.
    fn write(&self, out: &mut String);
    /// Reads a value, returning nothing if the text does not hold one.
    ///
    /// A value that does not parse leaves the preference at its default rather than
    /// failing the load: one bad line should not throw away the whole file.
    fn parse(text: &str) -> Option<Self>;
}

impl Stored for bool {
    fn write(&self, out: &mut String) {
        out.push_str(if *self { "true" } else { "false" });
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        }
    }
}

impl Stored for f32 {
    fn write(&self, out: &mut String) {
        let _ = write!(out, "{self}");
    }

    fn parse(text: &str) -> Option<Self> {
        text.parse().ok().filter(|value: &f32| value.is_finite())
    }
}

impl Stored for u32 {
    fn write(&self, out: &mut String) {
        let _ = write!(out, "{self}");
    }

    fn parse(text: &str) -> Option<Self> {
        text.parse().ok()
    }
}

/// Which colour scheme the window is drawn in.
///
/// `System` follows the light or dark setting the desktop reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorScheme {
    /// Whatever the desktop asks for.
    #[default]
    System,
    /// Force the light variant.
    Light,
    /// Force the dark variant.
    Dark,
}

impl Stored for ColorScheme {
    fn write(&self, out: &mut String) {
        out.push_str(match self {
            ColorScheme::System => "system",
            ColorScheme::Light => "light",
            ColorScheme::Dark => "dark",
        });
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "system" => Some(ColorScheme::System),
            "light" => Some(ColorScheme::Light),
            "dark" => Some(ColorScheme::Dark),
            _ => None,
        }
    }
}

/// The curve that maps the 3D view's light to the screen. Only the wgpu renderer
/// (`render_wgpu`) offers a choice; the GL renderer always uses ACES and ignores this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToneMapper {
    /// The fitted ACES curve the GL renderer uses.
    Aces,
    /// Troy Sobotka's AgX, Blender's default: highlights roll off without hue shifts.
    #[default]
    Agx,
    /// Khronos PBR Neutral: base colours shown as authored, highlights compressed.
    KhronosNeutral,
    /// Tomasz Stachowiak's TonyMcMapface.
    TonyMcMapface,
}

impl ToneMapper {
    /// Every choice, in the order a chooser lists them.
    pub const ALL: [ToneMapper; 4] = [
        ToneMapper::Agx,
        ToneMapper::KhronosNeutral,
        ToneMapper::TonyMcMapface,
        ToneMapper::Aces,
    ];

    /// The name a chooser shows: proper names, the same in every language.
    pub fn name(self) -> &'static str {
        match self {
            ToneMapper::Aces => "ACES",
            ToneMapper::Agx => "AgX",
            ToneMapper::KhronosNeutral => "Khronos PBR Neutral",
            ToneMapper::TonyMcMapface => "TonyMcMapface",
        }
    }
}

impl Stored for ToneMapper {
    fn write(&self, out: &mut String) {
        out.push_str(match self {
            ToneMapper::Aces => "aces",
            ToneMapper::Agx => "agx",
            ToneMapper::KhronosNeutral => "khronos-neutral",
            ToneMapper::TonyMcMapface => "tony-mc-mapface",
        });
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "aces" => Some(ToneMapper::Aces),
            "agx" => Some(ToneMapper::Agx),
            "khronos-neutral" => Some(ToneMapper::KhronosNeutral),
            "tony-mc-mapface" => Some(ToneMapper::TonyMcMapface),
            _ => None,
        }
    }
}

impl Stored for String {
    fn write(&self, out: &mut String) {
        // A store line is `key=value` to the end of the line, so anything but a newline
        // survives the trip; one is turned into a space rather than breaking the file.
        out.push_str(&self.replace('\n', " "));
    }

    fn parse(text: &str) -> Option<Self> {
        Some(text.to_string())
    }
}

impl Stored for crate::export::Target {
    fn write(&self, out: &mut String) {
        out.push_str(match self {
            crate::export::Target::Godot => "godot",
            crate::export::Target::Defold => "defold",
            crate::export::Target::ThreeJs => "three",
            crate::export::Target::Babylon => "babylon",
            crate::export::Target::Bevy => "bevy",
            crate::export::Target::Raylib => "raylib",
            crate::export::Target::Unity => "unity",
            crate::export::Target::Unreal => "unreal",
            crate::export::Target::Flax => "flax",
            crate::export::Target::Stride => "stride",
            crate::export::Target::Cocos => "cocos",
            crate::export::Target::GDevelop => "gdevelop",
            crate::export::Target::Obj => "obj",
            crate::export::Target::Gltf => "gltf",
            crate::export::Target::Glb => "glb",
        });
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "godot" => Some(crate::export::Target::Godot),
            "defold" => Some(crate::export::Target::Defold),
            "three" => Some(crate::export::Target::ThreeJs),
            "babylon" => Some(crate::export::Target::Babylon),
            "bevy" => Some(crate::export::Target::Bevy),
            "raylib" => Some(crate::export::Target::Raylib),
            "unity" => Some(crate::export::Target::Unity),
            "unreal" => Some(crate::export::Target::Unreal),
            "flax" => Some(crate::export::Target::Flax),
            "stride" => Some(crate::export::Target::Stride),
            "cocos" => Some(crate::export::Target::Cocos),
            "gdevelop" => Some(crate::export::Target::GDevelop),
            "obj" => Some(crate::export::Target::Obj),
            "gltf" => Some(crate::export::Target::Gltf),
            "glb" => Some(crate::export::Target::Glb),
            _ => None,
        }
    }
}

impl Stored for crate::i18n::Language {
    /// The word each language has in [`crate::i18n::LANGUAGES`] (`czech`, `portuguese_brazil`).
    fn write(&self, out: &mut String) {
        out.push_str(self.stored());
    }

    fn parse(text: &str) -> Option<Self> {
        crate::i18n::Language::from_stored(text)
    }
}

impl Stored for Keymap {
    fn write(&self, out: &mut String) {
        out.push_str(match self {
            Keymap::HeronBrush => "heron-brush",
            Keymap::Blender => "blender",
        });
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "heron-brush" => Some(Keymap::HeronBrush),
            "blender" => Some(Keymap::Blender),
            // Earlier releases wrote the default set under another name; an unknown
            // name leaves the default in place, which is that same set.
            _ => None,
        }
    }
}

impl Stored for Layout {
    fn write(&self, out: &mut String) {
        out.push_str(self.written());
    }

    fn parse(text: &str) -> Option<Self> {
        Layout::parse(text)
    }
}

impl Stored for Color {
    fn write(&self, out: &mut String) {
        let _ = write!(out, "{} {} {} {}", self.r, self.g, self.b, self.a);
    }

    fn parse(text: &str) -> Option<Self> {
        let mut channels = text.split_whitespace().map(str::parse::<f32>);
        let mut next = || channels.next().transpose().ok().flatten();
        let (r, g, b) = (next()?, next()?, next()?);
        // Three colour channels imply full opacity.
        let a = next().unwrap_or(1.0);
        Some(Color { r, g, b, a })
    }
}

/// Which set of shortcuts the keyboard uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Keymap {
    /// The editor's own, which is what the manual and the menus document.
    #[default]
    HeronBrush,
    /// Blender's, as near as a tool-based editor can come to them.
    Blender,
}

impl Keymap {
    /// Every keymap, in the order the chooser lists them.
    pub const ALL: [Keymap; 2] = [Keymap::HeronBrush, Keymap::Blender];

    /// The name shown in the preferences.
    pub fn name(self) -> &'static str {
        match self {
            Keymap::HeronBrush => "Heron Brush",
            Keymap::Blender => "Blender",
        }
    }
}

/// How the map views are laid out.
///
/// Available layouts in chooser order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// One pane, cycling through all four views.
    One,
    /// The 3D view beside one flat view.
    Two,
    /// The 3D view beside the top view over one of the side views.
    Three,
    /// The default four-view layout.
    #[default]
    Four,
}

impl Layout {
    /// Every layout in its stored order.
    pub const ALL: [Layout; 4] = [Layout::One, Layout::Two, Layout::Three, Layout::Four];

    /// How the store writes it, which is the name it is read back by.
    ///
    /// The impl of the preference store's own trait is beside the others, in
    /// this module; this is the pair of names it uses.
    pub fn written(self) -> &'static str {
        match self {
            Layout::One => "one",
            Layout::Two => "two",
            Layout::Three => "three",
            Layout::Four => "four",
        }
    }

    /// The layout a store line names, if it names one.
    pub fn parse(text: &str) -> Option<Layout> {
        Layout::ALL
            .into_iter()
            .find(|layout| layout.written() == text)
    }

    /// What to call it in the chooser.
    pub fn name(self) -> &'static str {
        match self {
            Layout::One => translation_key("One pane"),
            Layout::Two => translation_key("Two panes"),
            Layout::Three => translation_key("Three panes"),
            Layout::Four => translation_key("Four panes"),
        }
    }
}

/// Declares the preferences once: the field, its type, its default, and the key it is
/// stored under.
///
/// One declaration generates the preference fields, defaults and persistence keys.
macro_rules! preferences {
    ($(
        $(#[$note:meta])*
        $field:ident: $type:ty = $default:expr, $key:literal;
    )*) => {
        /// Every setting the editor keeps.
        #[derive(Debug, Clone, PartialEq)]
        pub struct Preferences {
            $($(#[$note])* pub $field: $type,)*
        }

        impl Default for Preferences {
            fn default() -> Self {
                Self { $($field: $default,)* }
            }
        }

        impl Preferences {
            /// Writes the store out, one `key=value` line per preference.
            fn write(&self) -> String {
                let mut out = String::new();
                $(
                    out.push_str($key);
                    out.push('=');
                    Stored::write(&self.$field, &mut out);
                    out.push('\n');
                )*
                out
            }

            /// Applies one line of the store.
            ///
            /// A key this build does not know is ignored, so a file written by a later
            /// version keeps what it holds rather than being rejected.
            fn set(&mut self, key: &str, value: &str) {
                match key {
                    $($key => {
                        if let Some(parsed) = Stored::parse(value) {
                            self.$field = parsed;
                        }
                    })*
                    _ => {}
                }
            }
        }
    };
}

preferences! {
    /// Text size in the offline manual, in points.
    manual_font_size: f32 = 13.0, "Manual/Font size";
    /// Whether the manual chapter sidebar is expanded.
    manual_show_contents: bool = true, "Manual/Show contents";
    /// Public GitHub repository used for release checks.
    update_repository: String = "MojzisekJan/HeronBrush".into(), "Updates/Repository";
    update_on_startup: bool = false, "Updates/Check on startup";
    update_prereleases: bool = false, "Updates/Include prereleases";
    /// Custom command shortcuts edited in the Keyboard preferences.
    keyboard_bindings: String = String::new(), "Keyboard/Custom bindings";
    /// Show the start/stop simulation preview control in the main tool strip.
    show_simulation_preview_button: bool = false, "View/Show simulation preview button";
    // Map-view display and colour settings.
    /// Which colour scheme the window is drawn in: the desktop's, or forced light or dark.
    panel_opacity: f32 = 96.0, "Appearance/Panel opacity";
    /// How large the whole interface is drawn, in percent of the display's own scale.
    interface_scale: f32 = 100.0, "Appearance/Interface scale";
    color_scheme: ColorScheme = ColorScheme::System, "ui/Color scheme";
    /// The game a new map is for, by the name its configuration gives it.
    ///
    /// Empty until one is chosen; see [`crate::games`].
    game: String = "heron_brush".to_string(), "ui/Game";
    /// Where that game's own files are: the directory holding its materials and the rest.
    ///
    /// The installation directory supplies the root for paths named by a game configuration.
    game_path: String = String::new(), "ui/Game path";
    /// Mod directories separated by semicolons, in increasing override priority.
    game_mods: String = String::new(), "ui/Game mods";
    /// Which engine an export was last written for, so the window opens on its tab.
    ///
    /// What each engine is *set to* is not here: that is a profile per engine, which is
    /// six times five settings, and it lives in its own file beside this one. See
    /// [`crate::export_settings`].
    export_target: crate::export::Target = crate::export::Target::Godot, "export/Target";
    /// How many panes the map is shown in, and which view is in each.
    ///
    /// The map-view layout; see [`Layout`].
    map_view_layout: Layout = Layout::Four, "ui/Map view layout";
    /// Which language the editor's own text is in.
    ///
    /// Interface language. See [`crate::i18n`].
    language: crate::i18n::Language = crate::i18n::Language::System, "ui/Language";
    /// Which set of keyboard shortcuts is in force.
    ///
    /// Shortcut preset. See [`crate::keymap`].
    keymap: Keymap = Keymap::HeronBrush, "ui/Keymap";
    /// The folders the asset library reads prefab `.map` files from, one per line.
    ///
    /// Ready-made brush assets; see [`crate::assets`].
    asset_library: String = bundled_asset_directory(), "ui/Asset library";
    /// The name the other person sees for you in a live collaborative session. Blank
    /// falls back to the system user name.
    ///
    /// Live collaboration settings. See [`crate::collab`].
    collab_nickname: String = String::new(), "ui/Collaboration nickname";
    /// Whether the 3D view draws the world axes through the origin.
    show_axes: bool = true, "render/Show axes";
    /// What a map view clears to.
    // Named once, in `BACKGROUND_SIDES`: the colour scheme swaps a background that is
    // still one of its two sides and leaves a colour the user chose alone, so a default
    // that is not the dark side would quietly stop the scheme from working.
    background_color: Color = BACKGROUND_SIDES.0, "render/Colors/Background";
    /// The X axis, in the axis display and the compass.
    x_axis_color: Color = Color::bytes(0xFF, 0x3D, 0x00), "render/Colors/X axis";
    /// The Y axis.
    y_axis_color: Color = Color::bytes(0x4B, 0x95, 0x00), "render/Colors/Y axis";
    /// The Z axis.
    z_axis_color: Color = Color::bytes(0x10, 0x9C, 0xFF), "render/Colors/Z axis";
    /// The line marking the bounds the map is meant to stay inside.
    soft_map_bounds_color: Color = Color::bytes(241, 125, 37), "render/Colors/Soft map bounds color";
    /// A face with no material to wear.
    ///
    /// This is the surface's albedo, and no real surface reflects everything that falls
    /// on it: white was tried and made every untextured face clip to the top of the tone
    /// map, which is what makes a viewport look flat.
    ///
    /// Colours are stored as display values and converted to linear light in the renderer. A
    /// display value of 0.5 corresponds to approximately 0.2 in linear light.
    face_color: Color = Color::rgb(0.5, 0.5, 0.5), "render/Colors/Faces";
    /// A selected face.
    selected_face_color: Color = Color::rgb(1.0, 0.85, 0.85), "render/Colors/Selected faces";
    /// A face on a locked layer.
    locked_face_color: Color = Color::rgb(0.85, 0.85, 1.0), "render/Colors/Locked faces";
    /// An unselected edge.
    edge_color: Color = Color::rgb(0.9, 0.9, 0.9), "render/Colors/Edges";
    /// A selected edge.
    selected_edge_color: Color = Color::rgb(1.0, 0.0, 0.0), "render/Colors/Selected edges";
    /// An edge on a locked layer.
    locked_edge_color: Color = Color::rgb(0.13, 0.3, 1.0), "render/Colors/Locked edges";
    /// An entity with no definition to give it one.
    undefined_entity_color: Color = Color::rgb(0.5, 0.5, 0.5), "render/Colors/Undefined entity";
    /// The box drawn around the selection.
    selection_bounds_color: Color = Color::rgba(1.0, 0.0, 0.0, 0.35), "render/Colors/Selection bounds";
    /// The text of a label drawn over the map: the selection's sizes, for one.
    info_overlay_text_color: Color = Color::rgb(1.0, 1.0, 1.0), "render/Colors/Info overlay text";
    /// The box behind such a label.
    info_overlay_background_color: Color = Color::rgba(0.0, 0.0, 0.0, 0.6), "render/Colors/Info overlay background";
    /// The text of a label about something picked: the coordinates of a handle a drag has
    /// hold of.
    selected_info_overlay_text_color: Color = Color::rgb(1.0, 1.0, 1.0), "render/Colors/Selected info overlay text";
    /// The box behind such a label.
    selected_info_overlay_background_color: Color = Color::rgba(1.0, 0.0, 0.0, 0.6), "render/Colors/Selected info overlay background";
    /// How opaque the box behind a size label is; `WeakInfoOverlayBackgroundAlpha`, which
    /// is fainter than the box behind an entity's name.
    weak_info_overlay_background_alpha: f32 = 0.3, "render/Colors/Weak info overlay background alpha";
    /// The line from an entity to what it targets.
    ///
    /// The default entity-link colour.
    entity_link_color: Color = Color::rgb(0.5, 1.0, 0.5), "render/Colors/Entity link";
    /// Such a line when the selection is one of its ends.
    selected_entity_link_color: Color = Color::rgb(1.0, 0.0, 0.0), "render/Colors/Selected entity link";
    /// A link that removes what it aims at (`killtarget`) rather than firing it.
    kill_entity_link_color: Color = Color::rgb(0.8, 0.35, 1.0), "render/Colors/Kill entity link";
    /// A group's outline and its name.
    group_color: Color = Color::rgb(0.7, 0.4, 1.0), "render/Colors/Groups";
    /// A linked group, which is drawn apart from an ordinary one.
    linked_group_color: Color = Color::rgb(1.0, 0.35, 0.87), "render/Colors/Linked Groups";
    /// A draggable handle.
    handle_color: Color = Color::rgb(0.97, 0.9, 0.23), "render/Colors/Handle";
    /// A handle that is selected.
    selected_handle_color: Color = Color::rgb(1.0, 0.0, 0.0), "render/Colors/Selected handle";
    /// A point placed by the clip tool.
    clip_handle_color: Color = Color::rgb(1.0, 0.5, 0.0), "render/Colors/Clip handle";
    /// The plane the clip tool would cut along.
    clip_face_color: Color = Color::rgba(0.6, 0.4, 0.0, 0.35), "render/Colors/Clip face";
    /// The extrude tool's handle.
    resize_handle_color: Color = Color::bytes(248, 230, 60), "render/Colors/Resize handle";
    /// The rotate tool's ring.
    rotate_handle_color: Color = Color::bytes(248, 230, 60), "render/Colors/Rotate handle";
    /// The scale tool's corners.
    scale_handle_color: Color = Color::bytes(77, 255, 80), "render/Colors/Scale handle";
    /// The shear tool's outline.
    shear_outline_color: Color = Color::bytes(45, 133, 255), "render/Colors/Shear outline";
    /// The grid in a 2D view.
    grid_color_2d: Color = Color::rgba(0.8, 0.8, 0.8, 0.8), "render/Grid/Color2D";
    /// How strongly the grid shows on a face in the 3D view.
    grid_alpha: f32 = 0.5, "render/Grid/Alpha";

    // Controls: the camera and the handles.
    /// How far the camera turns for a given mouse movement.
    camera_look_speed: f32 = 0.5, "Controls/Camera/Look speed";
    /// Whether turning left looks right.
    invert_horizontal_look: bool = false, "Controls/Camera/Invert horizontal look";
    /// Whether pushing forward looks down.
    invert_vertical_look: bool = false, "Controls/Camera/Invert vertical look";
    /// How far the camera slides for a given mouse movement.
    camera_pan_speed: f32 = 0.5, "Controls/Camera/Pan speed";
    /// Whether panning left moves right.
    invert_horizontal_pan: bool = false, "Controls/Camera/Invert horizontal pan";
    /// Whether panning up moves down.
    invert_vertical_pan: bool = false, "Controls/Camera/Invert vertical pan";
    /// How far the camera moves for a given drag or scroll.
    camera_move_speed: f32 = 0.3, "Controls/Camera/Move speed";
    /// Whether holding Alt during a middle-button drag moves the camera along its view
    /// axis rather than panning it.
    camera_enable_alt_move: bool = false, "Controls/Camera/Use alt to move";
    /// Whether that Alt drag moves the camera the other way along its view axis.
    camera_alt_move_invert: bool = false, "Controls/Camera/Invert zoom direction when using alt to move";
    /// How far the camera flies per key press.
    ///
    /// Scales the movement step for a keyboard press.
    camera_fly_move_speed: f32 = 0.5, "Controls/Camera/Fly move speed";
    /// The 3D camera's field of view, in degrees across the wider side.
    camera_fov: f32 = 90.0, "Controls/Camera/Field of vision";
    /// Whether moving forward follows the pointer rather than the camera.
    camera_move_in_cursor_dir: bool = false, "Controls/Camera/Move camera in cursor dir";
    /// Whether the wheel zooms the other way.
    invert_mouse_wheel: bool = false, "Controls/Camera/Invert mouse wheel";
    /// Whether the three 2D views share a centre.
    link_2d_cameras: bool = true, "Controls/Camera/Link 2D cameras";
    /// Whether a vertex drag in a flat view also moves vertices at the same projected position.
    drag_occluded_vertices_2d: bool = true, "Controls/Move occluded vertices in 2D";
    /// How large a handle is drawn and how near the pointer must come to grab it.
    handle_radius: f32 = 3.0, "Controls/Handle radius";
    /// The radius of the rotate tool's ring.
    rotate_handle_radius: f32 = 64.0, "Controls/Rotate handle radius";

    /// The line drawn from an entity out through a leak.
    ///
    /// The colour of compiler point-file paths and editor leak traces.
    point_file_color: Color = Color::rgb(0.0, 1.0, 0.0), "render/Colors/Point file";
    /// What a compiler's portals are drawn in.
    ///
    /// The outline colour for loaded portal polygons.
    portal_file_color: Color = Color::rgb(1.0, 1.0, 1.0), "render/Colors/Portal file";

    // Editing.
    /// How finely the leak check cuts up the space around the level, in world units.
    ///
    /// The check floods a grid of cells and cannot see a hole smaller than one, so this
    /// should be no larger than the thinnest wall in the level. Smaller is slower: the
    /// work grows with the cube of it.
    leak_check_cell_size: f32 = 8.0, "Editor/Leak check cell size";
    /// Whether a face keeps its mapping when the brush moves.
    alignment_lock: bool = true, "Editor/Texture lock";
    /// Whether a face keeps its mapping when the brush turns.
    uv_lock: bool = false, "Editor/UV lock";

    // Renderer pass settings.
    /// Enables physically based viewport shading.
    ///
    /// When off, each face uses its material's
    /// colour with a little directional shading so a box still reads as a box, and none
    /// of the shadow, occlusion, reflection or bloom passes touching it.
    realistic_shading: bool = true, "render/Realistic shading";
    /// Whether a selected brush is outlined in the 3D view.
    ///
    /// Draws selection outlines over scene geometry so they remain visible in any lighting.
    selection_outline: bool = true, "render/Selection outline";
    /// Whether the sun casts shadows.
    shadows: bool = true, "render/Shadows/Enabled";
    /// The side of the square shadow map, in pixels.
    ///
    /// Larger is sharper and slower; it is rounded to a power of two when the map is
    /// made.
    shadow_map_size: u32 = 2048, "render/Shadows/Map size";
    /// How far a surface is pushed out of its own shadow.
    ///
    /// Too little and a flat surface stripes itself; too much and a shadow floats away
    /// from what casts it.
    shadow_bias: f32 = 0.0015, "render/Shadows/Bias";
    /// How wide the shadow's soft edge is, as a fraction of the map.
    shadow_softness: f32 = 1.0, "render/Shadows/Softness";
    /// Whether the short-range shadows the shadow map is too coarse to hold are added.
    ///
    /// A shadow map stretched over a whole level has texels the size of a doorway, so the
    /// dark line where a wall meets a floor is not in it. This marches the depth buffer
    /// over a few units to find it.
    contact_shadows: bool = true, "render/Shadows/Contact shadows";
    /// How far the contact shadow march reaches, in world units.
    contact_shadow_distance: f32 = 16.0, "render/Shadows/Contact distance";
    /// How many samples the march takes.
    ///
    /// More is smoother and slower; the march is per pixel, so this is the one renderer
    /// setting that costs in proportion to the window's size.
    contact_shadow_samples: f32 = 12.0, "render/Shadows/Contact samples";

    /// Whether creases and corners are darkened.
    ambient_occlusion: bool = true, "render/Ambient occlusion/Enabled";
    /// How far from a point the occlusion looks, in world units.
    ao_radius: f32 = 24.0, "render/Ambient occlusion/Radius";
    /// How dark the occlusion goes.
    ao_strength: f32 = 1.0, "render/Ambient occlusion/Strength";
    /// Whether smooth surfaces reflect what is on screen.
    ///
    /// The reflection replaces the sky the shading already put there rather than being
    /// added to it, so turning this on does not make a surface brighter for free.
    reflections: bool = true, "render/Reflections/Enabled";
    /// How far a reflection ray marches, in world units.
    reflection_distance: f32 = 512.0, "render/Reflections/Distance";
    /// How many steps the march takes.
    reflection_samples: f32 = 24.0, "render/Reflections/Samples";
    /// How strongly what the march finds replaces the sky.
    reflection_strength: f32 = 1.0, "render/Reflections/Strength";

    /// Whether bright surfaces bleed into what is around them.
    bloom: bool = true, "render/Bloom/Enabled";
    /// How bright a pixel must be to bleed.
    bloom_threshold: f32 = 1.0, "render/Bloom/Threshold";
    /// How much of the bleed is added back.
    bloom_intensity: f32 = 0.5, "render/Bloom/Intensity";
    /// How much light reaches the film before the tone map.
    exposure: f32 = 1.0, "render/Exposure";
    /// The curve that maps light to the screen, in the wgpu 3D view.
    ///
    /// The GL renderer (a build without the `wgpu` feature) always uses ACES, so it does
    /// not offer the choice.
    tone_mapper: ToneMapper = ToneMapper::Agx, "render/Tone mapper";
    /// Whether light entities (point and spot lights) light the wgpu 3D view.
    ///
    /// The GL renderer draws the sun and the sky only, so a GL build does not offer it.
    entity_lights: bool = true, "render/Entity lights";
    /// How much GPU memory material textures may take in the wgpu 3D view, in MiB,
    /// before the least recently drawn are dropped (and streamed in again when seen).
    ///
    /// The GL renderer keeps every texture the map uses, so only the wgpu build offers it.
    texture_memory_mb: u32 = 1024, "render/Texture memory";
    /// Whether a still view keeps being drawn and averaged, to smooth its edges.
    ///
    /// The one thing that makes an edge genuinely smooth rather than guessed at: the same
    /// view drawn again and again with the sample point moved about inside the pixel. It
    /// costs nothing while the view is moving, since there is nothing to accumulate then.
    accumulate: bool = true, "render/Accumulate samples";
    /// How many samples a still view gathers before it stops redrawing.
    accumulation_samples: u32 = 16, "render/Accumulation samples";
    /// Whether the edges are smoothed after the tone map.
    antialiasing: bool = true, "render/Antialiasing";
    /// Draw the environment behind geometry in shaded 3D views.
    procedural_sky: bool = true, "render/Procedural sky";
    /// The colour the sky lights the scene with.
    sky_color: Color = Color::rgb(0.10980392, 0.44313726, 0.84705883), "render/Sky color";
    /// The haze just above the horizon.
    ///
    /// The brightest part of a real sky, and what stops every sideways-facing surface in
    /// a level taking the same middling grey.
    horizon_color: Color = Color::rgb(0.5254902, 0.36862746, 0.23529412), "render/Horizon color";
    /// The colour the ground bounces back.
    ground_color: Color = Color::rgb(0.10, 0.09, 0.08), "render/Ground color";
    /// How high the sun stands above the horizon, in degrees.
    ///
    /// The default is where `SUN_DIRECTION` used to put it: high enough that a box reads
    /// as a box, low enough that its walls do not all shade the same.
    sun_elevation: f32 = 46.0, "render/Sun elevation";
    /// Which way the sun lies, in degrees counter-clockwise from the X axis.
    sun_azimuth: f32 = 233.0, "render/Sun azimuth";
    /// The colour of the light standing in for the sun.
    sun_color: Color = Color::rgb(1.0, 0.9375, 0.84375), "render/Sun color";
    /// How bright the sun is.
    sun_intensity: f32 = 3.2, "render/Sun intensity";
    /// How bright the sky and ground are.
    ///
    /// Below the sun rather than beside it: the sky lights every face of a brush from
    /// every side at once, so at the same strength as the sun it left the three faces of
    /// a box within a few levels of each other and the box read as a flat white shape
    ///. At this the sun does the shaping and the sky fills the shadow.
    ambient_intensity: f32 = 0.45, "render/Ambient intensity";
    /// How rough a surface with no material is, from a mirror at zero to matte at one.
    default_roughness: f32 = 0.65, "render/Default roughness";
    /// How metallic a surface with no material is.
    default_metalness: f32 = 0.0, "render/Default metalness";
}

impl Preferences {
    /// Reads the store, falling back to the defaults for anything it does not hold.
    ///
    /// A store that cannot be read at all — because there is none yet, or because it is
    /// not readable — gives the defaults rather than an error: the editor has to start
    /// either way.
    pub fn load(path: &Path) -> Self {
        let mut preferences = Self::default();
        let Ok(text) = crate::files::read_to_string(path) else {
            return preferences;
        };
        // A store written before there was a version is version one.
        let mut version = 1;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let (key, value) = (key.trim(), value.trim());
                if key == STORE_VERSION_KEY {
                    version = value.parse().unwrap_or(1);
                    continue;
                }
                preferences.set(key, value);
            }
        }
        preferences.upgrade(version);
        if preferences.game == "Generic" {
            preferences.game = "heron_brush".into();
        }
        preferences
    }

    /// Brings a store written by an earlier version up to date.
    ///
    /// A preference that has changed meaning cannot simply be read: what is in the file
    /// was right for the code that wrote it. Each step here fixes one such change, and
    /// only where the value is the one that version left behind -- a value the user has
    /// since chosen is theirs, whatever it is.
    fn upgrade(&mut self, from: u32) {
        // Version 1 to 2 migrates face colours from linear light to display values, which the
        // renderer now converts to linear light when drawing.
        if from < 2 && self.face_color == Color::rgb(0.2, 0.2, 0.2) {
            self.face_color = Self::default().face_color;
        }
        // Version 2 to 3: the sky was as bright as the sun, which lit every face of a
        // brush to within a few levels of every other -- "as if the light came from all
        // sides", which is exactly what it was.
        if from < 3 && self.ambient_intensity == 1.0 {
            self.ambient_intensity = Self::default().ambient_intensity;
        }
        // Version 3 to 4: stores written by earlier builds kept the grey-blue sky of the
        // render camera's "Day" lighting (sky 0.34 0.42 0.55, horizon 0.62 0.68 0.78)
        // rather than the default blue sky over a brown horizon, so every view drew
        // overcast. Only those exact values are replaced.
        if from < 4 {
            let defaults = Self::default();
            if self.sky_color == Color::rgb(0.34, 0.42, 0.55) {
                self.sky_color = defaults.sky_color;
            }
            if self.horizon_color == Color::rgb(0.62, 0.68, 0.78) {
                self.horizon_color = defaults.horizon_color;
            }
            // Builds run from the repository found nothing beside their executable and
            // saved the asset library empty, which hid the bundled assets for good.
            if self.asset_library.trim().is_empty() {
                self.asset_library = defaults.asset_library;
            }
        }
    }

    /// Writes the store, making the directory it goes in if it is not there.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(directory) = path.parent() {
            crate::files::create_dir_all(directory)?;
        }
        crate::files::write(
            path,
            format!("{STORE_VERSION_KEY}={STORE_VERSION}\n{}", self.write()),
        )
    }

    /// Where the store lives.
    ///
    /// A file named `preferences` in the profile directory, [`config_dir`].
    pub fn default_path() -> PathBuf {
        config_dir().join("preferences")
    }
}

/// The profile directory, `<user config>/heron-brush`: preferences, recent maps,
/// stars, export settings, plugins, examples and caches.
pub fn config_dir() -> PathBuf {
    user_config_dir().join("heron-brush")
}

/// The platform's per-user settings directory: `%LOCALAPPDATA%` on Windows,
/// `$XDG_CONFIG_HOME` or `~/.config` everywhere else, macOS included, which is where
/// every release so far has kept its profile. [`set_platform_directories`] overrides it.
pub fn user_config_dir() -> PathBuf {
    if let Some(dir) = PLATFORM_DIRECTORIES
        .get()
        .and_then(|dirs| dirs.config.clone())
    {
        return dir;
    }
    #[cfg(windows)]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA").filter(|v| !v.is_empty()) {
            return PathBuf::from(local);
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
            let xdg = PathBuf::from(xdg);
            if xdg.is_absolute() {
                return xdg;
            }
        }
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.join(".config")
}

/// The shape of the store this build writes. See [`Preferences::upgrade`].
///
/// It is not a preference -- nothing reads it but the loader, and there is nothing in the
/// dialog to set it to -- so it is written and read here rather than being one of the
/// fields above.
const STORE_VERSION: u32 = 4;

/// The key the version is written under.
const STORE_VERSION_KEY: &str = "store/Version";

thread_local! {
    /// The preferences everything reads.
    ///
    /// Preferences are stored per thread because the UI runs on a single thread. Drawing code can read
    /// them without passing a settings reference through every callback or taking a lock.
    static CURRENT: RefCell<Preferences> = RefCell::new(Preferences::default());
}

/// Reads one preference.
///
/// Reads a preference from the current thread.
pub fn pref<T>(read: impl FnOnce(&Preferences) -> T) -> T {
    CURRENT.with(|current| read(&current.borrow()))
}

thread_local! {
    static VERSION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// A number that changes whenever the preferences might have, so what is drawn from
/// them (a flat view's cached picture) can tell it is out of date.
pub fn version() -> u64 {
    VERSION.with(std::cell::Cell::get)
}

fn bump_version() {
    VERSION.with(|v| v.set(v.get().wrapping_add(1)));
}

/// Changes the preferences.
///
/// Nothing redraws by itself afterwards: what to bring up to date is the caller's to
/// know, and the dialog is the only caller that has to.
pub fn edit(change: impl FnOnce(&mut Preferences)) {
    bump_version();
    CURRENT.with(|current| change(&mut current.borrow_mut()));
}

/// Puts the view colours on the side of light or dark the window is drawn on.
///
/// Theme colours cover viewport backgrounds and grids as well as the panels.
///
/// Only a colour still on one of the two sides is moved: one the user has picked out for
/// themselves is theirs, and stays put through a scheme change.
pub fn follow_color_scheme(dark: bool) {
    edit(|preferences| {
        for (colour, sides) in [
            (&mut preferences.background_color, BACKGROUND_SIDES),
            (&mut preferences.edge_color, EDGE_SIDES),
            (&mut preferences.grid_color_2d, GRID_SIDES),
            (&mut preferences.portal_file_color, PORTAL_SIDES),
        ] {
            let (dark_side, light_side) = sides;
            if *colour == dark_side || *colour == light_side {
                *colour = if dark { dark_side } else { light_side };
            }
        }
    });
}

/// What a view clears to, dark and light.
///
/// The default dark background provides contrast for grids and brush edges.
const BACKGROUND_SIDES: (Color, Color) = (Color::bytes(34, 34, 34), Color::bytes(214, 214, 214));
/// An unselected edge, which has to show against whichever background is behind it.
const EDGE_SIDES: (Color, Color) = (Color::rgb(0.9, 0.9, 0.9), Color::rgb(0.13, 0.13, 0.13));
/// The grid in a 2D view.
const GRID_SIDES: (Color, Color) = (
    Color::rgba(0.8, 0.8, 0.8, 0.8),
    Color::rgba(0.22, 0.22, 0.22, 0.55),
);
/// A compiler's portals, which are drawn as outlines.
const PORTAL_SIDES: (Color, Color) = (Color::rgb(1.0, 1.0, 1.0), Color::rgb(0.1, 0.1, 0.1));

/// Replaces the preferences wholesale, as loading the store does.
pub fn set_current(preferences: Preferences) {
    bump_version();
    CURRENT.with(|current| *current.borrow_mut() = preferences);
}

/// A copy of the preferences as they stand.
pub fn current() -> Preferences {
    CURRENT.with(|current| current.borrow().clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_three_store_gets_the_blue_sky_and_the_bundled_assets_back() {
        let path = std::env::temp_dir().join(format!("heron-prefs-v3-{}", std::process::id()));
        std::fs::write(
            &path,
            "store/Version=3\nui/Asset library=\nrender/Sky color=0.34 0.42 0.55 1\nrender/Horizon color=0.62 0.68 0.78 1\nrender/Ground color=0.2 0.2 0.2 1\n",
        )
        .unwrap();
        let loaded = Preferences::load(&path);
        std::fs::remove_file(&path).unwrap();
        let defaults = Preferences::default();
        assert_eq!(loaded.sky_color, defaults.sky_color);
        assert_eq!(loaded.horizon_color, defaults.horizon_color);
        assert_eq!(loaded.asset_library, defaults.asset_library);
        assert!(
            !loaded.asset_library.is_empty(),
            "the repository's assets are found"
        );
        // A colour the user chose is theirs.
        assert_eq!(loaded.ground_color, Color::rgb(0.2, 0.2, 0.2));
    }

    #[test]
    fn the_light_scheme_lights_the_views_and_the_dark_one_darkens_them_again() {
        set_current(Preferences::default());

        follow_color_scheme(false);
        assert!(!pref(|p| p.background_color).is_dark(), "a light view");
        assert!(pref(|p| p.edge_color).is_dark(), "dark edges over it");

        follow_color_scheme(true);
        assert_eq!(
            pref(|p| p.background_color),
            Preferences::default().background_color
        );
        assert_eq!(pref(|p| p.edge_color), Preferences::default().edge_color);
    }

    #[test]
    fn a_colour_the_user_chose_is_left_alone_by_a_scheme_change() {
        set_current(Preferences::default());
        let chosen = Color::bytes(20, 60, 90);
        edit(|preferences| preferences.background_color = chosen);

        follow_color_scheme(false);
        assert_eq!(pref(|p| p.background_color), chosen);
    }

    #[test]
    fn the_language_line_reads_back_every_language() {
        for language in crate::i18n::Language::all() {
            let mut text = String::new();
            Stored::write(&language, &mut text);
            assert_eq!(
                <crate::i18n::Language as Stored>::parse(&text),
                Some(language),
                "{text}"
            );
        }
        // What earlier versions wrote still reads back as what it was.
        for (text, language) in [
            ("system", crate::i18n::Language::System),
            ("english", crate::i18n::Language::English),
            ("czech", crate::i18n::Language::Czech),
            ("german", crate::i18n::Language::German),
            ("chinese", crate::i18n::Language::Chinese),
            ("spanish", crate::i18n::Language::Spanish),
            ("ukrainian", crate::i18n::Language::Ukrainian),
            ("polish", crate::i18n::Language::Polish),
            ("italian", crate::i18n::Language::Italian),
            ("french", crate::i18n::Language::French),
            ("portuguese_brazil", crate::i18n::Language::PortugueseBrazil),
            ("japanese", crate::i18n::Language::Japanese),
            ("korean", crate::i18n::Language::Korean),
            ("turkish", crate::i18n::Language::Turkish),
            ("indonesian", crate::i18n::Language::Indonesian),
        ] {
            assert_eq!(
                <crate::i18n::Language as Stored>::parse(text),
                Some(language),
                "{text}"
            );
        }
    }

    #[test]
    fn occluded_vertex_dragging_is_enabled_by_default() {
        assert!(Preferences::default().drag_occluded_vertices_2d);
    }

    #[test]
    fn a_store_that_is_not_there_gives_the_defaults() {
        let path = std::env::temp_dir().join("heron-preferences-missing");
        let _ = std::fs::remove_file(&path);
        assert_eq!(Preferences::load(&path), Preferences::default());
    }

    #[test]
    fn every_preference_survives_being_written_and_read() {
        // Changed away from the defaults so that a preference the store forgets shows up
        // as a difference rather than passing by accident.
        let written = Preferences {
            drag_occluded_vertices_2d: false,
            show_axes: false,
            background_color: Color::rgba(0.1, 0.2, 0.3, 0.4),
            camera_fov: 75.0,
            invert_vertical_look: true,
            shadow_map_size: 4096,
            shadow_bias: 0.004,
            exposure: 1.75,
            tone_mapper: ToneMapper::TonyMcMapface,
            entity_lights: false,
            sky_color: Color::rgb(0.9, 0.8, 0.7),
            uv_lock: true,
            ..Default::default()
        };

        let path = std::env::temp_dir().join("heron-preferences-roundtrip");
        written.save(&path).expect("it saves");
        assert_eq!(Preferences::load(&path), written);
    }

    #[test]
    fn a_line_that_does_not_parse_leaves_that_preference_alone() {
        let path = std::env::temp_dir().join("heron-preferences-broken");
        std::fs::write(
            &path,
            "# a comment\n\
             Controls/Camera/Field of vision=not a number\n\
             render/Show axes=false\n\
             something/Nobody knows=1\n\
             malformed line with no equals sign\n",
        )
        .expect("a written store");

        let preferences = Preferences::load(&path);
        assert_eq!(preferences.camera_fov, Preferences::default().camera_fov);
        assert!(!preferences.show_axes);
    }

    #[test]
    fn a_colour_with_three_channels_is_opaque() {
        let parsed = <Color as Stored>::parse("0.25 0.5 0.75").expect("a colour");
        assert_eq!(parsed, Color::rgb(0.25, 0.5, 0.75));
    }

    #[test]
    fn the_colour_scheme_reads_back_what_it_wrote() {
        for scheme in [ColorScheme::System, ColorScheme::Light, ColorScheme::Dark] {
            let mut text = String::new();
            Stored::write(&scheme, &mut text);
            assert_eq!(<ColorScheme as Stored>::parse(&text), Some(scheme));
        }
        assert_eq!(<ColorScheme as Stored>::parse("sepia"), None);
        assert_eq!(ColorScheme::default(), ColorScheme::System);
    }

    #[test]
    fn a_colour_can_be_dimmed_without_changing_it() {
        let colour = Color::rgb(1.0, 0.5, 0.0).with_alpha(0.35);
        assert_eq!(colour.rgb_f32(), [1.0, 0.5, 0.0]);
        assert_eq!(colour.rgba_f32(), [1.0, 0.5, 0.0, 0.35]);
        // Widening to a double keeps the value the float held, not the one it was
        // written as, so the comparison is of the same width.
        assert_eq!(colour.rgba_f64(), (1.0, 0.5, 0.0, f64::from(0.35f32)));
    }

    #[test]
    fn the_defaults_are_stable() {
        // Keep the initial appearance and interaction defaults stable.
        let defaults = Preferences::default();
        // The configured default background must match the dark colour scheme for automatic theme
        // switching.
        assert_eq!(defaults.background_color, Color::bytes(34, 34, 34));
        assert_eq!(defaults.background_color, BACKGROUND_SIDES.0);
        assert_eq!(defaults.edge_color, Color::rgb(0.9, 0.9, 0.9));
        assert_eq!(defaults.camera_fov, 90.0);
        assert_eq!(defaults.camera_move_speed, 0.3);
        assert_eq!(defaults.handle_radius, 3.0);
        assert_eq!(defaults.rotate_handle_radius, 64.0);
        assert_eq!(defaults.grid_alpha, 0.5);
        assert!(defaults.alignment_lock);
        assert!(!defaults.uv_lock);
    }
}
