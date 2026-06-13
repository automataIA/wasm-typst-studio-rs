use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::*;
use gloo_timers::future::sleep;
use std::time::Duration;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use wasm_bindgen::JsCast;
use typst::syntax::package::PackageSpec;

// Modules
mod components;
mod compiler;
mod utils;

// Top-Level components
use crate::components::{Editor, Preview, ImageGalleryDrawer};
use crate::compiler::{
    compile_to_pdf, compile_to_svg, install_package, packages, take_missing_packages,
};
use crate::utils::download_bytes;
use crate::utils::package_storage::PackageStorage;
use crate::utils::project::{load_files, save_files, TypstFile};
use crate::utils::share::{build_share_url, source_from_url, strip_url_fragment};
use crate::utils::editing::{byte_to_utf16, insert_text, selection, set_selection, utf16_to_byte};

/// Which file-management dialog (if any) is currently open.
#[derive(Clone, Copy)]
enum FileDialog {
    New,
    Rename(usize),
    Delete(usize),
}

/// Typst Studio main app component
#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // Theme state: true = dark, false = light.
    // Restore the saved preference; otherwise follow the OS color scheme.
    let initial_dark = {
        let window = web_sys::window();
        let stored = window
            .as_ref()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("typst_theme").ok().flatten());
        match stored.as_deref() {
            Some("dark") => true,
            Some("light") => false,
            _ => window
                .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok().flatten())
                .map(|mql| mql.matches())
                .unwrap_or(true),
        }
    };
    let (is_dark_theme, set_is_dark_theme) = signal(initial_dark);

    // Reflect the theme on <html data-theme> and persist the choice.
    Effect::new(move |_| {
        let theme = if is_dark_theme.get() { "dark" } else { "light" };

        if let Some(window) = web_sys::window() {
            if let Some(html) = window.document().and_then(|d| d.document_element()) {
                let _ = html.set_attribute("data-theme", theme);
            }
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("typst_theme", theme);
            }
        }
    });

    // Editor state
    // Load from localStorage or use default (full example.typ content)
    const DEFAULT_SOURCE: &str = include_str!("../examples/example.typ");

    // Bundled document templates (the picker replaces the whole project).
    const TEMPLATE_BLANK: &str = include_str!("../templates/blank.typ");
    const TEMPLATE_ARTICLE: &str = include_str!("../templates/article.typ");
    const TEMPLATE_IEEE: &str = include_str!("../templates/ieee.typ");
    const TEMPLATE_IEEE_BIB: &str = include_str!("../examples/refs.yml");

    // Build the initial project. A shared URL fragment always yields a fresh
    // single-file project (shared snapshots are single-file) and is then stripped
    // so reloads use the persisted project instead of the stale snapshot.
    // Otherwise restore the persisted multi-file project, falling back to a single
    // `main.typ` from the legacy single-file key or the bundled default.
    let initial_files: Vec<TypstFile> = if let Some(shared) = source_from_url() {
        strip_url_fragment();
        vec![TypstFile {
            name: "main.typ".to_string(),
            content: shared,
        }]
    } else if let Some(files) = load_files() {
        files
    } else {
        let content = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("typst_source").ok().flatten())
            .unwrap_or_else(|| DEFAULT_SOURCE.to_string());
        vec![TypstFile {
            name: "main.typ".to_string(),
            content,
        }]
    };

    // The editor buffer (`source`) holds the active file's content; `files` is
    // the source of truth for the whole project, `active` the selected tab.
    // The file at index 0 is the compilation entry point.
    let initial_source = initial_files[0].content.clone();
    let files = RwSignal::new(initial_files);
    let (active, set_active) = signal(0usize);
    let (source, set_source) = signal(initial_source);

    let (output, set_output) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (is_compiling, set_is_compiling) = signal(false);

    // Panel resize state (editor width percentage, default 50%)
    let (editor_width, set_editor_width) = signal(50.0);
    let (is_resizing, set_is_resizing) = signal(false);

    // Preview zoom (1.0 = fit width), persisted in localStorage.
    let initial_zoom = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("typst_zoom").ok().flatten())
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(1.0)
        .clamp(0.25, 4.0);
    let (zoom, set_zoom) = signal(initial_zoom);
    Effect::new(move |_| {
        let z = zoom.get();
        if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) {
            let _ = storage.set_item("typst_zoom", &z.to_string());
        }
    });

    // Preview page indicator: current visible page (1-based) and total.
    let (current_page, set_current_page) = signal(1usize);
    let (page_count, set_page_count) = signal(1usize);

    // Store textarea ref for cursor position tracking
    use leptos::html::Textarea;
    let textarea_ref = NodeRef::<Textarea>::new();

    // Bibliography YAML content - load from localStorage or use default
    let initial_bib = r##"netwok2020:
  type: article
  title: "The Challenges of Scientific Typesetting in the Modern Era"
  author: ["Network, A.", "Smith, B."]
  date: 2020
  journal: "Journal of Academic Publishing"
  volume: 15
  issue: 3
  pages: "234-256"
  doi: "10.1234/jap.2020.15.3.234"

netwok2022:
  type: article
  title: "LaTeX: A Historical Perspective on Scientific Document Preparation"
  author: ["Network, A.", "Johnson, C.", "Williams, D."]
  date: 2022
  journal: "Computing in Science & Engineering"
  volume: 24
  issue: 2
  pages: "45-62"
  doi: "10.1234/cise.2022.24.2.045"

example2024:
  type: article
  title: "Example Research Paper"
  author: ["Smith, J.", "Doe, A."]
  date: 2024
  journal: "Journal of Examples"
  volume: 10
  pages: "123-145"

