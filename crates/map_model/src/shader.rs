//! Reading a Quake 3 `.shader` script.
//!
//! Reads idTech 3 shader declarations from `scripts/*.shader`, resolving the named materials
//! referenced by map faces.
//!
//! What is kept is what an editor needs: the shader's own name, the image to show it as
//! (`qer_editorimage`), the light image, each stage's `map`, and the `surfaceparm`s -- the
//! last of these being how a map says a surface is a clip brush, a trigger or a hint,
//! which the tag rules already read from a game configuration. The rest of a shader
//! describes how the *engine* draws it, and is skipped.
//!
//! ```text
//! textures/base_wall/c_met5_2
//! {
//!     qer_editorimage textures/base_wall/c_met5_2.tga
//!     surfaceparm metalsteps
//!     {
//!         map $lightmap
//!         rgbGen identity
//!     }
//! }
//! ```

/// One shader from a `.shader` file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shader {
    /// The name the map file uses, such as `textures/base_wall/c_met5_2`.
    pub name: String,
    /// `qer_editorimage`: the image an editor is meant to show this as.
    pub editor_image: Option<String>,
    /// `q3map_lightimage`, which stands in when there is no editor image.
    pub light_image: Option<String>,
    /// `surfaceparm`s, in the order they are given: `nodraw`, `trans`, `sky` and the rest.
    pub surface_parms: Vec<String>,
    /// The `map` of each stage, in order, `$lightmap` and the like included.
    pub stage_maps: Vec<String>,
}

impl Shader {
    /// Whether the engine draws nothing for this surface, `surfaceparm nodraw`.
    pub fn is_nodraw(&self) -> bool {
        self.surface_parms.iter().any(|parm| parm == "nodraw")
    }

    /// Image candidates are ordered as editor preview, shader path, light image and stage maps.
    /// `$variables` identify engine resources rather than files.
    pub fn image_candidates(&self) -> Vec<&str> {
        let mut wanted: Vec<&str> = Vec::new();
        for candidate in [
            self.editor_image.as_deref(),
            Some(self.name.as_str()),
            self.light_image.as_deref(),
        ]
            .into_iter()
            .flatten()
            .chain(self.stage_maps.iter().map(String::as_str))
        {
            if candidate.starts_with('$') || candidate.is_empty() {
                continue;
            }
            if !wanted.contains(&candidate) {
                wanted.push(candidate);
            }
        }
        wanted
    }
}

/// Reads every shader in a `.shader` file.
///
/// A shader whose body will not parse is skipped and the reader moves on to the next: one
/// bad entry in a game's script directory should not cost the other hundreds. Nothing here
/// is an error, so there is no error type: what comes back is what could be read.
pub fn read(text: &str) -> Vec<Shader> {
    let mut shaders = Vec::new();
    let mut words = Words::new(text);

    while let Some(name) = words.next_word() {
        // The name, then a `{ ... }` body. Anything else is skipped until one turns up.
        if name == "{" || name == "}" {
            continue;
        }
        if words.peek_word() != Some("{") {
            continue;
        }
        words.next_word();

        // Accepts absolute-style shader names by removing the leading slash.
        let mut shader = Shader {
            name: name.trim_start_matches('/').to_string(),
            ..Shader::default()
        };
        read_body(&mut words, &mut shader);
        shaders.push(shader);
    }
    shaders
}

/// The `{ ... }` after a shader's name: its settings, and its stages.
fn read_body(words: &mut Words<'_>, shader: &mut Shader) {
    while let Some(word) = words.next_word() {
        match word {
            "}" => return,
            "{" => read_stage(words, shader),
            _ => {
                let key = word.to_ascii_lowercase();
                match key.as_str() {
                    "qer_editorimage" => shader.editor_image = words.next_on_line(),
                    "q3map_lightimage" => shader.light_image = words.next_on_line(),
                    "surfaceparm" => {
                        if let Some(parm) = words.next_on_line() {
                            shader.surface_parms.push(parm.to_ascii_lowercase());
                        }
                    }
                    _ => {}
                }
                words.skip_line();
            }
        }
    }
}

/// One stage of a shader: only its `map` is kept.
fn read_stage(words: &mut Words<'_>, shader: &mut Shader) {
    while let Some(word) = words.next_word() {
        match word {
            "}" => return,
            _ => {
                if word.eq_ignore_ascii_case("map") {
                    if let Some(map) = words.next_on_line() {
                        shader.stage_maps.push(map);
                    }
                }
                words.skip_line();
            }
        }
    }
}

