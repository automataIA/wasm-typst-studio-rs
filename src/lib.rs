use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_meta::*;
use gloo_timers::future::sleep;
use std::time::Duration;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::JsCast;

// Modules
mod components;
mod compiler;
mod utils;

// Top-Level components
use crate::components::{Editor, Preview, ImageGalleryDrawer};
use crate::compiler::TypstCompiler;

/// Typst Studio main app component
#[component]
pub fn App() -> impl IntoView {
    // Provides context that manages stylesheets, titles, meta tags, etc.
    provide_meta_context();

    // Theme state: true = dark, false = business (light)
    let (is_dark_theme, set_is_dark_theme) = signal(true);

    // Effect to update data-theme attribute on <html>
    Effect::new(move |_| {
        let theme = if is_dark_theme.get() { "dark" } else { "business" };

        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(html) = document.document_element() {
                    let _ = html.set_attribute("data-theme", theme);
                }
            }
        }
    });

    // Editor state
    // Load from localStorage or use default (full example.typ content)
    const DEFAULT_SOURCE: &str = include_str!("../examples/example.typ");

    let initial_source = if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            storage
                .get_item("typst_source")
                .ok()
                .flatten()
                .unwrap_or_else(|| DEFAULT_SOURCE.to_string())
        } else {
            DEFAULT_SOURCE.to_string()
        }
    } else {
        DEFAULT_SOURCE.to_string()
    };

    let (source, set_source) = signal(initial_source);

    let (output, set_output) = signal(String::new());
    let (error, set_error) = signal(Option::<String>::None);
    let (is_compiling, set_is_compiling) = signal(false);

    // Panel resize state (editor width percentage, default 50%)
    let (editor_width, set_editor_width) = signal(50.0);
    let (is_resizing, set_is_resizing) = signal(false);

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

    // Image gallery drawer state
    let (show_image_gallery, set_show_image_gallery) = signal(false);

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

    // Helper function to insert text at cursor position (wrapped in Rc for sharing)
    let insert_at_cursor = Rc::new(move |text: &str, select_text: Option<&str>| {
        if let Some(textarea) = textarea_ref.get() {
            let start = textarea.selection_start().unwrap_or(None).unwrap_or(0) as usize;
            let end = textarea.selection_end().unwrap_or(None).unwrap_or(0) as usize;

            let current = source.get_untracked();
            let before = &current[..start];
            let after = &current[end..];

            // If there's selected text, wrap it
            if start != end {
                let selected = &current[start..end];
                let new_text = text.replace("text", selected)
                                   .replace("Heading", selected)
                                   .replace("Item", selected)
                                   .replace("formula", selected)
                                   .replace("code", selected)
                                   .replace("citation", selected)
                                   .replace("label", selected);
                let new_source = format!("{}{}{}", before, new_text, after);
                set_source.set(new_source);

                // Restore selection
                let _ = textarea.set_selection_start(Some(start as u32));
                let _ = textarea.set_selection_end(Some((start + new_text.len()) as u32));
            } else {
                // No selection, insert template
                let new_source = format!("{}{}{}", before, text, after);
                set_source.set(new_source);

                // Select placeholder text if specified
                if let Some(placeholder) = select_text {
                    if let Some(pos) = text.find(placeholder) {
                        let _ = textarea.set_selection_start(Some((start + pos) as u32));
                        let _ = textarea.set_selection_end(Some((start + pos + placeholder.len()) as u32));
                    }
                } else {
                    let _ = textarea.set_selection_start(Some((start + text.len()) as u32));
                }
            }

            let _ = textarea.focus();
        }
    });

    // Save to localStorage on source change
    Effect::new(move |_| {
        let src = source.get();
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set_item("typst_source", &src);
            }
        }
    });

    // Debouncing: usa un counter per identificare l'ultimo update
    let (debounce_id, set_debounce_id) = signal(0u32);

    // Debounced compilation: compila dopo 500ms dall'ultimo cambiamento
    Effect::new(move |_| {
        let src = source.get();
        let bib = bibliography.get();

        // Incrementa l'ID per invalidare task precedenti
        let current_id = debounce_id.get_untracked() + 1;
        set_debounce_id.set(current_id);

        spawn_local(async move {
            // Debounce delay: aspetta 500ms
            sleep(Duration::from_millis(500)).await;

            // Compila solo se questo è ancora l'ultimo task
            // Usa get_untracked per non creare dipendenze reattive
            if current_id == debounce_id.get_untracked() {
                // Set compiling state
                set_is_compiling.set(true);

                // Passa la bibliografia se non è vuota
                let bib_option = if bib.trim().is_empty() {
                    None
                } else {
                    Some(bib.as_str())
                };

                // Get current image cache
                let images = image_cache.get_untracked();

                match TypstCompiler::new()
                    .and_then(|compiler| compiler.compile_to_svg(&src, bib_option, &images))
                {
                    Ok(svg) => {
                        set_output.set(svg);
                        set_error.set(None);

                        // Auto-scroll preview to top after compilation
                        if let Some(window) = web_sys::window() {
                            if let Some(document) = window.document() {
                                // Scroll the preview container to top
                                if let Some(preview_container) = document.query_selector(".flex-1.overflow-auto.bg-base-100").ok().flatten() {
                                    preview_container.set_scroll_top(0);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Compilation error: {}", e);
                        set_error.set(Some(e));
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
            // Header con icone Lucide
            <header class="navbar bg-base-200 shadow-lg min-h-16 px-4">
                <div class="flex-1 flex items-center gap-3">
                    <span class="icon-[lucide--file-type] text-2xl text-primary"></span>
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

                </div>
                <div class="flex-none flex items-center gap-2">

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
                                let target = ev.target().unwrap();
                                let input = target.dyn_into::<web_sys::HtmlInputElement>().unwrap();
                                if let Some(files) = input.files() {
                                    if let Some(file) = files.get(0) {
                                        let reader = web_sys::FileReader::new().unwrap();
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

                    // Download .typ file button
                    <button
                        class="btn btn-sm btn-ghost gap-2"
                        on:click=move |_| {
                            let content = source.get();
                            if !content.is_empty() {
                                if let Some(window) = web_sys::window() {
                                    if let Some(document) = window.document() {
                                        let blob_parts = js_sys::Array::new();
                                        blob_parts.push(&wasm_bindgen::JsValue::from_str(&content));
                                        let blob_options = web_sys::BlobPropertyBag::new();
                                        blob_options.set_type("text/plain;charset=utf-8");
                                        if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(
                                            &blob_parts,
                                            &blob_options,
                                        ) {
                                            let url = web_sys::Url::create_object_url_with_blob(&blob)
                                                .unwrap();
                                            if let Ok(link) = document.create_element("a") {
                                                let link = link
                                                    .dyn_into::<web_sys::HtmlAnchorElement>()
                                                    .unwrap();
                                                link.set_href(&url);
                                                link.set_download("document.typ");
                                                link.click();
                                                web_sys::Url::revoke_object_url(&url).unwrap();
                                            }
                                        }
                                    }
                                }
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
                                if let Some(window) = web_sys::window() {
                                    if let Some(document) = window.document() {
                                        let blob_parts = js_sys::Array::new();
                                        blob_parts
                                            .push(&wasm_bindgen::JsValue::from_str(&svg_content));
                                        if let Ok(blob) = web_sys::Blob::new_with_str_sequence(
                                            &blob_parts,
                                        ) {
                                            let url = web_sys::Url::create_object_url_with_blob(&blob)
                                                .unwrap();
                                            if let Ok(link) = document.create_element("a") {
                                                let link = link
                                                    .dyn_into::<web_sys::HtmlAnchorElement>()
                                                    .unwrap();
                                                link.set_href(&url);
                                                link.set_download("document.svg");
                                                link.click();
                                                web_sys::Url::revoke_object_url(&url).unwrap();
                                            }
                                        }
                                    }
                                }
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
                            let src = source.get();
                            if !src.is_empty() && error.get().is_none() {
                                let bib = bibliography.get();
                                let bib_option = if bib.is_empty() {
                                    None
                                } else {
                                    Some(bib.as_str())
                                };
                                let images = image_cache.get();
                                match TypstCompiler::new()
                                    .and_then(|c| c.compile_to_pdf(&src, bib_option, &images))
                                {
                                    Ok(pdf_bytes) => {
                                        if let Some(window) = web_sys::window() {
                                            if let Some(document) = window.document() {
                                                let uint8_array = js_sys::Uint8Array::from(&pdf_bytes[..]);
                                                let blob_parts = js_sys::Array::new();
                                                blob_parts.push(&uint8_array);
                                                let blob_options = web_sys::BlobPropertyBag::new();
                                                blob_options.set_type("application/pdf");
                                                if let Ok(blob) = web_sys::Blob::new_with_u8_array_sequence_and_options(
                                                    &blob_parts,
                                                    &blob_options,
                                                ) {
                                                    let url = web_sys::Url::create_object_url_with_blob(&blob)
                                                        .unwrap();
                                                    if let Ok(link) = document.create_element("a") {
                                                        let link = link
                                                            .dyn_into::<web_sys::HtmlAnchorElement>()
                                                            .unwrap();
                                                        link.set_href(&url);
                                                        link.set_download("document.pdf");
                                                        link.click();
                                                        web_sys::Url::revoke_object_url(&url).unwrap();
                                                    }
                                                }
                                            }
                                        }
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

            // Main layout con split panels
            <main
                class="flex-1 flex overflow-hidden relative min-h-0"
                on:mousemove=move |ev| {
                    if is_resizing.get() {
                        if let Some(window) = web_sys::window() {
                            let width = window.inner_width().unwrap().as_f64().unwrap();
                            let x = ev.client_x() as f64;
                            let percentage = (x / width) * 100.0;
                            // Limit between 20% and 80%
                            let clamped = percentage.clamp(20.0, 80.0);
                            set_editor_width.set(clamped);
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
                    <Editor
                        source=source
                        set_source=set_source
                        textarea_ref=textarea_ref
                        insert_at_cursor=insert_at_cursor.clone()
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
                    <Preview output=output error=error />
                </div>
            </main>

            // Bibliography modal
            {move || {
                show_bib_modal
                    .get()
                    .then(|| {
                        view! {
                            <div class="modal modal-open">
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
        </div>
    }
}
