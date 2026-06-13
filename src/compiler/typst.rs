use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use typst::diag::{FileError, FileResult};
use typst::foundations::Bytes;
use typst::layout::PagedDocument;
use typst::syntax::package::PackageSpec;
use typst::syntax::{FileId, Source, Span, VirtualPath};
use typst_as_lib::file_resolver::FileResolver;
use typst_as_lib::{
    typst_kit_options::TypstKitFontOptions, TypstAsLibError, TypstEngine,
};

// A single Typst engine is built once per browser tab and reused for every
// compilation. Building the engine parses the embedded fonts (the dominant
// cost), so rebuilding it on every keystroke — as the previous implementation
// did — was the main performance bottleneck. The persistent engine also lets
// comemo retain its incremental-compilation cache between runs.
thread_local! {
    static SESSION: RefCell<Option<CompilerSession>> = const { RefCell::new(None) };
}

/// Build the `FileId` for a virtual path, matching `typst-as-lib`'s `&str`
/// conversion so `#image("001")` / `bibliography("refs.yml")` keep resolving.
fn file_id(path: &str) -> FileId {
    FileId::new(None, VirtualPath::new(path))
}

fn not_found(id: FileId) -> FileError {
    FileError::NotFound(id.vpath().as_rootless_path().to_path_buf())
}

/// Strip an optional `data:...;base64,` prefix and decode the payload to raw bytes.
fn decode_image(base64_data: &str) -> Option<Vec<u8>> {
    let payload = base64_data
        .split_once(',')
        .map(|(_, b64)| b64)
        .unwrap_or(base64_data);
    STANDARD.decode(payload).ok()
}

/// Mutable inputs served to the persistent engine: the main source, any
/// additional project `.typ` files (resolved by `#include`/`#import`), plus the
/// bibliography and referenced images.
#[derive(Default)]
struct ResolverState {
    main: Option<Source>,
    sources: HashMap<FileId, Source>,
    binaries: HashMap<FileId, Bytes>,
    /// Number of leading lines occupied by the hidden settings preamble, so
    /// error locations can be reported against the user's editor content.
    preamble_lines: usize,
    /// Byte length of the settings preamble (incl. its trailing newline), so
    /// autocomplete can map cursor offsets between user and combined source.
    preamble_bytes: usize,
    /// Installed `@preview` package files, keyed by their package `FileId`.
    /// Persisted across compiles (NOT cleared by `set_inputs`) — they are the
    /// extracted, cached package contents.
    packages: HashMap<FileId, Bytes>,
    /// Package specs the resolver couldn't satisfy during the current compile.
    /// Cleared at the start of every `set_inputs`; drained afterwards by the
    /// lib.rs retry loop, which fetches and installs them.
    missing_packages: HashSet<PackageSpec>,
}

/// A `FileResolver` reading from shared, swappable state. `typst-as-lib`
/// requires resolvers to be `Send + Sync`, hence `Arc<Mutex<_>>` (there is no
/// real contention under WASM's single thread).
struct DynamicResolver {
    state: Arc<Mutex<ResolverState>>,
    main_id: FileId,
}

impl FileResolver for DynamicResolver {
    fn resolve_binary(&self, id: FileId) -> FileResult<Cow<'_, Bytes>> {
        let mut state = self.state.lock().expect("resolver state poisoned");
        if id.package().is_some() {
            if let Some(bytes) = state.packages.get(&id).cloned() {
                return Ok(Cow::Owned(bytes));
            }
            record_missing(&mut state, id);
            return Err(not_found(id));
        }
        state
            .binaries
            .get(&id)
            .cloned()
            .map(Cow::Owned)
            .ok_or_else(|| not_found(id))
    }

    fn resolve_source(&self, id: FileId) -> FileResult<Cow<'_, Source>> {
        let mut state = self.state.lock().expect("resolver state poisoned");
        if id == self.main_id {
            if let Some(source) = state.main.clone() {
                return Ok(Cow::Owned(source));
            }
        } else if id.package().is_some() {
            if let Some(bytes) = state.packages.get(&id).cloned() {
                let text = std::str::from_utf8(&bytes)
                    .map_err(|_| FileError::InvalidUtf8)?
                    .to_owned();
                return Ok(Cow::Owned(Source::new(id, text)));
            }
            record_missing(&mut state, id);
            return Err(not_found(id));
        } else if let Some(source) = state.sources.get(&id).cloned() {
            return Ok(Cow::Owned(source));
        }
        Err(not_found(id))
    }
}

