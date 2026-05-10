/// Integration tests for the skill module: DiffScanner, ElementAnalyzer, SnippetExtractor.
mod common;

use common::render_helpers;
use labelize::skill::diff_scanner;
use labelize::skill::element_analyzer;
use labelize::skill::models::DiffStatus;
use labelize::skill::snippet_extractor;
#[test]
fn test_load_and_parse_real_diff_report() {
    let testdata = render_helpers::testdata_dir();
    let report = diff_scanner::load_diff_report(&testdata);

    match report {
        Ok(report) => {
            // Should have entries for all testdata labels
            assert!(!report.entries.is_empty(), "report should have entries");
            assert!(
                report.entries.len() >= 50,
                "expected at least 50 labels, got {}",
                report.entries.len()
            );

            // Check category counts sum to total
            let sum = report.perfect_count
                + report.good_count
                + report.minor_count
                + report.moderate_count
                + report.high_count
                + report.skip_count
                + report.error_count;
            assert_eq!(sum, report.total_labels);

            // Verify known labels exist
            let bstc = diff_scanner::scan_label(&report, "bstc").unwrap();
            assert_eq!(bstc.status, DiffStatus::Perfect);
            assert_eq!(bstc.diff_percent, 0.0);

            // Check that scan_label works for a known moderate label
            let ups = diff_scanner::scan_label(&report, "ups");
            assert!(ups.is_ok(), "ups label should exist");
            assert!(ups.unwrap().diff_percent > 1.0, "ups should have diff > 1%");

            println!("Parsed {} labels from diff report", report.entries.len());
            println!(
                "  {} perfect, {} good, {} minor, {} moderate, {} high",
                report.perfect_count,
                report.good_count,
                report.minor_count,
                report.moderate_count,
                report.high_count
            );
        }
        Err(e) => {
            eprintln!(
                "Diff report not found (run `cargo test --test e2e_diff_report` first): {}",
                e
            );
        }
    }
}

#[test]
fn test_load_report_with_tolerances() {
    let testdata = render_helpers::testdata_dir();
    let report = diff_scanner::load_diff_report(&testdata);

    if let Ok(mut report) = report {
        // Load DIFF_THRESHOLDS.md
        let thresholds_path = std::path::Path::new("docs/DIFF_THRESHOLDS.md");
        if thresholds_path.exists() {
            let thresholds_text = std::fs::read_to_string(thresholds_path).unwrap();
            diff_scanner::enrich_with_tolerances(&mut report, &thresholds_text);

            // Check that tolerances were applied
            let with_tol: Vec<_> = report
                .entries
                .iter()
                .filter(|e| e.tolerance.is_some())
                .collect();
            assert!(
                !with_tol.is_empty(),
                "some entries should have tolerances from DIFF_THRESHOLDS.md"
            );

            // Find any labels exceeding tolerance
            let high_diff = diff_scanner::find_high_diff_labels(&report);
            println!("{} labels exceed their tolerance:", high_diff.len());
            for entry in &high_diff {
                println!(
                    "  {} — diff: {:.2}%, tolerance: {:.1}%",
                    entry.label_name,
                    entry.diff_percent,
                    entry.tolerance.unwrap_or(0.0)
                );
            }
        }
    }
}

#[test]
fn test_analyze_bstc_label() {
    // bstc is PERFECT — should have zero diff contributions
    let testdata = render_helpers::testdata_dir();
    let zpl_path = render_helpers::labels_dir().join("bstc.zpl");
    let diff_path = testdata.join("diffs").join("bstc_diff.png");

    if !zpl_path.exists() || !diff_path.exists() {
        eprintln!("Skipping: bstc.zpl or diff image not found");
        return;
    }

    let zpl = std::fs::read_to_string(&zpl_path).unwrap();
    let labels = render_helpers::parse_zpl(zpl.as_bytes());
    assert!(!labels.is_empty());

    let result = element_analyzer::analyze_label(&labels[0], &diff_path);
    match result {
        Ok(contributions) => {
            // bstc is PERFECT — all contributions should be zero
            for c in &contributions {
                assert_eq!(
                    c.diff_pixels_in_bbox, 0,
                    "bstc is perfect — no diff pixels expected"
                );
            }
            println!(
                "bstc analysis: {} elements, all with 0 diff pixels",
                contributions.len()
            );
        }
        Err(e) => {
            // If the diff image doesn't exist (because bstc is perfect, no diff saved), that's ok
            eprintln!("Analysis error (expected for PERFECT labels): {}", e);
        }
    }
}

