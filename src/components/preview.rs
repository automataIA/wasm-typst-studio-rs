use crate::compiler::resolve_click;
use leptos::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::{IntersectionObserver, IntersectionObserverEntry};

/// Parse a typst SVG length attribute like `595.28pt` into points.
fn parse_pt(attr: Option<String>) -> Option<f64> {
    attr?.trim_end_matches("pt").trim().parse::<f64>().ok()
}

/// Map a click in the preview to a byte offset in the user source: find the
/// clicked `.preview-page`, scale the cursor from rendered px to typst pt, and
/// resolve it against the retained document.
fn click_to_byte(ev: &web_sys::MouseEvent) -> Option<usize> {
    let target = ev.target()?.dyn_into::<web_sys::Element>().ok()?;
    let page_el = target.closest(".preview-page").ok().flatten()?;
    let page = page_el.get_attribute("data-page")?.parse::<usize>().ok()?;
    let svg = page_el.query_selector("svg").ok().flatten()?;
    let rect = svg.get_bounding_client_rect();
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return None;
    }
    let w_pt = parse_pt(svg.get_attribute("width"))?;
    let h_pt = parse_pt(svg.get_attribute("height"))?;
    let x_pt = (ev.client_x() as f64 - rect.left()) / rect.width() * w_pt;
    let y_pt = (ev.client_y() as f64 - rect.top()) / rect.height() * h_pt;
    resolve_click(page, x_pt, y_pt)
}

