use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{IdbDatabase, IdbTransactionMode};

/// IndexedDB-based image storage for Typst editor
/// Stores images as base64-encoded strings with unique IDs
pub struct ImageStorage {
    db_name: String,
    store_name: String,
}

impl ImageStorage {
    pub fn new() -> Self {
        Self {
            db_name: "typst_studio_db".to_string(),
            store_name: "images".to_string(),
        }
    }

    /// Helper to convert IdbRequest to Promise
    fn request_to_promise(request: &web_sys::IdbRequest) -> js_sys::Promise {
        let req = request.clone();
        js_sys::Promise::new(&mut |resolve, reject| {
            let resolve = resolve.clone();
            let reject = reject.clone();
            let req_success = req.clone();

            let onsuccess = Closure::wrap(Box::new(move |_: web_sys::Event| {
                if let Ok(result) = req_success.result() {
                    let _ = resolve.call1(&JsValue::NULL, &result);
                }
            }) as Box<dyn FnMut(_)>);

            let onerror = Closure::wrap(Box::new(move |_: web_sys::Event| {
                let err = JsValue::from_str("IndexedDB request failed");
                let _ = reject.call1(&JsValue::NULL, &err);
            }) as Box<dyn FnMut(_)>);

            req.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
            req.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onsuccess.forget();
            onerror.forget();
        })
    }

