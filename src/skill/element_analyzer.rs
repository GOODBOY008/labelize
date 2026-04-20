/// Analyzes which ZPL elements contribute most to the pixel diff.
use std::path::Path;

use image::RgbaImage;

use crate::elements::label_element::LabelElement;
use crate::elements::label_info::LabelInfo;

use super::error::AnalyzeError;
use super::models::{ElementBBox, ElementDiffContribution, ElementType};

/// Compute bounding box for a rendered element.
pub fn compute_element_bbox(element: &LabelElement, index: usize) -> Option<ElementBBox> {
    match element {
        LabelElement::Text(t) => {
            let font = &t.font;
            let h = font.height.max(10.0) as i32;
            let w = if let Some(ref block) = t.block {
                block.max_width.max(1)
            } else {
                // Estimate text width from font and data length
                let char_w = (font.width.max(1.0) * font.get_scale_x()) as i32;
                (char_w * t.text.len() as i32).max(1)
            };
            let block_h = if let Some(ref block) = t.block {
                let lines = block.max_lines.max(1);
                h * lines + block.line_spacing * (lines - 1)
            } else {
                h
            };
            Some(ElementBBox {
                x: t.position.x,
                y: t.position.y,
                width: w,
                height: block_h,
                element_index: index,
                element_type: ElementType::Text,
                zpl_command: "^A/^FD".to_string(),
            })
        }
        LabelElement::GraphicBox(g) => Some(ElementBBox {
            x: g.position.x,
            y: g.position.y,
            width: g.width.max(1),
            height: g.height.max(1),
            element_index: index,
            element_type: ElementType::GraphicBox,
            zpl_command: "^GB".to_string(),
        }),
        LabelElement::GraphicCircle(g) => Some(ElementBBox {
            x: g.position.x,
            y: g.position.y,
            width: g.circle_diameter.max(1),
            height: g.circle_diameter.max(1),
            element_index: index,
            element_type: ElementType::GraphicCircle,
            zpl_command: "^GC".to_string(),
        }),
        LabelElement::DiagonalLine(g) => Some(ElementBBox {
            x: g.position.x,
            y: g.position.y,
            width: g.width.max(1),
            height: g.height.max(1),
            element_index: index,
            element_type: ElementType::DiagonalLine,
            zpl_command: "^GD".to_string(),
        }),
        LabelElement::GraphicField(g) => {
            let rows = if g.row_bytes > 0 {
                g.total_bytes / g.row_bytes
            } else {
                1
            };
            Some(ElementBBox {
                x: g.position.x,
                y: g.position.y,
                width: (g.row_bytes * 8 * g.magnification_x).max(1),
                height: (rows * g.magnification_y).max(1),
                element_index: index,
                element_type: ElementType::GraphicField,
                zpl_command: "^GF".to_string(),
            })
        }
        LabelElement::Barcode128(b) => {
            // Width is pre-calculated by the parser
            let w = (b.width * 11 + 35).max(1); // rough estimate: 11 modules per char + start/stop
            Some(ElementBBox {
                x: b.position.x,
                y: b.position.y,
                width: w,
                height: b.barcode.height.max(1),
                element_index: index,
                element_type: ElementType::Barcode128,
                zpl_command: "^BC".to_string(),
            })
        }
        LabelElement::BarcodeEan13(b) => {
            // EAN-13 is always 95 modules wide
            let w = 95 * b.width.max(1);
            Some(ElementBBox {
                x: b.position.x,
                y: b.position.y,
                width: w,
                height: b.barcode.height.max(1),
                element_index: index,
                element_type: ElementType::BarcodeEan13,
                zpl_command: "^BE".to_string(),
            })
        }
        LabelElement::Barcode2of5(b) => {
            let data_len = b.data.len() as i32;
            let w = ((data_len * 2 + 3) * b.width.max(1) * 3).max(1);
            Some(ElementBBox {
                x: b.position.x,
                y: b.position.y,
                width: w,
                height: b.barcode.height.max(1),
                element_index: index,
                element_type: ElementType::Barcode2of5,
                zpl_command: "^B2".to_string(),
            })
        }
        LabelElement::Barcode39(b) => {
            let data_len = b.data.len() as i32;
            let w = ((data_len + 2) * 13 * b.width.max(1)).max(1);
            Some(ElementBBox {
                x: b.position.x,
                y: b.position.y,
                width: w,
                height: b.barcode.height.max(1),
                element_index: index,
                element_type: ElementType::Barcode39,
                zpl_command: "^B3".to_string(),
            })
        }
        LabelElement::BarcodePdf417(b) => {
            let cols = b.barcode.columns.max(1);
            let rows = b.barcode.rows.max(3);
            let mw = b.barcode.module_width.max(1);
            let w = (cols * 17 + 69) * mw;
            let h = rows * b.barcode.row_height.max(1);
            Some(ElementBBox {
                x: b.position.x,
                y: b.position.y,
                width: w.max(1),
                height: h.max(1),
                element_index: index,
                element_type: ElementType::BarcodePdf417,
                zpl_command: "^B7".to_string(),
            })
        }
        LabelElement::BarcodeAztec(b) => {
            let mag = b.barcode.magnification.max(1);
            // Aztec default size depends on data; estimate conservatively
            let size = if b.barcode.size > 0 {
                b.barcode.size * mag
            } else {
                // Default: roughly 50 modules
                50 * mag
            };
            Some(ElementBBox {
                x: b.position.x,
                y: b.position.y,
                width: size.max(1),
                height: size.max(1),
                element_index: index,
                element_type: ElementType::BarcodeAztec,
                zpl_command: "^BO".to_string(),
            })
        }
        LabelElement::BarcodeDatamatrix(b) => {
            let h = b.barcode.height.max(1);
            // DataMatrix is roughly square; height parameter controls module size
            let size = h * 10; // rough estimate
            Some(ElementBBox {
                x: b.position.x,
                y: b.position.y,
                width: size.max(1),
                height: size.max(1),
                element_index: index,
                element_type: ElementType::BarcodeDatamatrix,
                zpl_command: "^BX".to_string(),
            })
        }
        LabelElement::BarcodeQr(b) => {
            let mag = b.barcode.magnification.max(1);
            // QR size depends on data; estimate
            let size = (21 + 4) * mag; // Version 1 + quiet zone
            Some(ElementBBox {
                x: b.position.x,
                y: b.position.y,
                width: size.max(1) * b.height.max(1),
                height: size.max(1) * b.height.max(1),
                element_index: index,
                element_type: ElementType::BarcodeQr,
                zpl_command: "^BQ".to_string(),
            })
        }
        LabelElement::Maxicode(m) => {
            // MaxiCode is always 1 inch square at any DPI
            // At 8 dpmm ≈ 200 DPI: ~200 pixels
            Some(ElementBBox {
                x: m.position.x,
                y: m.position.y,
                width: 200,
                height: 200,
                element_index: index,
                element_type: ElementType::Maxicode,
                zpl_command: "^BD".to_string(),
            })
        }
        // Config and template elements are not drawn
        _ => None,
    }
}

