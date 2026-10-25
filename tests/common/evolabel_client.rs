//! Client for layoutview.evolabel.com — the only hosted server-side DPL
//! renderer found (Evolabel's print-applicator layout preview; an
//! ~11.8-px/mm ≈ 300 dpi preview of the DPL guide's semantics, not
//! printer-verified). Mirrors the labelixa client: SHA-256 keyed disk
//! cache under `testdata/labelixa_cache/` (gitignored), rate limiting,
//! and a None-on-failure contract so callers fall back to the renderer
//! baseline.

use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

static RATE_LIMITER: Mutex<Option<Instant>> = Mutex::new(None);

const MIN_INTERVAL_MS: u128 = 600;

fn cache_dir() -> PathBuf {
    let dir = PathBuf::from("testdata/labelixa_cache");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn cache_key(dpl: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"evolabel");
    hasher.update(dpl);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

fn rate_limit() {
    let mut last = RATE_LIMITER.lock().unwrap();
    if let Some(prev) = *last {
        let elapsed = prev.elapsed().as_millis();
        if elapsed < MIN_INTERVAL_MS {
            std::thread::sleep(std::time::Duration::from_millis(
                (MIN_INTERVAL_MS - elapsed) as u64,
            ));
        }
    }
    *last = Some(Instant::now());
}

/// Hand-built multipart body — the test deps enable only reqwest's
/// `blocking` feature (no `multipart`), and the payload is one text field
/// plus three constants.
fn multipart_body(dpl: &[u8], width_mm: u32, height_mm: u32) -> Vec<u8> {
    // Field order and headers match what the form actually accepts — the
    // server rejects a `filename=` on the code field with a 400.
    let boundary = "----labelizeCorpusBoundary7351";
    let mut body = Vec::new();
    let mut field = |body: &mut Vec<u8>, name: &str, value: &[u8]| {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
        );
        body.extend_from_slice(value);
        body.extend_from_slice(b"\r\n");
    };
    field(&mut body, "code", dpl);
    field(&mut body, "width", width_mm.to_string().as_bytes());
    field(&mut body, "height", height_mm.to_string().as_bytes());
    field(&mut body, "variables", b"{}");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

/// Render DPL via the evolabel layout preview, returning the PNG bytes
/// (≈11.8 px/mm) with the declared label size in millimetres. `None` when
/// the service is unreachable, errors, or returns no rendered image —
/// callers fall back to the renderer baseline.
pub fn evolabel_render(dpl: &[u8], width_mm: u32, height_mm: u32) -> Option<Vec<u8>> {
    let key = cache_key(dpl);
    let cache_path = cache_dir().join(format!("{}.png", key));

    if let Some(cached) = std::fs::read(&cache_path).ok() {
        return Some(cached);
    }

    rate_limit();
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .ok()?;
    let resp = client
        .post("https://layoutview.evolabel.com/")
        .header(
            "Content-Type",
            "multipart/form-data; boundary=----labelizeCorpusBoundary7351",
        )
        .body(multipart_body(dpl, width_mm, height_mm))
        .send()
        .ok()?;

    if !resp.status().is_success() {
        eprintln!(
            "evolabel layout view returned status {} — falling back to renderer baseline",
            resp.status()
        );
        return None;
    }
    let html = resp.text().ok()?;
    let b64 = extract_data_png(&html)?;
    use base64::Engine;
    let png = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .ok()?;
    if png.len() < 8 || &png[..4] != b"\x89PNG" {
        eprintln!(
            "evolabel layout view returned a non-PNG body — falling back to renderer baseline"
        );
        return None;
    }

    std::fs::write(&cache_path, &png).ok();
    Some(png)
}

/// The preview embeds its render as `<img src="data:image/png;base64,…">`.
fn extract_data_png(html: &str) -> Option<&str> {
    let marker = "data:image/png;base64,";
    let start = html.find(marker)? + marker.len();
    let rest = &html[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '+' && c != '/' && c != '=')
        .unwrap_or(rest.len());
    Some(&rest[..end])
}
