//! # Repub Web
//!
//! WebAssembly build for browser-based document editing.
//!
//! This crate provides a full-featured web application that runs entirely
//! in the browser using WebAssembly. It shares most of its code with the
//! desktop application.
//!
//! ## Features
//!
//! - Same document editing capabilities as desktop
//! - File drag & drop support
//! - Local storage for settings and auto-save
//! - Export/download files
//! - Responsive design for tablets
//!
//! ## Usage
//!
//! Build with [Trunk](https://trunkrs.dev) from this crate's directory:
//! ```bash
//! trunk serve            # development server
//! trunk build --release  # writes the site to dist/
//! ```
//!
//! The `Deploy to GitHub Pages` workflow publishes `dist/` on every push to `main`.



#[cfg(target_arch = "wasm32")]
mod app;
#[cfg(target_arch = "wasm32")]
mod platform;
#[cfg(target_arch = "wasm32")]
mod storage;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
pub use app::WebApp;

/// Initialize the WASM module and start the application.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main() {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();
    
    // Initialize logging
    wasm_logger::init(wasm_logger::Config::default());
    
    log::info!("Repub Web starting...");
}

/// Start the application in the given canvas element.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn start(canvas_id: &str) -> Result<(), JsValue> {
    log::info!("Starting Repub in canvas: {}", canvas_id);
    
    let web_options = eframe::WebOptions::default();

    let canvas = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(canvas_id))
        .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        .ok_or_else(|| JsValue::from_str(&format!("no canvas element with id \"{canvas_id}\"")))?;

    wasm_bindgen_futures::spawn_local(async move {
        let runner = eframe::WebRunner::new();
        let result = runner
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(WebApp::new(cc)))),
            )
            .await;

        if let Err(e) = result {
            log::error!("Failed to start eframe: {:?}", e);
        }
    });

    Ok(())
}

/// Get the application version.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// Stub for non-WASM builds (for cargo check)
#[cfg(not(target_arch = "wasm32"))]
pub fn main() {
    println!("Repub Web is designed to run in a browser via WebAssembly.");
    println!("Build with: wasm-pack build --target web");
}
