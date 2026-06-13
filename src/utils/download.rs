use wasm_bindgen::JsCast;

/// Trigger a browser download of `bytes` saved as `filename` with the given MIME type.
///
/// Replaces the previously duplicated blob+anchor logic and fails silently
/// (rather than panicking) if any DOM/Blob/URL step is unavailable.
pub fn download_bytes(filename: &str, mime: &str, bytes: &[u8]) {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };

    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&array);

    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);

    let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options) else {
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        return;
    };

    if let Ok(element) = document.create_element("a") {
        if let Ok(anchor) = element.dyn_into::<web_sys::HtmlAnchorElement>() {
            anchor.set_href(&url);
            anchor.set_download(filename);
            anchor.click();
        }
    }

    let _ = web_sys::Url::revoke_object_url(&url);
}
