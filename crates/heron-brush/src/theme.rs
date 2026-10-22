//! The editor's look: colours, sizes, fonts, and the painted widgets that egui has no
//! close enough equivalent of.

thread_local! { static DARK: std::cell::Cell<bool> = const { std::cell::Cell::new(true) }; }

use editor_core::i18n::Language;
use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Rect, Response, Sense, Stroke, Ui,
};

const fn pick(dark: bool, d: u32, l: u32) -> Color32 {
    let c = if dark { d } else { l };
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// Whether the dark scheme is in use.
pub fn is_dark() -> bool {
    DARK.get()
}
/// The window, the header bar, the toolbar and the inspector.
pub fn bg() -> Color32 {
    pick(DARK.get(), 0x1b1c1f, 0xeceef1)
}
/// The main window's background at the "Panel opacity (%)" preference, clamped to 80 to
/// 100 %: the desktop shows faintly through the window's
/// chrome and panels while the map views, which paint their own background, stay opaque.
pub fn panel() -> Color32 {
    let percent = editor_core::preferences::pref(|p| p.panel_opacity);
    let alpha = if percent.is_finite() {
        percent.clamp(80.0, 100.0) / 100.0
    } else {
        0.96
    };
    bg().gamma_multiply(alpha)
}
/// What a panel drawn over the window's background fills with: nothing while that
/// background is translucent, since a second translucent layer would make the panel more
/// opaque than the rest of the window, and the plain background otherwise.
pub fn panel_layer() -> Color32 {
    if panel().a() < 255 {
        Color32::TRANSPARENT
    } else {
        bg()
    }
}
/// The darker band some panels use (tab strips, the output pane), at the panel opacity.
pub fn panel_base() -> Color32 {
    if panel().a() < 255 {
        base().gamma_multiply(0.35)
    } else {
        base()
    }
}
/// The darkest layer: gutters between panes, the header bar's underside, wells.
pub fn base() -> Color32 {
    pick(DARK.get(), 0x141517, 0xdfe2e6)
}
/// Raised surfaces: cards, popups, menus, the current tab.
pub fn surface() -> Color32 {
    pick(DARK.get(), 0x23252a, 0xf8f9fa)
}
/// Hairline borders around controls and cards.
pub fn border() -> Color32 {
    pick(DARK.get(), 0x2f3237, 0xcdd1d6)
}
/// A 2D view's grid lines, and the lines between panes.
pub fn grid() -> Color32 {
    pick(DARK.get(), 0x3a3d43, 0xbfc3c9)
}
use egui_native_look as look;
pub use egui_native_look::Look;

thread_local! {
    /// The theme [`install`] gave the controls, and its colours in the scheme in force.
    static THEME: std::cell::RefCell<Option<look::Theme>> = const { std::cell::RefCell::new(None) };
    static COLORS: std::cell::Cell<Option<look::Colors>> = const { std::cell::Cell::new(None) };
}

/// The look of the controls ([`egui_native_look`]): this system's, or the one
/// `HERON_BRUSH_LOOK=plain|mac|windows|gtk` names, so a developer can see one system's
/// controls on another.
pub fn look() -> Look {
    THEME
        .with(|t| t.borrow().as_ref().map(|t| t.look))
        .unwrap_or_else(chosen_look)
}

/// The editor's theme for the controls: the chosen look, the editor's palette, its
/// 28-point controls and its text faces.
fn controls_theme() -> look::Theme {
    let chosen = chosen_look();
    let metrics = look::Metrics {
        control_height: CONTROL_HEIGHT,
        radius: look::Metrics::of(chosen).radius,
        body: body(),
        bold: body_bold(),
    };
    look::Theme::new(chosen).schemes(schemes()).metrics(metrics)
}

/// Hands the controls' theme to `ctx` and keeps its colours for the scheme in force.
fn keep_theme(ctx: &egui::Context) {
    let theme = look::set_theme(ctx, controls_theme());
    COLORS.set(Some(look::Colors::resolve(&theme, DARK.get())));
    THEME.with(|t| *t.borrow_mut() = Some(theme));
}

/// The controls' colours in the scheme in force.
fn colors() -> look::Colors {
    COLORS
        .get()
        .unwrap_or_else(|| look::Colors::resolve(&controls_theme(), DARK.get()))
}

/// Resolves the controls' colours again after the scheme changed.
fn refresh_colors() {
    let colors = THEME.with(|t| {
        t.borrow()
            .as_ref()
            .map(|theme| look::Colors::resolve(theme, DARK.get()))
    });
    COLORS.set(colors);
}
/// The look [`install`] picks.
fn chosen_look() -> Look {
    std::env::var("HERON_BRUSH_LOOK")
        .ok()
        .as_deref()
        .and_then(Look::from_name)
        .unwrap_or_else(Look::system)
}
/// Whether the controls take the macOS look.
pub fn mac() -> bool {
    look() == Look::Mac
}
/// Whether the controls take the WinUI look.
pub fn win() -> bool {
    look() == Look::Windows
}

/// The editor's palette for each scheme, which the controls of every look start from.
fn schemes() -> look::Schemes {
    let palette = |dark: bool| look::Palette {
        fg: pick(dark, 0xe4e6ea, 0x1d2025),
        dim: pick(dark, 0x8b9199, 0x5e646c),
        disabled: pick(dark, 0x52575e, 0xa3a8af),
        bg: pick(dark, 0x1b1c1f, 0xeceef1),
        surface: pick(dark, 0x1b1c1f, 0xeceef1),
        border: pick(dark, 0x2f3237, 0xcdd1d6),
        grid: pick(dark, 0x3a3d43, 0xbfc3c9),
        button: pick(dark, 0x2c2f34, 0xe1e4e8),
        button_hover: pick(dark, 0x373a41, 0xd5d9de),
        button_pressed: pick(dark, 0x25272b, 0xc9cdd3),
        button_disabled: pick(dark, 0x222428, 0xe6e8eb),
        entry: pick(dark, 0x121315, 0xffffff),
        accent: EDITOR_ACCENT,
        accent_hover: EDITOR_ACCENT_HOVER,
        row_hover: if dark {
            Color32::from_white_alpha(10)
        } else {
            Color32::from_black_alpha(12)
        },
    };
    look::Schemes {
        dark: palette(true),
        light: palette(false),
    }
}

/// A button's fill.
pub fn button_fill() -> Color32 {
    colors().button
}
/// A hovered button's fill.
pub fn button_hover() -> Color32 {
    colors().button_hover
}
/// A pressed button's fill.
pub fn button_pressed() -> Color32 {
    colors().button_pressed
}
/// A text entry's fill.
pub fn entry() -> Color32 {
    colors().entry
}
/// A focused text entry's fill: WinUI's entry turns solid while typed in.
pub fn entry_focused() -> Color32 {
    colors().entry_focused
}
/// The edge of a button, an entry or a drop-down.
pub fn control_border() -> Color32 {
    colors().control_border
}
/// WinUI's strong stroke: an entry's underline, a switch's and an empty check box's edge.
pub fn control_strong() -> Color32 {
    colors().control_strong
}
/// The inspector's current tab.
#[allow(dead_code)]
pub fn tab_active() -> Color32 {
    surface()
}
/// The editor's own accent, where the system offers none.
const EDITOR_ACCENT: Color32 = Color32::from_rgb(0x3d, 0x8b, 0xfd);
/// The editor's accent under the pointer.
const EDITOR_ACCENT_HOVER: Color32 = Color32::from_rgb(0x5a, 0x9d, 0xff);

/// The one accent colour: selection, switched-on toggles, focus rings.
pub fn accent() -> Color32 {
    colors().accent
}
/// Text and glyphs on an accent fill: black on WinUI's light accent of the dark scheme,
/// white elsewhere.
pub fn on_accent() -> Color32 {
    colors().on_accent
}
/// The accent under the pointer.
pub fn accent_hover() -> Color32 {
    colors().accent_hover
}
/// The accent as text or a thin line on the window's background: links, accent
/// captions. The plain accent is too light to read on the light scheme's surfaces.
pub fn accent_text() -> Color32 {
    if win() {
        accent()
    } else if accent() == EDITOR_ACCENT {
        pick(DARK.get(), 0x5a9dff, 0x1f63c9)
    } else if DARK.get() {
        accent_hover()
    } else {
        accent().gamma_multiply(0.78).to_opaque()
    }
}
/// A "running" or "all good" mark: the live-server dot, connected indicators.
pub fn success() -> Color32 {
    pick(DARK.get(), 0x4cc36b, 0x1e8e3e)
}
/// A starred item's star: gold, deeper on the light scheme so it stands off the page.
pub fn star() -> Color32 {
    pick(DARK.get(), 0xf2b838, 0xd9930b)
}
/// Ordinary text.
pub fn fg() -> Color32 {
    pick(DARK.get(), 0xe4e6ea, 0x1d2025)
}
/// Secondary text: captions, hints, units.
pub fn dim() -> Color32 {
    pick(DARK.get(), 0x8b9199, 0x5e646c)
}
/// Disabled text.
pub fn disabled() -> Color32 {
    pick(DARK.get(), 0x52575e, 0xa3a8af)
}
/// The line under the header bar.
#[allow(dead_code)]
pub fn header_line() -> Color32 {
    base()
}
/// Error text.
pub fn error() -> Color32 {
    pick(DARK.get(), 0xff6b6b, 0xc62828)
}
/// Warning text.
pub fn warning() -> Color32 {
    pick(DARK.get(), 0xf0b429, 0x9a6700)
}

/// A list row under the pointer.
pub fn row_hover() -> Color32 {
    if DARK.get() {
        Color32::from_white_alpha(10)
    } else {
        Color32::from_black_alpha(12)
    }
}
/// The picked row of a list.
pub fn row_selected() -> Color32 {
    accent().gamma_multiply(if DARK.get() { 0.38 } else { 0.24 })
}
/// The well a list sits in.
pub fn well() -> Color32 {
    pick(DARK.get(), 0x16171a, 0xf4f5f7)
}
/// A section header's fill.
pub fn section() -> Color32 {
    pick(DARK.get(), 0x26282d, 0xe1e4e8)
}
/// A section header's fill under the pointer.
pub fn section_hover() -> Color32 {
    pick(DARK.get(), 0x2d3035, 0xd7dbe0)
}
/// The colour a vector's component is marked with: X red, Y green, Z blue, muted so
/// the marks guide the eye without shouting.
pub fn axis(index: usize) -> Color32 {
    let dark = DARK.get();
    match index {
        0 => pick(dark, 0xe06a6f, 0xc0392b),
        1 => pick(dark, 0x8cc25e, 0x3f8f1f),
        2 => pick(dark, 0x6a9df0, 0x2e6bd0),
        _ => dim(),
    }
}

/// Colours that tell overlaid shapes apart in a preview (the corner join's two brushes):
/// blue then orange, deep enough to read on the light scheme's wells.
pub fn series(index: usize) -> Color32 {
    let dark = DARK.get();
    match index % 2 {
        0 => pick(dark, 0x4db2ff, 0x1f6fd1),
        _ => pick(dark, 0xffa640, 0xc8620a),
    }
}

/// The height of an ordinary control: buttons, entries, spins, drop-downs. On Android
/// Material's 40, a fingertip's.
pub const CONTROL_HEIGHT: f32 = if TOUCH_SIZES { 40.0 } else { 28.0 };

/// Body text size in points, before the user's interface scale; Material's 14 on Android.
const BASE_TEXT_SIZE: f32 = if TOUCH_SIZES { 14.0 } else { 13.0 };

/// Whether the controls take a touch screen's sizes rather than a desktop's.
const TOUCH_SIZES: bool = cfg!(target_os = "android");

/// The body text size in use, as f32 bits. The desktop's text scale (Xft.dpi) can differ
/// from the whole-number window scale: text follows the former and widgets the latter,
/// and [`fit_scale`] sets both.
static TEXT_SIZE: std::sync::atomic::AtomicU32 =
    std::sync::atomic::AtomicU32::new(BASE_TEXT_SIZE.to_bits());

/// The body text size in points.
pub fn text_size() -> f32 {
    f32::from_bits(TEXT_SIZE.load(std::sync::atomic::Ordering::Relaxed))
}
/// The title bar's height.
pub const HEADER_HEIGHT: f32 = 36.0;
/// The tool strip's height.
pub const TOOLBAR_HEIGHT: f32 = 34.0;
/// A tool button's side.
pub const TOOL_BUTTON: f32 = 28.0;
/// A tool icon's side.
pub const TOOL_ICON: f32 = 22.0;
/// Rounding of an ordinary button: rounder on macOS, as its own controls are.
pub fn radius() -> u8 {
    look::Metrics::of(look()).radius
}

/// The bold family, registered when the system has Ubuntu Bold.
pub fn bold() -> FontFamily {
    FontFamily::Name("bold".into())
}

/// Body text.
pub fn body() -> FontId {
    FontId::new(text_size(), FontFamily::Proportional)
}

/// The family holding the Phosphor icons.
const ICON_FAMILY: &str = "phosphor";

/// The font for an icon from `egui_phosphor::regular` set beside body text.
pub fn icon() -> FontId {
    FontId::new(
        (text_size() * 1.15).round(),
        FontFamily::Name(ICON_FAMILY.into()),
    )
}

/// Bold body text.
pub fn body_bold() -> FontId {
    FontId::new(text_size(), bold())
}

/// Small bold text: pills, badges and captions.
pub fn small_bold() -> FontId {
    FontId::new(text_size() - 1.0, bold())
}

/// The range of the "Interface scale (%)" preference.
pub const INTERFACE_SCALE: (f32, f32) = (50.0, 300.0);

/// Sets the text size and scales egui by the "Interface scale (%)" preference on top of
/// the display's own scale; at 100 % every widget and every glyph lands on whole device
/// pixels. Returns whether anything changed. Call every frame: the native scale is known
/// only after the first frames and changes when the window moves to another monitor.
pub fn fit_scale(ctx: &egui::Context) -> bool {
    if ctx.native_pixels_per_point().is_none() {
        return false;
    }
    // While the scale slider is held, the window would grow under the pointer; the new
    // scale waits for the release.
    if ctx.input(|i| i.pointer.any_down()) {
        return false;
    }
    let zoom = editor_core::preferences::pref(|p| p.interface_scale)
        .clamp(INTERFACE_SCALE.0, INTERFACE_SCALE.1)
        / 100.0;
    let text = BASE_TEXT_SIZE;
    if (ctx.zoom_factor() - zoom).abs() < 1e-4 && (text_size() - text).abs() < 1e-4 {
        return false;
    }
    ctx.set_zoom_factor(zoom);
    TEXT_SIZE.store(text.to_bits(), std::sync::atomic::Ordering::Relaxed);
    keep_theme(ctx);
    ctx.all_styles_mut(apply_text_styles);
    true
}

fn apply_text_styles(style: &mut egui::Style) {
    use egui::TextStyle;
    let size = text_size();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(size, bold())),
        (TextStyle::Body, FontId::new(size, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(size, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(size * 0.75, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(size * 0.85, FontFamily::Monospace),
        ),
    ]
    .into();
}

/// The names the system's faces take in egui's font table.
const SYSTEM_REGULAR: &str = "system";
const SYSTEM_SEMIBOLD: &str = "system-semibold";
const SYSTEM_MONO: &str = "system-mono";

/// Loads Inter, bundled so the editor reads the same on every system, and sets egui's
/// visuals to the editor's palette. On macOS the system's own San Francisco comes first,
/// with Inter behind it, and the accent is the one chosen in System Settings.
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "inter".into(),
        egui::FontData::from_static(include_bytes!("../fonts/Inter-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "inter-semibold".into(),
        egui::FontData::from_static(include_bytes!("../fonts/Inter-SemiBold.ttf")).into(),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "inter".into());
    let mut bold_first = vec!["inter-semibold".to_string()];
    // A developer's input script clicks at coordinates laid out in Inter, the same on
    // every system, so under one the text keeps Inter's widths, unless
    // HERON_BRUSH_SCRIPT_FONTS=system asks to look at the system's.
    let scripted = std::env::var_os("HERON_BRUSH_SCRIPT").is_some()
        && std::env::var("HERON_BRUSH_SCRIPT_FONTS").as_deref() != Ok("system");
    keep_theme(ctx);
    // The system's own face (San Francisco, Segoe UI Variable) first, Inter behind it.
    if let Some(system) = look::system_fonts(look()).filter(|_| !scripted) {
        fonts
            .font_data
            .insert(SYSTEM_REGULAR.into(), system.regular.into());
        fonts
            .font_data
            .insert(SYSTEM_SEMIBOLD.into(), system.semibold.into());
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, SYSTEM_REGULAR.into());
        bold_first.insert(0, SYSTEM_SEMIBOLD.into());
        if let Some(mono) = system.mono {
            fonts.font_data.insert(SYSTEM_MONO.into(), mono.into());
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(0, SYSTEM_MONO.into());
        }
    }
    // Inter has no Chinese, Japanese or Korean, which the language list and those
    // translations need; the system's own fonts fill in, and in the browser, which lends
    // none, the ones the web build carries.
    // Inter covers Latin, Greek, Cyrillic and Vietnamese itself.
    for (name, data) in system_fallback_fonts() {
        fonts.font_data.insert(name.clone(), data.into());
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push(name.clone());
        }
    }
    let mut bold_family = bold_first;
    bold_family.extend(
        fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    fonts.families.insert(bold(), bold_family);
    // Phosphor's icons in a family of their own, Phosphor first: the icons sit in the
    // private use area, where Inter has glyphs of its own that would otherwise win. The
    // text fonts after it give egui its replacement glyph.
    fonts.font_data.insert(
        ICON_FAMILY.into(),
        egui::FontData::from_static(egui_phosphor::Variant::Regular.font_bytes()).into(),
    );
    let mut icon_family = vec![ICON_FAMILY.to_string()];
    icon_family.extend(
        fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    fonts
        .families
        .insert(FontFamily::Name(ICON_FAMILY.into()), icon_family);
    ctx.set_fonts(fonts);
    ctx.all_styles_mut(|style| {
        apply_text_styles(style);
        apply_visuals(style);
    });
    // The interface scale is a preference that `fit_scale` applies every frame, so egui's
    // own Ctrl+Plus and Ctrl+Minus zoom would be undone at once.
    ctx.options_mut(|options| options.zoom_with_keyboard = false);
}

/// Candidate system fonts for the scripts Inter lacks, by platform, as (path, face index),
/// each group with the language it is drawn for: Simplified Chinese, then Japanese, Korean
/// and Traditional Chinese. The first file found for each script wins. Han characters are
/// shared between the four, so the group of the language the editor speaks goes first and
/// draws them in that language's forms; the others only fill in what it lacks.
const FALLBACK_FONTS: &[(Language, &[(&str, u32)])] = &[
    // Simplified Chinese (and the kana GB 2312 carries).
    (
        Language::Chinese,
        &[
            ("/System/Library/Fonts/Hiragino Sans GB.ttc", 0),
            // Face 1 is Heiti SC; face 0 is TC.
            ("/System/Library/Fonts/STHeiti Light.ttc", 1),
            ("C:\\Windows\\Fonts\\msyh.ttc", 0),
            ("C:\\Windows\\Fonts\\simsun.ttc", 0),
            ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 2),
            ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 2),
            (
                "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
                2,
            ),
            // Android.
            ("/system/fonts/NotoSansCJK-Regular.ttc", 2),
            ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc", 0),
            (
                "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
                0,
            ),
            (
                "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
                0,
            ),
            // The web build, which fetches them for these languages, and otherwise has only
            // the face for the language's own name (scripts/pack_web_bundle.py).
            ("/heron-brush/install/fonts/NotoSansSC-Regular.otf", 0),
            (
                "/heron-brush/install/fonts/picker/NotoSansSC-Regular.otf",
                0,
            ),
        ],
    ),
    (
        Language::Japanese,
        &[
            ("/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc", 0),
            ("C:\\Windows\\Fonts\\YuGothR.ttc", 0),
            ("C:\\Windows\\Fonts\\meiryo.ttc", 0),
            ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 0),
            ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 0),
            (
                "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
                0,
            ),
            // Android.
            ("/system/fonts/NotoSansCJK-Regular.ttc", 0),
            // The web build.
            ("/heron-brush/install/fonts/NotoSansJP-Regular.otf", 0),
            (
                "/heron-brush/install/fonts/picker/NotoSansJP-Regular.otf",
                0,
            ),
        ],
    ),
    (
        Language::Korean,
        &[
            ("/System/Library/Fonts/AppleSDGothicNeo.ttc", 0),
            ("C:\\Windows\\Fonts\\malgun.ttf", 0),
            ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 1),
            ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 1),
            (
                "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
                1,
            ),
            // Android.
            ("/system/fonts/NotoSansCJK-Regular.ttc", 1),
            ("/usr/share/fonts/truetype/nanum/NanumGothic.ttf", 0),
            // The web build.
            ("/heron-brush/install/fonts/NotoSansKR-Regular.otf", 0),
            (
                "/heron-brush/install/fonts/picker/NotoSansKR-Regular.otf",
                0,
            ),
        ],
    ),
    // Traditional Chinese, with the characters Simplified has no use for.
    (
        Language::ChineseTraditional,
        &[
            // Face 0 is Heiti TC.
            ("/System/Library/Fonts/STHeiti Light.ttc", 0),
            ("/System/Library/Fonts/STHeiti Medium.ttc", 0),
            ("C:\\Windows\\Fonts\\msjh.ttc", 0),
            ("C:\\Windows\\Fonts\\mingliu.ttc", 0),
            ("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc", 3),
            ("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc", 3),
            (
                "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
                3,
            ),
            // Android.
            ("/system/fonts/NotoSansCJK-Regular.ttc", 3),
            (
                "/usr/share/fonts/opentype/noto/NotoSansCJKtc-Regular.otf",
                0,
            ),
            ("/usr/share/fonts/noto-cjk/NotoSansCJKtc-Regular.otf", 0),
            (
                "/usr/share/fonts/google-noto-cjk/NotoSansCJKtc-Regular.otf",
                0,
            ),
            // The web build.
            ("/heron-brush/install/fonts/NotoSansTC-Regular.otf", 0),
            (
                "/heron-brush/install/fonts/picker/NotoSansTC-Regular.otf",
                0,
            ),
        ],
    ),
];

