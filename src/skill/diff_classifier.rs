/// Diff classification: determine whether each element's diff is a content
/// difference (element renders wrong) or a position difference (element is
/// placed at wrong coordinates), or both (Mixed).
///
/// The key signal is comparing an isolated snippet render of the element
/// against the Labelary reference render of the same snippet:
/// - Isolated renders match → element renders correctly but is misplaced → PositionDiff
/// - Isolated renders differ → element rendering is wrong → ContentDiff
/// - Both → Mixed
use std::io::Cursor;
use std::path::{Path, PathBuf};

use image::RgbaImage;

use crate::{DrawerOptions, Renderer, ZplParser};

use super::error::AnalyzeError;
use super::models::{
    DiffClassification, ElementBBox, ElementDiffContribution, OffsetDetectionMethod,
    PositionOffsetInfo,
};
use super::snippet_extractor;

// ─── Rendering helpers ────────────────────────────────────────────────────────

/// Render a ZPL string to an RGBA image using the standard 8-dpmm canvas.
pub fn render_snippet_isolated(zpl: &str) -> Result<RgbaImage, AnalyzeError> {
    let opts = DrawerOptions {
        label_width_mm: 101.625,
        label_height_mm: 203.25,
        dpmm: 8,
        ..Default::default()
    };
    let mut parser = ZplParser::new();
    let labels = parser
        .parse(zpl.as_bytes())
        .map_err(|e| AnalyzeError::RenderFailed(e.to_string()))?;
    let label = labels
        .into_iter()
        .next()
        .ok_or_else(|| AnalyzeError::RenderFailed("no label parsed from snippet".to_string()))?;

    let renderer = Renderer::new();
    let mut buf = Cursor::new(Vec::new());
    renderer
        .draw_label_as_png(&label, &mut buf, opts)
        .map_err(|e| AnalyzeError::RenderFailed(e.to_string()))?;

    let png_bytes = buf.into_inner();
    let img = image::load_from_memory(&png_bytes)
        .map_err(|e| AnalyzeError::RenderFailed(e.to_string()))?
        .to_rgba8();
    Ok(img)
}

/// Fetch the Labelary reference render for a snippet, with local cache.
///
/// Cache key: SHA-256 of the ZPL content stored under
/// `testdata/snippets/labelary_cache/<hash>.png`.
///
/// Returns `None` when the Labelary API is unreachable; callers should fall
/// back to skipping snippet comparison in that case.
pub fn fetch_labelary_snippet_render(zpl: &str, cache_dir: &Path) -> Option<RgbaImage> {
    // Compute cache key from content
    let hash = simple_hash(zpl);
    let cache_path = cache_dir.join(format!("{:016x}.png", hash));

    // Return cached image if available
    if cache_path.exists() {
        if let Ok(img) = image::open(&cache_path) {
            return Some(img.to_rgba8());
        }
    }

    // Attempt Labelary API call
    let result = fetch_from_labelary(zpl);
    if let Some(ref img) = result {
        // Try to cache, ignore errors
        let _ = std::fs::create_dir_all(cache_dir);
        let _ = img.save(&cache_path);
    }
    result
}

fn fetch_from_labelary(zpl: &str) -> Option<RgbaImage> {
    use std::io::Read;

    let url = "http://api.labelary.com/v1/printers/8dpmm/labels/4.005x8.01/0/";
    let client = std::net::TcpStream::connect_timeout(
        &"api.labelary.com:80".parse().ok()?,
        std::time::Duration::from_secs(5),
    );
    if client.is_err() {
        return None;
    }
    drop(client); // just checked connectivity

    // Use ureq-style minimal HTTP — build raw request manually to avoid
    // pulling in a heavy HTTP client dependency.
    let request_body = zpl.as_bytes();
    let request = format!(
        "POST {} HTTP/1.0\r\nHost: api.labelary.com\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\nAccept: image/png\r\n\r\n",
        url, request_body.len()
    );

    let mut stream = std::net::TcpStream::connect("api.labelary.com:80").ok()?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .ok()?;

    use std::io::Write;
    stream.write_all(request.as_bytes()).ok()?;
    stream.write_all(request_body).ok()?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response).ok()?;

    // Strip HTTP headers (find \r\n\r\n boundary)
    let header_end = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)?;

    let body = &response[header_end..];

    // Check for HTTP 200 in the header
    let header = std::str::from_utf8(&response[..header_end]).ok()?;
    if !header.starts_with("HTTP/1") || !header.contains("200") {
        return None;
    }

    image::load_from_memory(body).ok().map(|i| i.to_rgba8())
}

// ─── Image comparison ─────────────────────────────────────────────────────────

/// Compute the percentage of differing pixels between two RGBA images.
/// Images are compared at the same dimensions; if sizes differ, they are
/// considered 100% different (to flag gross mismatches).
pub fn compute_image_diff_percent(a: &RgbaImage, b: &RgbaImage) -> f64 {
    if a.width() != b.width() || a.height() != b.height() {
        return 100.0;
    }
    let total = (a.width() * a.height()) as u64;
    if total == 0 {
        return 0.0;
    }
    let diffs = a
        .pixels()
        .zip(b.pixels())
        .filter(|(pa, pb)| {
            // Compare luminance channels (treat images as grayscale for diff)
            let la = luma(pa);
            let lb = luma(pb);
            la.abs_diff(lb) > 10
        })
        .count() as u64;
    diffs as f64 / total as f64 * 100.0
}

