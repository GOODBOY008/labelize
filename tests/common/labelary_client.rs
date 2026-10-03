use image::{ImageFormat, RgbaImage};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

static RATE_LIMITER: Mutex<Option<Instant>> = Mutex::new(None);

/// Minimum interval between Labelary API requests (~333ms = 3 req/sec).
const MIN_INTERVAL_MS: u128 = 334;

fn cache_dir() -> PathBuf {
    let dir = PathBuf::from("testdata/labelary_cache");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn cache_key(zpl: &str, dpmm: u8, width_inches: f64, height_inches: f64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(zpl.as_bytes());
    hasher.update(dpmm.to_le_bytes());
    hasher.update(width_inches.to_le_bytes());
    hasher.update(height_inches.to_le_bytes());
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

/// Pad (or crop) a PNG to exactly `target_w × target_h` pixels.
///
/// If the source is smaller than the target in either dimension the missing area
/// is filled with white (fully opaque). If larger it is cropped to fit.
/// This is used to normalise Labelary responses (which may be off-by-one due to
/// server-side floating-point rounding) to the exact canvas size our renderer
/// produces (813×1626), eliminating size-mismatch noise in diff reports.
pub fn pad_png_to_size(png: &[u8], target_w: u32, target_h: u32) -> Vec<u8> {
    let src = image::load_from_memory(png)
        .expect("decode PNG for padding")
        .to_rgba8();

    if src.width() == target_w && src.height() == target_h {
        return png.to_vec();
    }

    let mut canvas = RgbaImage::from_pixel(target_w, target_h, image::Rgba([255, 255, 255, 255]));
    let copy_w = src.width().min(target_w);
    let copy_h = src.height().min(target_h);
    for y in 0..copy_h {
        for x in 0..copy_w {
            canvas.put_pixel(x, y, *src.get_pixel(x, y));
        }
    }

    let mut buf = Cursor::new(Vec::new());
    canvas
        .write_to(&mut buf, ImageFormat::Png)
        .expect("encode padded PNG");
    buf.into_inner()
}

/// Fetch a reference PNG from the Labelary API with caching and rate limiting.
/// Returns None if the API is unreachable or returns an error.
pub fn labelary_render(
    zpl: &str,
    dpmm: u8,
    width_inches: f64,
    height_inches: f64,
) -> Option<Vec<u8>> {
    let key = cache_key(zpl, dpmm, width_inches, height_inches);
    let cache_path = cache_dir().join(format!("{}.png", key));

    // Check cache first
    if cache_path.exists() {
        return std::fs::read(&cache_path).ok();
    }

    rate_limit();

    let url = format!(
        "http://api.labelary.com/v1/printers/{}dpmm/labels/{}x{}/0/",
        dpmm, width_inches, height_inches
    );

    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(&url)
        .header("Accept", "image/png")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(zpl.to_string())
        .send()
        .ok()?;

    if !resp.status().is_success() {
        eprintln!("Labelary API returned status {} for {}", resp.status(), url);
        return None;
    }

    let bytes = resp.bytes().ok()?.to_vec();
    // Cache the response
    std::fs::write(&cache_path, &bytes).ok();
    Some(bytes)
}
