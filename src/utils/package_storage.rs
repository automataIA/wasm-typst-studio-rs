//! IndexedDB cache for `@preview` package tarballs.
//!
//! Modeled on [`crate::utils::image_storage`], but stores raw `.tar.gz` bytes
//! keyed by the canonical package spec string (e.g. `@preview/cetz:0.3.1`). A
//! separate database avoids an object-store version bump on the images DB.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{IdbDatabase, IdbTransactionMode};

pub struct PackageStorage {
    db_name: String,
    store_name: String,
}

impl PackageStorage {
    pub fn new() -> Self {
        Self {
            db_name: "typst_studio_packages".to_string(),
            store_name: "packages".to_string(),
        }
    }

    fn request_to_promise(request: &web_sys::IdbRequest) -> js_sys::Promise {
        let req = request.clone();
        js_sys::Promise::new(&mut |resolve, reject| {
            let req_success = req.clone();
            let onsuccess = Closure::wrap(Box::new(move |_: web_sys::Event| {
                if let Ok(result) = req_success.result() {
                    let _ = resolve.call1(&JsValue::NULL, &result);
                }
            }) as Box<dyn FnMut(_)>);
            let onerror = Closure::wrap(Box::new(move |_: web_sys::Event| {
                let _ = reject.call1(&JsValue::NULL, &JsValue::from_str("IndexedDB request failed"));
            }) as Box<dyn FnMut(_)>);
            req.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
            req.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onsuccess.forget();
            onerror.forget();
        })
    }

    async fn init(&self) -> Result<IdbDatabase, String> {
        let window = web_sys::window().ok_or("No window found")?;
        let idb_factory = window
            .indexed_db()
            .map_err(|_| "IndexedDB not supported")?
            .ok_or("IndexedDB not available")?;
        let open_request = idb_factory
            .open_with_u32(&self.db_name, 1)
            .map_err(|e| format!("Failed to open DB: {e:?}"))?;

        let store_name = self.store_name.clone();
        let onupgradeneeded =
            Closure::wrap(Box::new(move |event: web_sys::IdbVersionChangeEvent| {
                if let Some(target) = event.target() {
                    if let Ok(request) = target.dyn_into::<web_sys::IdbOpenDbRequest>() {
                        if let Ok(result) = request.result() {
                            if let Ok(db) = result.dyn_into::<IdbDatabase>() {
                                if !db.object_store_names().contains(&store_name) {
                                    let _ = db.create_object_store(&store_name);
                                }
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(_)>);
        open_request.set_onupgradeneeded(Some(onupgradeneeded.as_ref().unchecked_ref()));
        onupgradeneeded.forget();

        let promise = Self::request_to_promise(&open_request);
        let result = JsFuture::from(promise)
            .await
            .map_err(|e| format!("Failed to open DB: {e:?}"))?;
        result
            .dyn_into::<IdbDatabase>()
            .map_err(|_| "Failed to cast to IdbDatabase".to_string())
    }

    /// Store a raw tarball under its spec key.
    pub async fn store(&self, key: &str, bytes: &[u8]) -> Result<(), String> {
        let db = self.init().await?;
        let transaction = db
            .transaction_with_str_and_mode(&self.store_name, IdbTransactionMode::Readwrite)
            .map_err(|e| format!("Failed to create transaction: {e:?}"))?;
        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| format!("Failed to get object store: {e:?}"))?;

        let value: JsValue = js_sys::Uint8Array::from(bytes).into();
        let request = store
            .put_with_key(&value, &JsValue::from_str(key))
            .map_err(|e| format!("Failed to put package: {e:?}"))?;
        JsFuture::from(Self::request_to_promise(&request))
            .await
            .map_err(|e| format!("Failed to store package: {e:?}"))?;
        Ok(())
    }

    /// List every cached package as `(spec_key, tarball_bytes)`.
    pub async fn list_all(&self) -> Result<Vec<(String, Vec<u8>)>, String> {
        let db = self.init().await?;
        let transaction = db
            .transaction_with_str(&self.store_name)
            .map_err(|e| format!("Failed to create transaction: {e:?}"))?;
        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| format!("Failed to get object store: {e:?}"))?;

        let keys_req = store
            .get_all_keys()
            .map_err(|e| format!("Failed to get keys: {e:?}"))?;
        let keys_result = JsFuture::from(Self::request_to_promise(&keys_req))
            .await
            .map_err(|e| format!("Failed to retrieve keys: {e:?}"))?;
        let keys: js_sys::Array = keys_result
            .dyn_into()
            .map_err(|_| "Failed to cast keys to Array")?;

        let mut out = Vec::new();
        for i in 0..keys.length() {
            let Some(key) = keys.get(i).as_string() else {
                continue;
            };
            let get_req = store
                .get(&JsValue::from_str(&key))
                .map_err(|e| format!("Failed to get package: {e:?}"))?;
            let value = JsFuture::from(Self::request_to_promise(&get_req))
                .await
                .map_err(|e| format!("Failed to retrieve package: {e:?}"))?;
            if let Ok(arr) = value.dyn_into::<js_sys::Uint8Array>() {
                out.push((key, arr.to_vec()));
            }
        }
        Ok(out)
    }
}

impl Default for PackageStorage {
    fn default() -> Self {
        Self::new()
    }
}