fn luma(p: &image::Rgba<u8>) -> u8 {
    // Fast approximate luma: (2R + 4G + B) / 7
    let r = p[0] as u16;
    let g = p[1] as u16;
    let b = p[2] as u16;
    ((2 * r + 4 * g + b) / 7) as u8
}

// ─── Position offset detection ────────────────────────────────────────────────

/// Compute the centroid (cx, cy) of diff pixels within a bounding box region.
/// Returns `None` if no diff pixels exist in the region.
pub fn compute_diff_centroid(diff_image: &RgbaImage, bbox: &ElementBBox) -> Option<(f64, f64)> {
    let x1 = bbox.x.max(0) as u32;
    let y1 = bbox.y.max(0) as u32;
    let x2 = (bbox.x + bbox.width).min(diff_image.width() as i32).max(0) as u32;
    let y2 = (bbox.y + bbox.height)
        .min(diff_image.height() as i32)
        .max(0) as u32;

    let mut sum_x = 0u64;
    let mut sum_y = 0u64;
    let mut count = 0u64;

    for y in y1..y2 {
        for x in x1..x2 {
            let p = diff_image.get_pixel(x, y);
            if p[0] > 200 && p[1] < 50 && p[2] < 50 && p[3] > 200 {
                sum_x += x as u64;
                sum_y += y as u64;
                count += 1;
            }
        }
    }

    if count == 0 {
        return None;
    }
    Some((sum_x as f64 / count as f64, sum_y as f64 / count as f64))
}

/// Detect a "shadow" pattern: the diff image shows the element at two positions
/// (once where we render it, once where Labelary places it).  This manifests as
/// two distinct clusters of red pixels inside or near the bbox.
///
/// Returns an estimated (dx, dy) offset if the pattern is found.
pub fn detect_shadow_pattern(diff_image: &RgbaImage, bbox: &ElementBBox) -> Option<(i32, i32)> {
    // Expand search window by 50% around the element
    let margin_x = bbox.width / 2;
    let margin_y = bbox.height / 2;
    let sx1 = (bbox.x - margin_x).max(0) as u32;
    let sy1 = (bbox.y - margin_y).max(0) as u32;
    let sx2 = (bbox.x + bbox.width + margin_x)
        .min(diff_image.width() as i32)
        .max(0) as u32;
    let sy2 = (bbox.y + bbox.height + margin_y)
        .min(diff_image.height() as i32)
        .max(0) as u32;

    // Build list of diff pixel positions
    let mut points: Vec<(i32, i32)> = Vec::new();
    for y in sy1..sy2 {
        for x in sx1..sx2 {
            let p = diff_image.get_pixel(x, y);
            if p[0] > 200 && p[1] < 50 && p[2] < 50 && p[3] > 200 {
                points.push((x as i32, y as i32));
            }
        }
    }

    if points.len() < 20 {
        return None;
    }

    // Simple two-cluster heuristic: split points by median x then median y,
    // check if the two halves have clearly separated centroids.
    let mut xs: Vec<i32> = points.iter().map(|(x, _)| *x).collect();
    let mut ys: Vec<i32> = points.iter().map(|(_, y)| *y).collect();
    xs.sort_unstable();
    ys.sort_unstable();
    let med_x = xs[xs.len() / 2];
    let med_y = ys[ys.len() / 2];

    let cluster_a: Vec<_> = points
        .iter()
        .filter(|(x, y)| *x < med_x || *y < med_y)
        .collect();
    let cluster_b: Vec<_> = points
        .iter()
        .filter(|(x, y)| *x >= med_x && *y >= med_y)
        .collect();

    if cluster_a.is_empty() || cluster_b.is_empty() {
        return None;
    }

    let cx_a = cluster_a.iter().map(|(x, _)| *x).sum::<i32>() / cluster_a.len() as i32;
    let cy_a = cluster_a.iter().map(|(_, y)| *y).sum::<i32>() / cluster_a.len() as i32;
    let cx_b = cluster_b.iter().map(|(x, _)| *x).sum::<i32>() / cluster_b.len() as i32;
    let cy_b = cluster_b.iter().map(|(_, y)| *y).sum::<i32>() / cluster_b.len() as i32;

    let dx = cx_b - cx_a;
    let dy = cy_b - cy_a;

    // Only report if separation is meaningful (> 5 px)
    if dx.abs() < 5 && dy.abs() < 5 {
        return None;
    }

    Some((dx, dy))
}

