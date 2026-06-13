use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::html::{Input, Textarea};
use crate::compiler::{autocomplete_at, CompletionItem};
use crate::utils::highlight_typst;
use crate::utils::editing::{
    auto_pair_close, byte_to_utf16, find_matches, indent_block, insert_text, outdent_block,
    selection, set_selection, utf16_to_byte, INDENT,
};
use gloo_timers::future::sleep;
use std::time::Duration;
use wasm_bindgen::JsCast;
use web_sys::HtmlTextAreaElement;
use std::rc::Rc;

type InsertFn = Rc<dyn Fn(&str, Option<&str>)>;

// Cached monospace character width (px) of the editor font.
thread_local! {
    static CHAR_WIDTH: std::cell::Cell<Option<f64>> = const { std::cell::Cell::new(None) };
}

/// Width of one editor character, measured once via a hidden span. The editor
/// is monospace + no-wrap, so caret position is an exact `(col*cw, line*lh)` grid.
fn char_width() -> f64 {
    if let Some(w) = CHAR_WIDTH.with(|c| c.get()) {
        return w;
    }
    let measured = (|| {
        let doc = web_sys::window()?.document()?;
        let span = doc.create_element("span").ok()?;
        span.set_attribute(
            "style",
            "position:absolute;visibility:hidden;white-space:pre;\
             font-family:'Monaco','Menlo','Courier New',monospace;font-size:14px;",
        )
        .ok()?;
        span.set_text_content(Some("0000000000"));
        let body = doc.body()?;
        body.append_child(&span).ok()?;
        let el = span.dyn_ref::<web_sys::HtmlElement>()?;
        let w = el.offset_width() as f64 / 10.0;
        let _ = body.remove_child(&span);
        Some(w)
    })()
    .unwrap_or(8.43);
    CHAR_WIDTH.with(|c| c.set(Some(measured)));
    measured
}

/// Caret pixel position within the editor scroll area. Exact for a monospace,
/// no-wrap textarea: column × char-width and line × line-height, minus scroll.
fn caret_xy(ta: &HtmlTextAreaElement, source: &str, cursor_byte: usize) -> (f64, f64) {
    let before = &source[..cursor_byte.min(source.len())];
    let line = before.matches('\n').count();
    let col = before.rsplit('\n').next().unwrap_or("").chars().count();
    let cw = char_width();
    let lh = 22.4; // 1.6 × 14px, matches the CSS line-height
    let pad = 8.0; // textarea padding
    let x = pad + col as f64 * cw - ta.scroll_left() as f64;
    let y = pad + (line as f64 + 1.0) * lh - ta.scroll_top() as f64;
    (x, y)
}

