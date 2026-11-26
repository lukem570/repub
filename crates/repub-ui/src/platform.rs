//! What differs between the desktop app and the browser: clocks, opening links,
//! and getting files in and out (the browser has no file system, so files are
//! picked asynchronously and saved by downloading them).

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::{Instant, SystemTime, UNIX_EPOCH};
#[cfg(target_arch = "wasm32")]
pub use web_time::{Instant, SystemTime, UNIX_EPOCH};

/// The current time as Unix seconds.
pub fn unix_now() -> Option<i64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as i64)
}

/// Open `address` in the system's browser or mail app (a new tab in the browser).
pub fn open_address(address: &str) -> std::io::Result<()> {
    #[cfg(target_arch = "wasm32")]
    {
        let opened =
            web_sys::window().and_then(|w| w.open_with_url_and_target(address, "_blank").ok());
        return opened
            .map(|_| ())
            .ok_or_else(|| std::io::Error::other("the browser blocked the new tab"));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        #[cfg(target_os = "macos")]
        let mut command = std::process::Command::new("open");
        #[cfg(target_os = "windows")]
        let mut command = {
            let mut c = std::process::Command::new("rundll32");
            c.arg("url.dll,FileProtocolHandler");
            c
        };
        #[cfg(all(unix, not(target_os = "macos")))]
        let mut command = std::process::Command::new("xdg-open");
        command.arg(address).spawn().map(|_| ())
    }
}

/// File picking and downloading in the browser.
#[cfg(target_arch = "wasm32")]
pub mod web {
    use std::cell::RefCell;

    use repub_model::ObjectId;
    use wasm_bindgen::JsCast;

    /// What a picked file is for.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Purpose {
        /// Open it as a publication.
        OpenDocument,
        /// Place it on the current page as a picture.
        InsertPicture,
        /// Replace the picture in this frame.
        ChangePicture(ObjectId),
    }

    /// A file the user picked, with its contents.
    pub struct Picked {
        pub purpose: Purpose,
        pub name: String,
        pub bytes: Vec<u8>,
    }

    thread_local! {
        static PICKED: RefCell<Vec<Picked>> = const { RefCell::new(Vec::new()) };
        static CONTEXT: RefCell<Option<egui::Context>> = const { RefCell::new(None) };
    }

    /// Remember the egui context, so a finished file pick can wake the UI.
    pub fn set_context(ctx: &egui::Context) {
        CONTEXT.with(|c| *c.borrow_mut() = Some(ctx.clone()));
    }

    /// Ask the user for a file (a dialog the browser shows); it arrives later through
    /// [`take_picked`].
    pub fn pick_file(purpose: Purpose, filter_name: &str, extensions: &[&str]) {
        let filter_name = filter_name.to_string();
        let extensions: Vec<String> = extensions.iter().map(|e| e.to_string()).collect();
        wasm_bindgen_futures::spawn_local(async move {
            let refs: Vec<&str> = extensions.iter().map(String::as_str).collect();
            let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter(&filter_name, &refs)
                .pick_file()
                .await
            else {
                return;
            };
            let name = file.file_name();
            let bytes = file.read().await;
            PICKED.with(|p| {
                p.borrow_mut().push(Picked {
                    purpose,
                    name,
                    bytes,
                })
            });
            CONTEXT.with(|c| {
                if let Some(ctx) = c.borrow().as_ref() {
                    ctx.request_repaint();
                }
            });
        });
    }

    /// The files picked since the last call.
    pub fn take_picked() -> Vec<Picked> {
        PICKED.with(|p| std::mem::take(&mut *p.borrow_mut()))
    }

    /// Save `bytes` as a file called `name` through the browser's download.
    pub fn download(name: &str, mime: &str, bytes: &[u8]) -> Result<(), String> {
        let js = |e: wasm_bindgen::JsValue| format!("{e:?}");
        let parts = js_sys::Array::new();
        parts.push(&js_sys::Uint8Array::from(bytes));
        let options = web_sys::BlobPropertyBag::new();
        options.set_type(mime);
        let blob =
            web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options).map_err(js)?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).map_err(js)?;
        let document = web_sys::window()
            .and_then(|w| w.document())
            .ok_or("no document")?;
        let anchor: web_sys::HtmlAnchorElement = document
            .create_element("a")
            .map_err(js)?
            .dyn_into()
            .map_err(|_| "not an anchor")?;
        anchor.set_href(&url);
        anchor.set_download(name);
        anchor.click();
        let _ = web_sys::Url::revoke_object_url(&url);
        Ok(())
    }
}

/// The MIME type of a file written by an export, from its extension.
pub fn mime_for(extension: &str) -> &'static str {
    match extension {
        "pdf" => "application/pdf",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "tiff" | "tif" => "image/tiff",
        "html" | "htm" => "text/html",
        "repub" | "json" => "application/json",
        _ => "application/octet-stream",
    }
}
