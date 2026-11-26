//! Platform abstractions for web.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, HtmlInputElement, Url};

/// Download a file to the user's computer.
pub fn download_file(filename: &str, data: &[u8], mime_type: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;
    
    // Create blob
    let array = js_sys::Uint8Array::from(data);
    let blob_parts = js_sys::Array::new();
    blob_parts.push(&array);
    
    let mut options = BlobPropertyBag::new();
    options.type_(mime_type);
    
    let blob = Blob::new_with_u8_array_sequence_and_options(&blob_parts, &options)?;
    
    // Create download URL
    let url = Url::create_object_url_with_blob(&blob)?;
    
    // Create and click download link
    let link = document.create_element("a")?;
    link.set_attribute("href", &url)?;
    link.set_attribute("download", filename)?;
    link.dyn_ref::<web_sys::HtmlElement>()
        .ok_or("Not an HtmlElement")?
        .click();
    
    // Clean up
    Url::revoke_object_url(&url)?;
    
    Ok(())
}

/// Create a file input for opening files.
pub fn create_file_input(accept: &str, callback: impl Fn(Vec<u8>, String) + 'static) -> Result<HtmlInputElement, JsValue> {
    let window = web_sys::window().ok_or("No window")?;
    let document = window.document().ok_or("No document")?;
    
    let input = document.create_element("input")?;
    let input = input.dyn_into::<HtmlInputElement>()?;
    input.set_type("file");
    input.set_accept(accept);
    input.set_attribute("style", "display: none")?;
    
    // Set up change handler
    let callback = std::rc::Rc::new(callback);
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        let input = event.target()
            .and_then(|t| t.dyn_into::<HtmlInputElement>().ok());
        
        if let Some(input) = input {
            if let Some(files) = input.files() {
                if let Some(file) = files.get(0) {
                    let filename = file.name();
                    let callback = callback.clone();
                    
                    let reader = web_sys::FileReader::new().unwrap();
                    let reader_clone = reader.clone();
                    
                    let onload = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                        if let Ok(result) = reader_clone.result() {
                            if let Some(array_buffer) = result.dyn_ref::<js_sys::ArrayBuffer>() {
                                let array = js_sys::Uint8Array::new(array_buffer);
                                let data = array.to_vec();
                                callback(data, filename.clone());
                            }
                        }
                    }) as Box<dyn FnMut(_)>);
                    
                    reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                    onload.forget();
                    
                    let _ = reader.read_as_array_buffer(&file);
                }
            }
        }
    }) as Box<dyn FnMut(_)>);
    
    input.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())?;
    closure.forget();
    
    // Add to document (hidden)
    document.body().ok_or("No body")?.append_child(&input)?;
    
    Ok(input)
}

/// Open the file picker.
pub fn open_file_picker(input: &HtmlInputElement) {
    input.click();
}

/// Get a query parameter from the URL.
pub fn get_query_param(name: &str) -> Option<String> {
    let window = web_sys::window()?;
    let location = window.location();
    let search = location.search().ok()?;
    
    let params = web_sys::UrlSearchParams::new_with_str(&search).ok()?;
    params.get(name)
}

/// Open a URL in a new tab.
pub fn open_url(url: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.open_with_url_and_target(url, "_blank");
    }
}

/// Set the page title.
pub fn set_title(title: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            document.set_title(title);
        }
    }
}

/// Show a browser alert.
pub fn alert(message: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.alert_with_message(message);
    }
}

/// Show a browser confirm dialog.
pub fn confirm(message: &str) -> bool {
    web_sys::window()
        .and_then(|w| w.confirm_with_message(message).ok())
        .unwrap_or(false)
}

/// Log to the browser console.
pub fn console_log(message: &str) {
    web_sys::console::log_1(&JsValue::from_str(message));
}

/// Check if running on a mobile device (approximate).
pub fn is_mobile() -> bool {
    if let Some(window) = web_sys::window() {
        if let Some(navigator) = Some(window.navigator()) {
            if let Ok(user_agent) = navigator.user_agent() {
                let ua = user_agent.to_lowercase();
                return ua.contains("mobile") || 
                       ua.contains("android") || 
                       ua.contains("iphone") ||
                       ua.contains("ipad");
            }
        }
    }
    false
}

/// Check if running on a touch device.
pub fn is_touch_device() -> bool {
    if let Some(window) = web_sys::window() {
        // Check for touch support
        if let Ok(has_touch) = js_sys::Reflect::get(&window, &JsValue::from_str("ontouchstart")) {
            return !has_touch.is_undefined();
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_version() {
        let v = crate::version();
        assert!(!v.is_empty());
    }
}