/// Record the package of a missing `FileId` so the retry loop can fetch it.
fn record_missing(state: &mut ResolverState, id: FileId) {
    if let Some(spec) = id.package() {
        state.missing_packages.insert(spec.clone());
    }
}

struct CompilerSession {
    engine: TypstEngine,
    state: Arc<Mutex<ResolverState>>,
    main_id: FileId,
    /// Last successfully compiled document, retained for IDE features
    /// (label/citation completion) and preview click-to-jump.
    last_doc: RefCell<Option<PagedDocument>>,
}

impl CompilerSession {
    fn new() -> Self {
        // The main file uses Typst's detached source id, exactly as the previous
        // `.main_file(&str)` path did, so relative paths resolve identically.
        let main_id = Source::detached(String::new()).id();
        let state = Arc::new(Mutex::new(ResolverState::default()));
        let resolver = DynamicResolver {
            state: Arc::clone(&state),
            main_id,
        };

        let mut builder = TypstEngine::builder().add_file_resolver(resolver).search_fonts_with(
            TypstKitFontOptions::default()
                .include_system_fonts(false) // not available under WASM
                .include_embedded_fonts(true),
        );
        // Retain the incremental cache across compilations instead of evicting
        // everything after each run (the crate default is `Some(0)`).
        builder.comemo_evict_max_age(Some(10));

        Self {
            engine: builder.build(),
            state,
            main_id,
            last_doc: RefCell::new(None),
        }
    }

    /// Swap the inputs served to the engine for the next compilation.
    ///
    /// `extra_files` are additional project `.typ` files keyed by their virtual
    /// path (e.g. `chapter1.typ`), reachable from the main file via
    /// `#include`/`#import`.
    fn set_inputs(
        &self,
        source: &str,
        settings: &str,
        bibliography: Option<&str>,
        images: &HashMap<String, String>,
        extra_files: &[(String, String)],
    ) {
        let mut state = self.state.lock().expect("resolver state poisoned");
        // Reset the per-compile record of unsatisfied packages (installed
        // packages in `state.packages` are deliberately retained).
        state.missing_packages.clear();
        // The settings preamble is prepended to the user's source so its `#set`
        // rules apply without appearing in the editor. We remember how many
        // lines it added so diagnostics can be reported against the user's line
        // numbers (see `format_error`).
        let (full_source, preamble_lines, preamble_bytes) = if settings.trim().is_empty() {
            (source.to_owned(), 0, 0)
        } else {
            (
                format!("{settings}\n{source}"),
                settings.matches('\n').count() + 1,
                settings.len() + 1,
            )
        };
        state.preamble_lines = preamble_lines;
        state.preamble_bytes = preamble_bytes;
        state.main = Some(Source::new(self.main_id, full_source));
        state.sources.clear();
        for (name, content) in extra_files {
            let id = file_id(name);
            state.sources.insert(id, Source::new(id, content.clone()));
        }
        state.binaries.clear();
        if let Some(bib) = bibliography {
            state
                .binaries
                .insert(file_id("refs.yml"), Bytes::new(bib.as_bytes().to_vec()));
        }
        for (id, data) in images {
            if let Some(bytes) = decode_image(data) {
                state.binaries.insert(file_id(id), Bytes::new(bytes));
            }
        }
    }

    fn compile(&self) -> Result<PagedDocument, String> {
        let result = self.engine.compile::<_, PagedDocument>(self.main_id);
        for warning in &result.warnings {
            log::warn!("Typst warning: {warning:?}");
        }
        match result.output {
            Ok(doc) => {
                // Retain the document for IDE features and preview click-to-jump.
                self.last_doc.replace(Some(doc.clone()));
                Ok(doc)
            }
            Err(err) => {
                // Resolve diagnostic spans against the main source so each error
                // can be prefixed with its `line:col` location.
                let (main, preamble_lines) = self
                    .state
                    .lock()
                    .ok()
                    .map(|s| (s.main.clone(), s.preamble_lines))
                    .unwrap_or((None, 0));
                Err(format_error(err, main.as_ref(), preamble_lines))
            }
        }
    }
}

