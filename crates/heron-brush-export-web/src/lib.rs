//! Headless export entry points for a dedicated Web Worker. No DOM or frontend imports.
#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;
    use std::path::{Path, PathBuf};

    #[wasm_bindgen(inline_js = "export function report_export_panic(message) { globalThis.heronExportPanic = message; console.error(message); }")]
    extern "C" {
        fn report_export_panic(message: &str);
    }

    #[wasm_bindgen(start)]
    pub fn initialize() {
        std::panic::set_hook(Box::new(|info| report_export_panic(&info.to_string())));
        editor_core::preferences::set_platform_directories(
            "/heron-brush/config".into(), "/heron-brush/install".into());
    }

    #[wasm_bindgen]
    pub fn apply_files(changes: js_sys::Array) -> Result<(), JsValue> {
        editor_core::files::set_fallback(|path| editor_core::samples::bundled(path).map(str::as_bytes));
        for change in changes.iter() {
            let entry = js_sys::Array::from(&change);
            let path = PathBuf::from(entry.get(0).as_string().ok_or_else(|| JsValue::from_str("Invalid file path"))?);
            let bytes = entry.get(1);
            if bytes.is_null() {
                let _ = editor_core::files::remove_file(&path);
            } else {
                editor_core::files::store(path, js_sys::Uint8Array::new(&bytes).to_vec());
            }
        }
        let preferences = editor_core::preferences::Preferences::load(
            &editor_core::preferences::Preferences::default_path());
        editor_core::preferences::set_current(preferences);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn export_playtest(snapshot: &str, directory: &str, options: &str) -> Result<JsValue, JsValue> {
        let start = web_time::Instant::now();
        let run = || -> Result<(String, Vec<String>), String> {
            let snapshot = serde_json::from_str(snapshot).map_err(|e| e.to_string())?;
            let document = editor_core::document::Document::from_export_snapshot(snapshot)?;
            let mut options: editor_core::export::Options = serde_json::from_str(options).map_err(|e| e.to_string())?;
            options.target = editor_core::export::Target::ThreeJs;
            let (report, files) = editor_core::export::export_files(&document, Path::new(directory), &options).map_err(|e|e.to_string())?;
            let html = files.into_iter().find_map(|(path, contents)| match contents {
                editor_core::export::Contents::Text(text) if path.extension().is_some_and(|e| e == "html") => Some(text),
                _ => None,
            }).ok_or_else(|| "Playtest export produced no HTML".to_string())?;
            Ok((html, report.warnings))
        };
        let (html, warnings) = run().map_err(|error| {
            let details = editor_core::console::lines().into_iter().rev().take(8)
                .map(|(_, line)| line).collect::<Vec<_>>().join("\n");
            JsValue::from_str(&format!("{error}\n{details}"))
        })?;
        let result = js_sys::Object::new();
        js_sys::Reflect::set(&result, &"html".into(), &html.into())?;
        let list = js_sys::Array::new(); for warning in warnings { list.push(&warning.into()); }
        js_sys::Reflect::set(&result, &"warnings".into(), &list)?;
        js_sys::Reflect::set(&result, &"exportMs".into(), &JsValue::from_f64(start.elapsed().as_secs_f64()*1000.0))?;
        Ok(result.into())
    }
}
