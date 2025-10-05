use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::JsCast;
use crate::utils::image_manager::{ImageManager, ImageMetadata};
use std::collections::HashMap;

#[component]
pub fn ImageGalleryDrawer(
    show: ReadSignal<bool>,
    set_show: WriteSignal<bool>,
    set_image_cache: WriteSignal<HashMap<String, String>>,
) -> impl IntoView {
    let (images, set_images) = signal(Vec::<ImageMetadata>::new());
    let (uploading, set_uploading) = signal(false);
    let (toast_message, set_toast_message) = signal(Option::<String>::None);

    // Load images when drawer opens
    Effect::new(move |_| {
        if show.get() {
            spawn_local(async move {
                let manager = ImageManager::new();
                match manager.list_all_images().await {
                    Ok(imgs) => {
                        // Update image gallery display
                        set_images.set(imgs.clone());

                        // Update global image cache for compiler
                        let mut cache = HashMap::new();
                        for img in &imgs {
                            cache.insert(img.id.clone(), img.data.clone());
                        }
                        set_image_cache.set(cache);
                        log::info!("Image cache updated with {} images", imgs.len());
                    },
                    Err(e) => log::error!("Failed to load images: {}", e),
                }
            });
        }
    });

    // Handle file upload
    let handle_upload = move |ev: web_sys::Event| {
        let target = ev.target().unwrap();
        let input = target.dyn_into::<web_sys::HtmlInputElement>().unwrap();

        if let Some(files) = input.files() {
            if let Some(file) = files.get(0) {
                let filename = file.name();
                let reader = web_sys::FileReader::new().unwrap();
                let reader_clone = reader.clone();

                set_uploading.set(true);

                let onload = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::Event| {
                    if let Ok(result) = reader_clone.result() {
                        if let Some(data_url) = result.as_string() {
                            let filename_clone = filename.clone();

                            spawn_local(async move {
                                let manager = ImageManager::new();
                                match manager.store_image(&data_url, &filename_clone).await {
                                    Ok(id) => {
                                        log::info!("Image uploaded with ID: {}", id);
                                        set_toast_message.set(Some(format!("Image uploaded: {}", id)));

                                        // Reload images
                                        match manager.list_all_images().await {
                                            Ok(imgs) => set_images.set(imgs),
                                            Err(e) => log::error!("Failed to reload images: {}", e),
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Upload failed: {}", e);
                                        set_toast_message.set(Some(format!("Upload failed: {}", e)));
                                    }
                                }
                                set_uploading.set(false);
                            });
                        }
                    }
                }) as Box<dyn FnMut(_)>);

                reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                let _ = reader.read_as_data_url(&file);
                onload.forget();
            }
        }
    };

    // Copy image code to clipboard
    let copy_to_clipboard = move |id: String| {
        let code = format!("#image(\"{}\")", id);

        if let Some(window) = web_sys::window() {
            let clipboard = window.navigator().clipboard();
            let promise = clipboard.write_text(&code);

            spawn_local(async move {
                match wasm_bindgen_futures::JsFuture::from(promise).await {
                    Ok(_) => {
                        set_toast_message.set(Some(format!("Copied: {}", code)));
                        log::info!("Code copied to clipboard: {}", code);
                    }
                    Err(e) => log::error!("Failed to copy: {:?}", e),
                }
            });
        }
    };

    // Delete image
    let delete_image = move |id: String| {
        spawn_local(async move {
            let manager = ImageManager::new();
            match manager.delete_image(&id).await {
                Ok(_) => {
                    log::info!("Image deleted: {}", id);
                    set_toast_message.set(Some(format!("Deleted: {}", id)));

                    // Reload images
                    match manager.list_all_images().await {
                        Ok(imgs) => set_images.set(imgs),
                        Err(e) => log::error!("Failed to reload images: {}", e),
                    }
                }
                Err(e) => {
                    log::error!("Delete failed: {}", e);
                    set_toast_message.set(Some(format!("Delete failed: {}", e)));
                }
            }
        });
    };

    view! {
        // Drawer overlay
        <Show when=move || show.get()>
            <div class="drawer-overlay" on:click=move |_| set_show.set(false)></div>

            <div class="drawer-container">
                // Header
                <div class="drawer-header">
                    <div class="flex items-center gap-2">
                        <span class="icon-[lucide--image] text-2xl text-primary"></span>
                        <h2 class="text-xl font-bold">"Image Gallery"</h2>
                    </div>
                    <button
                        class="btn btn-sm btn-circle btn-ghost"
                        on:click=move |_| set_show.set(false)
                    >
                        <span class="icon-[lucide--x] text-xl"></span>
                    </button>
                </div>

                // Upload section
                <div class="drawer-content">
                    <div class="upload-zone">
                        <p class="text-sm text-warning font-semibold mb-3">
                            "⚠️ After uploading, refresh the page (F5) to use images in the compiler"
                        </p>
                        <label class="btn btn-primary gap-2 cursor-pointer">
                            <input
                                type="file"
                                accept="image/*"
                                class="hidden"
                                on:change=handle_upload
                                disabled=uploading
                            />
                            <Show
                                when=move || uploading.get()
                                fallback=move || view! {
                                    <span class="icon-[lucide--upload] text-xl"></span>
                                    "Upload Image"
                                }
                            >
                                <span class="loading loading-spinner"></span>
                                "Uploading..."
                            </Show>
                        </label>
                        <p class="text-sm text-base-content/60 mt-2">
                            "Images will be assigned sequential IDs (001, 002, ...)"
                        </p>
                    </div>

                    // Image gallery grid
                    <div class="mt-6">
                        <h3 class="text-lg font-semibold mb-4">
                            {move || format!("Images ({})", images.get().len())}
                        </h3>

                        <div class="image-gallery-grid">
                            <For
                                each=move || images.get()
                                key=|img| img.id.clone()
                                children=move |img: ImageMetadata| {
                                    let id_copy = img.id.clone();
                                    let id_delete = img.id.clone();
                                    let filename = img.filename.clone();
                                    let filename_title = filename.clone();

                                    view! {
                                        <div class="image-card">
                                            <div class="image-preview">
                                                <img src=img.data.clone() alt=filename.clone() />
                                            </div>

                                            <div class="image-info">
                                                <div class="badge badge-primary badge-lg">{img.id.clone()}</div>
                                                <p class="text-xs truncate mt-1" title=filename_title>
                                                    {filename}
                                                </p>
                                            </div>

                                            <div class="image-actions">
                                                <button
                                                    class="btn btn-xs btn-success gap-1"
                                                    on:click=move |_| copy_to_clipboard(id_copy.clone())
                                                    title="Copy code"
                                                >
                                                    <span class="icon-[lucide--copy] text-sm"></span>
                                                    "Copy"
                                                </button>
                                                <button
                                                    class="btn btn-xs btn-error gap-1"
                                                    on:click=move |_| delete_image(id_delete.clone())
                                                    title="Delete image"
                                                >
                                                    <span class="icon-[lucide--trash-2] text-sm"></span>
                                                    "Delete"
                                                </button>
                                            </div>
                                        </div>
                                    }
                                }
                            />
                        </div>

                        <Show when=move || images.get().is_empty()>
                            <div class="text-center py-12 text-base-content/50">
                                <span class="icon-[lucide--image-off] text-5xl block mb-4 opacity-30"></span>
                                <p>"No images uploaded yet"</p>
                                <p class="text-sm">"Upload your first image to get started"</p>
                            </div>
                        </Show>
                    </div>
                </div>
            </div>

            // Toast notification
            <Show when=move || toast_message.get().is_some()>
                <div class="toast toast-top toast-end">
                    <div class="alert alert-success">
                        <span>{toast_message.get().unwrap_or_default()}</span>
                        <button
                            class="btn btn-xs btn-circle btn-ghost"
                            on:click=move |_| set_toast_message.set(None)
                        >
                            "×"
                        </button>
                    </div>
                </div>
            </Show>
        </Show>
    }
}