/// Detect the position offset vector for an element whose diff may be positional.
///
/// Tries shadow detection first (highest confidence), then falls back to
/// centroid-shift analysis.  Returns `None` if no offset can be determined.
pub fn detect_position_offset(
    diff_image: &RgbaImage,
    bbox: &ElementBBox,
) -> Option<PositionOffsetInfo> {
    // Try shadow pattern first
    if let Some((dx, dy)) = detect_shadow_pattern(diff_image, bbox) {
        let magnitude = ((dx * dx + dy * dy) as f64).sqrt();
        let confidence = (0.5 + magnitude / 200.0).min(0.95);
        return Some(PositionOffsetInfo {
            dx,
            dy,
            confidence,
            method: OffsetDetectionMethod::ShadowDetection,
        });
    }

    // Centroid-shift fallback
    let centroid = compute_diff_centroid(diff_image, bbox)?;
    let elem_cx = bbox.x as f64 + bbox.width as f64 / 2.0;
    let elem_cy = bbox.y as f64 + bbox.height as f64 / 2.0;
    let dx = (centroid.0 - elem_cx) as i32;
    let dy = (centroid.1 - elem_cy) as i32;

    if dx.abs() < 3 && dy.abs() < 3 {
        return None;
    }

    let magnitude = ((dx * dx + dy * dy) as f64).sqrt();
    let confidence = (magnitude / 100.0).min(0.7);

    Some(PositionOffsetInfo {
        dx,
        dy,
        confidence,
        method: OffsetDetectionMethod::CentroidShift,
    })
}

// ─── Main classification ───────────────────────────────────────────────────────

/// Options for classifying element diffs.
pub struct ClassifyOptions {
    /// Snippet diff threshold below which we consider the isolated renders "matching".
    /// Default: 2.0%
    pub snippet_threshold: f64,
    /// Directory for Labelary response cache.
    pub cache_dir: PathBuf,
    /// Whether to actually call Labelary API (set false in offline tests).
    pub use_labelary: bool,
}

impl Default for ClassifyOptions {
    fn default() -> Self {
        Self {
            snippet_threshold: 2.0,
            cache_dir: PathBuf::from("testdata/snippets/labelary_cache"),
            use_labelary: true,
        }
    }
}

/// Classify each element's diff contribution as ContentDiff, PositionDiff,
/// or Mixed, and populate the `classification` and `position_offset` fields.
///
/// Elements with zero diff pixels are left with `classification = None`.
pub fn classify_element_diffs(
    contributions: &mut [ElementDiffContribution],
    label_zpl: &str,
    diff_image: &RgbaImage,
    opts: &ClassifyOptions,
) -> Result<(), AnalyzeError> {
    for contrib in contributions.iter_mut() {
        // Skip elements with no diff
        if contrib.diff_pixels_in_bbox == 0 {
            contrib.classification = None;
            contrib.position_offset = None;
            continue;
        }

        // Extract a standalone snippet for this element
        let snippet_result = snippet_extractor::extract_element(
            label_zpl,
            "classify_tmp",
            contrib.bbox.element_index,
            contrib.local_diff_percent,
        );

        let snippet_zpl = match snippet_result {
            Ok(s) => s.zpl_content,
            Err(_) => {
                // Cannot extract snippet — default to ContentDiff
                contrib.classification = Some(DiffClassification::ContentDiff);
                continue;
            }
        };

        // Render our version of the snippet
        let our_render = match render_snippet_isolated(&snippet_zpl) {
            Ok(img) => img,
            Err(_) => {
                contrib.classification = Some(DiffClassification::ContentDiff);
                continue;
            }
        };

        // Get Labelary reference (may be None if offline)
        let labelary_render = if opts.use_labelary {
            fetch_labelary_snippet_render(&snippet_zpl, &opts.cache_dir)
        } else {
            None
        };

        let snippet_diff_pct = match labelary_render {
            Some(ref ref_img) => compute_image_diff_percent(&our_render, ref_img),
            // Without Labelary, fall back to positional analysis only
            None => {
                // We can't classify without a reference — use offset heuristic
                if let Some(offset) = detect_position_offset(diff_image, &contrib.bbox) {
                    contrib.classification = Some(DiffClassification::PositionDiff);
                    contrib.position_offset = Some(offset);
                } else {
                    contrib.classification = Some(DiffClassification::ContentDiff);
                }
                continue;
            }
        };

        // Classify based on snippet diff
        let is_content_diff = snippet_diff_pct > opts.snippet_threshold;
        let is_position_diff = contrib.local_diff_percent > opts.snippet_threshold;

        contrib.classification = Some(match (is_content_diff, is_position_diff) {
            (true, true) => DiffClassification::Mixed,
            (true, false) => DiffClassification::ContentDiff,
            (false, true) => DiffClassification::PositionDiff,
            (false, false) => {
                // Both diffs are low — effectively no meaningful diff
                contrib.classification = None;
                continue;
            }
        });

        // Attempt position offset detection for PositionDiff or Mixed
        if matches!(
            contrib.classification,
            Some(DiffClassification::PositionDiff) | Some(DiffClassification::Mixed)
        ) {
            contrib.position_offset = detect_position_offset(diff_image, &contrib.bbox);
        }
    }
    Ok(())
}

// ─── Utilities ────────────────────────────────────────────────────────────────

/// A fast non-cryptographic hash for cache key generation.
fn simple_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
