//! Editor text-editing helpers.
//!
//! Programmatic edits that go through `HtmlTextAreaElement::set_value` (or
//! Leptos `prop:value`) wipe the browser's native undo stack. Routing edits
//! through `document.execCommand("insertText", …)` instead keeps Ctrl+Z working
//! because the browser records them as user edits. The pure helpers
//! (`indent_block` / `outdent_block` / `find_matches`) carry the editing logic
//! that is unit-tested on the host.

use wasm_bindgen::JsCast;
use web_sys::{HtmlDocument, HtmlTextAreaElement};

/// Insert `text` at the textarea's current selection, replacing it, while
/// preserving the native undo stack. The textarea is focused first so the
/// command targets it. This dispatches an `input` event, so reactive state that
/// listens on `on:input` stays in sync without an explicit signal write.
/// `execCommand` lives on `HtmlDocument`, so the `Document` is cast first.
pub fn insert_text(textarea: &HtmlTextAreaElement, text: &str) {
    let _ = textarea.focus();
    if let Some(doc) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.dyn_into::<HtmlDocument>().ok())
    {
        let _ = doc.exec_command_with_show_ui_and_value("insertText", false, text);
    }
}

/// Convert a UTF-16 code-unit offset (as used by DOM textarea selections) into a
/// byte offset valid for slicing a Rust `str`. Returns `s.len()` if past the end.
pub fn utf16_to_byte(s: &str, utf16_offset: usize) -> usize {
    let mut units = 0;
    for (byte_idx, ch) in s.char_indices() {
        if units >= utf16_offset {
            return byte_idx;
        }
        units += ch.len_utf16();
    }
    s.len()
}

/// Convert a byte offset into the corresponding UTF-16 code-unit offset (the
/// unit DOM textarea selection APIs expect). Offsets past the end clamp.
pub fn byte_to_utf16(s: &str, byte_offset: usize) -> usize {
    let mut units = 0;
    for (byte_idx, ch) in s.char_indices() {
        if byte_idx >= byte_offset {
            return units;
        }
        units += ch.len_utf16();
    }
    units
}

/// The textarea's current selection as UTF-16 offsets `(start, end)`.
pub fn selection(textarea: &HtmlTextAreaElement) -> (usize, usize) {
    let start = textarea.selection_start().ok().flatten().unwrap_or(0) as usize;
    let end = textarea.selection_end().ok().flatten().unwrap_or(0) as usize;
    (start, end)
}

/// Set the textarea selection from UTF-16 offsets.
pub fn set_selection(textarea: &HtmlTextAreaElement, start: usize, end: usize) {
    let _ = textarea.set_selection_start(Some(start as u32));
    let _ = textarea.set_selection_end(Some(end as u32));
}

/// Number of leading spaces a tab inserts.
pub const INDENT: &str = "  ";

/// Indent every line that the `[start, end]` byte range touches by [`INDENT`].
/// Returns `(new_text, added)` where `added` is the total number of bytes
/// inserted, so callers can shift the selection.
pub fn indent_block(text: &str, start: usize, end: usize) -> (String, usize) {
    transform_block(text, start, end, |line| {
        Some(format!("{INDENT}{line}"))
    })
}

/// Remove up to [`INDENT`]-worth of leading spaces from every line the
/// `[start, end]` range touches. Returns `(new_text, removed)` where `removed`
/// is the total bytes stripped.
pub fn outdent_block(text: &str, start: usize, end: usize) -> (String, usize) {
    transform_block(text, start, end, |line| {
        line.strip_prefix(INDENT)
            .or_else(|| line.strip_prefix(' '))
            .map(|s| s.to_string())
    })
}

/// Apply `f` to each line spanned by the byte range `[start, end]`, where `f`
/// returns the replacement line (without its trailing newline) or `None` to
/// leave it unchanged. Returns `(new_text, byte_delta)` where `byte_delta` is
/// the absolute change in length of the affected block.
fn transform_block(
    text: &str,
    start: usize,
    end: usize,
    f: impl Fn(&str) -> Option<String>,
) -> (String, usize) {
    let start = start.min(text.len());
    let end = end.min(text.len()).max(start);

    // Expand the range to whole lines.
    let block_start = text[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let block_end = text[end..].find('\n').map(|i| end + i).unwrap_or(text.len());

    let block = &text[block_start..block_end];
    let mut new_block = String::with_capacity(block.len());
    for (i, line) in block.split('\n').enumerate() {
        if i > 0 {
            new_block.push('\n');
        }
        match f(line) {
            Some(replacement) => new_block.push_str(&replacement),
            None => new_block.push_str(line),
        }
    }

    let mut result = String::with_capacity(text.len() + INDENT.len());
    result.push_str(&text[..block_start]);
    result.push_str(&new_block);
    result.push_str(&text[block_end..]);

    let delta = new_block.len().abs_diff(block.len());
    (result, delta)
}

/// The closing delimiter to auto-insert for an opening character, if any.
/// `$` and `"` are their own closers.
pub fn auto_pair_close(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '$' => Some('$'),
        '"' => Some('"'),
        _ => None,
    }
}

/// Byte ranges of every (case-sensitive) occurrence of `needle` in `haystack`.
/// Empty `needle` yields no matches.
pub fn find_matches(haystack: &str, needle: &str) -> Vec<(usize, usize)> {
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(needle) {
        let s = from + rel;
        let e = s + needle.len();
        out.push((s, e));
        from = e;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indent_single_line() {
        let (out, added) = indent_block("abc", 0, 0);
        assert_eq!(out, "  abc");
        assert_eq!(added, 2);
    }

    #[test]
    fn indent_multiple_lines() {
        let (out, added) = indent_block("a\nb\nc", 0, 5);
        assert_eq!(out, "  a\n  b\n  c");
        assert_eq!(added, 6);
    }

    #[test]
    fn indent_only_spans_selected_lines() {
        // Range only touches the middle line "b".
        let (out, _) = indent_block("a\nb\nc", 2, 3);
        assert_eq!(out, "a\n  b\nc");
    }

    #[test]
    fn outdent_removes_indent() {
        let (out, removed) = outdent_block("  a\n  b", 0, 7);
        assert_eq!(out, "a\nb");
        assert_eq!(removed, 4);
    }

    #[test]
    fn outdent_partial_and_noop_lines() {
        // First line has one space, second has none.
        let (out, _) = outdent_block(" a\nb", 0, 4);
        assert_eq!(out, "a\nb");
    }

    #[test]
    fn utf16_byte_roundtrip() {
        assert_eq!(utf16_to_byte("hello", 3), 3);
        assert_eq!(utf16_to_byte("héllo", 2), 3);
        assert_eq!(utf16_to_byte("a😀b", 3), 5);
        assert_eq!(utf16_to_byte("hi", 99), 2);
        assert_eq!(byte_to_utf16("héllo", 3), 2);
        assert_eq!(byte_to_utf16("a😀b", 5), 3);
        assert_eq!(byte_to_utf16("hello", 3), 3);
    }

    #[test]
    fn find_matches_basic() {
        assert_eq!(find_matches("ababa", "a"), vec![(0, 1), (2, 3), (4, 5)]);
        assert_eq!(find_matches("aaa", "aa"), vec![(0, 2)]);
        assert_eq!(find_matches("abc", "x"), vec![]);
        assert_eq!(find_matches("abc", ""), vec![]);
    }
}
