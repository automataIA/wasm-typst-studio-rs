use serde::{Deserialize, Serialize};

/// A single editable project file. The file at index 0 is the compilation entry
/// point ("main"); the others are served to the compiler as `extra_files` so the
/// main file can reach them via `#include` / `#import`.
#[derive(Clone, Serialize, Deserialize)]
pub struct TypstFile {
    pub name: String,
    pub content: String,
}

/// localStorage key holding the whole multi-file project as JSON.
const FILES_KEY: &str = "typst_files";

/// Load the persisted project files, or `None` if absent / unparsable / empty.
pub fn load_files() -> Option<Vec<TypstFile>> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item(FILES_KEY).ok()??;
    let files: Vec<TypstFile> = serde_json::from_str(&json).ok()?;
    (!files.is_empty()).then_some(files)
}

/// Persist the whole project to localStorage (best-effort, fails silently).
pub fn save_files(files: &[TypstFile]) {
    let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else {
        return;
    };
    if let Ok(json) = serde_json::to_string(files) {
        let _ = storage.set_item(FILES_KEY, &json);
    }
}
