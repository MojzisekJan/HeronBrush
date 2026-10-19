//! What a plugin says about itself: `plugin.toml`.
//!
//! An optional `plugin.toml` supplies display metadata and the required editor API version. An
//! incompatible [`API_VERSION`] prevents execution; a missing manifest uses the current API.
//!
//! A manifest may also declare the **settings** a plugin wants asked for, one
//! `[[setting]]` block each. The Plugins window draws a control per setting under the
//! list and hands the answers to the script, so a plugin with something to tune -- how
//! many copies, which colour, how far -- is set from the window rather than by editing
//! the script. See [`Setting`].
//!
//! The format is a `key = "value"` subset of TOML: values are strings, numbers or
//! booleans, and the supported arrays are `[[setting]]` and `[[action]]`. No multi-line strings, no
//! nesting. That is the whole of what a manifest needs, and it saves a dependency for a
//! file this small. A `[section]` header the reader does not know is skipped along with
//! the keys under it, so a manifest that grows one later still reads its top keys here.

use std::path::Path;

/// The plugin API this build of the editor offers. Raise it when a plugin function
/// changes shape or goes away -- adding one does not need it.
pub const API_VERSION: u32 = 1;

/// What a `plugin.toml` says. Every field is optional; an absent one falls back to what
/// the script itself can tell us.
#[derive(Debug, Default, PartialEq)]
pub struct Manifest {
    /// The name to show, instead of the file or folder name.
    pub name: Option<String>,
    /// A one-line summary, instead of the script's first `//` line.
    pub description: Option<String>,
    /// The plugin's own version, shown beside its name. Never interpreted.
    pub version: Option<String>,
    /// Who wrote it. Shown, never interpreted.
    pub author: Option<String>,
    /// The editor API it was written against. `None` means "whatever is current".
    pub api: Option<u32>,
    /// What the plugin wants asked before it runs, in the order it declared them.
    pub settings: Vec<Setting>,
    /// Explicit user-triggered buttons contributed by the plugin.
    pub actions: Vec<Action>,
    /// Optional read-only action refreshed when the visible inspector changes.
    pub panel_action: Option<String>,
}

/// A named button. The script receives its identifier through `action()`.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Action {
    pub id: String,
    pub label: String,
    pub description: String,
}

/// One thing a plugin wants asked before it runs: `[[setting]]` in the manifest.
///
/// ```toml
/// [[setting]]
/// key = "copies"          # what the script asks for: setting("copies")
/// label = "Copies"        # what the person reads beside the control
/// kind = "number"         # number | text | check | choice
/// default = 5
/// min = 1                 # numbers only
/// max = 50
/// choices = "up, down"    # a choice's options, comma separated
/// ```
///
/// A setting with no `key` is dropped: the script has no way to ask for it. Everything
/// else falls back -- an unknown `kind` is text, a missing `label` is the key itself, a
/// missing `default` is empty or zero or off.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Setting {
    /// The name the script asks for.
    pub key: String,
    /// What the person reads beside the control.
    pub label: String,
    /// Which control to draw.
    pub kind: SettingKind,
    /// What it starts at, as written in the manifest.
    pub default: String,
    /// The range a number is held to. Absent means unbounded either way.
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// What a `choice` may be, in order.
    pub choices: Vec<String>,
}

/// The kinds of control a setting can ask for. Deliberately few: a plugin declares what
/// it needs, not how the window looks.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum SettingKind {
    /// A spin button. The script sees a float.
    Number,
    /// A line of text. The script sees a string.
    #[default]
    Text,
    /// A switch. The script sees `true` or `false`.
    Check,
    /// One of a list. The script sees the chosen string.
    Choice,
}

impl Setting {
    /// The default as a number, for a [`SettingKind::Number`].
    pub fn default_number(&self) -> f64 {
        self.default.trim().parse().unwrap_or(0.0)
    }

    /// The default as a switch position, for a [`SettingKind::Check`].
    pub fn default_check(&self) -> bool {
        matches!(self.default.trim(), "true" | "yes" | "1" | "on")
    }
}