#[component]
pub fn Preview(
    output: ReadSignal<String>,
    error: ReadSignal<Option<String>>,
    is_compiling: ReadSignal<bool>,
    zoom: ReadSignal<f64>,
    set_zoom: WriteSignal<f64>,
    current_page: ReadSignal<usize>,
    page_count: ReadSignal<usize>,
    set_current_page: WriteSignal<usize>,
    /// Invoked with a user-source byte offset when a glyph is clicked.
    on_jump: Callback<usize>,
) -> impl IntoView {
    // Keep the active IntersectionObserver so it can be disconnected and rebuilt
    // whenever the output changes.
    let observer = StoredValue::new_local(Option::<IntersectionObserver>::None);

    // Rebuild the observer after each successful render so the header page
    // indicator tracks the most-visible `.preview-page`.
    Effect::new(move |_| {
        let out = output.get();
        // Tear down any previous observer.
        observer.update_value(|o| {
            if let Some(obs) = o.take() {
                obs.disconnect();
            }
        });
        if out.is_empty() {
            return;
        }
        // Defer until Leptos has painted the new pages into the DOM.
        leptos::task::spawn_local(async move {
            gloo_timers::future::sleep(std::time::Duration::from_millis(50)).await;
            let Some(document) = web_sys::window().and_then(|w| w.document()) else {
                return;
            };
            let Ok(nodes) = document.query_selector_all(".preview-page") else {
                return;
            };
            if nodes.length() == 0 {
                return;
            }
            let cb = Closure::<dyn FnMut(js_sys::Array, IntersectionObserver)>::new(
                move |entries: js_sys::Array, _obs: IntersectionObserver| {
                    let mut best_ratio = 0.0;
                    let mut best_page: Option<usize> = None;
                    for entry in entries.iter() {
                        let Ok(entry) = entry.dyn_into::<IntersectionObserverEntry>() else {
                            continue;
                        };
                        let ratio = entry.intersection_ratio();
                        if ratio <= best_ratio {
                            continue;
                        }
                        if let Some(page) = entry
                            .target()
                            .get_attribute("data-page")
                            .and_then(|p| p.parse::<usize>().ok())
                        {
                            best_ratio = ratio;
                            best_page = Some(page);
                        }
                    }
                    if let Some(page) = best_page {
                        set_current_page.set(page + 1);
                    }
                },
            );
            let Ok(obs) = IntersectionObserver::new(cb.as_ref().unchecked_ref()) else {
                return;
            };
            cb.forget();
            for i in 0..nodes.length() {
                if let Some(node) = nodes.item(i) {
                    if let Ok(el) = node.dyn_into::<web_sys::Element>() {
                        obs.observe(&el);
                    }
                }
            }
            observer.update_value(|o| *o = Some(obs));
        });
    });

    let zoom_pct = move || format!("{}%", (zoom.get() * 100.0).round() as i32);

    view! {
        <div class="flex flex-col h-full">
            // Fixed header with preview controls
            <div class="flex items-center gap-2 px-4 py-3 bg-base-200 border-b border-base-300 flex-shrink-0">
                <span class="icon-[lucide--eye] text-lg text-accent"></span>
                <h2 class="text-sm font-semibold uppercase tracking-wide text-base-content/70">"Preview"</h2>

                // Page indicator (shown when there's output)
                {move || {
                    (!output.get().is_empty())
                        .then(|| {
                            view! {
                                <span class="text-sm text-base-content/60 tabular-nums">
                                    {move || format!("p. {} / {}", current_page.get(), page_count.get())}
                                </span>
                            }
                        })
                }}

                // Zoom controls
                <div class="join join-horizontal ml-auto">
                    <button
                        class="btn btn-xs join-item"
                        title="Zoom out"
                        aria-label="Zoom out"
                        on:click=move |_| set_zoom.update(|z| *z = (*z - 0.1).clamp(0.25, 4.0))
                    >
                        <span class="icon-[lucide--minus] text-sm"></span>
                    </button>
                    <button
                        class="btn btn-xs join-item tabular-nums"
                        title="Reset zoom to 100%"
                        aria-label="Reset zoom"
                        on:click=move |_| set_zoom.set(1.0)
                    >
                        {zoom_pct}
                    </button>
                    <button
                        class="btn btn-xs join-item"
                        title="Zoom in"
                        aria-label="Zoom in"
                        on:click=move |_| set_zoom.update(|z| *z = (*z + 0.1).clamp(0.25, 4.0))
                    >
                        <span class="icon-[lucide--plus] text-sm"></span>
                    </button>
                </div>
            </div>

            // Scrollable area
            <div class="preview-scroll-area bg-base-100">
                {move || {
                    if let Some(err) = error.get() {
                        // Error state — text is selectable so users can copy the message.
                        view! {
                            <div class="alert alert-error m-4 shadow-lg" role="alert">
                                <span class="icon-[lucide--alert-circle] text-2xl"></span>
                                <div>
                                    <h3 class="font-bold">"Compilation Error"</h3>
                                    <div class="text-sm whitespace-pre-wrap select-text">{err}</div>
                                </div>
                            </div>
                        }
                        .into_any()
                    } else if output.get().is_empty() {
                        if is_compiling.get() {
                            view! {
                                <div class="flex flex-col items-center justify-center h-full gap-3 text-base-content/60">
                                    <span class="loading loading-spinner loading-lg"></span>
                                    <p>"Compiling…"</p>
                                </div>
                            }
                            .into_any()
                        } else {
                            view! {
                                <div class="flex flex-col items-center justify-center h-full gap-3 text-base-content/50 text-center px-6">
                                    <span class="icon-[lucide--file-text] text-5xl opacity-30"></span>
                                    <p>"Your typeset preview will appear here"</p>
                                    <p class="text-sm">"Start writing Typst markup in the editor"</p>
                                </div>
                            }
                            .into_any()
                        }
                    } else {
                        // Real SVG content from Typst; width drives the zoom.
                        view! {
                            <div
                                class="preview-content mx-auto"
                                style:width=move || format!("{}%", zoom.get() * 100.0)
                                inner_html=move || output.get()
                                on:click=move |ev: web_sys::MouseEvent| {
                                    // Map the click to a source position and jump the editor.
                                    let Some(byte) = click_to_byte(&ev) else { return };
                                    on_jump.run(byte);
                                }
                            ></div>
                        }
                        .into_any()
                    }
                }}
            </div>
        </div>
    }
}
