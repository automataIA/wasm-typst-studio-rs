# Changelog

All notable changes to this project are documented here.
Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/);
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> Tauri desktop-integration history lives in [CHANGELOG_TAURI.md](CHANGELOG_TAURI.md).

## [0.2.0] - 2026-06-13

### 2026-06-13 — Branding & docs

#### Added
- **App logo** (`ty_bolt.svg`) shown in the in-app header (replaces the generic
  Lucide `file-type` icon) and used as an SVG favicon (crisp at any size; `.ico`
  kept as fallback). The SVG is copied to the dist root via a Trunk `copy-file`
  hook and referenced with a relative path so it resolves under both the dev `/`
  and the GitHub Pages sub-path.
- **README screenshots**: editor/preview overview and the autocomplete dropdown
  (from `assets/`), plus the logo as the project header.

#### Changed
- `public/favicon.ico` refreshed to the new logo (was a stale 4 KB icon).
- **README rewritten for a more professional tone**: removed all emoji, added a
  badge row, table of contents, and tech-stack/artifact tables; credited Typst with
  its GitHub repository link. Fixed inaccuracies — the release build outputs to
  `dist/` (not `docs/`), and dropped links to the gitignored `info/` guides and the
  non-existent `LICENSE` file.

### 2026-06-13 — CI / GitHub Pages

#### Changed
- **Pages deploy is now fully artifact-based GitHub Actions** (no `/docs` folder
  method): `Trunk-release.toml` builds into `dist/` (ephemeral, gitignored) instead
  of `docs/`, and `.github/workflows/deploy.yml` uploads `dist/` via
  `upload-pages-artifact` + `deploy-pages`. Added `sccache` (keyed on `Cargo.lock`)
  to speed up the heavy Typst WASM build across runs. **Repo setting required:**
  Settings → Pages → Source = "GitHub Actions".

Work since the last commit (`4e3c5e6`, Tauri integration, 2025-10-13), grouped by
date and topic. Verified throughout with `cargo clippy --target wasm32-unknown-unknown
-D warnings`, `cargo test` (29 tests), and `trunk build`. The 2026-06-12 entries
(W1–W6) are the SOTA editor/feature workstreams; older entries are the prior audit.

---

### 2026-06-12 — Editor core UX (W1)

#### Added
- **Line-number gutter**: the editor is now a flex row of a scroll-synced gutter +
  the textarea/overlay scroll area. The textarea and overlay switched from
  soft-wrap (`pre-wrap`) to no-wrap (`white-space: pre`, `wrap="off"`,
  horizontal scroll) so rows line up 1:1 with gutter numbers. A `Memo` over the
  source recomputes the numbers; `sync_scroll` now also syncs the gutter's
  vertical scroll.
- **Undo-safe programmatic edits**: new `src/utils/editing.rs` with `insert_text`
  routing edits through `document.execCommand("insertText")` (on `HtmlDocument` —
  added the `HtmlDocument` web-sys feature), preserving the native undo stack.
  `prop:value` on the textarea was replaced by a guarded `Effect` that writes
  `.value` only when it differs from the signal. `insert_at_cursor` refactored
  onto this path.
- **Tab / Shift-Tab indentation**: single line inserts two spaces; a multi-line
  selection indents/outdents the whole block via pure, unit-tested
  `indent_block` / `outdent_block`, applied as one undo-able edit.
- **Ctrl/Cmd+B / +I** wrap the selection in `*…*` / `_…_`; **Ctrl/Cmd+S**
  prevents the browser dialog, force-persists the project, and flashes a "Saved"
  toast. Native shortcuts (undo/copy/paste) untouched.
- **Auto-pairing** of `() [] {} $ "`: opener inserts the pair with the cursor
  between, wraps a selection, and skips over a closer typed in front of one.
- **Find / replace bar** (Ctrl/Cmd+F): inline bar with live match count,
  prev/next (UTF-16 selection + scroll), replace-one and replace-all (single
  undo-able edit). Pure `find_matches` helper with tests; Escape closes only the
  bar.