impl Manifest {
    /// Reads the manifest a plugin folder holds, or [`None`] when it has none. A
    /// manifest that cannot be read or parsed is the same as none -- a broken manifest
    /// must not hide a working script.
    pub fn beside(script: &Path) -> Option<Manifest> {
        let file = script.parent()?.join("plugin.toml");
        let text = crate::files::read_to_string(&file).ok()?;
        Some(Manifest::parse(&text))
    }

    /// Reads a manifest from the text of a `plugin.toml`.
    pub fn parse(text: &str) -> Manifest {
        let mut manifest = Manifest::default();
        // Which block the keys belong to: the top of the file until a header says
        // otherwise, and a fresh setting each time `[[setting]]` comes round again.
        let mut setting: Option<Setting> = None;
        let mut action: Option<Action> = None;
        let mut in_a_block_we_do_not_know = false;

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                if let Some(done) = setting.take() {
                    manifest.take_setting(done);
                }
                if let Some(done) = action.take() {
                    manifest.take_action(done);
                }
                in_a_block_we_do_not_know = false;
                let header = line.trim_matches(|c| c == '[' || c == ']').trim();
                match header {
                    "setting" => setting = Some(Setting::default()),
                    "action" => action = Some(Action::default()),
                    _ => in_a_block_we_do_not_know = true,
                }
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), unquote(value.trim()));
            if let Some(action) = action.as_mut() {
                match key {
                    "id" => action.id = value,
                    "label" => action.label = value,
                    "description" => action.description = value,
                    _ => {}
                }
                continue;
            }
            if let Some(setting) = setting.as_mut() {
                match key {
                    "key" => setting.key = value,
                    "label" => setting.label = value,
                    "kind" => setting.kind = SettingKind::named(&value),
                    "default" => setting.default = value,
                    "min" => setting.min = value.parse().ok(),
                    "max" => setting.max = value.parse().ok(),
                    "choices" => {
                        setting.choices = value
                            .split(',')
                            .map(str::trim)
                            .filter(|choice| !choice.is_empty())
                            .map(str::to_string)
                            .collect();
                    }
                    _ => {}
                }
                continue;
            }
            if in_a_block_we_do_not_know {
                continue;
            }
            match key {
                "name" => manifest.name = Some(value),
                "description" => manifest.description = Some(value),
                "version" => manifest.version = Some(value),
                "author" => manifest.author = Some(value),
                "api" => manifest.api = value.parse().ok(),
                "panel_action" if value.len() <= 64 && !value.is_empty() => manifest.panel_action = Some(value),
                _ => {}
            }
        }
        if let Some(done) = setting.take() {
            manifest.take_setting(done);
        }
        if let Some(done) = action.take() {
            manifest.take_action(done);
        }
        manifest
    }

    fn take_action(&mut self, mut action: Action) {
        if self.actions.len() >= 16
            || action.id.is_empty()
            || action.id.len() > 64
            || !action
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
            || self.actions.iter().any(|a| a.id == action.id)
        {
            return;
        }
        if action.label.is_empty() {
            action.label = action.id.clone();
        }
        action.label = action.label.chars().take(120).collect();
        action.description = action.description.chars().take(1024).collect();
        self.actions.push(action);
    }

    /// Adds a finished `[[setting]]` block, filling in what it left out. One with no key
    /// is dropped: a script has no way to ask for it.
    fn take_setting(&mut self, mut setting: Setting) {
        if setting.key.is_empty() {
            return;
        }
        if setting.label.is_empty() {
            setting.label = setting.key.clone();
        }
        // A choice with no default starts on its first option, which is the one a person
        // would expect to see in the box before touching it.
        if setting.kind == SettingKind::Choice && setting.default.is_empty() {
            if let Some(first) = setting.choices.first() {
                setting.default = first.clone();
            }
        }
        self.settings.push(setting);
    }

    /// Whether this build can run the plugin: it asked for the API we offer, or asked
    /// for nothing.
    pub fn api_matches(&self) -> bool {
        self.api.is_none_or(|asked| asked == API_VERSION)
    }

    /// Every setting at its declared default, which is what a headless run answers with
    /// and what the window's controls start on.
    pub fn defaults(&self) -> Vec<(String, SettingValue)> {
        self.settings
            .iter()
            .map(|setting| (setting.key.clone(), SettingValue::of(setting)))
            .collect()
    }
}