typst2023:
  type: web
  title: "Typst Documentation"
  author: "Typst Team"
  date: 2023
  url: "https://typst.app"
"##;

    let loaded_bib = if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            storage
                .get_item("typst_bibliography")
                .ok()
                .flatten()
                .unwrap_or_else(|| initial_bib.to_string())
        } else {
            initial_bib.to_string()
        }
    } else {
        initial_bib.to_string()
    };

    let (bibliography, set_bibliography) = signal(loaded_bib);
    let (show_bib_modal, set_show_bib_modal) = signal(false);

    // Document settings preamble: hidden `#set` rules prepended at compile time
    // so they apply without appearing in the editor. Restored from localStorage.
    const DEFAULT_SETTINGS: &str =
        "#set math.equation(numbering: \"(1)\")\n#set page(numbering: \"1\")";
    let loaded_settings = web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|s| s.get_item("typst_settings").ok().flatten())
        .unwrap_or_else(|| DEFAULT_SETTINGS.to_string());
    let (settings, set_settings) = signal(loaded_settings);
    let (show_settings, set_show_settings) = signal(false);

    // Append a preset `#set` rule to the settings preamble (on its own line).
    let add_setting = move |line: &str| {
        set_settings.update(|s| {
            if !s.is_empty() && !s.ends_with('\n') {
                s.push('\n');
            }
            s.push_str(line);
        });
    };

    // Image gallery drawer state
    let (show_image_gallery, set_show_image_gallery) = signal(false);

    // Template picker modal state
    let (show_templates, set_show_templates) = signal(false);

    // Inline file-management dialog (replaces native window.prompt/confirm).
    let file_dialog = RwSignal::new(Option::<FileDialog>::None);
    let dialog_input = RwSignal::new(String::new());

    // Close any open overlay (modal/drawer/dialog) on the Escape key.
    {
        use wasm_bindgen::closure::Closure;
        let on_keydown = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
            move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "Escape" {
                    set_show_bib_modal.set(false);
                    set_show_image_gallery.set(false);
                    set_show_settings.set(false);
                    set_show_templates.set(false);
                    file_dialog.set(None);
                }
            },
        );
        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("keydown", on_keydown.as_ref().unchecked_ref());
        }
        on_keydown.forget();
    }

    // Transient toast shown after copying a share link
    let (share_toast, set_share_toast) = signal(Option::<String>::None);

    // In-memory image cache: image_id -> base64_data
    // This allows synchronous access during compilation
    let (image_cache, set_image_cache) = signal(HashMap::<String, String>::new());

    // Load images from IndexedDB into cache on app start
    {
        spawn_local(async move {
            use crate::utils::image_manager::ImageManager;
            let manager = ImageManager::new();
            match manager.list_all_images().await {
                Ok(images) => {
                    let mut cache = HashMap::new();
                    for img in images {
                        cache.insert(img.id, img.data);
                    }
                    let count = cache.len();
                    set_image_cache.set(cache);
                    log::info!("Image cache initialized with {} images", count);
                }
                Err(e) => log::error!("Failed to load images: {}", e),
            }
        });
    }

    // Insert text at the cursor (wrapped in Rc for sharing). Routed through
    // `insert_text` (execCommand) so the browser's native undo stack survives;
    // the dispatched `input` event keeps the `source` signal in sync.
    let insert_at_cursor = Rc::new(move |text: &str, select_text: Option<&str>| {
        let Some(textarea) = textarea_ref.get() else { return };

        // Textarea selection offsets are UTF-16 code units (JS semantics).
        let (sel_start, sel_end) = selection(&textarea);
        let current = source.get_untracked();
        let byte_start = utf16_to_byte(&current, sel_start);
        let byte_end = utf16_to_byte(&current, sel_end);

        if sel_start != sel_end {
            // Wrap the selection: substitute the known placeholder token once.
            let selected = &current[byte_start..byte_end];
            let new_text = match select_text {
                Some(placeholder) => text.replacen(placeholder, selected, 1),
                None => text.to_string(),
            };
            let new_end = sel_start + new_text.encode_utf16().count();
            insert_text(&textarea, &new_text);
            set_selection(&textarea, sel_start, new_end);
        } else {
            insert_text(&textarea, text);
            match select_text.and_then(|ph| text.find(ph).map(|pos| (ph, pos))) {
                Some((placeholder, pos)) => {
                    // Select the placeholder so the user can type over it.
                    let ph_start = sel_start + text[..pos].encode_utf16().count();
                    let ph_end = ph_start + placeholder.encode_utf16().count();
                    set_selection(&textarea, ph_start, ph_end);
                }
                None => {
                    // Collapse the cursor after the inserted text.
                    let cursor = sel_start + text.encode_utf16().count();
                    set_selection(&textarea, cursor, cursor);
                }
            }
        }
    });

    // Ctrl+S: force-persist the whole project (files + bibliography + settings)
    // and flash a "Saved" toast. Autosave already runs on change; this gives the
    // shortcut explicit, visible feedback.
    let save_project = Callback::new(move |_: ()| {
        save_files(&files.get_untracked());
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("typst_bibliography", &bibliography.get_untracked());
                let _ = storage.set_item("typst_settings", &settings.get_untracked());
            }
        }
        set_share_toast.set(Some("Saved".to_string()));
        spawn_local(async move {
            sleep(Duration::from_millis(1500)).await;
            set_share_toast.set(None);
        });
    });

    // Jump the editor caret to a byte offset (from a preview click), scrolling
    // the clicked source into view.
    let jump_to = Callback::new(move |byte: usize| {
        let Some(ta) = textarea_ref.get() else {
            return;
        };
        let cur = source.get_untracked();
        let byte = byte.min(cur.len());
        let u = byte_to_utf16(&cur, byte);
        let _ = ta.focus();
        set_selection(&ta, u, u);
        let line = cur[..byte].matches('\n').count() as f64;
        ta.set_scroll_top(((line * 22.4) - 60.0).max(0.0) as i32);
    });

    // Mirror the editor buffer into the active file slot on every edit, so
    // `files` always reflects the latest content of every tab. The first
    // (mount) run is skipped: `source` already equals the active file's content,
    // so writing it back would only trigger a spurious persist.
    Effect::new(move |prev: Option<()>| {
        let src = source.get();
        if prev.is_some() {
            let idx = active.get_untracked();
            files.update(|f| {
                if let Some(file) = f.get_mut(idx) {
                    file.content = src;
                }
            });
        }
    });

    // Persist the whole project to localStorage on any change. The first (mount)
    // run is skipped so merely opening the app — or following a shared link —
    // never overwrites an existing stored project before the user edits it.
    Effect::new(move |prev: Option<()>| {
        let snapshot = files.get();
        if prev.is_some() {
            save_files(&snapshot);
        }
    });

    // Switch the editor to another tab (the outgoing file is already mirrored
    // into `files` by the effect above).
    let switch_to = move |idx: usize| {
        if files.with_untracked(|f| idx < f.len()) {
            set_active.set(idx);
            set_source.set(files.with_untracked(|f| f[idx].content.clone()));
        }
    };

    // Apply a bundled template: replace the whole project with a single
    // `main.typ`, optionally swapping in a matching bibliography.
    let apply_template = move |content: &str, bib: Option<&str>| {
        files.set(vec![TypstFile {
            name: "main.typ".to_string(),
            content: content.to_string(),
        }]);
        set_active.set(0);
        set_source.set(content.to_string());
        if let Some(b) = bib {
            set_bibliography.set(b.to_string());
        }
        set_show_templates.set(false);
    };

    // Open the inline dialogs (replacing native prompt/confirm).
    let open_new_file = move |_| {
        dialog_input.set(format!("file{}.typ", files.with_untracked(|f| f.len())));
        file_dialog.set(Some(FileDialog::New));
    };
    let open_rename = move |idx: usize| {
        if let Some(current) = files.with_untracked(|f| f.get(idx).map(|x| x.name.clone())) {
            dialog_input.set(current);
            file_dialog.set(Some(FileDialog::Rename(idx)));
        }
    };
    let open_delete = move |idx: usize| {
        if files.with_untracked(|f| f.len()) > 1 {
            file_dialog.set(Some(FileDialog::Delete(idx)));
        }
    };

    // Apply the currently open dialog's action, then close it.
    let apply_dialog = move || {
        let Some(dialog) = file_dialog.get_untracked() else {
            return;
        };
        match dialog {
            FileDialog::New => {
                let name = dialog_input.get_untracked().trim().to_string();
                if name.is_empty() {
                    return;
                }
                files.update(|f| {
                    f.push(TypstFile {
                        name,
                        content: String::new(),
                    })
                });
                let new_idx = files.with_untracked(|f| f.len() - 1);
                set_active.set(new_idx);
                set_source.set(String::new());
            }
            FileDialog::Rename(idx) => {
                let name = dialog_input.get_untracked().trim().to_string();
                if name.is_empty() {
                    return;
                }
                files.update(|f| {
                    if let Some(file) = f.get_mut(idx) {
                        file.name = name;
                    }
                });
            }
            FileDialog::Delete(idx) => {
                if files.with_untracked(|f| f.len()) <= 1 {
                    file_dialog.set(None);
                    return;
                }
                let old_active = active.get_untracked();
                files.update(|f| {
                    f.remove(idx);
                });
                let new_len = files.with_untracked(|f| f.len());
                let new_active = if old_active > idx {
                    old_active - 1
                } else {
                    old_active.min(new_len - 1)
                };
                set_active.set(new_active);
                set_source.set(files.with_untracked(|f| f[new_active].content.clone()));
            }
        }
        file_dialog.set(None);
    };

    // ----- @preview package support -----
    // `package_epoch` is a reactive trigger: installing a package bumps it so the
    // compile Effect reruns. `in_flight` / `failed` are non-reactive bookkeeping
    // (StoredValue is Copy + 'static) so we don't re-fetch or loop forever.
    let package_epoch = RwSignal::new(0u32);
    let package_status = RwSignal::new(Option::<String>::None);
    let in_flight = StoredValue::new(HashSet::<String>::new());
    let failed = StoredValue::new(HashSet::<String>::new());
    // Hard cap on retry rounds; transitive deps surface one level per compile.
    const MAX_PACKAGE_EPOCHS: u32 = 10;

    // Fetch, extract, install and cache a single package, then bump the epoch to
    // recompile. Copy closure (only Copy captures) so it can be reused freely.
    let start_download = move |spec: PackageSpec| {
        let key = spec.to_string();
        if failed.with_value(|f| f.contains(&key)) || in_flight.with_value(|f| f.contains(&key)) {
            return;
        }
        in_flight.update_value(|f| {
            f.insert(key.clone());
        });
        package_status.set(Some(format!("Downloading {key}…")));
        spawn_local(async move {
            let outcome = match packages::fetch_package_tarball(&spec).await {
                Ok(raw) => packages::extract_targz(&raw).map(|files| (raw, files)),
                Err(e) => Err(e),
            };
            match outcome {
                Ok((raw, files)) => {
                    install_package(&spec, files);
                    let _ = PackageStorage::new().store(&key, &raw).await;
                }
                Err(e) => {
                    log::error!("Package {key} failed: {e}");
                    failed.update_value(|f| {
                        f.insert(key.clone());
                    });
                }
            }
            in_flight.update_value(|f| {
                f.remove(&key);
            });
            if in_flight.with_value(|f| f.is_empty()) {
                package_status.set(None);
            }
            // Recompile: success makes the package resolve; failure surfaces the
            // error (the spec is now in `failed`, so it won't be re-fetched).
            package_epoch.update(|e| *e += 1);
        });
    };

    // On startup, install any packages cached in IndexedDB, then trigger a
    // recompile so a document using them renders without a network round-trip.
    spawn_local(async move {
        match PackageStorage::new().list_all().await {
            Ok(list) => {
                let mut installed = 0;
                for (key, bytes) in list {
                    if let Ok(spec) = key.parse::<PackageSpec>() {
                        if let Ok(files) = packages::extract_targz(&bytes) {
                            install_package(&spec, files);
                            installed += 1;
                        }
                    }
                }
                if installed > 0 {
                    package_epoch.update(|e| *e += 1);
                }
            }
            Err(e) => log::error!("Failed to load cached packages: {e}"),
        }
    });

    // Debouncing: use a counter to identify the latest update
    let (debounce_id, set_debounce_id) = signal(0u32);

    // Debounced compilation: compile 500ms after the last change.
    // Compiles the whole project: file 0 is the entry point, the rest are served
    // to the compiler so the main file can `#include` / `#import` them.
    Effect::new(move |_| {
        let project = files.get();
        let bib = bibliography.get();
        let settings_val = settings.get();
        // Recompile when a package is installed (the download loop bumps this).
        let epoch = package_epoch.get();

        // Increment the ID to invalidate previous tasks
        let current_id = debounce_id.get_untracked() + 1;
        set_debounce_id.set(current_id);

        spawn_local(async move {
            // Debounce delay: wait 500ms
            sleep(Duration::from_millis(500)).await;

            // Only compile if this is still the latest task
            // Use get_untracked to avoid creating reactive dependencies
            if current_id == debounce_id.get_untracked() {
                // Set compiling state
                set_is_compiling.set(true);

                // Pass the bibliography if it is not empty
                let bib_option = if bib.trim().is_empty() {
                    None
                } else {
                    Some(bib.as_str())
                };

                // Get current image cache
                let images = image_cache.get_untracked();

                let main = project.first().map(|f| f.content.clone()).unwrap_or_default();
                let extra: Vec<(String, String)> = project
                    .iter()
                    .skip(1)
                    .map(|f| (f.name.clone(), f.content.clone()))
                    .collect();

                match compile_to_svg(&main, &settings_val, bib_option, &images, &extra) {
                    Ok(svg) => {
                        // Count pages for the indicator (preserve scroll position
                        // — no scroll-to-top reset on recompile).
                        let pages = svg.matches("class=\"preview-page\"").count().max(1);
                        set_page_count.set(pages);
                        set_output.set(svg);
                        set_error.set(None);
                        package_status.set(None);
                    }
                    Err(e) => {
                        // The compile may have failed only because an `@preview`
                        // package isn't installed yet. Fetch any not-yet-tried
                        // ones and suppress the error while downloads are in
                        // flight; otherwise surface the error.
                        let pending: Vec<PackageSpec> = take_missing_packages()
                            .into_iter()
                            .filter(|s| !failed.with_value(|f| f.contains(&s.to_string())))
                            .collect();
                        if pending.is_empty() || epoch >= MAX_PACKAGE_EPOCHS {
                            log::error!("Compilation error: {}", e);
                            set_error.set(Some(e));
                            package_status.set(None);
                        } else {
                            set_error.set(None);
                            for spec in pending {
                                start_download(spec);
                            }
                        }
                    }
                }

                // Clear compiling state
                set_is_compiling.set(false);
            }
        });
    });

    view! {
        <Html attr:lang="en" attr:dir="ltr" attr:data-theme="dark" />
        <Title text="Typst Studio - Pure Rust WASM" />
        <Meta charset="UTF-8" />
        <Meta name="viewport" content="width=device-width, initial-scale=1.0" />

        <div class="flex flex-col h-screen">
            // Header with Lucide icons
            <header class="navbar bg-base-200 shadow-lg min-h-16 px-4">
                <div class="flex-1 flex items-center gap-3">
                    <img src="ty_bolt.svg" alt="Typst Studio" class="h-7 w-7" />
                    <h1 class="text-xl font-bold">"Typst Studio"</h1>
                    <span class="text-sm text-base-content/60">"(Pure Rust WASM)"</span>

                    // Compilation indicator
                    {move || {
                        is_compiling
                            .get()
                            .then(|| {
                                view! {
                                    <span class="flex items-center gap-2 text-sm text-info">
                                        <span class="loading loading-spinner loading-sm"></span>
                                        "Compiling..."
                                    </span>
                                }
                            })
                    }}

                    // Package download indicator (@preview fetch in progress)
                    {move || {
                        package_status
                            .get()
                            .map(|msg| {
                                view! {
                                    <span class="flex items-center gap-2 text-sm text-warning">
                                        <span class="loading loading-spinner loading-sm"></span>
                                        {msg}
                                    </span>
                                }
                            })
                    }}

                </div>
                <div class="flex-none flex items-center gap-2">

                    // New document (template picker)
                    <button
                        class="btn btn-sm btn-ghost gap-2"
                        on:click=move |_| set_show_templates.set(true)
                    >
                        <span class="icon-[lucide--file-plus] text-lg"></span>
                        "New"
                    </button>

                    // Image Gallery button
                    <button
                        class="btn btn-sm btn-ghost gap-2"
                        on:click=move |_| set_show_image_gallery.set(true)
                    >
                        <span class="icon-[lucide--image] text-lg"></span>
                        "Images"
                    </button>

                    // Bibliography button
                    <button
                        class="btn btn-sm btn-ghost gap-2"
                        on:click=move |_| set_show_bib_modal.set(true)
                    >
                        <span class="icon-[lucide--book-open] text-lg"></span>
                        "Bibliography"
                    </button>

                    // Upload .typ file button
                    <label class="btn btn-sm btn-ghost gap-2 cursor-pointer">
                        <input
                            type="file"
                            accept=".typ"
                            class="hidden"
                            on:change=move |ev| {
                                let Some(input) = ev.target()
                                    .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                                else { return };
                                if let Some(files) = input.files() {
                                    if let Some(file) = files.get(0) {
                                        let Ok(reader) = web_sys::FileReader::new() else { return };
                                        let reader_clone = reader.clone();
                                        let onload = wasm_bindgen::closure::Closure::wrap(
                                            Box::new(move |_: web_sys::Event| {
                                                if let Ok(result) = reader_clone.result() {
                                                    if let Some(text) = result.as_string() {
                                                        set_source.set(text);
                                                    }
                                                }
                                            }) as Box<dyn FnMut(_)>,
                                        );
                                        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                                        let _ = reader.read_as_text(&file);
                                        onload.forget();
                                    }
                                }
                            }
                        />
                        <span class="icon-[lucide--upload] text-lg"></span>
                        "Upload"
                    </label>

                    // Share button: copy a URL with the source encoded in the fragment
                    <button
                        class="btn btn-sm btn-ghost gap-2"
                        on:click=move |_| {
                            let Some(url) = build_share_url(&source.get()) else { return };
                            let Some(window) = web_sys::window() else { return };
                            let promise = window.navigator().clipboard().write_text(&url);
                            spawn_local(async move {
                                let msg = match wasm_bindgen_futures::JsFuture::from(promise).await {
                                    Ok(_) => "Share link copied to clipboard!",
                                    Err(_) => "Failed to copy share link",
                                };
                                set_share_toast.set(Some(msg.to_string()));
                                sleep(Duration::from_millis(2500)).await;
                                set_share_toast.set(None);
                            });
                        }
                    >
                        <span class="icon-[lucide--share-2] text-lg"></span>
                        "Share"
                    </button>

                    // Download .typ file button
                    <button
                        class="btn btn-sm btn-ghost gap-2"
                        on:click=move |_| {
                            let content = source.get();
                            if !content.is_empty() {
                                let name = files
                                    .with_untracked(|f| f.get(active.get_untracked()).map(|x| x.name.clone()))
                                    .unwrap_or_else(|| "document.typ".to_string());
                                download_bytes(&name, "text/plain;charset=utf-8", content.as_bytes());
                            }
                        }
                    >
                        <span class="icon-[lucide--file-down] text-lg"></span>
                        "Download"
                    </button>

                    // Download SVG button
                    <button
                        class="btn btn-sm btn-ghost gap-2"
                        on:click=move |_| {
                            let svg_content = output.get();
                            if !svg_content.is_empty() && error.get().is_none() {
                                download_bytes("document.svg", "image/svg+xml", svg_content.as_bytes());
                            }
                        }
                    >
                        <span class="icon-[lucide--download] text-lg"></span>
                        "SVG"
                    </button>

                    // Download PDF button
                    <button
                        class="btn btn-sm btn-ghost gap-2"
                        on:click=move |_| {
                            let project = files.get();
                            let main = project.first().map(|f| f.content.clone()).unwrap_or_default();
                            if !main.is_empty() && error.get().is_none() {
                                let bib = bibliography.get();
                                let bib_option = if bib.is_empty() {
                                    None
                                } else {
                                    Some(bib.as_str())
                                };
                                let images = image_cache.get();
                                let settings_val = settings.get();
                                let extra: Vec<(String, String)> = project
                                    .iter()
                                    .skip(1)
                                    .map(|f| (f.name.clone(), f.content.clone()))
                                    .collect();
                                match compile_to_pdf(&main, &settings_val, bib_option, &images, &extra) {
                                    Ok(pdf_bytes) => {
                                        download_bytes("document.pdf", "application/pdf", &pdf_bytes);
                                    }
                                    Err(e) => {
                                        log::error!("PDF compilation failed: {}", e);
                                        set_error.set(Some(e));
                                    }
                                }
                            }
                        }
                    >
                        <span class="icon-[lucide--file-text] text-lg"></span>
                        "PDF"
                    </button>

                    // Theme toggle
                    <label class="swap swap-rotate">
                        // Hidden checkbox controls theme state
                        <input
                            type="checkbox"
                            class="theme-controller"
                            aria-label="Toggle dark theme"
                            checked=is_dark_theme
                            on:change=move |_| set_is_dark_theme.update(|v| *v = !*v)
                        />

                        // Sun icon (light mode - shown when dark theme is OFF)
                        <svg
                            class="swap-off h-8 w-8 fill-current"
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                        >
                            <path d="M5.64,17l-.71.71a1,1,0,0,0,0,1.41,1,1,0,0,0,1.41,0l.71-.71A1,1,0,0,0,5.64,17ZM5,12a1,1,0,0,0-1-1H3a1,1,0,0,0,0,2H4A1,1,0,0,0,5,12Zm7-7a1,1,0,0,0,1-1V3a1,1,0,0,0-2,0V4A1,1,0,0,0,12,5ZM5.64,7.05a1,1,0,0,0,.7.29,1,1,0,0,0,.71-.29,1,1,0,0,0,0-1.41l-.71-.71A1,1,0,0,0,4.93,6.34Zm12,.29a1,1,0,0,0,.7-.29l.71-.71a1,1,0,1,0-1.41-1.41L17,5.64a1,1,0,0,0,0,1.41A1,1,0,0,0,17.66,7.34ZM21,11H20a1,1,0,0,0,0,2h1a1,1,0,0,0,0-2Zm-9,8a1,1,0,0,0-1,1v1a1,1,0,0,0,2,0V20A1,1,0,0,0,12,19ZM18.36,17A1,1,0,0,0,17,18.36l.71.71a1,1,0,0,0,1.41,0,1,1,0,0,0,0-1.41ZM12,6.5A5.5,5.5,0,1,0,17.5,12,5.51,5.51,0,0,0,12,6.5Zm0,9A3.5,3.5,0,1,1,15.5,12,3.5,3.5,0,0,1,12,15.5Z" />
                        </svg>

                        // Moon icon (dark mode - shown when dark theme is ON)
                        <svg
                            class="swap-on h-8 w-8 fill-current"
                            xmlns="http://www.w3.org/2000/svg"
                            viewBox="0 0 24 24"
                        >
                            <path d="M21.64,13a1,1,0,0,0-1.05-.14,8.05,8.05,0,0,1-3.37.73A8.15,8.15,0,0,1,9.08,5.49a8.59,8.59,0,0,1,.25-2A1,1,0,0,0,8,2.36,10.14,10.14,0,1,0,22,14.05,1,1,0,0,0,21.64,13Zm-9.5,6.69A8.14,8.14,0,0,1,7.08,5.22v.27A10.15,10.15,0,0,0,17.22,15.63a9.79,9.79,0,0,0,2.1-.22A8.11,8.11,0,0,1,12.14,19.73Z" />
                        </svg>
                    </label>
                </div>
            </header>

            // Main layout with split panels
            <main
                class="flex-1 flex overflow-hidden relative min-h-0"
                on:mousemove=move |ev| {
                    if is_resizing.get() {
                        if let Some(width) = web_sys::window()
                            .and_then(|w| w.inner_width().ok())
                            .and_then(|v| v.as_f64())
                        {
                            let x = ev.client_x() as f64;
                            let percentage = (x / width) * 100.0;
                            // Limit between 20% and 80%
                            set_editor_width.set(percentage.clamp(20.0, 80.0));
                        }
                    }
                }
                on:mouseup=move |_| {
                    set_is_resizing.set(false);
                }
            >
                // Editor panel with dynamic width
                <div
                    class="overflow-hidden flex flex-col"
                    style:flex=move || format!("0 0 {}%", editor_width.get())
                >
                    // File tab bar (file 0 is the compiled entry point)
                    <div class="flex items-stretch bg-base-200 border-b border-base-300 overflow-x-auto">
                        {move || {
                            let active_idx = active.get();
                            files
                                .get()
                                .into_iter()
                                .enumerate()
                                .map(|(i, file)| {
                                    let base = "flex items-center gap-1 px-3 py-1.5 text-sm border-r border-base-300 cursor-pointer whitespace-nowrap";
                                    let cls = if i == active_idx {
                                        format!("{base} bg-base-100 text-primary font-semibold")
                                    } else {
                                        format!("{base} hover:bg-base-300/50")
                                    };
                                    view! {
                                        <div class=cls>
                                            <span
                                                title="Double-click to rename"
                                                on:click=move |_| switch_to(i)
                                                on:dblclick=move |_| open_rename(i)
                                            >
                                                {file.name.clone()}
                                            </span>
                                            <button
                                                class="icon-[lucide--x] text-xs opacity-50 hover:opacity-100"
                                                title="Delete file"
                                                aria-label="Delete file"
                                                on:click=move |_| open_delete(i)
                                            ></button>
                                        </div>
                                    }
                                })
                                .collect::<Vec<_>>()
                        }}
                        <button
                            class="px-2 py-1.5 text-base-content/60 hover:text-primary"
                            title="New file"
                            aria-label="New file"
                            on:click=open_new_file
                        >
                            <span class="icon-[lucide--plus] text-sm"></span>
                        </button>
                    </div>

                    <Editor
                        source=source
                        set_source=set_source
                        settings=settings
                        textarea_ref=textarea_ref
                        insert_at_cursor=insert_at_cursor.clone()
                        set_show_settings=set_show_settings
                        on_save=save_project
                    />
                </div>

                // Resizer handle
                <div
                    class="w-1 bg-base-300 hover:bg-primary cursor-col-resize transition-colors relative group"
                    on:mousedown=move |ev| {
                        ev.prevent_default();
                        set_is_resizing.set(true);
                    }
                >
                    // Visual indicator on hover
                    <div class="absolute inset-y-0 -left-1 -right-1 group-hover:bg-primary/20"></div>
                </div>

                // Preview panel with remaining width
                <div class="flex-1 min-h-0">
                    <Preview
                        output=output
                        error=error
                        is_compiling=is_compiling
                        zoom=zoom
                        set_zoom=set_zoom
                        current_page=current_page
                        page_count=page_count
                        set_current_page=set_current_page
                        on_jump=jump_to
                    />
                </div>
            </main>

            // Bibliography modal
            {move || {
                show_bib_modal
                    .get()
                    .then(|| {
                        view! {
                            <div class="modal modal-open" role="dialog" aria-modal="true">
                                <div class="modal-box max-w-4xl">
                                    <h3 class="font-bold text-lg flex items-center gap-2">
                                        <span class="icon-[lucide--book-open] text-xl"></span>
                                        "Bibliography Manager (YAML)"
                                    </h3>
                                    <p class="py-2 text-sm text-base-content/70">
                                        "Edit your bibliography in Hayagriva YAML format. The file will be registered as 'refs.yml' for your documents."
                                    </p>

                                    <div class="form-control">
                                        <label class="label">
                                            <span class="label-text">"Bibliography YAML (refs.yml)"</span>
                                        </label>
                                        <textarea
                                            class="textarea textarea-bordered h-96 font-mono text-sm"
                                            aria-label="Bibliography YAML"
                                            prop:value=move || bibliography.get()
                                            on:input=move |ev| {
                                                set_bibliography.set(event_target_value(&ev));
                                            }
                                            placeholder="key:\n  type: article\n  title: \"Title\"\n  author: [\"Author\"]\n  date: 2024"
                                        />
                                    </div>

                                    <div class="alert alert-info mt-4">
                                        <span class="icon-[lucide--info] text-xl"></span>
                                        <div class="text-sm">
                                            <p class="font-bold">"Usage:"</p>
                                            <ul class="list-disc list-inside mt-1">
                                                <li>"Use @key to cite entries in your document"</li>
                                                <li>"Add #bibliography(\"refs.yml\") at the end of your document"</li>
                                                <li>"Changes are saved automatically to localStorage"</li>
                                            </ul>
                                        </div>
                                    </div>

                                    <div class="modal-action">
                                        <button
                                            class="btn btn-primary gap-2"
                                            on:click=move |_| {
                                                // Save bibliography to localStorage
                                                if let Some(window) = web_sys::window() {
                                                    if let Ok(Some(storage)) = window.local_storage() {
                                                        let _ = storage.set_item("typst_bibliography", &bibliography.get());
                                                        log::info!("Bibliography saved to localStorage");
                                                    }
                                                }
                                                set_show_bib_modal.set(false);
                                            }
                                        >
                                            <span class="icon-[lucide--save] text-lg"></span>
                                            "Save & Close"
                                        </button>
                                        <button
                                            class="btn btn-ghost"
                                            on:click=move |_| set_show_bib_modal.set(false)
                                        >
                                            "Cancel"
                                        </button>
                                    </div>
                                </div>
                                <div
                                    class="modal-backdrop"
                                    on:click=move |_| set_show_bib_modal.set(false)
                                ></div>
                            </div>
                        }
                    })
            }}

            // Document settings modal (hidden #set preamble, applied under the hood)
            {move || {
                show_settings
                    .get()
                    .then(|| {
                        view! {
                            <div class="modal modal-open" role="dialog" aria-modal="true">
                                <div class="modal-box max-w-3xl">
                                    <h3 class="font-bold text-lg flex items-center gap-2">
                                        <span class="icon-[lucide--settings] text-xl"></span>
                                        "Document Settings"
                                    </h3>
                                    <p class="py-2 text-sm text-base-content/70">
                                        "These #set rules are applied to your document at compile time without appearing in the editor. Click a preset to add it, or edit the rules directly."
                                    </p>

                                    // Quick-add presets
                                    <div class="flex flex-wrap gap-2 my-2">
                                        <button class="btn btn-xs" on:click=move |_| add_setting("#set page(paper: \"a4\", margin: 2cm, numbering: \"1\")")>"Page"</button>
                                        <button class="btn btn-xs" on:click=move |_| add_setting("#set text(font: \"Libertinus Serif\", size: 11pt, lang: \"en\")")>"Text"</button>
                                        <button class="btn btn-xs" on:click=move |_| add_setting("#set par(justify: true, leading: 0.65em)")>"Paragraph"</button>
                                        <button class="btn btn-xs" on:click=move |_| add_setting("#set heading(numbering: \"1.1\")")>"Heading"</button>
                                        <button class="btn btn-xs" on:click=move |_| add_setting("#set math.equation(numbering: \"(1)\")")>"Math"</button>
                                        <button class="btn btn-xs" on:click=move |_| add_setting("#set enum(numbering: \"1.a.\")")>"Enum"</button>
                                        <button class="btn btn-xs" on:click=move |_| add_setting("#set document(title: \"Title\", author: \"Author\")")>"Document"</button>
                                    </div>

                                    <textarea
                                        class="textarea textarea-bordered w-full h-64 font-mono text-sm"
                                        aria-label="Document settings (Typst #set rules)"
                                        prop:value=move || settings.get()
                                        on:input=move |ev| set_settings.set(event_target_value(&ev))
                                        placeholder="#set page(...)\n#set text(...)"
                                    />

                                    <div class="modal-action">
                                        <button
                                            class="btn btn-primary gap-2"
                                            on:click=move |_| {
                                                if let Some(window) = web_sys::window() {
                                                    if let Ok(Some(storage)) = window.local_storage() {
                                                        let _ = storage.set_item("typst_settings", &settings.get());
                                                    }
                                                }
                                                set_show_settings.set(false);
                                            }
                                        >
                                            <span class="icon-[lucide--save] text-lg"></span>
                                            "Save & Close"
                                        </button>
                                        <button
                                            class="btn btn-ghost"
                                            on:click=move |_| set_show_settings.set(false)
                                        >
                                            "Close"
                                        </button>
                                    </div>
                                </div>
                                <div
                                    class="modal-backdrop"
                                    on:click=move |_| set_show_settings.set(false)
                                ></div>
                            </div>
                        }
                    })
            }}

            // Inline file dialog (new / rename / delete) — replaces window.prompt/confirm
            {move || {
                file_dialog
                    .get()
                    .map(|dialog| {
                        let is_delete = matches!(dialog, FileDialog::Delete(_));
                        let (title, confirm_label, confirm_class) = match dialog {
                            FileDialog::New => ("New file", "Create", "btn btn-primary"),
                            FileDialog::Rename(_) => ("Rename file", "Rename", "btn btn-primary"),
                            FileDialog::Delete(_) => ("Delete file", "Delete", "btn btn-error"),
                        };
                        view! {
                            <div class="modal modal-open" role="dialog" aria-modal="true">
                                <div class="modal-box">
                                    <h3 class="font-bold text-lg">{title}</h3>
                                    {(!is_delete)
                                        .then(|| {
                                            view! {
                                                <input
                                                    autofocus=true
                                                    class="input input-bordered w-full mt-4"
                                                    aria-label="File name"
                                                    prop:value=move || dialog_input.get()
                                                    on:input=move |ev| {
                                                        dialog_input.set(event_target_value(&ev))
                                                    }
                                                    on:keydown=move |ev| {
                                                        if ev.key() == "Enter" {
                                                            apply_dialog();
                                                        }
                                                    }
                                                />
                                            }
                                        })}
                                    {is_delete
                                        .then(|| {
                                            view! {
                                                <p class="py-4">
                                                    "This file will be permanently removed."
                                                </p>
                                            }
                                        })}
                                    <div class="modal-action">
                                        <button class=confirm_class on:click=move |_| apply_dialog()>
                                            {confirm_label}
                                        </button>
                                        <button
                                            class="btn btn-ghost"
                                            on:click=move |_| file_dialog.set(None)
                                        >
                                            "Cancel"
                                        </button>
                                    </div>
                                </div>
                                <div
                                    class="modal-backdrop"
                                    on:click=move |_| file_dialog.set(None)
                                ></div>
                            </div>
                        }
                    })
            }}

            // Template picker modal — applying a template replaces the project.
            {move || {
                show_templates
                    .get()
                    .then(|| {
                        view! {
                            <div class="modal modal-open" role="dialog" aria-modal="true">
                                <div class="modal-box max-w-2xl">
                                    <h3 class="font-bold text-lg flex items-center gap-2">
                                        <span class="icon-[lucide--file-plus] text-xl"></span>
                                        "New from template"
                                    </h3>
                                    <div class="alert alert-warning mt-2">
                                        <span class="icon-[lucide--triangle-alert] text-lg"></span>
                                        <span class="text-sm">
                                            "This replaces your current project (all files)."
                                        </span>
                                    </div>

                                    <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 mt-4">
                                        <button
                                            class="btn h-auto py-4 flex-col gap-2 normal-case"
                                            on:click=move |_| apply_template(TEMPLATE_BLANK, None)
                                        >
                                            <span class="icon-[lucide--file] text-2xl"></span>
                                            <span class="font-semibold">"Blank"</span>
                                            <span class="text-xs opacity-70">"Empty document"</span>
                                        </button>
                                        <button
                                            class="btn h-auto py-4 flex-col gap-2 normal-case"
                                            on:click=move |_| apply_template(TEMPLATE_ARTICLE, None)
                                        >
                                            <span class="icon-[lucide--file-text] text-2xl"></span>
                                            <span class="font-semibold">"Article"</span>
                                            <span class="text-xs opacity-70">"Title + sections"</span>
                                        </button>
                                        <button
                                            class="btn h-auto py-4 flex-col gap-2 normal-case"
                                            on:click=move |_| {
                                                apply_template(TEMPLATE_IEEE, Some(TEMPLATE_IEEE_BIB))
                                            }
                                        >
                                            <span class="icon-[lucide--newspaper] text-2xl"></span>
                                            <span class="font-semibold">"IEEE"</span>
                                            <span class="text-xs opacity-70">"Paper + refs"</span>
                                        </button>
                                    </div>

                                    <div class="modal-action">
                                        <button
                                            class="btn btn-ghost"
                                            on:click=move |_| set_show_templates.set(false)
                                        >
                                            "Cancel"
                                        </button>
                                    </div>
                                </div>
                                <div
                                    class="modal-backdrop"
                                    on:click=move |_| set_show_templates.set(false)
                                ></div>
                            </div>
                        }
                    })
            }}

            // Image Gallery Drawer
            <ImageGalleryDrawer
                show=show_image_gallery
                set_show=set_show_image_gallery
                set_image_cache=set_image_cache
            />

            // Share-link confirmation toast
            {move || {
                share_toast
                    .get()
                    .map(|msg| {
                        view! {
                            <div class="toast toast-end toast-bottom z-50">
                                <div class="alert alert-success">
                                    <span class="icon-[lucide--link] text-lg"></span>
                                    <span>{msg}</span>
                                </div>
                            </div>
                        }
                    })
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::utf16_to_byte;

    #[test]
    fn ascii_offsets_match_bytes() {
        assert_eq!(utf16_to_byte("hello", 0), 0);
        assert_eq!(utf16_to_byte("hello", 3), 3);
        assert_eq!(utf16_to_byte("hello", 5), 5);
    }

    #[test]
    fn multibyte_chars_map_correctly() {
        // 'é' is 2 UTF-8 bytes but 1 UTF-16 unit.
        assert_eq!(utf16_to_byte("héllo", 2), 3);
        // '😀' is 4 UTF-8 bytes and 2 UTF-16 units; offset past it lands on 'b'.
        assert_eq!(utf16_to_byte("a😀b", 3), 5);
    }

    #[test]
    fn offset_past_end_clamps_to_len() {
        assert_eq!(utf16_to_byte("hi", 99), 2);
    }
}