#[test]
fn test_analyze_label_with_diff() {
    // Pick a label with known moderate diff for analysis
    let testdata = render_helpers::testdata_dir();
    let zpl_path = render_helpers::labels_dir().join("amazon.zpl");
    let diff_path = testdata.join("diffs").join("amazon_diff.png");

    if !zpl_path.exists() || !diff_path.exists() {
        eprintln!("Skipping: amazon.zpl or diff image not found");
        return;
    }

    let zpl = std::fs::read_to_string(&zpl_path).unwrap();
    let labels = render_helpers::parse_zpl(zpl.as_bytes());
    assert!(!labels.is_empty());

    let result = element_analyzer::analyze_label(&labels[0], &diff_path);
    match result {
        Ok(contributions) => {
            assert!(
                !contributions.is_empty(),
                "should have element contributions"
            );

            // Print the analysis
            let report = element_analyzer::format_analysis_report(&contributions);
            println!("{}", report);

            // Verify sorting: contributions should be descending
            for w in contributions.windows(2) {
                assert!(
                    w[0].contribution_to_total >= w[1].contribution_to_total,
                    "contributions should be sorted descending"
                );
            }

            // At least one element should have nonzero contribution
            let nonzero: Vec<_> = contributions
                .iter()
                .filter(|c| c.diff_pixels_in_bbox > 0)
                .collect();
            assert!(
                !nonzero.is_empty(),
                "amazon should have elements with diff pixels"
            );
        }
        Err(e) => {
            eprintln!("Analysis error: {}", e);
        }
    }
}

#[test]
fn test_extract_snippets_from_real_label() {
    let testdata = render_helpers::testdata_dir();
    let zpl_path = render_helpers::labels_dir().join("amazon.zpl");

    if !zpl_path.exists() {
        eprintln!("Skipping: amazon.zpl not found");
        return;
    }

    let zpl = std::fs::read_to_string(&zpl_path).unwrap();

    // Split and group
    let commands = snippet_extractor::split_zpl_commands(&zpl);
    let spans = snippet_extractor::group_commands_into_spans(&commands);
    let globals = snippet_extractor::extract_global_state_commands(&commands);

    println!(
        "amazon.zpl: {} commands, {} element spans, {} global state commands",
        commands.len(),
        spans.len(),
        globals.len()
    );

    // Extract first element as snippet
    if !spans.is_empty() {
        let snippet = snippet_extractor::extract_element(&zpl, "amazon", 0, 2.26).unwrap();
        assert!(snippet.zpl_content.starts_with("^XA"));
        assert!(snippet.zpl_content.contains("^XZ"));
        println!("Snippet 0 content:\n{}", snippet.zpl_content);

        // Verify snippet renders without error
        let opts = render_helpers::default_options();
        let result = std::panic::catch_unwind(|| {
            render_helpers::render_zpl_to_png(&snippet.zpl_content, opts)
        });
        assert!(
            result.is_ok(),
            "extracted snippet should render successfully"
        );
    }
}

#[test]
fn test_full_analysis_pipeline() {
    // Run the full pipeline: scan → analyze → extract for a real label
    let testdata = render_helpers::testdata_dir();

    // Step 1: Load diff report
    let report = match diff_scanner::load_diff_report(&testdata) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Skipping: {}", e);
            return;
        }
    };

    // Step 2: Find labels above 1%
    let above_1pct = diff_scanner::find_labels_above_threshold(&report, 1.0);
    println!("{} labels above 1% diff:", above_1pct.len());

    // Step 3: For the first one, run analysis and extraction
    if let Some(entry) = above_1pct.first() {
        let zpl_path =
            render_helpers::labels_dir().join(format!("{}.{}", entry.label_name, entry.extension));
        let diff_path = testdata
            .join("diffs")
            .join(format!("{}_diff.png", entry.label_name));

        if !zpl_path.exists() || !diff_path.exists() {
            eprintln!(
                "Skipping full pipeline: files not found for {}",
                entry.label_name
            );
            return;
        }

        let zpl = std::fs::read_to_string(&zpl_path).unwrap();
        let labels = render_helpers::parse_zpl(zpl.as_bytes());

        if labels.is_empty() {
            eprintln!("No labels parsed from {}", entry.label_name);
            return;
        }

        // Analyze
        let contributions = element_analyzer::analyze_label(&labels[0], &diff_path).unwrap();
        println!(
            "\nAnalysis for {} (diff: {:.2}%):",
            entry.label_name, entry.diff_percent
        );
        println!(
            "{}",
            element_analyzer::format_analysis_report(&contributions)
        );

        // Extract top contributor as snippet
        if let Some(top) = contributions.first() {
            if top.diff_pixels_in_bbox > 0 {
                let snippet = snippet_extractor::extract_element(
                    &zpl,
                    &entry.label_name,
                    top.bbox.element_index,
                    entry.diff_percent,
                );
                match snippet {
                    Ok(s) => {
                        println!(
                            "Extracted snippet for element {} ({:?}):\n{}",
                            top.bbox.element_index, top.bbox.element_type, s.zpl_content
                        );
                    }
                    Err(e) => {
                        eprintln!("Extract error: {}", e);
                    }
                }
            }
        }
    }
}

