use super::image_storage::ImageStorage;
use wasm_bindgen::JsCast;

/// Image manager with sequential 3-digit IDs (001-999)
pub struct ImageManager {
    storage: ImageStorage,
}

impl ImageManager {
    pub fn new() -> Self {
        Self {
            storage: ImageStorage::new(),
        }
    }

    /// Get current counter from localStorage
    fn get_counter() -> u32 {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                if let Ok(Some(counter_str)) = storage.get_item("image_counter") {
                    return counter_str.parse().unwrap_or(0);
                }
            }
        }
        0
    }

    /// Set counter in localStorage
    fn set_counter(value: u32) {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("image_counter", &value.to_string());
            }
        }
    }

    /// Generate next sequential ID (001, 002, ..., 999)
    pub fn generate_next_id() -> Result<String, String> {
        let current = Self::get_counter();
        let next = current + 1;

        if next > 999 {
            return Err("Maximum image limit reached (999)".to_string());
        }

        Self::set_counter(next);
        Ok(format!("{:03}", next))
    }

    /// Store image with sequential ID
    pub async fn store_image(&self, base64_data: &str, filename: &str) -> Result<String, String> {
        let id = Self::generate_next_id()?;

        // Store image with metadata in a JSON structure
        let metadata = format!(
            r#"{{"id":"{}","filename":"{}","data":"{}","timestamp":{}}}"#,
            id,
            filename,
            base64_data,
            js_sys::Date::now() as u64
        );

        self.storage.store_image(&id, &metadata).await?;
        log::info!("Image stored with ID: {} ({})", id, filename);

        Ok(id)
    }

    /// List all images with metadata
    pub async fn list_all_images(&self) -> Result<Vec<ImageMetadata>, String> {
        let images = self.storage.list_all_images().await?;
        let mut result = Vec::new();

        for (id, metadata_json) in images {
            if let Ok(metadata) = Self::parse_metadata(&metadata_json) {
                result.push(ImageMetadata {
                    id,
                    filename: metadata.0,
                    data: metadata.1,
                    timestamp: metadata.2,
                });
            }
        }

        // Sort by ID (which is sequential)
        result.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(result)
    }

    /// Parse metadata JSON
    fn parse_metadata(json: &str) -> Result<(String, String, u64), String> {
        if let Ok(parsed) = js_sys::JSON::parse(json) {
            if let Ok(obj) = parsed.dyn_into::<js_sys::Object>() {
                let filename_key = wasm_bindgen::JsValue::from_str("filename");
                let data_key = wasm_bindgen::JsValue::from_str("data");
                let timestamp_key = wasm_bindgen::JsValue::from_str("timestamp");

                let filename = js_sys::Reflect::get(&obj, &filename_key)
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();

                let data = js_sys::Reflect::get(&obj, &data_key)
                    .ok()
                    .and_then(|v| v.as_string())
                    .unwrap_or_default();

                let timestamp = js_sys::Reflect::get(&obj, &timestamp_key)
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;

                return Ok((filename, data, timestamp));
            }
        }

        Err("Failed to parse metadata".to_string())
    }

    /// Delete image by ID
    pub async fn delete_image(&self, id: &str) -> Result<(), String> {
        self.storage.delete_image(id).await
    }
}

impl Default for ImageManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct ImageMetadata {
    pub id: String,
    pub filename: String,
    pub data: String,
    #[allow(dead_code)]
    pub timestamp: u64,
}
