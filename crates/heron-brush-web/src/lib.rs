//! The WebAssembly module `index.html` loads: its start function hands the page to the
//! shared application in `heron_brush`. Build and serve it with `trunk serve` in this
//! folder (docs/web.md).

/// Runs when the module is loaded: starts the editor on the page's canvas, then takes the
/// loading screen away, or puts the reason on it when the editor could not start.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub async fn start() {
    let result = heron_brush::web::run().await;
    let note = web_sys::window().and_then(|window| window.document()).and_then(|document| document.get_element_by_id("loading"));
    match (result, note) {
        (Ok(()), Some(note)) => note.remove(),
        (Err(why), Some(note)) => {
            if let Ok(Some(status)) = note.query_selector(".status") {
                status.set_text_content(Some(&format!("Heron Brush could not start: {why}")));
            }
            note.set_class_name("failed");
        }
        (_, None) => {}
    }
}
