//! Explicit, discoverable editor preferences for live agents.
use super::*;
use crate::{i18n::Language, preferences};

/// The codes agents may choose: following the desktop and every complete language.
fn codes() -> Vec<&'static str> {
    Language::choices(Language::System)
        .into_iter()
        .map(Language::code)
        .collect()
}

pub(super) fn definitions() -> Vec<Value> {
    vec![
        tool(
            "get_editor_settings",
            "Read editor preferences and supported language codes. writable is true only on the live editor connection.",
            json!({}),
            &[],
        ),
        tool(
            "set_editor_settings",
            "Change and save live editor preferences. Unspecified fields stay unchanged. Language applies to newly opened windows; restart the editor to translate its existing main window. Does not restart or edit the map. dry_run validates without saving.",
            json!({
                "language":{"type":"string","enum":codes()},
                "panel_opacity":{"type":"number","minimum":80,"maximum":100,"description":"Panel opacity in percent"},
                "show_axes":{"type":"boolean"},
                "dry_run":{"type":"boolean"}
            }),
            &[],
        ),
    ]
}
fn snapshot(p: &preferences::Preferences, writable: bool) -> Value {
    json!({"language":p.language.code(), "panel_opacity":p.panel_opacity,"show_axes":p.show_axes,
        "writable":writable,"languages":Language::choices(Language::System).into_iter().map(|language|json!({"code":language.code(),"name":language.name()})).collect::<Vec<_>>()})
}
pub(super) fn call(collab: &mut Collab, name: &str, args: &Value) -> Result<String, String> {
    let current = preferences::current();
    if name == "get_editor_settings" {
        return Ok(snapshot(&current, collab.settings_path.is_some()).to_string());
    }
    let path = collab
        .settings_path
        .as_ref()
        .ok_or("Connect to the running editor's live MCP server to change its settings")?;
    let mut draft = current.clone();
    if let Some(code) = args.get("language") {
        draft.language = code
            .as_str()
            .and_then(Language::from_code)
            .ok_or("Unsupported language code")?;
    }
    if let Some(value) = args.get("panel_opacity") {
        draft.panel_opacity = value.as_f64().ok_or("Invalid opacity")? as f32;
    }
    if let Some(value) = args.get("show_axes") {
        draft.show_axes = value.as_bool().ok_or("Invalid show_axes")?;
    }
    let changed = draft.language != current.language
        || draft.panel_opacity != current.panel_opacity
        || draft.show_axes != current.show_axes;
    let dry_run = args["dry_run"].as_bool().unwrap_or(false);
    let mut result = snapshot(&draft, true);
    result["changed"] = json!(changed);
    result["saved"] = json!(changed && !dry_run);
    result["restart_required"] = json!(draft.language != current.language);
    result["language_application"] = json!(
        "New windows use the chosen language. Restart the editor to translate the existing main window."
    );
    if changed && !dry_run {
        draft
            .save(path)
            .map_err(|e| format!("Could not save editor settings: {e}"))?;
        preferences::set_current(draft);
        collab.settings_changed = true;
    }
    Ok(result.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_validate_persist_and_leave_map_untouched() {
        let original = preferences::current();
        preferences::set_current(preferences::Preferences::default());
        let path =
            std::env::temp_dir().join(format!("heron-agent-settings-{}", std::process::id()));
        let mut collab = Collab {
            settings_path: Some(path.clone()),
            ..Default::default()
        };
        let mut document = Document::new(WorldBounds::default());
        let revision = document.revision();
        let args = json!({"language":"cs","show_axes":false,"panel_opacity":92,"dry_run":true});
        call_tool(&mut document, &mut collab, "set_editor_settings", &args).unwrap();
        assert!(!collab.settings_changed);
        assert_eq!(preferences::pref(|p| p.language), Language::System);
        let mut args = args;
        args["dry_run"] = json!(false);
        call_tool(&mut document, &mut collab, "set_editor_settings", &args).unwrap();
        assert!(collab.settings_changed);
        assert_eq!(preferences::pref(|p| p.language), Language::Czech);
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("ui/Language=czech"));
        for invalid in [
            json!({"language":"unknown"}),
            json!({"panel_opacity":101}),
            json!({"show_axes":"yes"}),
            json!({"unknown":true}),
        ] {
            assert!(
                call_tool(&mut document, &mut collab, "set_editor_settings", &invalid).is_err()
            );
        }
        collab.settings_path = Some(path.join("impossible"));
        assert!(call(
            &mut collab,
            "set_editor_settings",
            &json!({"language":"en"})
        )
        .is_err());
        assert_eq!(preferences::pref(|p| p.language), Language::Czech);
        assert_eq!(document.revision(), revision);
        assert!(call(
            &mut Collab::default(),
            "set_editor_settings",
            &json!({"language":"en"})
        )
        .is_err());
        std::fs::remove_file(path).unwrap();
        preferences::set_current(original);
    }
}