/// A reader over the file's words that knows where the lines end, since a shader entry is
/// a key and its values on one line.
struct Words<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Words<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, at: 0 }
    }

    /// The next word, skipping whitespace, line ends and `//` comments. Braces are words
    /// of their own however they are written.
    fn next_word(&mut self) -> Option<&'a str> {
        let (word, next) = self.scan(self.at, true)?;
        self.at = next;
        Some(word)
    }

    fn peek_word(&mut self) -> Option<&'a str> {
        self.scan(self.at, true).map(|(word, _)| word)
    }

    /// The next word on the current line, which is how a key reads its value. Nothing if
    /// the line ends first.
    fn next_on_line(&mut self) -> Option<String> {
        let (word, next) = self.scan(self.at, false)?;
        self.at = next;
        Some(word.to_string())
    }

    /// Throws away whatever is left of the current line.
    fn skip_line(&mut self) {
        let rest = &self.text[self.at..];
        self.at += rest.find('\n').map_or(rest.len(), |at| at + 1);
    }

    /// Scans a word from `from`. With `across_lines` off it stops at the end of the line.
    /// Returns the word and where the scan left off.
    fn scan(&self, from: usize, across_lines: bool) -> Option<(&'a str, usize)> {
        let bytes = self.text.as_bytes();
        let mut at = from;
        loop {
            // Whitespace, and the line ends when the caller says they matter.
            while at < bytes.len() && (bytes[at] as char).is_whitespace() {
                if !across_lines && bytes[at] == b'\n' {
                    return None;
                }
                at += 1;
            }
            // A `//` comment runs to the end of the line.
            if bytes[at..].starts_with(b"//") {
                if !across_lines {
                    return None;
                }
                let rest = &self.text[at..];
                at += rest.find('\n').map_or(rest.len(), |end| end + 1);
                continue;
            }
            // A `/* ... */` comment, which the id shaders use for their headers.
            if bytes[at..].starts_with(b"/*") {
                let rest = &self.text[at + 2..];
                at += 2 + rest.find("*/")? + 2;
                continue;
            }
            break;
        }
        if at >= bytes.len() {
            return None;
        }
        // A brace is a word of its own, whatever it is written against.
        if bytes[at] == b'{' || bytes[at] == b'}' {
            return Some((&self.text[at..at + 1], at + 1));
        }
        let start = at;
        while at < bytes.len() {
            let c = bytes[at] as char;
            if c.is_whitespace() || c == '{' || c == '}' {
                break;
            }
            at += 1;
        }
        Some((&self.text[start..at], at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCRIPT: &str = r#"
// a comment
textures/base_wall/c_met5_2
{
    qer_editorimage textures/base_wall/c_met5_2.tga
    surfaceparm metalsteps
    q3map_lightimage textures/base_wall/c_met5_2_glow.tga
    {
        map $lightmap
        rgbGen identity
    }
    {
        map textures/base_wall/c_met5_2.tga
        blendFunc GL_DST_COLOR GL_ZERO
    }
}

textures/common/caulk
{
    qer_editorimage textures/common/caulk.tga
    surfaceparm nodraw
    surfaceparm nolightmap
}
"#;

    #[test]
    fn a_script_gives_one_shader_per_named_block() {
        let shaders = read(SCRIPT);
        assert_eq!(shaders.len(), 2);
        assert_eq!(shaders[0].name, "textures/base_wall/c_met5_2");
        assert_eq!(shaders[1].name, "textures/common/caulk");
    }

    #[test]
    fn the_editor_image_the_light_image_and_the_stages_are_kept() {
        let shader = &read(SCRIPT)[0];
        assert_eq!(
            shader.editor_image.as_deref(),
            Some("textures/base_wall/c_met5_2.tga")
        );
        assert_eq!(
            shader.light_image.as_deref(),
            Some("textures/base_wall/c_met5_2_glow.tga")
        );
        assert_eq!(
            shader.stage_maps,
            ["$lightmap", "textures/base_wall/c_met5_2.tga"]
        );
        assert_eq!(shader.surface_parms, ["metalsteps"]);
    }

    #[test]
    fn the_images_are_tried_in_upstreams_order_and_a_variable_is_not_one() {
        let shader = &read(SCRIPT)[0];
        assert_eq!(
            shader.image_candidates(),
            [
                "textures/base_wall/c_met5_2.tga",
                "textures/base_wall/c_met5_2",
                "textures/base_wall/c_met5_2_glow.tga",
            ],
            "the editor image first, then the shader's own name, then the light image"
        );
    }

    #[test]
    fn a_shader_the_engine_draws_nothing_for_says_so() {
        let shaders = read(SCRIPT);
        assert!(!shaders[0].is_nodraw());
        assert!(shaders[1].is_nodraw());
        assert_eq!(shaders[1].surface_parms, ["nodraw", "nolightmap"]);
    }

    #[test]
    fn an_absolute_shader_path_loses_its_leading_slash() {
        let shaders = read("/textures/x/y\n{\n}\n");
        assert_eq!(shaders[0].name, "textures/x/y");
    }

    #[test]
    fn a_brace_against_the_name_is_still_a_brace() {
        let shaders = read("textures/x/y {\nqer_editorimage a.tga\n}\n");
        assert_eq!(shaders.len(), 1);
        assert_eq!(shaders[0].editor_image.as_deref(), Some("a.tga"));
    }

    #[test]
    fn a_key_with_no_value_on_its_line_takes_nothing_from_the_next() {
        let shaders = read("textures/x/y\n{\nqer_editorimage\nsurfaceparm sky\n}\n");
        assert_eq!(shaders[0].editor_image, None);
        assert_eq!(shaders[0].surface_parms, ["sky"]);
    }

    #[test]
    fn a_block_comment_between_shaders_is_read_past() {
        let text = "/* the header\n   over two lines */\ntextures/a/b\n{\n}\ntextures/c/d\n{\n}\n";
        let shaders = read(text);
        assert_eq!(shaders.len(), 2);
        assert_eq!(shaders[0].name, "textures/a/b");
    }

    #[test]
    fn nothing_in_an_empty_file() {
        assert!(read("").is_empty());
        assert!(read("// just a comment\n").is_empty());
    }
}