/// The groups of [`FALLBACK_FONTS`], that of the language the editor speaks first.
fn fallback_order(speaking: Language) -> Vec<&'static [(&'static str, u32)]> {
    let (first, rest): (Vec<_>, Vec<_>) = FALLBACK_FONTS
        .iter()
        .partition(|(language, _)| *language == speaking);
    first
        .into_iter()
        .chain(rest)
        .map(|&(_, fonts)| fonts)
        .collect()
}

/// The system fonts found for [`FALLBACK_FONTS`], named for egui's font table. A file
/// that several scripts share is loaded once.
fn system_fallback_fonts() -> Vec<(String, egui::FontData)> {
    let speaking = editor_core::preferences::pref(|p| p.language).resolved();
    let mut found: Vec<(String, egui::FontData)> = Vec::new();
    for script in fallback_order(speaking) {
        for &(path, index) in script {
            let name = format!("system:{path}#{index}");
            if found.iter().any(|(n, _)| *n == name) {
                break;
            }
            if let Ok(bytes) = editor_core::files::read(std::path::Path::new(path)) {
                let mut data = egui::FontData::from_owned(bytes);
                data.index = index;
                found.push((name, data));
                break;
            }
        }
    }
    found
}

/// The palette, corner radii and spacing, for whichever scheme is current.
fn apply_visuals(style: &mut egui::Style) {
    let dark = DARK.get();
    let visuals = &mut style.visuals;
    *visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.panel_fill = bg();
    visuals.window_fill = surface();
    visuals.faint_bg_color = surface();
    visuals.extreme_bg_color = entry();
    visuals.code_bg_color = base();
    visuals.override_text_color = Some(fg());
    visuals.weak_text_color = Some(dim());
    visuals.hyperlink_color = accent_text();
    visuals.error_fg_color = error();
    visuals.warn_fg_color = warning();
    visuals.selection.bg_fill = accent().gamma_multiply(if dark { 0.55 } else { 0.35 });
    visuals.selection.stroke = Stroke::new(1.0, accent_hover());
    // macOS draws its windows, sheets and menus rounder, with a wider and softer shadow;
    // WinUI rounds its overlays by 8.
    let mac = mac();
    visuals.window_corner_radius = CornerRadius::same(if mac {
        10
    } else if win() {
        8
    } else {
        6
    });
    visuals.menu_corner_radius = CornerRadius::same(if mac || win() { 8 } else { 6 });
    visuals.window_stroke = Stroke::new(1.0, border());
    visuals.window_shadow = if mac {
        egui::Shadow {
            offset: [0, 10],
            blur: 32,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 120 } else { 45 }),
        }
    } else {
        egui::Shadow {
            offset: [0, 6],
            blur: 18,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 110 } else { 40 }),
        }
    };
    if win() {
        visuals.window_shadow = egui::Shadow {
            offset: [0, 8],
            blur: 32,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 110 } else { 50 }),
        };
    }
    visuals.popup_shadow = if mac || win() {
        egui::Shadow {
            offset: [0, 6],
            blur: 20,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 110 } else { 40 }),
        }
    } else {
        egui::Shadow {
            offset: [0, 4],
            blur: 12,
            spread: 0,
            color: Color32::from_black_alpha(if dark { 100 } else { 35 }),
        }
    };
    visuals.slider_trailing_fill = true;
    visuals.striped = false;
    let radius = CornerRadius::same(radius());
    let w = &mut visuals.widgets;
    w.noninteractive.bg_fill = surface();
    w.noninteractive.weak_bg_fill = surface();
    w.noninteractive.bg_stroke = Stroke::new(1.0, border());
    w.noninteractive.fg_stroke = Stroke::new(1.0, fg());
    w.noninteractive.corner_radius = radius;
    for (state, fill) in [
        (&mut w.inactive, button_fill()),
        (&mut w.hovered, button_hover()),
        (&mut w.active, button_pressed()),
        (&mut w.open, button_hover()),
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.corner_radius = radius;
        state.expansion = 0.0;
        state.bg_stroke = Stroke::new(1.0, border());
        state.fg_stroke = Stroke::new(1.0, fg());
    }
    w.hovered.bg_stroke = Stroke::new(1.0, grid());
    w.active.bg_stroke = Stroke::new(1.0, accent());
    w.inactive.fg_stroke = Stroke::new(1.0, fg());
    if mac || win() {
        // egui's own controls (text edits, selectable rows) in the same manner: a hairline
        // edge that does not light up on hover, and the accent round a focused entry (a
        // soft band on macOS).
        for state in [&mut w.inactive, &mut w.hovered] {
            state.bg_stroke = Stroke::new(1.0, control_border());
        }
        visuals.selection.stroke = if mac {
            Stroke::new(2.0, accent().gamma_multiply(0.7))
        } else {
            Stroke::new(1.0, accent())
        };
    }
    let spacing = &mut style.spacing;
    spacing.item_spacing = egui::vec2(6.0, 4.0);
    spacing.button_padding = egui::vec2(10.0, 4.0);
    spacing.interact_size = egui::vec2(28.0, CONTROL_HEIGHT);
    spacing.window_margin = egui::Margin::same(10);
    spacing.menu_margin = egui::Margin::same(4);
    spacing.indent = 14.0;
    spacing.icon_width = 14.0;
    spacing.icon_width_inner = 8.0;
    spacing.icon_spacing = 6.0;
    spacing.combo_height = 320.0;
    spacing.scroll = egui::style::ScrollStyle::thin();
    style.interaction.selectable_labels = false;
}

