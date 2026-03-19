use image::RgbaImage;
use std::path::Path;

/// Result of comparing two images.
pub struct CompareResult {
    pub diff_percent: f64,
    pub dimensions_match: bool,
    pub actual_dims: (u32, u32),
    pub expected_dims: (u32, u32),
    pub diff_image: Option<RgbaImage>,
}

/// Compare two PNG byte slices. Generates a diff image when pixels differ.
/// Uses per-channel threshold of 32 (matching existing implementation).
pub fn compare_images(actual: &[u8], expected: &[u8], _tolerance: f64) -> CompareResult {
    let actual_img = image::load_from_memory(actual)
        .expect("decode actual PNG")
        .to_rgba8();
    let expected_img = image::load_from_memory(expected)
        .expect("decode expected PNG")
        .to_rgba8();

    let actual_dims = (actual_img.width(), actual_img.height());
    let expected_dims = (expected_img.width(), expected_img.height());
    let dimensions_match = actual_dims == expected_dims;

    let w = actual_img.width().min(expected_img.width());
    let h = actual_img.height().min(expected_img.height());
    let total = (w as u64) * (h as u64);
    if total == 0 {
        return CompareResult {
            diff_percent: 100.0,
            dimensions_match,
            actual_dims,
            expected_dims,
            diff_image: None,
        };
    }

    let mut diff_count: u64 = 0;
    let mut diff_img = RgbaImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let a = actual_img.get_pixel(x, y);
            let e = expected_img.get_pixel(x, y);
            let differs = (0..4).any(|i| (a[i] as i16 - e[i] as i16).unsigned_abs() > 32);
            if differs {
                diff_count += 1;
                diff_img.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            } else {
                diff_img.put_pixel(x, y, image::Rgba([0, 0, 0, 0]));
            }
        }
    }

    // Count extra pixels from size mismatch as differences
    let max_w = actual_img.width().max(expected_img.width());
    let max_h = actual_img.height().max(expected_img.height());
    let max_total = (max_w as u64) * (max_h as u64);
    let size_diff = max_total - total;

    let diff_percent = (diff_count + size_diff) as f64 / max_total as f64 * 100.0;

    CompareResult {
        diff_percent,
        dimensions_match,
        actual_dims,
        expected_dims,
        diff_image: if diff_count > 0 { Some(diff_img) } else { None },
    }
}

/// Save a diff image highlighting differing pixels to testdata/diffs/.
pub fn save_diff_image(name: &str, diff: &RgbaImage) {
    let dir = Path::new("testdata/diffs");
    std::fs::create_dir_all(dir).ok();
    let path = dir.join(format!("{}_diff.png", name));
    diff.save(&path).ok();
}
