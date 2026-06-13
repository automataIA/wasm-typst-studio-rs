//! Fetching and extracting `@preview` packages from `packages.typst.org`.
//!
//! Tarballs are served with `access-control-allow-origin: *`, so the browser
//! fetches them directly (no proxy). `extract_targz` is a pure gunzip/untar
//! function unit-tested on the host; `fetch_package_tarball` performs the
//! browser network request.

use flate2::read::GzDecoder;
use std::io::Read;
use tar::Archive;
use typst::syntax::package::PackageSpec;

/// The download URL for a package tarball, e.g.
/// `https://packages.typst.org/preview/cetz-0.3.1.tar.gz`.
pub fn package_url(spec: &PackageSpec) -> String {
    format!(
        "https://packages.typst.org/{}/{}-{}.tar.gz",
        spec.namespace, spec.name, spec.version
    )
}

/// Gunzip + untar a package tarball into `(path, bytes)` pairs, one per file
/// (directories skipped). Paths are relative to the package root (the tarball
/// has no leading directory), matching the `VirtualPath`s Typst requests.
pub fn extract_targz(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut archive = Archive::new(GzDecoder::new(bytes));
    let mut out = Vec::new();
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        if entry.header().entry_type().is_dir() {
            continue;
        }
        let path = entry
            .path()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into_owned();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(|e| e.to_string())?;
        out.push((path, data));
    }
    Ok(out)
}

/// Fetch a package tarball from `packages.typst.org` in the browser. Returns the
/// raw `.tar.gz` bytes (caller extracts + caches them).
pub async fn fetch_package_tarball(spec: &PackageSpec) -> Result<Vec<u8>, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, Response};

    let url = package_url(spec);
    let request = Request::new_with_str(&url).map_err(|_| "failed to build request".to_string())?;
    let window = web_sys::window().ok_or("no window")?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| format!("network error fetching {url}"))?;
    let resp: Response = resp_value
        .dyn_into()
        .map_err(|_| "unexpected fetch response".to_string())?;
    if !resp.ok() {
        return Err(format!("HTTP {} fetching {url}", resp.status()));
    }
    let buf = JsFuture::from(resp.array_buffer().map_err(|_| "no response body")?)
        .await
        .map_err(|_| "failed to read response body".to_string())?;
    Ok(js_sys::Uint8Array::new(&buf).to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::GzEncoder, Compression};
    use std::io::Write;

    /// Build an in-memory `.tar.gz` from `(path, contents)` pairs.
    fn make_targz(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (path, contents) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, *contents).unwrap();
        }
        let tar = builder.into_inner().unwrap();
        let mut enc = GzEncoder::new(Vec::new(), Compression::default());
        enc.write_all(&tar).unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn extracts_files_from_targz() {
        let targz = make_targz(&[
            ("typst.toml", b"[package]\nname = \"x\"\n"),
            ("lib.typ", b"#let hi = 1"),
        ]);
        let files = extract_targz(&targz).expect("extract");
        assert_eq!(files.len(), 2);
        let toml = files.iter().find(|(p, _)| p == "typst.toml").unwrap();
        assert!(toml.1.starts_with(b"[package]"));
        let lib = files.iter().find(|(p, _)| p == "lib.typ").unwrap();
        assert_eq!(lib.1, b"#let hi = 1");
    }
}