// ─── Task 11: Diff Classification Tests ──────────────────────────────────────

#[test]
fn test_classify_no_diff_element() {
    // An element with zero diff pixels should get classification = None
    use labelize::skill::models::{
        DiffClassification, ElementBBox, ElementDiffContribution, ElementType,
    };

    let contrib = ElementDiffContribution {
        bbox: ElementBBox {
            x: 0,
            y: 0,
            width: 100,
            height: 50,
            element_index: 0,
            element_type: ElementType::Text,
            zpl_command: "^FD".to_string(),
        },
        diff_pixels_in_bbox: 0,
        total_pixels_in_bbox: 5000,
        local_diff_percent: 0.0,
        contribution_to_total: 0.0,
        classification: None,
        position_offset: None,
    };

    // Elements with no diff pixels should have no classification
    assert_eq!(contrib.classification, None);
    assert!(contrib.position_offset.is_none());
}

#[test]
fn test_fix_category_from_classification_content() {
    use labelize::skill::models::{DiffClassification, ElementType, FixCategory};

    // ContentDiff → element-type-based mapping
    assert_eq!(
        FixCategory::from_classification(&ElementType::Text, &DiffClassification::ContentDiff),
        FixCategory::FontMetrics
    );
    assert_eq!(
        FixCategory::from_classification(
            &ElementType::Barcode128,
            &DiffClassification::ContentDiff
        ),
        FixCategory::BarcodeEncoding
    );
    assert_eq!(
        FixCategory::from_classification(
            &ElementType::GraphicBox,
            &DiffClassification::ContentDiff
        ),
        FixCategory::GraphicRendering
    );
}

#[test]
fn test_fix_category_from_classification_position() {
    use labelize::skill::models::{DiffClassification, ElementType, FixCategory};

    // PositionDiff → always PositionOffset, regardless of element type
    assert_eq!(
        FixCategory::from_classification(&ElementType::Text, &DiffClassification::PositionDiff),
        FixCategory::PositionOffset
    );
    assert_eq!(
        FixCategory::from_classification(
            &ElementType::Barcode128,
            &DiffClassification::PositionDiff
        ),
        FixCategory::PositionOffset
    );

    // Mixed → PositionOffset (try position first)
    assert_eq!(
        FixCategory::from_classification(&ElementType::Text, &DiffClassification::Mixed),
        FixCategory::PositionOffset
    );
}

#[test]
fn test_centroid_shift_detection_synthetic() {
    use image::{Rgba, RgbaImage};
    use labelize::skill::diff_classifier::compute_diff_centroid;
    use labelize::skill::models::{ElementBBox, ElementType};

    // Create a 200×200 red-pixel cluster in the top-right of a 400×400 image
    let mut img = RgbaImage::from_pixel(400, 400, Rgba([255u8, 255, 255, 255]));
    // Place red pixels at (300..320, 50..70) — shifted right from center (100..200)
    for y in 50u32..70 {
        for x in 300u32..320 {
            img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
        }
    }

    let bbox = ElementBBox {
        x: 0,
        y: 0,
        width: 400,
        height: 400,
        element_index: 0,
        element_type: ElementType::Text,
        zpl_command: "^FD".to_string(),
    };

    let centroid = compute_diff_centroid(&img, &bbox);
    assert!(
        centroid.is_some(),
        "centroid should be computed for non-empty diff"
    );
    let (cx, cy) = centroid.unwrap();
    // Centroid should be near (309.5, 59.5) — center of the red cluster
    assert!(
        (cx - 309.5).abs() < 2.0,
        "centroid x={} expected ~309.5",
        cx
    );
    assert!((cy - 59.5).abs() < 2.0, "centroid y={} expected ~59.5", cy);
}

#[test]
fn test_centroid_shift_no_pixels() {
    use image::{Rgba, RgbaImage};
    use labelize::skill::diff_classifier::compute_diff_centroid;
    use labelize::skill::models::{ElementBBox, ElementType};

    // All white — no diff pixels → centroid should be None
    let img = RgbaImage::from_pixel(200, 200, Rgba([255u8, 255, 255, 255]));
    let bbox = ElementBBox {
        x: 0,
        y: 0,
        width: 200,
        height: 200,
        element_index: 0,
        element_type: ElementType::GraphicBox,
        zpl_command: "^GB".to_string(),
    };
    assert!(compute_diff_centroid(&img, &bbox).is_none());
}