#[component]
pub fn Editor(
    source: ReadSignal<String>,
    set_source: WriteSignal<String>,
    settings: ReadSignal<String>,
    textarea_ref: NodeRef<Textarea>,
    insert_at_cursor: InsertFn,
    set_show_settings: WriteSignal<bool>,
    /// Invoked on Ctrl/Cmd+S to persist the project.
    on_save: Callback<()>,
) -> impl IntoView {
    // Sync scroll between textarea, overlay and the line-number gutter.
    let sync_scroll = move |_| {
        if let Some(textarea) = textarea_ref.get() {
            let scroll_top = textarea.scroll_top();
            let scroll_left = textarea.scroll_left();

            if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                if let Some(overlay) = document
                    .query_selector(".syntax-overlay")
                    .ok()
                    .flatten()
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                {
                    overlay.set_scroll_top(scroll_top);
                    overlay.set_scroll_left(scroll_left);
                }
                if let Some(gutter) = document
                    .query_selector(".editor-gutter")
                    .ok()
                    .flatten()
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
                {
                    gutter.set_scroll_top(scroll_top);
                }
            }
        }
    };

    // Line numbers for the gutter; recomputed only when the line count changes.
    let line_numbers = Memo::new(move |_| {
        let count = source.get().lines().count().max(1);
        // A trailing newline starts a new (empty) visual line in the textarea.
        let count = if source.with(|s| s.ends_with('\n')) {
            count + 1
        } else {
            count
        };
        (1..=count).fold(String::new(), |mut acc, n| {
            if !acc.is_empty() {
                acc.push('\n');
            }
            acc.push_str(&n.to_string());
            acc
        })
    });

    // Keep the textarea value in sync with `source` WITHOUT clobbering the undo
    // stack: setting `.value` (what `prop:value` does on every change) wipes
    // browser undo, so only write when the DOM actually differs from the signal.
    // Our own edits go through execCommand and leave `.value == source`, so this
    // becomes a no-op for them and Ctrl+Z survives.
    Effect::new(move |_| {
        let s = source.get();
        if let Some(ta) = textarea_ref.get() {
            if ta.value() != s {
                ta.set_value(&s);
            }
        }
    });

    // ----- Find / replace bar state -----
    let show_find = RwSignal::new(false);
    let find_query = RwSignal::new(String::new());
    let replace_query = RwSignal::new(String::new());
    let match_idx = RwSignal::new(0usize);
    let find_input_ref = NodeRef::<Input>::new();

    // Byte ranges of all current matches; recomputed on source/query change.
    let matches = Memo::new(move |_| find_matches(&source.get(), &find_query.get()));

    // Focus the find field whenever the bar opens.
    Effect::new(move |_| {
        if show_find.get() {
            if let Some(el) = find_input_ref.get() {
                let _ = el.focus();
            }
        }
    });

    // Select the i-th match (modulo count) and scroll it into view.
    let go_to_match = move |idx: usize| {
        let ms = matches.get_untracked();
        if ms.is_empty() {
            return;
        }
        let i = idx % ms.len();
        match_idx.set(i);
        let (bs, be) = ms[i];
        let cur = source.get_untracked();
        let (u_s, u_e) = (byte_to_utf16(&cur, bs), byte_to_utf16(&cur, be));
        if let Some(ta) = textarea_ref.get() {
            let _ = ta.focus();
            set_selection(&ta, u_s, u_e);
            // Approximate scroll: line index * line-height (1.6 * 14px).
            let line = cur[..bs].matches('\n').count() as f64;
            ta.set_scroll_top(((line * 22.4) - 60.0).max(0.0) as i32);
        }
    };

    let find_next = move || {
        go_to_match(match_idx.get_untracked() + 1);
    };
    let find_prev = move || {
        let len = matches.get_untracked().len();
        if len > 0 {
            go_to_match(match_idx.get_untracked() + len - 1);
        }
    };

    // Replace the currently-highlighted match with the replacement text.
    let replace_one = move || {
        let ms = matches.get_untracked();
        if ms.is_empty() {
            return;
        }
        let i = match_idx.get_untracked().min(ms.len() - 1);
        let (bs, be) = ms[i];
        let cur = source.get_untracked();
        let (u_s, u_e) = (byte_to_utf16(&cur, bs), byte_to_utf16(&cur, be));
        if let Some(ta) = textarea_ref.get() {
            set_selection(&ta, u_s, u_e);
            insert_text(&ta, &replace_query.get_untracked());
        }
    };

    // Replace every match in a single undo-able edit (select-all + insertText).
    let replace_all = move || {
        let q = find_query.get_untracked();
        if q.is_empty() {
            return;
        }
        let cur = source.get_untracked();
        let new = cur.replace(&q, &replace_query.get_untracked());
        if let Some(ta) = textarea_ref.get() {
            let total = cur.encode_utf16().count();
            set_selection(&ta, 0, total);
            insert_text(&ta, &new);
        }
    };

    // ----- Autocomplete state (typst-ide) -----
    let completions = RwSignal::new(Vec::<CompletionItem>::new());
    let ac_open = RwSignal::new(false);
    let ac_index = RwSignal::new(0usize);
    let ac_pos = RwSignal::new((0.0_f64, 0.0_f64));
    let ac_debounce = RwSignal::new(0u32);

    // Compute completions at the caret. `explicit` = Ctrl+Space (vs. typing).
    let run_autocomplete = move |explicit: bool| {
        let Some(ta) = textarea_ref.get() else {
            return;
        };
        let cur = source.get_untracked();
        // Skip on very large docs unless explicitly requested (latency guard).
        if !explicit && cur.len() > 200_000 {
            ac_open.set(false);
            return;
        }
        let (s, e) = selection(&ta);
        if s != e {
            ac_open.set(false);
            return;
        }
        let cursor_byte = utf16_to_byte(&cur, s);
        let items = autocomplete_at(&cur, &settings.get_untracked(), cursor_byte, explicit);
        if items.is_empty() {
            ac_open.set(false);
            return;
        }
        ac_pos.set(caret_xy(&ta, &cur, cursor_byte));
        completions.set(items);
        ac_index.set(0);
        ac_open.set(true);
    };

    // Insert the highlighted completion (undo-safe), then place the caret.
    let apply_completion = move || {
        let items = completions.get_untracked();
        let idx = ac_index.get_untracked();
        let Some(item) = items.get(idx).cloned() else {
            return;
        };
        let Some(ta) = textarea_ref.get() else {
            return;
        };
        let cur = source.get_untracked();
        let (_, caret_now) = selection(&ta);
        let from_u16 = byte_to_utf16(&cur, item.replace_from.min(cur.len()));
        // Replace the already-typed prefix with the completion.
        set_selection(&ta, from_u16, caret_now);
        insert_text(&ta, &item.apply);
        let within = item.cursor_offset.unwrap_or(item.apply.len());
        let caret = from_u16 + item.apply[..within].encode_utf16().count();
        set_selection(&ta, caret, caret);
        ac_open.set(false);
    };

    // ----- Keyboard shortcuts on the textarea -----
    let on_keydown = {
        let insert = insert_at_cursor.clone();
        move |ev: web_sys::KeyboardEvent| {
            let Some(textarea) = textarea_ref.get() else {
                return;
            };
            let key = ev.key();
            let ctrl = ev.ctrl_key() || ev.meta_key();

            // While the completion dropdown is open, intercept navigation keys
            // before the editing handlers (so Tab applies a completion).
            if ac_open.get_untracked() {
                match key.as_str() {
                    "ArrowDown" => {
                        ev.prevent_default();
                        let len = completions.with_untracked(|c| c.len()).max(1);
                        ac_index.update(|i| *i = (*i + 1) % len);
                        return;
                    }
                    "ArrowUp" => {
                        ev.prevent_default();
                        let len = completions.with_untracked(|c| c.len()).max(1);
                        ac_index.update(|i| *i = (*i + len - 1) % len);
                        return;
                    }
                    "Enter" | "Tab" => {
                        ev.prevent_default();
                        apply_completion();
                        return;
                    }
                    "Escape" => {
                        ev.prevent_default();
                        ac_open.set(false);
                        return;
                    }
                    _ => {}
                }
            }

            if ctrl {
                match key.as_str() {
                    "s" | "S" => {
                        ev.prevent_default();
                        on_save.run(());
                    }
                    " " => {
                        // Ctrl+Space: explicit completion request.
                        ev.prevent_default();
                        run_autocomplete(true);
                    }
                    "f" | "F" => {
                        ev.prevent_default();
                        show_find.set(true);
                    }
                    "b" | "B" => {
                        ev.prevent_default();
                        insert("*text*", Some("text"));
                    }
                    "i" | "I" => {
                        ev.prevent_default();
                        insert("_text_", Some("text"));
                    }
                    // Leave native shortcuts (undo/copy/paste/...) untouched.
                    _ => {}
                }
                return;
            }

            if key == "Tab" {
                ev.prevent_default();
                let (s, e) = selection(&textarea);
                let cur = source.get_untracked();
                let bs = utf16_to_byte(&cur, s);
                let be = utf16_to_byte(&cur, e);
                let multiline = cur[bs..be].contains('\n');
                if !multiline && !ev.shift_key() {
                    insert_text(&textarea, INDENT);
                } else {
                    let block_start = cur[..bs].rfind('\n').map(|i| i + 1).unwrap_or(0);
                    let block_end = cur[be..].find('\n').map(|i| be + i).unwrap_or(cur.len());
                    let (new_full, _) = if ev.shift_key() {
                        outdent_block(&cur, bs, be)
                    } else {
                        indent_block(&cur, bs, be)
                    };
                    let suffix_len = cur.len() - block_end;
                    let new_block = &new_full[block_start..new_full.len() - suffix_len];
                    let u_bs = byte_to_utf16(&cur, block_start);
                    let u_be = byte_to_utf16(&cur, block_end);
                    set_selection(&textarea, u_bs, u_be);
                    insert_text(&textarea, new_block);
                    let new_len = new_block.encode_utf16().count();
                    set_selection(&textarea, u_bs, u_bs + new_len);
                }
                return;
            }

            // Single-character keys: auto-pairing and skip-over.
            let mut chars = key.chars();
            let (Some(ch), None) = (chars.next(), chars.next()) else {
                return;
            };
            let (s, e) = selection(&textarea);
            let cur = source.get_untracked();
            let bs = utf16_to_byte(&cur, s);

            // Skip over a closing bracket the user re-types in front of.
            if matches!(ch, ')' | ']' | '}') {
                if s == e && cur[bs..].starts_with(ch) {
                    ev.prevent_default();
                    set_selection(&textarea, s + 1, s + 1);
                }
                return;
            }

            if let Some(close) = auto_pair_close(ch) {
                // For self-closing pairs ($ or "), skip over an existing closer.
                if s == e && ch == close && cur[bs..].starts_with(close) {
                    ev.prevent_default();
                    set_selection(&textarea, s + 1, s + 1);
                    return;
                }
                ev.prevent_default();
                if s != e {
                    let be = utf16_to_byte(&cur, e);
                    let selected = &cur[bs..be];
                    let wrapped = format!("{ch}{selected}{close}");
                    let inner = selected.encode_utf16().count();
                    insert_text(&textarea, &wrapped);
                    set_selection(&textarea, s + 1, s + 1 + inner);
                } else {
                    insert_text(&textarea, &format!("{ch}{close}"));
                    set_selection(&textarea, s + 1, s + 1);
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
                        aria-label="Bold"
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
                        aria-label="Italic"
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
                        aria-label="Inline code"
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
                        aria-label="Heading"
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
                        aria-label="List"
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
                        aria-label="Math formula"
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
                        aria-label="Figure"
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
                        aria-label="Table"
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
                        aria-label="Citation"
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
                        aria-label="Reference"
                        on:click={
                            let insert = insert_at_cursor.clone();
                            move |_| insert("@label", Some("label"))
                        }
                    >
                        <span class="icon-[lucide--link] text-sm"></span>
                    </button>
                </div>

                <div class="divider divider-horizontal mx-0"></div>

                // Document settings (hidden #set preamble) popup
                <button
                    class="btn btn-xs"
                    title="Document settings"
                    aria-label="Document settings"
                    on:click=move |_| set_show_settings.set(true)
                >
                    <span class="icon-[lucide--settings] text-sm"></span>
                </button>
            </div>

            // Find / replace bar (Ctrl+F). Escape (handled on the field) closes
            // only this bar and returns focus to the editor.
            {move || {
                show_find
                    .get()
                    .then(|| {
                        view! {
                            <div class="flex items-center gap-1 px-2 py-1 bg-base-200 border-b border-base-300 text-sm">
                                <input
                                    node_ref=find_input_ref
                                    class="input input-xs input-bordered w-40"
                                    placeholder="Find"
                                    aria-label="Find"
                                    prop:value=move || find_query.get()
                                    on:input=move |ev| {
                                        find_query.set(event_target_value(&ev));
                                        match_idx.set(0);
                                    }
                                    on:keydown=move |ev| {
                                        match ev.key().as_str() {
                                            "Enter" => {
                                                ev.prevent_default();
                                                find_next();
                                            }
                                            "Escape" => {
                                                ev.prevent_default();
                                                show_find.set(false);
                                                if let Some(ta) = textarea_ref.get() {
                                                    let _ = ta.focus();
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                />
                                <span class="opacity-70 tabular-nums min-w-12 text-center">
                                    {move || {
                                        let t = matches.get().len();
                                        if t == 0 {
                                            "0/0".to_string()
                                        } else {
                                            format!("{}/{}", match_idx.get() + 1, t)
                                        }
                                    }}
                                </span>
                                <button
                                    class="btn btn-xs"
                                    title="Previous match"
                                    aria-label="Previous match"
                                    on:click=move |_| find_prev()
                                >
                                    <span class="icon-[lucide--chevron-up]"></span>
                                </button>
                                <button
                                    class="btn btn-xs"
                                    title="Next match"
                                    aria-label="Next match"
                                    on:click=move |_| find_next()
                                >
                                    <span class="icon-[lucide--chevron-down]"></span>
                                </button>
                                <input
                                    class="input input-xs input-bordered w-40"
                                    placeholder="Replace"
                                    aria-label="Replace"
                                    prop:value=move || replace_query.get()
                                    on:input=move |ev| replace_query.set(event_target_value(&ev))
                                />
                                <button class="btn btn-xs" on:click=move |_| replace_one()>
                                    "Replace"
                                </button>
                                <button class="btn btn-xs" on:click=move |_| replace_all()>
                                    "All"
                                </button>
                                <button
                                    class="btn btn-xs btn-ghost ml-auto"
                                    title="Close"
                                    aria-label="Close find"
                                    on:click=move |_| show_find.set(false)
                                >
                                    <span class="icon-[lucide--x]"></span>
                                </button>
                            </div>
                        }
                    })
            }}

            // Editor container with syntax highlighting
            <div class="flex-1 min-h-0 relative bg-base-100 overflow-hidden">
                <div class="editor-container h-full flex">
                    // Line-number gutter (scroll-synced with the textarea).
                    <div class="editor-gutter" aria-hidden="true">
                        {move || line_numbers.get()}
                    </div>
                    // Scroll area holding the overlay + transparent textarea.
                    <div class="editor-scroll relative flex-1">
                        // Overlay con syntax highlighting.
                        // aria-hidden: the overlay only mirrors the textarea's text for
                        // visual highlighting; exposing it to the a11y tree made screen
                        // readers announce the whole document twice.
                        <div
                            class="syntax-overlay"
                            aria-hidden="true"
                            role="presentation"
                            inner_html=move || highlight_typst(&source.get())
                        />
                        // Transparent textarea for editing
                        // `prop:value` is intentionally omitted: the guarded
                        // Effect above writes `.value` only when it differs, so
                        // undo survives. Initial value is set on mount there.
                        <textarea
                            node_ref=textarea_ref
                            class="typst-editor"
                            aria-label="Typst source editor"
                            on:input=move |ev| {
                                set_source.set(event_target_value(&ev));
                                // Debounced (200ms) autocomplete on typing.
                                let id = ac_debounce.get_untracked() + 1;
                                ac_debounce.set(id);
                                spawn_local(async move {
                                    sleep(Duration::from_millis(200)).await;
                                    if ac_debounce.get_untracked() == id {
                                        run_autocomplete(false);
                                    }
                                });
                            }
                            on:scroll=sync_scroll
                            on:keydown=on_keydown
                            on:blur=move |_| ac_open.set(false)
                            placeholder="Write Typst markup here..."
                            spellcheck="false"
                            wrap="off"
                        />
                        // Autocomplete dropdown (absolute, positioned at the caret).
                        {move || {
                            ac_open
                                .get()
                                .then(|| {
                                    let (x, y) = ac_pos.get();
                                    view! {
                                        <ul
                                            class="ac-dropdown bg-base-200 border border-base-300 rounded shadow-lg text-sm"
                                            style=format!(
                                                "position:absolute; left:{x}px; top:{y}px; z-index:20; max-height:240px; overflow-y:auto; min-width:200px;",
                                            )
                                        >
                                            {move || {
                                                let idx = ac_index.get();
                                                completions
                                                    .get()
                                                    .into_iter()
                                                    .enumerate()
                                                    .map(|(i, item)| {
                                                        let cls = if i == idx {
                                                            "flex justify-between gap-3 px-2 py-0.5 cursor-pointer bg-primary text-primary-content"
                                                        } else {
                                                            "flex justify-between gap-3 px-2 py-0.5 cursor-pointer hover:bg-base-300"
                                                        };
                                                        view! {
                                                            <li
                                                                class=cls
                                                                title=item.detail.clone().unwrap_or_default()
                                                                on:mousedown=move |ev| {
                                                                    ev.prevent_default();
                                                                    ac_index.set(i);
                                                                    apply_completion();
                                                                }
                                                            >
                                                                <span class="font-mono truncate">{item.label.clone()}</span>
                                                                <span class="opacity-50 text-xs self-center shrink-0">
                                                                    {item.kind}
                                                                </span>
                                                            </li>
                                                        }
                                                    })
                                                    .collect::<Vec<_>>()
                                            }}
                                        </ul>
                                    }
                                })
                        }}
                    </div>
                </div>
            </div>
        </div>
    }
}
