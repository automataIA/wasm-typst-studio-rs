use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

/// Fragment key under which the encoded source lives: `#src=<base64>`.
const HASH_PREFIX: &str = "src=";

/// Encode source text into a URL-safe, unpadded base64 string suitable for a URL fragment.
///
/// URL-safe alphabet (`-`/`_`) and no padding (`=`) mean the result needs no further
/// percent-escaping inside a fragment.
pub fn encode_source(src: &str) -> String {
    URL_SAFE_NO_PAD.encode(src.as_bytes())
}

/// Decode a base64 (URL-safe, unpadded) fragment payload back into source text.
///
/// Returns `None` on malformed base64 or non-UTF-8 bytes so a mangled hash falls
/// through to localStorage / the default document instead of crashing.
pub fn decode_source(encoded: &str) -> Option<String> {
    let bytes = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    String::from_utf8(bytes).ok()
}

/// Read shared source from the current URL fragment (`#src=...`), if present and valid.
pub fn source_from_url() -> Option<String> {
    let hash = web_sys::window()?.location().hash().ok()?;
    let hash = hash.strip_prefix('#').unwrap_or(&hash);
    let payload = hash.strip_prefix(HASH_PREFIX)?;
    decode_source(payload)
}

/// Remove the fragment from the URL bar without adding a history entry.
///
/// Called right after consuming a shared link so subsequent reloads restore the
/// (possibly edited) localStorage content rather than the stale shared snapshot.
pub fn strip_url_fragment() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let location = window.location();
    let path = location.pathname().unwrap_or_default();
    let search = location.search().unwrap_or_default();
    let url = format!("{path}{search}");
    if let Ok(history) = window.history() {
        let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&url));
    }
}

/// Build a shareable absolute URL embedding `src` in the fragment.
pub fn build_share_url(src: &str) -> Option<String> {
    let location = web_sys::window()?.location();
    let origin = location.origin().ok()?;
    let pathname = location.pathname().ok()?;
    Some(format!(
        "{origin}{pathname}#{HASH_PREFIX}{}",
        encode_source(src)
    ))
}

#[cfg(test)]
mod tests {
    use super::{decode_source, encode_source};

    #[test]
    fn ascii_roundtrips() {
        let src = "= Title\n\nHello, Typst!";
        assert_eq!(decode_source(&encode_source(src)).as_deref(), Some(src));
    }

    #[test]
    fn unicode_roundtrips() {
        let src = "Accénts, emoji 😀 and math $x^2$";
        assert_eq!(decode_source(&encode_source(src)).as_deref(), Some(src));
    }

    #[test]
    fn malformed_base64_returns_none() {
        // '#' and '+' are not in the URL-safe alphabet.
        assert_eq!(decode_source("###"), None);
        assert_eq!(decode_source("a+b/c"), None);
    }

    #[test]
    fn invalid_utf8_returns_none() {
        // Valid base64 that decodes to bytes which are not valid UTF-8.
        let encoded = encode_source_bytes(&[0xff, 0xfe]);
        assert_eq!(decode_source(&encoded), None);
    }

    // Helper mirroring encode_source but for raw bytes, to craft invalid UTF-8.
    fn encode_source_bytes(bytes: &[u8]) -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
        URL_SAFE_NO_PAD.encode(bytes)
    }
}