/// Run `f` with the (lazily initialized) per-thread compiler session.
fn with_session<R>(f: impl FnOnce(&CompilerSession) -> R) -> R {
    SESSION.with(|cell| {
        let mut slot = cell.borrow_mut();
        let session = slot.get_or_insert_with(CompilerSession::new);
        f(session)
    })
}

/// Resolve a diagnostic span to a 1-based `(line, column)` within `source`.
///
/// Returns `None` for detached spans or spans belonging to another file, so
/// callers simply omit the location prefix in that case.
fn span_location(source: &Source, span: Span) -> Option<(usize, usize)> {
    let range = source.range(span)?;
    let line = source.byte_to_line(range.start)?;
    let column = source.byte_to_column(range.start)?;
    Some((line + 1, column + 1))
}

/// Turn a compilation error into a human-readable message.
///
/// For source errors this surfaces the structured diagnostic messages (and
/// hints) instead of dumping the Debug representation, prefixing each with its
/// `line:col` location when the span resolves against the main source.
fn format_error(err: TypstAsLibError, main: Option<&Source>, preamble_lines: usize) -> String {
    match err {
        TypstAsLibError::TypstSource(diagnostics) => diagnostics
            .iter()
            .map(|diag| {
                let mut line = String::new();
                if let Some((row, col)) = main.and_then(|s| span_location(s, diag.span)) {
                    // Map the row back to the user's editor: rows beyond the
                    // hidden preamble are user content; rows inside it are
                    // labelled `settings:` (guard against usize underflow).
                    if row > preamble_lines {
                        line.push_str(&format!("{}:{}: ", row - preamble_lines, col));
                    } else {
                        line.push_str(&format!("settings:{row}:{col}: "));
                    }
                }
                line.push_str(&diag.message);
                for hint in &diag.hints {
                    line.push_str("\nhint: ");
                    line.push_str(hint);
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
        other => other.to_string(),
    }
}

/// Compile Typst source to a combined multi-page SVG string.
///
/// `extra_files` are additional project `.typ` files reachable from `source`
/// via `#include`/`#import`.
pub fn compile_to_svg(
    source: &str,
    settings: &str,
    bibliography: Option<&str>,
    images: &HashMap<String, String>,
    extra_files: &[(String, String)],
) -> Result<String, String> {
    if source.trim().is_empty() {
        return Err("Source code is empty".to_string());
    }

    with_session(|session| {
        session.set_inputs(source, settings, bibliography, images, extra_files);
        let doc = session.compile()?;

        let mut combined = String::new();
        for (i, page) in doc.pages.iter().enumerate() {
            // Each page is wrapped in a `.preview-page` with its index so the UI
            // can track the visible page (IntersectionObserver) and zoom it.
            combined.push_str(&format!("<div class=\"preview-page\" data-page=\"{i}\">"));
            combined.push_str(&typst_svg::svg(page));
            combined.push_str("</div>");
        }
        Ok(combined)
    })
}

/// Compile Typst source to PDF bytes.
///
/// `extra_files` are additional project `.typ` files reachable from `source`
/// via `#include`/`#import`.
pub fn compile_to_pdf(
    source: &str,
    settings: &str,
    bibliography: Option<&str>,
    images: &HashMap<String, String>,
    extra_files: &[(String, String)],
) -> Result<Vec<u8>, String> {
    if source.trim().is_empty() {
        return Err("Source code is empty".to_string());
    }

    with_session(|session| {
        session.set_inputs(source, settings, bibliography, images, extra_files);
        let doc = session.compile()?;
        typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default())
            .map_err(|e| format!("PDF generation error: {e:?}"))
    })
}

/// Install an extracted `@preview` package into the persistent engine: every
/// `(path, bytes)` is stored under its package `FileId` so the resolver can
/// serve it on the next compile. `comemo` is evicted so a read that previously
/// failed with `NotFound` (and was memoized) is retried against the new files.
pub fn install_package(spec: &PackageSpec, files: Vec<(String, Vec<u8>)>) {
    with_session(|session| {
        let mut state = session.state.lock().expect("resolver state poisoned");
        for (path, data) in files {
            let id = FileId::new(Some(spec.clone()), VirtualPath::new(&path));
            state.packages.insert(id, Bytes::new(data));
        }
    });
    // Drop memoized NotFound results so the freshly-installed files are seen.
    typst::comemo::evict(0);
}

/// Drain the set of package specs the last compile could not resolve.
pub fn take_missing_packages() -> Vec<PackageSpec> {
    with_session(|session| {
        let mut state = session.state.lock().expect("resolver state poisoned");
        state.missing_packages.drain().collect()
    })
}

/// A single autocomplete suggestion, mapped into user-source coordinates.
#[derive(Clone)]
pub struct CompletionItem {
    /// Text shown in the dropdown.
    pub label: String,
    /// Plain text inserted when chosen (snippet placeholders stripped).
    pub apply: String,
    /// Caret offset within `apply` (first placeholder), if any.
    pub cursor_offset: Option<usize>,
    /// Category, for the dropdown icon.
    pub kind: &'static str,
    /// Optional one-line description.
    pub detail: Option<String>,
    /// Byte offset in the USER source where the inserted text replaces from.
    pub replace_from: usize,
}

fn kind_label(kind: &typst_ide::CompletionKind) -> &'static str {
    use typst_ide::CompletionKind::*;
    match kind {
        Syntax => "syntax",
        Func => "func",
        Type => "type",
        Param => "param",
        Constant => "const",
        Path => "path",
        Package => "package",
        Label => "label",
        Font => "font",
        Symbol(_) => "symbol",
    }
}

/// Compute autocomplete suggestions for `source` at `cursor_byte` (a byte offset
/// into the user's source). `settings` is the hidden preamble; offsets are
/// mapped back to user coordinates. Uses the last successfully compiled document
/// for label/citation completion. Capped at 50 items.
pub fn autocomplete_at(
    source: &str,
    settings: &str,
    cursor_byte: usize,
    explicit: bool,
) -> Vec<CompletionItem> {
    with_session(|session| {
        let state = session.state.lock().expect("resolver state poisoned");
        let (full, preamble_bytes) = if settings.trim().is_empty() {
            (source.to_owned(), 0)
        } else {
            (format!("{settings}\n{source}"), settings.len() + 1)
        };
        let main = Source::new(session.main_id, full);
        let cursor = cursor_byte + preamble_bytes;
        let doc = session.last_doc.borrow();

        let Some((offset, comps)) = crate::compiler::ide::complete(
            &main,
            session.main_id,
            &state.sources,
            &state.binaries,
            &state.packages,
            doc.as_ref(),
            cursor,
            explicit,
        ) else {
            return Vec::new();
        };

        let replace_from = offset.saturating_sub(preamble_bytes);
        // typst-ide returns the whole candidate set plus the offset where the
        // word starts; the editor filters by the already-typed prefix (LSP-style).
        let prefix = source
            .get(replace_from.min(source.len())..cursor_byte.min(source.len()))
            .unwrap_or("")
            .to_lowercase();
        comps
            .into_iter()
            .filter(|c| prefix.is_empty() || c.label.to_lowercase().starts_with(&prefix))
            .take(50)
            .map(|c| {
                let raw = c.apply.as_deref().unwrap_or(c.label.as_str());
                let (apply, cursor_offset) = crate::compiler::ide::clean_snippet(raw);
                CompletionItem {
                    label: c.label.to_string(),
                    apply,
                    cursor_offset,
                    kind: kind_label(&c.kind),
                    detail: c.detail.map(|d| d.to_string()),
                    replace_from,
                }
            })
            .collect()
    })
}

/// Resolve a click on rendered page `page` at `(x_pt, y_pt)` (in typst points)
/// back to a byte offset in the USER source, using the retained document.
/// Returns `None` for clicks that don't map to the main source.
pub fn resolve_click(page: usize, x_pt: f64, y_pt: f64) -> Option<usize> {
    with_session(|session| {
        let state = session.state.lock().ok()?;
        let main = state.main.clone()?;
        let preamble_bytes = state.preamble_bytes;
        let doc = session.last_doc.borrow();
        let doc = doc.as_ref()?;
        let frame = &doc.pages.get(page)?.frame;
        let jump = crate::compiler::ide::jump(
            &main,
            session.main_id,
            &state.sources,
            &state.binaries,
            &state.packages,
            doc,
            frame,
            x_pt,
            y_pt,
        )?;
        match jump {
            typst_ide::Jump::File(id, cursor) if id == session.main_id => {
                Some(cursor.saturating_sub(preamble_bytes))
            }
            _ => None,
        }
    })
}

/// Whether a package's files are already installed (entrypoint or otherwise).
#[cfg(test)]
pub fn is_package_installed(spec: &PackageSpec) -> bool {
    with_session(|session| {
        let state = session.state.lock().expect("resolver state poisoned");
        state.packages.keys().any(|id| id.package() == Some(spec))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_images() -> HashMap<String, String> {
        HashMap::new()
    }

    const NO_FILES: &[(String, String)] = &[];

    #[test]
    fn empty_source_is_rejected() {
        assert!(compile_to_svg("   ", "", None, &no_images(), NO_FILES).is_err());
    }

    #[test]
    fn persistent_engine_returns_fresh_output_after_edit() {
        // Both compilations share the per-thread session, so this exercises the
        // reused engine + retained comemo cache. The second result must reflect
        // the new source, not a cached copy of the first (the key R1 risk).
        let first = compile_to_svg("= Hello", "", None, &no_images(), NO_FILES).expect("first compile");
        let second =
            compile_to_svg("= Goodbye", "", None, &no_images(), NO_FILES).expect("second compile");

        assert!(!first.is_empty());
        assert!(!second.is_empty());
        // SVG encodes text as glyph paths, so compare the rendered output, not text.
        assert_ne!(first, second, "edited source produced stale output");

        // Recompiling the original source is deterministic.
        let first_again =
            compile_to_svg("= Hello", "", None, &no_images(), NO_FILES).expect("recompile");
        assert_eq!(first, first_again);
    }

    #[test]
    fn source_error_is_prefixed_with_line_and_column() {
        // `#undefined_fn()` on the second line is an unknown-variable error whose
        // span resolves against the main source, so the message must carry a
        // `line:col` prefix (here line 2).
        let err = compile_to_svg("Hello\n#undefined_fn()", "", None, &no_images(), NO_FILES)
            .expect_err("undefined function should fail");
        assert!(
            err.starts_with("2:"),
            "expected a `2:col` location prefix, got: {err}"
        );
    }

    #[test]
    fn settings_preamble_does_not_shift_user_error_lines() {
        // With a non-empty settings preamble, an error in the user's content
        // must still be reported at the editor line (2), not the combined line.
        let err = compile_to_svg(
            "Hello\n#undefined_fn()",
            "#set page(numbering: \"1\")",
            None,
            &no_images(),
            NO_FILES,
        )
        .expect_err("undefined function should fail");
        assert!(
            err.starts_with("2:"),
            "preamble shifted the user error line, got: {err}"
        );
    }

    #[test]
    fn bibliography_resolves_on_persistent_engine() {
        let bib = "key:\n  type: article\n  title: Title\n  author: Author\n  date: 2020\n";
        let source = "Cite @key. #bibliography(\"refs.yml\")";
        let svg =
            compile_to_svg(source, "", Some(bib), &no_images(), NO_FILES).expect("bib compile");
        assert!(!svg.is_empty());
    }

    #[test]
    fn included_file_is_resolved() {
        // The main file pulls in a second project file via `#include`; the
        // resolver must serve it from `extra_files` by its virtual path.
        let main = "= Main\n#include \"chapter1.typ\"";
        let extra = vec![("chapter1.typ".to_string(), "== Chapter One".to_string())];
        let svg = compile_to_svg(main, "", None, &no_images(), &extra).expect("multi-file compile");
        assert!(!svg.is_empty());

        // Without the extra file the include fails, proving it was really used.
        let err = compile_to_svg(main, "", None, &no_images(), NO_FILES)
            .expect_err("missing included file should fail");
        assert!(err.to_lowercase().contains("chapter1"), "got: {err}");
    }

    #[test]
    fn missing_package_recorded_then_resolves_after_install() {
        let spec: PackageSpec = "@preview/testpkg:0.1.0".parse().unwrap();
        let main = "#import \"@preview/testpkg:0.1.0\": greet\n#greet()";

        // First compile: the package isn't installed → compile fails and the
        // resolver records the missing spec for the retry loop to fetch.
        let _ = compile_to_svg(main, "", None, &no_images(), NO_FILES)
            .expect_err("missing package should fail");
        let missing = take_missing_packages();
        assert!(
            missing.iter().any(|s| s == &spec),
            "missing spec not recorded: {missing:?}"
        );

        // Install the package and recompile on the SAME persistent session.
        // This proves `comemo::evict` clears the cached NotFound so the new
        // files are seen.
        let files = vec![
            (
                "typst.toml".to_string(),
                b"[package]\nname = \"testpkg\"\nversion = \"0.1.0\"\nentrypoint = \"lib.typ\"\n"
                    .to_vec(),
            ),
            ("lib.typ".to_string(), b"#let greet() = [Hi]".to_vec()),
        ];
        assert!(!is_package_installed(&spec));
        install_package(&spec, files);
        assert!(is_package_installed(&spec));

        let svg = compile_to_svg(main, "", None, &no_images(), NO_FILES)
            .expect("compiles after package install");
        assert!(!svg.is_empty());
    }

    #[test]
    fn resolve_click_maps_glyph_to_source() {
        use typst::layout::{Frame, FrameItem, Point};

        // Recursively find the first text glyph's point (in pt), nudged onto it.
        fn find_text(frame: &Frame, origin: Point) -> Option<(f64, f64)> {
            for (pos, item) in frame.items() {
                let p = origin + *pos;
                match item {
                    FrameItem::Text(_) => return Some((p.x.to_pt() + 1.0, p.y.to_pt() - 2.0)),
                    FrameItem::Group(g) => {
                        if let Some(r) = find_text(&g.frame, p) {
                            return Some(r);
                        }
                    }
                    _ => {}
                }
            }
            None
        }

        compile_to_svg("Hello world.", "", None, &no_images(), NO_FILES).expect("compile");
        let (x, y) = with_session(|s| {
            let doc = s.last_doc.borrow();
            let frame = &doc.as_ref().unwrap().pages[0].frame;
            find_text(frame, Point::zero())
        })
        .expect("a text glyph in the document");

        let byte = resolve_click(0, x, y).expect("click should resolve to a source byte");
        // The first glyph is at/near the start of the user source.
        assert!(byte <= 5, "expected an early byte offset, got {byte}");
    }

    #[test]
    fn autocomplete_suggests_functions() {
        // Typing `#im` should offer the `image` function.
        let items = autocomplete_at("#im", "", 3, true);
        assert!(
            items
                .iter()
                .any(|i| i.label.contains("image") || i.apply.contains("image")),
            "no image completion; got: {:?}",
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn autocomplete_suggests_labels_from_document() {
        // Compile a document with a label so it's retained, then complete a
        // reference `@i` — the label `intro` must be offered.
        let settings = "#set heading(numbering: \"1.\")";
        let doc_src = "= Introduction <intro>\n\nSee @intro.";
        compile_to_svg(doc_src, settings, None, &no_images(), NO_FILES)
            .expect("compile with label");

        let src = "= Introduction <intro>\n\nSee @i";
        let cursor = src.len(); // just after `@i`
        let items = autocomplete_at(src, settings, cursor, false);
        assert!(
            items.iter().any(|i| i.label.contains("intro")),
            "no label completion; got: {:?}",
            items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn bundled_templates_compile() {
        // Every template the picker offers must compile out of the box.
        let blank = include_str!("../../templates/blank.typ");
        let article = include_str!("../../templates/article.typ");
        let ieee = include_str!("../../templates/ieee.typ");
        let ieee_bib = include_str!("../../examples/refs.yml");

        assert!(!compile_to_svg(blank, "", None, &no_images(), NO_FILES)
            .expect("blank template")
            .is_empty());
        assert!(!compile_to_svg(article, "", None, &no_images(), NO_FILES)
            .expect("article template")
            .is_empty());
        // The IEEE template cites entries from the bundled bibliography.
        assert!(!compile_to_svg(ieee, "", Some(ieee_bib), &no_images(), NO_FILES)
            .expect("ieee template")
            .is_empty());
    }
}
