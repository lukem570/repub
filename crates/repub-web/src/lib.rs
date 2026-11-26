//! # Repub Web
//!
//! The Repub desktop app compiled to WebAssembly: the browser runs the same
//! `repub-ui` application, drawn on a canvas with the same renderer, so the two
//! look and behave alike. Differences come only from the browser: files are
//! picked and downloaded instead of opened and saved in place, and there are no
//! system fonts or native menus.
//!
//! ## Building
//!
//! Build with [Trunk](https://trunkrs.dev) from this crate's directory:
//! ```bash
//! trunk serve            # development server
//! trunk build --release  # writes the site to dist/
//! ```
//!
//! The `Deploy to GitHub Pages` workflow publishes `dist/` on every push to `main`.
//! This crate is empty on every target but `wasm32-unknown-unknown`.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Set up logging and panic reporting when the module loads.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::default());
}

/// Start the application in the canvas element with id `canvas_id`.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), JsValue> {
    let canvas = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(canvas_id))
        .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        .ok_or_else(|| JsValue::from_str(&format!("no canvas element with id \"{canvas_id}\"")))?;

    // wgpu 22 asks WebGPU for a limit current browsers no longer recognize, so it can't create
    // a device there; WebGL 2 works everywhere WebGPU does and beyond
    let mut options = eframe::WebOptions::default();
    options.wgpu_options.supported_backends = wgpu::Backends::GL;

    wasm_bindgen_futures::spawn_local(async move {
        let result = eframe::WebRunner::new()
            .start(
                canvas,
                options,
                Box::new(|cc| Ok(Box::new(repub_ui::RepubApp::new(cc)))),
            )
            .await;
        if let Err(e) = result {
            log::error!("Failed to start Repub: {e:?}");
        }
    });
    Ok(())
}

/// The application version.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