#### Changed
- UTF-16↔byte conversion centralized in `editing.rs` (`utf16_to_byte` + new
  inverse `byte_to_utf16`); `lib.rs` imports it.

### 2026-06-12 — Template picker (W5)

#### Added
- **New-from-template modal** (header "New" button): Blank / Article / IEEE,
  bundled via `include_str!` from new `templates/*.typ`. Applying a template
  replaces the whole project with a single `main.typ` (the IEEE template also
  swaps in the bundled `examples/refs.yml` bibliography), behind a "replaces your
  current project" warning. Added to the global Escape-to-close handler.
- Host test `bundled_templates_compile` compiles all three templates via
  `compile_to_svg` (IEEE with its bibliography).

### 2026-06-12 — E2E smoke tests (W6)

#### Added
- **Playwright smoke suite** under `tests/e2e/` (`smoke.mjs` + `run.sh`), dev
  tooling **not** wired into CI. `run.sh` builds, serves on :1420, runs the
  suite, and tears down. Five scenarios, all green locally: app loads → SVG,
  edit → re-render, `#undefined_fn()` on line 2 → error contains `2:`,
  multi-file tab isolation, and `#src=` share-link roundtrip. Assertions use
  DOM/`evaluate` checks rather than brittle snapshot refs. Playwright/Chromium
  are dev-only (the no-npm-runtime rule covers the shipped bundle). `.gitignore`
  gained an exception so the suite is tracked (the repo's `tests/` was ignored).

### 2026-06-12 — @preview package support (W3)

#### Added
- **`@preview` packages now resolve** by fetching their tarballs from
  `packages.typst.org` (CORS-open, no proxy), gunzipping + untarring them in the
  browser, and installing the files into the persistent engine. New deps
  `flate2` (pure-Rust `miniz_oxide` backend) and `tar` — both verified to
  compile on `wasm32`, so the hand-rolled-ustar fallback was not needed. New
  web-sys features `Request`/`Response`/`HtmlDocument`.
- New `src/compiler/packages.rs`: `package_url`, pure `extract_targz`
  (host-tested), and `fetch_package_tarball` (browser fetch). New
  `src/utils/package_storage.rs`: an IndexedDB cache (separate DB from images)
  storing raw tarballs keyed by spec, loaded on startup so cached packages need
  no network.
- Compiler: `ResolverState` gained `packages` (persisted across compiles) and
  `missing_packages` (cleared per compile); the resolver serves package
  `FileId`s from `packages` for both source and binary reads and records the
  spec on a miss. New `install_package` (with `comemo::evict(0)` so a cached
  `NotFound` is retried) and `take_missing_packages`.
- lib.rs: a retry loop drives it — on a compile error the missing specs are
  drained and downloaded (`package_epoch` bumps to recompile), with a
  "Downloading @preview/…" header indicator, dedup via `in_flight`/`failed`
  sets, a 10-epoch cap, and error display suppressed while fetches are in
  flight. Cached packages install on startup.
- Tests: `extracts_files_from_targz` (in-memory fixture) and
  `missing_package_recorded_then_resolves_after_install` (proves install +
  comemo eviction on the persistent engine). Browser-verified end-to-end with
  `@preview/oxifmt:0.2.1`: first compile downloads + renders; a reload with the
  network blocked still renders from the IndexedDB cache.

#### Size
- Release wasm after W1+W5+W3: **23.32 MB** uncompressed / **11.40 MB** gzip
  (pre-W1 baseline was 24 MB / 11.3 MB — `flate2` + `tar` add negligible bytes).

### 2026-06-12 — Preview UX (W4)

#### Added
- **Preview zoom**: −/100%/+ controls in the preview header applied as the
  content `width` (not a transform), clamped 25–400%, persisted in
  `typst_zoom`. The scroll area now allows horizontal overflow for zoom-in.
- **Page indicator** "p. N / M": pages are wrapped in `.preview-page[data-page]`
  (compiler change), an `IntersectionObserver` (rebuilt per render) tracks the
  most-visible page, and the total comes from the page count.
- **Click-to-jump**: clicking a glyph in the preview moves the editor caret to
  the corresponding source position. The compiler retains the last
  `PagedDocument`; `resolve_click(page, x_pt, y_pt)` maps a point (via
  `typst_ide::jump_from_click`) to a user-source byte offset, and the editor
  scrolls/selects there. Click coordinates are scaled from rendered px to typst
  pt using the SVG's `width`/`height` attributes. Browser-verified: a click on
  text lands on the right line; host test `resolve_click_maps_glyph_to_source`
  proves the mapping.

#### Changed
- The preview **no longer resets scroll-to-top on recompile** — your reading
  position is preserved across edits.

### 2026-06-12 — Autocomplete (W2)

#### Added
- **Autocomplete via `typst-ide`** (new dep `typst-ide = "0.13"`, verified on
  `wasm32`). `compiler/ide.rs` implements a minimal `IdeWorld` over borrowed
  compiler state (sources / binaries / installed `@preview` packages; empty
  `FontBook` → no font-name completions in v1, thread-local `Library`). New
  `autocomplete_at(source, settings, cursor_byte, explicit)` in `compiler/typst.rs`
  runs against the combined preamble+user source, maps offsets back to user
  coordinates, filters by the typed prefix (typst-ide returns the full candidate
  set, LSP-style), strips snippet placeholders, and returns `CompletionItem`s
  (capped at 50).
- The compiler now **retains the last successfully compiled `PagedDocument`**
  (`CompilerSession.last_doc`) so label/citation completions work (and W4's
  click-to-jump can reuse it).
- Editor dropdown UI in `components/editor.rs`: caret positioning via an exact
  monospace grid (`caret_xy`, no mirror-div needed thanks to W1's no-wrap),
  200ms debounced trigger on typing + Ctrl+Space for explicit, keyboard
  nav (↑/↓/Enter/Tab/Esc, intercepted before the Tab-indent handler), and
  undo-safe apply via execCommand. New `settings` prop on `Editor`.
- Tests: `clean_snippet` (3), `autocomplete_suggests_functions` (`#im` → image),
  and `autocomplete_suggests_labels_from_document` (`@i` → `intro` from the
  retained doc). Browser-verified: `#im` dropdown lists image/import; applying
  `#image` yields `#image()` with the caret inside, and Ctrl+Z reverts it.

#### Size
- Release wasm after W2: **23.44 MB** uncompressed / **11.44 MB** gzip (+0.12 MB
  over W3 — `typst-ide` adds little since `typst-eval`/the library are already
  linked by the compiler).

---

### 2026-06-03 — Tailwind 4 / daisyUI 5 migration + GitHub Pages deploy overhaul

#### Changed
- **Styling stack migrated to Tailwind CSS 4 + daisyUI 5** (was Tailwind 3 with a v3
  `tailwind.config.js`, an officially-unsupported pairing). `tailwind.css` now uses the
  v4 CSS-first model: `@import "tailwindcss"`, `@source "./src/**/*.rs"` (so the `view!`
  macro classes are scanned), `@plugin "daisyui" { themes: business --default, dark
  --prefersdark }`, and `@plugin "@iconify/tailwind4"` for the `icon-[lucide--*]` icons.
  The existing `html[data-theme="dark"]{ --color-* }` VSCode-palette override is unchanged.
- **CSS is now compiled by the full `@tailwindcss/cli`** (Node) via a Trunk `pre_build`
  hook into `tailwind.gen.css` (gitignored), included through `data-trunk rel="css"` in
  `index.html` — **not** Trunk's bundled standalone Tailwind binary. The standalone binary's
  restricted runtime cannot load `@iconify/tailwind4` (it imports `@iconify/tools`); daisyUI
  alone loads fine, but the icons require the full CLI. `node_modules` is therefore a hard
  build dependency (`npm install` runs in CI).
- **`Trunk.toml` watch list** is now explicit (`src`, `tailwind.css`, `index.html`, `public`)
  instead of the whole project root, so the hook-generated `tailwind.gen.css` is not watched
  and `trunk serve` doesn't loop.
- **GitHub Pages workflow** (`.github/workflows/deploy.yml`) rewritten to a single job:
  installs the **prebuilt Trunk binary** (~10 s vs ~5 min for `cargo install`), builds on
  the pinned **nightly** toolchain (was `stable`), uses `Swatinem/rust-cache`, runs
  `npm install` (the lockfile is intentionally gitignored, so `npm ci` is not usable),
  triggers on `push` to `main` + `workflow_dispatch` only (dropped `pull_request`/`master`),
  drops `contents: write`, sets `concurrency: pages` with `cancel-in-progress: false`, and
  adds a `404.html` SPA fallback.
- **README** updated: dropped the removed `npm run build-css` step and the `tailwind.config.js`
  entry from the project-structure listing.

#### Fixed
- **Theme toggle now actually switches the whole UI.** The light state mapped to daisyUI's
  `business` theme, which is itself a *dark* theme, so both toggle states were dark — only
  `.editor-container` (which has explicit per-`data-theme` background overrides) visibly
  flipped while the rest of the interface, driven by daisyUI `--color-*` vars, stayed dark.
  Replaced `business` with the real `light` theme in the `@plugin` themes list, the
  `.editor-container` selector, and the `is_dark_theme` ↔ `data-theme` mapping in `lib.rs`.
- **Editor is now readable in light mode.** The syntax-overlay base text (`#d4d4d4`), every
  token color (VSCode Dark+ pastels) and the caret (`#fff`) were hardcoded for a dark
  background, so on the now-white light editor it was light-on-light / invisible. Added a
  `[data-theme="light"]` override block with the VSCode Light+ palette (dark base text, green
  comments, purple keywords, red strings, etc.) and a black caret.

#### Removed
- `tailwind.config.js` (v3 config; its content/themes/plugins now live in `tailwind.css`).
- The dead `npm run build-css` script — it wrote `public/styles.css`, which nothing consumed.
- npm deps `tailwindcss@3` and `@iconify/tailwind` (v3 plugin); added `@tailwindcss/cli@4`,
  `@iconify/tailwind4`, and kept `daisyui@5` / `@iconify-json/lucide`.

> Manual one-time repo setting (cannot be done from code): **Settings → Pages → Source =
> "GitHub Actions"**.

---

### 2026-06-03 — UI/UX audit, internationalisation, document settings

#### Added
- **Document Settings popup** (gear button in the editor toolbar). A modal with an
  editable textarea of Typst `#set` rules plus quick-add presets (Page, Text,
  Paragraph, Heading, Math, Enum, Document). The rules are applied as a **hidden
  preamble** prepended to the source at compile time, so they never appear in the
  editor. Persisted to `localStorage` (`typst_settings`); default sets equation and
  page numbering.
- **Drag-and-drop image upload** in the gallery, with an `image/*` type guard and a
  drag-over visual state.
- Preview **loading** and **empty** states; compilation error text is now selectable.
- **Theme persistence**: the choice is saved to `localStorage` and falls back to the
  OS `prefers-color-scheme` on first load.
- `Escape`-to-close and backdrop-click on all modals and the image drawer;
  `role="dialog"`/`aria-modal` and `aria-label`s across interactive controls.
- SEO metadata: `<meta name="description">`, Open Graph tags, and a `robots.txt`.
- `prefers-reduced-motion` guard for drawer/card animations.
- web-sys features `MediaQueryList`, `KeyboardEvent`, `DragEvent`, `DataTransfer`.

#### Changed
- **Theme fix**: the custom VSCode-style "dark" palette is now applied by overriding
  daisyUI's CSS theme variables (daisyUI 5 ignores the JS-config theme object),
  resolving the WCAG contrast failure (stock primary 3.39:1 → `#569CD6` ~5.9:1).
- Native `window.prompt`/`confirm` for new/rename/delete file replaced with inline
  daisyUI dialogs.
- Image-drawer colours moved from hardcoded hex to daisyUI theme tokens, so the
  drawer follows the active theme.
- The syntax-highlight overlay is now `aria-hidden`, so screen readers no longer read
  the document twice.
- `compile_to_svg` / `compile_to_pdf` gained a `settings` preamble parameter; error
  line numbers are reported against the editor content (the preamble offset is
  subtracted, with rows inside the preamble labelled `settings:`).
- Default document (`examples/example.typ`) no longer shows the global `#set` lines
  (moved into the hidden settings preamble).
- **Internationalisation**: all Italian source comments, the default-document prose
  and labels (`mia-figura` → `my-figure`), and test strings translated to English.

#### Removed
- Four stale duplicate CSS files in `public/` (`styles.css`, `output.css`,
  `styles.scss`, `tailwind.css`); the wired root `tailwind.css` is self-contained.

#### Audit result
- Live Lighthouse (desktop): Accessibility **79 → 100**, Best Practices **96 → 100**,
  SEO **82 → 100**, Agentic Browsing **33 → 67**. Full report:
  [AUDIT_UIUX_2026.md](AUDIT_UIUX_2026.md).

---

### 2026-05-31 — Audit, performance & feature refactor

#### Performance
- **Persistent Typst compiler engine** (the dominant performance win). The engine is
  built once per browser tab — embedded fonts are parsed once — instead of on every
  keystroke. A `DynamicResolver` over `Arc<Mutex<ResolverState>>` swaps the main
  source, extra files, bibliography and images per compile, and comemo's incremental
  cache is retained between runs (`comemo_evict_max_age(Some(10))`). Held in a
  `thread_local`.

#### Compiler
- Replaced the `TypstCompiler` struct with free functions `compile_to_svg` /
  `compile_to_pdf` (the struct held no state worth keeping); de-duplicated the SVG and
  PDF compile paths.
- Structured `SourceDiagnostic` error reporting (messages + hints) instead of dumping
  the `Debug` representation.
- **Inline diagnostics**: source errors are prefixed with their `line:col` location
  (`span_location` via `Source::range` + `byte_to_line`/`byte_to_column`).

#### Added — features
- **Multi-file projects** with a tab bar above the editor (file 0 is the compiled
  entry point); `#include` / `#import` of project files now resolve. New
  `utils/project.rs` (`TypstFile`, persisted to `localStorage` via `serde_json`).
- **Shareable links**: the source is encoded as URL-safe base64 in the URL fragment
  (`#src=…`) and copied to the clipboard by a Share button. Load priority is
  hash > `localStorage` > default; the fragment is consumed once and stripped via
  `history.replaceState`. New `utils/share.rs`; added web-sys `Location` + `History`.
- **Bibliography** served to the compiler as the `refs.yml` virtual file, so `@key`
  citations and `#bibliography("refs.yml")` resolve.
- `download_bytes` helper for all file downloads (new `utils/download.rs`).

#### Fixed
- Crash when inserting over a selection containing multi-byte characters: textarea
  UTF-16 selection offsets are now converted to byte offsets before slicing
  (`utf16_to_byte`, with unit tests).
- Toolbar snippet insertion replaced only the **first** placeholder occurrence
  (`replacen`) instead of all of them.
- Removed panicking `.unwrap()` calls on the download/upload/resize UI paths.
- Image cache made reactive so newly uploaded images are usable without a reload.

#### Removed / cleanup
- Deleted the tracked `rustc-ice-*.txt` compiler-crash dump and several dead functions
  (trimmed `utils/image_manager.rs` and `utils/image_storage.rs`).
- Reduced log spam (console log level `Debug` → `Info`) and removed unnecessary clones.
- Expanded `.gitignore` (build artifacts, temp/backup files, OS cruft).

#### Notes
- Binary-size investigation (release WASM ≈ 24 MB, gzip ≈ 11.3 MB; fonts ≈ 8.3 MB /
  35%) concluded that `wasm-opt`/brotli give no real gain on GitHub Pages and that a
  Web Worker split is high-cost/low-benefit after the persistent-engine win — all
  **deferred by decision**, no code change.
