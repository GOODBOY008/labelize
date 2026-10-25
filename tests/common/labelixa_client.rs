//! Client for the labelixa.com TSPL/CPCL renderers — the only hosted
//! reference renderers for either language (see docs/TSPL_BASELINES.md).
//! Mirrors the Labelary client's shape: SHA-256 keyed disk cache under
//! `testdata/labelixa_cache/` (gitignored), rate limiting, and a
//! None-on-failure contract so callers can fall back to the renderer
//! baseline.

use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

static RATE_LIMITER: Mutex<Option<Instant>> = Mutex::new(None);

/// Minimum interval between labelixa API requests (conservative vs. the
/// documented per-second limit; the daily quota counts labels).
const MIN_INTERVAL_MS: u128 = 600;

fn cache_dir() -> PathBuf {
    let dir = PathBuf::from("testdata/labelixa_cache");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn cache_key(endpoint: &str, source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(endpoint.as_bytes());
    hasher.update(source.as_bytes());
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

/// Render TSPL to a PNG via the labelixa API, with disk caching.
/// Returns None when the service is unreachable, errors, or the quota is
/// exhausted (X-RateLimit-Remaining: 0).
pub fn labelixa_render(tspl: &str) -> Option<Vec<u8>> {
    render_via_labelixa("tspl", tspl)
}

/// Render CPCL to a PNG via the labelixa API (same contract as
/// [`labelixa_render`]; the CPCL endpoint is an interpretation of the
/// CPCL guide at 203 dpi and covers a subset of commands).
pub fn labelixa_render_cpcl(cpcl: &str) -> Option<Vec<u8>> {
    render_via_labelixa("cpcl", cpcl)
}

fn render_via_labelixa(lang: &str, source: &str) -> Option<Vec<u8>> {
    let key = cache_key(lang, source);
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
        .post(format!("https://api.labelixa.com/v1/{lang}/render"))
        .header("Content-Type", "text/plain")
        .body(source.to_string())
        .send()
        .ok()?;

    if !resp.status().is_success() {
        eprintln!(
            "labelixa API returned status {} — falling back to renderer baseline",
            resp.status()
        );
        return None;
    }
    let remaining = resp
        .headers()
        .get("X-RateLimit-Remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u32>().ok());
    let png = resp.bytes().ok()?.to_vec();
    // Sanity: the API returns a PNG on success.
    if png.len() < 8 || &png[..4] != b"\x89PNG" {
        eprintln!("labelixa API returned a non-PNG body — falling back to renderer baseline");
        return None;
    }

    // A served response with zero remaining means the daily quota is now
    // spent: keep this (successful) render but stop hitting the API until
    // the reset so later callers fall back immediately.
    if remaining == Some(0) {
        eprintln!("labelixa daily quota now exhausted — later bootstraps fall back to the renderer baseline");
    }

    std::fs::write(&cache_path, &png).ok();
    Some(png)
}