    /// Initialize IndexedDB database
    pub async fn init(&self) -> Result<IdbDatabase, String> {
        let window = web_sys::window().ok_or("No window found")?;
        let idb_factory = window
            .indexed_db()
            .map_err(|_| "IndexedDB not supported")?
            .ok_or("IndexedDB not available")?;

        let open_request = idb_factory
            .open_with_u32(&self.db_name, 1)
            .map_err(|e| format!("Failed to open DB: {:?}", e))?;

        // Setup onupgradeneeded callback
        let store_name = self.store_name.clone();
        let onupgradeneeded = Closure::wrap(Box::new(move |event: web_sys::IdbVersionChangeEvent| {
            log::info!("IndexedDB upgrade needed");
            if let Some(target) = event.target() {
                if let Ok(request) = target.dyn_into::<web_sys::IdbOpenDbRequest>() {
                    if let Ok(result) = request.result() {
                        if let Ok(db) = result.dyn_into::<IdbDatabase>() {
                            // Check if object store exists
                            let store_names = db.object_store_names();
                            if !store_names.contains(&store_name) {
                                let _ = db.create_object_store(&store_name);
                                log::info!("Created object store: {}", store_name);
                            }
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        open_request.set_onupgradeneeded(Some(onupgradeneeded.as_ref().unchecked_ref()));
        onupgradeneeded.forget();

        // Wait for DB to open using a Promise wrapper
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let resolve = resolve.clone();
            let reject = reject.clone();
            let req = open_request.clone();

            let onsuccess = Closure::wrap(Box::new(move |_: web_sys::Event| {
                if let Ok(result) = req.result() {
                    resolve.call1(&JsValue::NULL, &result).unwrap();
                }
            }) as Box<dyn FnMut(_)>);

            let onerror = Closure::wrap(Box::new(move |_: web_sys::Event| {
                let err = JsValue::from_str("Failed to open IndexedDB");
                let _ = reject.call1(&JsValue::NULL, &err);
            }) as Box<dyn FnMut(_)>);

            open_request.set_onsuccess(Some(onsuccess.as_ref().unchecked_ref()));
            open_request.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onsuccess.forget();
            onerror.forget();
        });

        let result = JsFuture::from(promise)
            .await
            .map_err(|e| format!("Failed to open DB: {:?}", e))?;

        let db: IdbDatabase = result
            .dyn_into()
            .map_err(|_| "Failed to cast to IdbDatabase")?;

        log::info!("IndexedDB initialized successfully");
        Ok(db)
    }

    /// Store an image and return its ID
    pub async fn store_image(&self, image_id: &str, base64_data: &str) -> Result<(), String> {
        let db = self.init().await?;

        let transaction = db
            .transaction_with_str_and_mode(
                &self.store_name,
                IdbTransactionMode::Readwrite,
            )
            .map_err(|e| format!("Failed to create transaction: {:?}", e))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| format!("Failed to get object store: {:?}", e))?;

        let value = JsValue::from_str(base64_data);
        let key = JsValue::from_str(image_id);

        let request = store
            .put_with_key(&value, &key)
            .map_err(|e| format!("Failed to put image: {:?}", e))?;

        let promise = Self::request_to_promise(&request);
        JsFuture::from(promise)
            .await
            .map_err(|e| format!("Failed to store image: {:?}", e))?;

        log::info!("Image stored with ID: {}", image_id);
        Ok(())
    }

    /// Retrieve an image by ID
    #[allow(dead_code)]
    pub async fn get_image(&self, image_id: &str) -> Result<String, String> {
        let db = self.init().await?;

        let transaction = db
            .transaction_with_str(&self.store_name)
            .map_err(|e| format!("Failed to create transaction: {:?}", e))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| format!("Failed to get object store: {:?}", e))?;

        let key = JsValue::from_str(image_id);
        let request = store
            .get(&key)
            .map_err(|e| format!("Failed to get image: {:?}", e))?;

        let promise = Self::request_to_promise(&request);
        let result = JsFuture::from(promise)
            .await
            .map_err(|e| format!("Failed to retrieve image: {:?}", e))?;

        if result.is_undefined() || result.is_null() {
            return Err(format!("Image not found: {}", image_id));
        }

        result
            .as_string()
            .ok_or_else(|| format!("Image data is not a string: {}", image_id))
    }

    /// Delete an image by ID
    #[allow(dead_code)]
    pub async fn delete_image(&self, image_id: &str) -> Result<(), String> {
        let db = self.init().await?;

        let transaction = db
            .transaction_with_str_and_mode(
                &self.store_name,
                IdbTransactionMode::Readwrite,
            )
            .map_err(|e| format!("Failed to create transaction: {:?}", e))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| format!("Failed to get object store: {:?}", e))?;

        let key = JsValue::from_str(image_id);
        let request = store
            .delete(&key)
            .map_err(|e| format!("Failed to delete image: {:?}", e))?;

        let promise = Self::request_to_promise(&request);
        JsFuture::from(promise)
            .await
            .map_err(|e| format!("Failed to delete image: {:?}", e))?;

        log::info!("Image deleted: {}", image_id);
        Ok(())
    }

    /// Generate a unique image ID based on timestamp and random value
    #[allow(dead_code)]
    pub fn generate_image_id() -> String {
        use js_sys::Date;
        let timestamp = Date::now() as u64;
        let random = (js_sys::Math::random() * 1_000_000.0) as u32;
        format!("img_{}_{}", timestamp, random)
    }

    /// List all images with their IDs and data
    pub async fn list_all_images(&self) -> Result<Vec<(String, String)>, String> {
        let db = self.init().await?;

        let transaction = db
            .transaction_with_str(&self.store_name)
            .map_err(|e| format!("Failed to create transaction: {:?}", e))?;

        let store = transaction
            .object_store(&self.store_name)
            .map_err(|e| format!("Failed to get object store: {:?}", e))?;

        // Get all keys
        let request = store
            .get_all_keys()
            .map_err(|e| format!("Failed to get keys: {:?}", e))?;

        let promise = Self::request_to_promise(&request);
        let keys_result = JsFuture::from(promise)
            .await
            .map_err(|e| format!("Failed to retrieve keys: {:?}", e))?;

        // Convert JsValue to Array
        let keys_array: js_sys::Array = keys_result
            .dyn_into()
            .map_err(|_| "Failed to cast keys to Array")?;

        let mut images = Vec::new();

        // For each key, get the corresponding value
        for i in 0..keys_array.length() {
            if let Some(key) = keys_array.get(i).as_string() {
                if let Ok(data) = self.get_image(&key).await {
                    images.push((key, data));
                }
            }
        }

        Ok(images)
    }
}

impl Default for ImageStorage {
    fn default() -> Self {
        Self::new()
    }
}