/// Count red diff pixels (R>200, G<50, B<50, A>200) in the entire image.
pub fn count_red_pixels(img: &RgbaImage) -> u64 {
    let mut count = 0u64;
    for pixel in img.pixels() {
        if pixel[0] > 200 && pixel[1] < 50 && pixel[2] < 50 && pixel[3] > 200 {
            count += 1;
        }
    }
    count
}

/// Count red diff pixels within a rectangular region.
pub fn count_red_pixels_in_rect(
    img: &RgbaImage,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
) -> u64 {
    let mut count = 0u64;
    let img_w = img.width() as i32;
    let img_h = img.height() as i32;

    let x1 = x1.max(0);
    let y1 = y1.max(0);
    let x2 = x2.min(img_w);
    let y2 = y2.min(img_h);

    for y in y1..y2 {
        for x in x1..x2 {
            let pixel = img.get_pixel(x as u32, y as u32);
            if pixel[0] > 200 && pixel[1] < 50 && pixel[2] < 50 && pixel[3] > 200 {
                count += 1;
            }
        }
    }
    count
}

/// Correlate diff pixels with element bounding boxes.
pub fn correlate_diff_regions(
    diff_image: &RgbaImage,
    bboxes: &[ElementBBox],
) -> Vec<ElementDiffContribution> {
    let total_red = count_red_pixels(diff_image);
    if total_red == 0 {
        return Vec::new();
    }

    let mut contributions: Vec<ElementDiffContribution> = bboxes
        .iter()
        .map(|bbox| {
            let diff_in_bbox = count_red_pixels_in_rect(
                diff_image,
                bbox.x,
                bbox.y,
                bbox.x + bbox.width,
                bbox.y + bbox.height,
            );
            let total_in_bbox = (bbox.width as u64) * (bbox.height as u64);
            let local_diff = if total_in_bbox > 0 {
                diff_in_bbox as f64 / total_in_bbox as f64 * 100.0
            } else {
                0.0
            };
            let contribution = diff_in_bbox as f64 / total_red as f64 * 100.0;

            ElementDiffContribution {
                bbox: bbox.clone(),
                diff_pixels_in_bbox: diff_in_bbox,
                total_pixels_in_bbox: total_in_bbox,
                local_diff_percent: local_diff,
                contribution_to_total: contribution,
                classification: None,
                position_offset: None,
            }
        })
        .collect();

    // Sort by contribution descending
    contributions.sort_by(|a, b| {
        b.contribution_to_total
            .partial_cmp(&a.contribution_to_total)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    contributions
}

/// Analyze a label: parse ZPL → compute bboxes → load diff image → correlate.
///
/// Classification is NOT performed here (requires the original ZPL string and
/// optionally the Labelary API).  Use `analyze_label_with_classification()`
/// for the full pipeline.
pub fn analyze_label(
    label: &LabelInfo,
    diff_image_path: &Path,
) -> Result<Vec<ElementDiffContribution>, AnalyzeError> {
    // Compute bounding boxes
    let bboxes: Vec<ElementBBox> = label
        .elements
        .iter()
        .enumerate()
        .filter_map(|(i, el)| compute_element_bbox(el, i))
        .collect();

    if bboxes.is_empty() {
        return Err(AnalyzeError::NoElements);
    }

    // Load diff image
    let diff_img = image::open(diff_image_path)
        .map_err(|_| AnalyzeError::DiffImageNotFound {
            path: diff_image_path.display().to_string(),
        })?
        .to_rgba8();

    Ok(correlate_diff_regions(&diff_img, &bboxes))
}

/// Full pipeline: analyze + classify diffs as ContentDiff / PositionDiff / Mixed.
///
/// `label_zpl` is needed to extract isolated snippets for classification.
/// Pass `use_labelary = false` to skip network calls (offline/unit-test mode).
pub fn analyze_label_with_classification(
    label: &LabelInfo,
    diff_image_path: &Path,
    label_zpl: &str,
    use_labelary: bool,
) -> Result<Vec<ElementDiffContribution>, AnalyzeError> {
    use super::diff_classifier::{classify_element_diffs, ClassifyOptions};

    let mut contributions = analyze_label(label, diff_image_path)?;

    // Load diff image again for offset detection (already decoded once above,
    // but that image is not accessible here — open a second time).
    let diff_img = image::open(diff_image_path)
        .map_err(|_| AnalyzeError::DiffImageNotFound {
            path: diff_image_path.display().to_string(),
        })?
        .to_rgba8();

    let classify_opts = ClassifyOptions {
        use_labelary,
        ..Default::default()
    };

    classify_element_diffs(&mut contributions, label_zpl, &diff_img, &classify_opts)?;

    Ok(contributions)
}

/// Format analysis results as a human-readable report.
pub fn format_analysis_report(contributions: &[ElementDiffContribution]) -> String {
    use super::models::{DiffClassification, PositionOffsetInfo};

    let mut report = String::new();
    report.push_str("Element Diff Contribution Analysis\n");
    report.push_str("==================================\n\n");
    report.push_str(&format!(
        "{:<6} {:<16} {:<8} {:<12} {:<8} {:<10} {:<14} {}\n",
        "Index", "Type", "Command", "DiffPixels", "Local%", "Contrib%", "Classification", "Offset"
    ));
    report.push_str(&"-".repeat(95));
    report.push('\n');

    for c in contributions {
        if c.diff_pixels_in_bbox == 0 {
            continue;
        }
        let class_str = match &c.classification {
            None => "-".to_string(),
            Some(DiffClassification::ContentDiff) => "Content".to_string(),
            Some(DiffClassification::PositionDiff) => "Position".to_string(),
            Some(DiffClassification::Mixed) => "Mixed".to_string(),
        };
        let offset_str = match &c.position_offset {
            None => String::new(),
            Some(PositionOffsetInfo { dx, dy, confidence, .. }) => {
                format!("dx={:+} dy={:+} ({:.0}%)", dx, dy, confidence * 100.0)
            }
        };
        report.push_str(&format!(
            "{:<6} {:<16} {:<8} {:<12} {:<8.2} {:<10.2} {:<14} {}\n",
            c.bbox.element_index,
            format!("{:?}", c.bbox.element_type),
            c.bbox.zpl_command,
            c.diff_pixels_in_bbox,
            c.local_diff_percent,
            c.contribution_to_total,
            class_str,
            offset_str,
        ));
    }
    report
}