/// What a setting is answered with. The script sees a number, a string or a boolean.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum SettingValue {
    Number(f64),
    Text(String),
    Check(bool),
}

impl SettingValue {
    /// A setting sitting at its default.
    fn of(setting: &Setting) -> SettingValue {
        match setting.kind {
            SettingKind::Number => SettingValue::Number(setting.default_number()),
            SettingKind::Check => SettingValue::Check(setting.default_check()),
            SettingKind::Text | SettingKind::Choice => SettingValue::Text(setting.default.clone()),
        }
    }
}

impl SettingKind {
    /// The kind a manifest names, or text for one this build does not know -- a plugin
    /// asking for a control that does not exist still runs, it just types its answer.
    fn named(name: &str) -> SettingKind {
        match name.trim() {
            "number" => SettingKind::Number,
            "check" | "bool" | "boolean" => SettingKind::Check,
            "choice" => SettingKind::Choice,
            _ => SettingKind::Text,
        }
    }
}

/// The text of a TOML string value: the quotes come off, a trailing comment goes with
/// them, and a bare value is taken as it stands.
fn unquote(value: &str) -> String {
    let value = value.trim();
    for quote in ['"', '\''] {
        if let Some(rest) = value.strip_prefix(quote) {
            // Everything up to the closing quote. A value with no closing quote runs to
            // the end of the line, which is what a person meant by writing it.
            return match rest.find(quote) {
                Some(end) => rest[..end].to_string(),
                None => rest.to_string(),
            };
        }
    }
    match value.split_once('#') {
        Some((before, _)) => before.trim().to_string(),
        None => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_reads_its_five_keys() {
        let manifest = Manifest::parse(
            r#"
            # A plugin of mine.
            name = "Bevel every edge"
            description = "Rounds the selection's edges."
            version = "2.1"
            author = 'A Mapper'
            api = 1
            "#,
        );
        assert_eq!(manifest.name.as_deref(), Some("Bevel every edge"));
        assert_eq!(
            manifest.description.as_deref(),
            Some("Rounds the selection's edges.")
        );
        assert_eq!(manifest.version.as_deref(), Some("2.1"));
        assert_eq!(manifest.author.as_deref(), Some("A Mapper"));
        assert_eq!(manifest.api, Some(1));
        assert!(manifest.api_matches());
    }

    #[test]
    fn an_empty_manifest_asks_for_nothing_and_runs() {
        let manifest = Manifest::parse("");
        assert_eq!(manifest, Manifest::default());
        assert!(manifest.api_matches());
    }

    #[test]
    fn a_plugin_written_for_another_api_is_refused() {
        assert!(!Manifest::parse("api = 2").api_matches());
        assert!(!Manifest::parse("api = 0").api_matches());
        // An api that is not a number is no api at all, not a refusal: the plugin is
        // taken as written for the current one.
        assert!(Manifest::parse("api = \"one\"").api_matches());
    }

    #[test]
    fn a_comment_after_a_bare_value_is_not_part_of_it() {
        assert_eq!(Manifest::parse("api = 1 # the first").api, Some(1));
    }

    #[test]
    fn settings_are_read_in_order_with_their_kinds_and_bounds() {
        let manifest = Manifest::parse(
            r#"
            name = "Stack"

            [[setting]]
            key = "copies"
            label = "Copies"
            kind = "number"
            default = 5
            min = 1
            max = 200

            [[setting]]
            key = "direction"
            kind = "choice"
            choices = "up, down"

            [[setting]]
            key = "align"
            kind = "check"
            default = true
            "#,
        );
        assert_eq!(manifest.name.as_deref(), Some("Stack"));
        let keys: Vec<&str> = manifest.settings.iter().map(|s| s.key.as_str()).collect();
        assert_eq!(
            keys,
            ["copies", "direction", "align"],
            "declared order is kept"
        );

        let copies = &manifest.settings[0];
        assert_eq!(copies.kind, SettingKind::Number);
        assert_eq!(copies.default_number(), 5.0);
        assert_eq!((copies.min, copies.max), (Some(1.0), Some(200.0)));

        let direction = &manifest.settings[1];
        assert_eq!(direction.choices, ["up", "down"]);
        // No label of its own, so it wears its key; no default, so it starts on the
        // first choice.
        assert_eq!(direction.label, "direction");
        assert_eq!(direction.default, "up");

        assert!(manifest.settings[2].default_check());
    }

    #[test]
    fn a_setting_with_no_key_is_dropped_and_an_unknown_kind_is_text() {
        let manifest = Manifest::parse(
            "[[setting]]\nlabel = \"Nameless\"\n\n[[setting]]\nkey = \"k\"\nkind = \"colour\"\n",
        );
        assert_eq!(manifest.settings.len(), 1, "the keyless one is gone");
        assert_eq!(manifest.settings[0].kind, SettingKind::Text);
    }

    #[test]
    fn a_block_the_reader_does_not_know_takes_its_keys_with_it() {
        // The top keys are read; `name` under [panel] must not overwrite the real one,
        // and no setting comes out of a block that is not [[setting]].
        let manifest =
            Manifest::parse("name = \"Mine\"\n[panel]\nname = \"Not this\"\nkey = \"nope\"\n");
        assert_eq!(manifest.name.as_deref(), Some("Mine"));
        assert!(manifest.settings.is_empty());
    }

    #[test]
    fn the_defaults_are_what_a_headless_run_answers_with() {
        let manifest = Manifest::parse(
            "[[setting]]\nkey = \"n\"\nkind = \"number\"\ndefault = 7\n\n\
             [[setting]]\nkey = \"t\"\ndefault = \"hello\"\n",
        );
        assert_eq!(
            manifest.defaults(),
            vec![
                ("n".to_string(), SettingValue::Number(7.0)),
                ("t".to_string(), SettingValue::Text("hello".to_string())),
            ]
        );
    }

    #[test]
    fn a_manifest_beside_a_script_is_found_and_a_missing_one_is_not_an_error() {
        let dir = std::env::temp_dir().join(format!("bb-manifest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a temp dir");
        let script = dir.join("main.rhai");
        std::fs::write(&script, "say(\"hello\");").expect("the script");
        assert_eq!(Manifest::beside(&script), None);

        std::fs::write(dir.join("plugin.toml"), "name = \"Named\"\napi = 1\n")
            .expect("the manifest");
        let manifest = Manifest::beside(&script).expect("a manifest");
        assert_eq!(manifest.name.as_deref(), Some("Named"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod action_tests {
    use super::*;
    #[test]
    fn actions_and_settings_can_alternate_after_unknown_sections() {
        let manifest = Manifest::parse(
            r#"
name = "Tools"
[future]
ignored = true
[[action]]
id = "inspect"
label = "Inspect"
[[setting]]
key = "step"
kind = "number"
default = 16
[[action]]
id = "raise"
[[action]]
id = "raise"
[[action]]
id = "bad/id"
"#,
        );
        assert_eq!(manifest.actions.len(), 2);
        assert_eq!(manifest.actions[1].label, "raise");
        assert_eq!(manifest.settings.len(), 1);
        assert_eq!(manifest.settings[0].key, "step");
        assert!(manifest.api_matches());
    }
    #[test]
    fn contributions_are_bounded_and_empty_ids_are_not_buttons() {
        let text = (0..40)
            .map(|n| format!("[[action]]\nid = \"a{n}\"\n"))
            .collect::<String>();
        assert_eq!(Manifest::parse(&text).actions.len(), 16);
        assert!(Manifest::parse("[[action]]\nlabel = \"No ID\"")
            .actions
            .is_empty());
    }
}
