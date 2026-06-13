//! IDE support (autocomplete) via `typst-ide`.
//!
//! `typst-as-lib` keeps its own `World` private, so we implement a minimal
//! [`IdeWorld`] over borrowed compiler state (the same sources / binaries /
//! installed packages the resolver uses). Fonts are intentionally empty in v1
//! (no font-name completions). The `Library` and `FontBook` are built once per
//! thread and borrowed for the duration of a completion call.

use std::collections::HashMap;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::{Abs, Frame, PagedDocument, Point};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst_ide::{autocomplete, jump_from_click, Completion, IdeWorld, Jump};

thread_local! {
    static IDE_LIB: LazyHash<Library> = LazyHash::new(Library::default());
    static IDE_BOOK: LazyHash<FontBook> = LazyHash::new(FontBook::new());
}

fn not_found(id: FileId) -> FileError {
    FileError::NotFound(id.vpath().as_rootless_path().to_path_buf())
}

/// A read-only `World` over borrowed compiler state, just enough for `typst-ide`.
struct IdeWorldImpl<'a> {
    library: &'a LazyHash<Library>,
    book: &'a LazyHash<FontBook>,
    main_id: FileId,
    main: &'a Source,
    sources: &'a HashMap<FileId, Source>,
    binaries: &'a HashMap<FileId, Bytes>,
    packages: &'a HashMap<FileId, Bytes>,
}

impl World for IdeWorldImpl<'_> {
    fn library(&self) -> &LazyHash<Library> {
        self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        self.book
    }
    fn main(&self) -> FileId {
        self.main_id
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            return Ok(self.main.clone());
        }
        if let Some(s) = self.sources.get(&id) {
            return Ok(s.clone());
        }
        if id.package().is_some() {
            if let Some(b) = self.packages.get(&id) {
                let text = std::str::from_utf8(b).map_err(|_| FileError::InvalidUtf8)?;
                return Ok(Source::new(id, text.to_owned()));
            }
        }
        Err(not_found(id))
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Some(b) = self.binaries.get(&id) {
            return Ok(b.clone());
        }
        if id.package().is_some() {
            if let Some(b) = self.packages.get(&id) {
                return Ok(b.clone());
            }
        }
        Err(not_found(id))
    }
    fn font(&self, _index: usize) -> Option<Font> {
        None
    }
    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

impl IdeWorld for IdeWorldImpl<'_> {
    fn upcast(&self) -> &dyn World {
        self
    }
}

/// Run `typst-ide` autocomplete against the given state. `cursor` is a byte
/// offset into `main`. Returns the `(replace_from, completions)` pair.
#[allow(clippy::too_many_arguments)]
pub fn complete(
    main: &Source,
    main_id: FileId,
    sources: &HashMap<FileId, Source>,
    binaries: &HashMap<FileId, Bytes>,
    packages: &HashMap<FileId, Bytes>,
    document: Option<&PagedDocument>,
    cursor: usize,
    explicit: bool,
) -> Option<(usize, Vec<Completion>)> {
    IDE_LIB.with(|library| {
        IDE_BOOK.with(|book| {
            let world = IdeWorldImpl {
                library,
                book,
                main_id,
                main,
                sources,
                binaries,
                packages,
            };
            autocomplete(&world, document, main, cursor, explicit)
        })
    })
}

/// Resolve a click on a rendered page (point in pt) back to a source position
/// via `typst_ide::jump_from_click`.
#[allow(clippy::too_many_arguments)]
pub fn jump(
    main: &Source,
    main_id: FileId,
    sources: &HashMap<FileId, Source>,
    binaries: &HashMap<FileId, Bytes>,
    packages: &HashMap<FileId, Bytes>,
    document: &PagedDocument,
    frame: &Frame,
    x_pt: f64,
    y_pt: f64,
) -> Option<Jump> {
    IDE_LIB.with(|library| {
        IDE_BOOK.with(|book| {
            let world = IdeWorldImpl {
                library,
                book,
                main_id,
                main,
                sources,
                binaries,
                packages,
            };
            let click = Point::new(Abs::pt(x_pt), Abs::pt(y_pt));
            jump_from_click(&world, document, frame, click)
        })
    })
}

/// Turn a `typst-ide` snippet (`apply` field, e.g. `image(${path})`) into plain
/// text plus the byte offset where the cursor should land (the first
/// placeholder), so the editor can position the caret usefully.
pub fn clean_snippet(s: &str) -> (String, Option<usize>) {
    let mut out = String::new();
    let mut cursor = None;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' {
            // `${placeholder}` → keep the inner text, mark the caret position.
            if chars.peek() == Some(&'{') {
                chars.next();
                if cursor.is_none() {
                    cursor = Some(out.len());
                }
                while let Some(&n) = chars.peek() {
                    chars.next();
                    if n == '}' {
                        break;
                    }
                    out.push(n);
                }
                continue;
            }
            // `$0` / `$1` … → bare tab-stop, drop it but remember the caret.
            if chars.peek().is_some_and(|d| d.is_ascii_digit()) {
                chars.next();
                if cursor.is_none() {
                    cursor = Some(out.len());
                }
                continue;
            }
        }
        out.push(c);
    }
    (out, cursor)
}

#[cfg(test)]
mod tests {
    use super::clean_snippet;

    #[test]
    fn snippet_placeholder_extracted() {
        let (text, cursor) = clean_snippet("image(${path})");
        assert_eq!(text, "image(path)");
        assert_eq!(cursor, Some(6));
    }

    #[test]
    fn snippet_tab_stop_dropped() {
        let (text, cursor) = clean_snippet("strong[$0]");
        assert_eq!(text, "strong[]");
        assert_eq!(cursor, Some(7));
    }

    #[test]
    fn snippet_plain_passthrough() {
        let (text, cursor) = clean_snippet("image");
        assert_eq!(text, "image");
        assert_eq!(cursor, None);
    }
}