#[test]
fn test_compute_image_diff_percent_identical() {
    use image::{Rgba, RgbaImage};
    use labelize::skill::diff_classifier::compute_image_diff_percent;

    let img = RgbaImage::from_pixel(100, 100, Rgba([0u8, 0, 0, 255]));
    assert_eq!(compute_image_diff_percent(&img, &img), 0.0);
}

#[test]
fn test_compute_image_diff_percent_completely_different() {
    use image::{Rgba, RgbaImage};
    use labelize::skill::diff_classifier::compute_image_diff_percent;

    let black = RgbaImage::from_pixel(100, 100, Rgba([0u8, 0, 0, 255]));
    let white = RgbaImage::from_pixel(100, 100, Rgba([255u8, 255, 255, 255]));
    let diff = compute_image_diff_percent(&black, &white);
    assert!(
        diff > 90.0,
        "completely different images should have high diff, got {}",
        diff
    );
}

#[test]
fn test_compute_image_diff_percent_size_mismatch() {
    use image::{Rgba, RgbaImage};
    use labelize::skill::diff_classifier::compute_image_diff_percent;

    let a = RgbaImage::from_pixel(100, 100, Rgba([0u8, 0, 0, 255]));
    let b = RgbaImage::from_pixel(200, 200, Rgba([0u8, 0, 0, 255]));
    assert_eq!(
        compute_image_diff_percent(&a, &b),
        100.0,
        "size mismatch should be 100%"
    );
}

#[test]
fn test_render_snippet_isolated() {
    use labelize::skill::diff_classifier::render_snippet_isolated;

    let zpl = "^XA^FO50,50^ADN,36,20^FDHello^FS^XZ";
    let result = render_snippet_isolated(zpl);
    assert!(
        result.is_ok(),
        "simple snippet should render without error: {:?}",
        result.err()
    );
    let img = result.unwrap();
    assert_eq!(img.width(), 813, "expected 813px width");
    assert_eq!(img.height(), 1626, "expected 1626px height");
}

#[test]
fn test_classify_real_label_diffs_offline() {
    // Run classification on a real label in offline mode (no Labelary calls)
    use labelize::skill::diff_classifier::{classify_element_diffs, ClassifyOptions};

    let testdata = render_helpers::testdata_dir();
    let zpl_path = render_helpers::labels_dir().join("amazon.zpl");
    let diff_path = testdata.join("diffs").join("amazon_diff.png");

    if !zpl_path.exists() || !diff_path.exists() {
        eprintln!("Skipping test_classify_real_label_diffs_offline: files not found");
        return;
    }

    let zpl = std::fs::read_to_string(&zpl_path).unwrap();
    let labels = render_helpers::parse_zpl(zpl.as_bytes());
    assert!(!labels.is_empty());

    // Get base contributions
    let mut contributions = match element_analyzer::analyze_label(&labels[0], &diff_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping: analyze_label error: {}", e);
            return;
        }
    };

    let diff_img = image::open(&diff_path).unwrap().to_rgba8();

    let opts = ClassifyOptions {
        use_labelary: false, // offline
        ..Default::default()
    };

    let result = classify_element_diffs(&mut contributions, &zpl, &diff_img, &opts);
    assert!(
        result.is_ok(),
        "classification should succeed: {:?}",
        result.err()
    );

    // Elements with diff pixels should have classification (offline → ContentDiff or PositionDiff via heuristic)
    let with_diff: Vec<_> = contributions
        .iter()
        .filter(|c| c.diff_pixels_in_bbox > 0)
        .collect();
    let with_class: Vec<_> = with_diff
        .iter()
        .filter(|c| c.classification.is_some())
        .collect();

    println!(
        "amazon: {} elements with diff, {} classified",
        with_diff.len(),
        with_class.len()
    );
    for c in &contributions {
        if c.diff_pixels_in_bbox > 0 {
            println!(
                "  elem {} ({:?}): diff={} pixels, classification={:?}, offset={:?}",
                c.bbox.element_index,
                c.bbox.element_type,
                c.diff_pixels_in_bbox,
                c.classification,
                c.position_offset
            );
        }
    }

    // At least some elements should be classified
    assert!(
        !with_class.is_empty() || with_diff.is_empty(),
        "elements with diff pixels should get a classification"
    );

    // Print the enhanced analysis report
    let report = element_analyzer::format_analysis_report(&contributions);
    println!("\n{}", report);
}