/// A flat button the height of the other controls.
pub fn button(ui: &mut Ui, label: &str, width: f32, enabled: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, CONTROL_HEIGHT),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    paint_button(ui, rect, &response, label, enabled, false);
    response
}

pub use egui_native_look::focus_ring;

/// Paints a button in `rect`. `accent` is a toggle that is switched on, or the default
/// action of a dialog.
pub fn paint_button(
    ui: &Ui,
    rect: Rect,
    response: &Response,
    label: &str,
    enabled: bool,
    accent: bool,
) {
    look::paint_button(ui.painter(), rect, response, label, body(), enabled, accent);
}

/// A section heading, as the inspector's panels start.
pub fn heading(ui: &mut Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(egui::RichText::new(text).font(body_bold()).color(fg()));
}

/// Applies the desktop or explicit colour scheme when it changes.
pub fn sync(ctx: &egui::Context) {
    use editor_core::preferences::{self, ColorScheme};
    let dark = match preferences::pref(|p| p.color_scheme) {
        ColorScheme::Dark => true,
        ColorScheme::Light => false,
        ColorScheme::System => ctx.system_theme() != Some(egui::Theme::Light),
    };
    static FOLLOWED: std::sync::Once = std::sync::Once::new();
    let changed = DARK.replace(dark) != dark;
    // The accent chosen in the system's settings while the editor runs.
    look::follow_system(ctx);
    let followed = look::theme(ctx);
    let accent_changed = THEME.with(|t| {
        let mut kept = t.borrow_mut();
        let moved = kept
            .as_ref()
            .is_some_and(|kept| kept.accent != followed.accent);
        if moved {
            *kept = Some(followed);
        }
        moved
    });
    refresh_colors();
    if accent_changed && !changed {
        ctx.all_styles_mut(apply_visuals);
        ctx.request_repaint();
    }
    if changed || !FOLLOWED.is_completed() {
        FOLLOWED.call_once(|| {});
        preferences::follow_color_scheme(dark);
        ctx.set_theme(if dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        ctx.all_styles_mut(apply_visuals);
        ctx.request_repaint();
    }
}

/// Puts this thread's interface in the light or the dark palette without consulting or
/// changing the preferences, for headless tests.
#[cfg(test)]
pub fn use_scheme(ctx: &egui::Context, dark: bool) {
    DARK.set(dark);
    refresh_colors();
    ctx.set_theme(if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
    ctx.all_styles_mut(apply_visuals);
}

#[cfg(test)]
mod fallback_tests {
    use super::*;

    #[test]
    fn the_language_spoken_draws_han_in_its_own_forms() {
        let first = |language| fallback_order(language)[0];
        let group = |language| {
            FALLBACK_FONTS
                .iter()
                .find(|(l, _)| *l == language)
                .unwrap()
                .1
        };
        assert_eq!(
            first(Language::ChineseTraditional),
            group(Language::ChineseTraditional)
        );
        assert_eq!(first(Language::Japanese), group(Language::Japanese));
        // Everyone else keeps Simplified Chinese first, as before.
        assert_eq!(first(Language::English), group(Language::Chinese));
        assert_eq!(fallback_order(Language::Czech).len(), FALLBACK_FONTS.len());
    }
}

pub use egui_native_look::{galley_top, text};
