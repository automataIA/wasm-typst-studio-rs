use leptos::prelude::*;
use leptos::html::Textarea;
use crate::utils::highlight_typst;
use wasm_bindgen::JsCast;
use std::rc::Rc;

type InsertFn = Rc<dyn Fn(&str, Option<&str>)>;

#[component]
pub fn Editor(
    source: ReadSignal<String>,
    set_source: WriteSignal<String>,
    textarea_ref: NodeRef<Textarea>,
    insert_at_cursor: InsertFn,
) -> impl IntoView {
    // Sync scroll between textarea and overlay
    let sync_scroll = move |_| {
        if let Some(textarea) = textarea_ref.get() {
            let scroll_top = textarea.scroll_top();
            let scroll_left = textarea.scroll_left();

            // Find overlay element and sync scroll
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Some(overlay) = document.query_selector(".syntax-overlay").ok().flatten() {
                        if let Some(overlay_elem) = overlay.dyn_ref::<web_sys::HtmlElement>() {
                            overlay_elem.set_scroll_top(scroll_top);
                            overlay_elem.set_scroll_left(scroll_left);
                        }
                    }
                }
            }
        }
    };

    view! {
        <div class="flex-1 flex flex-col overflow-hidden border-r border-base-300">
            // Header con icona e toolbar di formattazione (tutto sulla stessa riga)
            <div class="flex items-center gap-2 px-4 py-2 bg-base-200 border-b border-base-300">
                <span class="icon-[lucide--code] text-lg text-secondary"></span>
                <h2 class="text-sm font-semibold uppercase tracking-wide text-base-content/70">"Source"</h2>

                // Divider between title and toolbar
                <div class="divider divider-horizontal mx-2"></div>

                // Formatting toolbar on the same line
                // Text formatting group
                <div class="join join-horizontal">
                    <button
                        class="btn btn-xs join-item"
                        title="Bold (*text*)"
                        on:click={
                            let insert = insert_at_cursor.clone();
                            move |_| insert("*text*", Some("text"))
                        }
                    >
                        <span class="icon-[lucide--bold] text-sm"></span>
                    </button>
                    <button
                        class="btn btn-xs join-item"
                        title="Italic (_text_)"
                        on:click={
                            let insert = insert_at_cursor.clone();
                            move |_| insert("_text_", Some("text"))
                        }
                    >
                        <span class="icon-[lucide--italic] text-sm"></span>
                    </button>
                    <button
                        class="btn btn-xs join-item"
                        title="Code (`code`)"
                        on:click={
                            let insert = insert_at_cursor.clone();
                            move |_| insert("`code`", Some("code"))
                        }
                    >
                        <span class="icon-[lucide--code] text-sm"></span>
                    </button>
                </div>

                <div class="divider divider-horizontal mx-0"></div>

                // Structure group
                <div class="join join-horizontal">
                    <button
                        class="btn btn-xs join-item"
                        title="Heading (= Title)"
                        on:click={
                            let insert = insert_at_cursor.clone();
                            move |_| insert("= Heading\n", Some("Heading"))
                        }
                    >
                        <span class="icon-[lucide--heading] text-sm"></span>
                    </button>
                    <button
                        class="btn btn-xs join-item"
                        title="List (- Item)"
                        on:click={
                            let insert = insert_at_cursor.clone();
                            move |_| insert("- Item\n", Some("Item"))
                        }
                    >
                        <span class="icon-[lucide--list] text-sm"></span>
                    </button>
                    <button
                        class="btn btn-xs join-item"
                        title="Math ($ formula $)"
                        on:click={
                            let insert = insert_at_cursor.clone();
                            move |_| insert("$ formula $", Some("formula"))
                        }
                    >
                        <span class="icon-[lucide--square-function] text-sm"></span>
                    </button>
                </div>

                <div class="divider divider-horizontal mx-0"></div>

                // Advanced group
                <div class="join join-horizontal">
                    <button
                        class="btn btn-xs join-item"
                        title="Figure (rect placeholder)"
                        on:click={
                            let insert = insert_at_cursor.clone();
                            move |_| {
                                insert(
                                    "#figure(\n  rect(width: 80%, height: 120pt, fill: rgb(\"#e0e0e0\")),\n  caption: [Your caption here],\n)\n",
                                    Some("Your caption here"),
                                )
                            }
                        }
                    >
                        <span class="icon-[lucide--image] text-sm"></span>
                    </button>
                    <button
                        class="btn btn-xs join-item"
                        title="Table (#table)"
                        on:click={
                            let insert = insert_at_cursor.clone();
                            move |_| {
                                insert(
                                    "#table(\n  columns: 2,\n  [Header 1], [Header 2],\n  [Row 1], [Data],\n)\n",
                                    Some("Header 1"),
                                )
                            }
                        }
                    >
                        <span class="icon-[lucide--table] text-sm"></span>
                    </button>
                    <button
                        class="btn btn-xs join-item"
                        title="Citation (@ref)"
                        on:click={
                            let insert = insert_at_cursor.clone();
                            move |_| insert("@citation", Some("citation"))
                        }
                    >
                        <span class="icon-[lucide--quote] text-sm"></span>
                    </button>
                    <button
                        class="btn btn-xs join-item"
                        title="Reference (@label)"
                        on:click={
                            let insert = insert_at_cursor.clone();
                            move |_| insert("@label", Some("label"))
                        }
                    >
                        <span class="icon-[lucide--link] text-sm"></span>
                    </button>
                </div>
            </div>

            // Editor container con syntax highlighting
            <div class="flex-1 min-h-0 relative bg-base-100 overflow-hidden">
                <div class="editor-container h-full">
                    // Overlay con syntax highlighting
                    <div
                        class="syntax-overlay p-4"
                        inner_html=move || highlight_typst(&source.get())
                    />
                    // Textarea trasparente per editing
                    <textarea
                        node_ref=textarea_ref
                        class="typst-editor p-4"
                        prop:value=move || source.get()
                        on:input=move |ev| {
                            set_source.set(event_target_value(&ev));
                        }
                        on:scroll=sync_scroll
                        placeholder="Write Typst markup here..."
                        spellcheck="false"
                    />
                </div>
            </div>
        </div>
    }
}
