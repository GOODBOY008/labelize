mod common;

use common::image_compare;
use common::render_helpers;

/// Maximum allowed pixel-difference percentage for migrated tests.
const MIGRATED_TOLERANCE: f64 = 50.0;

fn testdata_dir() -> std::path::PathBuf {
    render_helpers::testdata_dir()
}

/// Run a golden-file comparison for a ZPL test case.
fn golden_zpl(name: &str) {
    golden_zpl_with_tolerance(name, MIGRATED_TOLERANCE);
}

fn golden_zpl_with_tolerance(name: &str, tolerance: f64) {
    let dir = testdata_dir();
    let input = dir.join(format!("{}.zpl", name));
    let expected = dir.join(format!("{}.png", name));

    if !input.exists() || !expected.exists() {
        eprintln!("SKIP {}: missing input or golden file", name);
        return;
    }

    let content = std::fs::read_to_string(&input).expect("read input");
    let actual_png = render_helpers::render_zpl_to_png(&content, render_helpers::default_options());
    let expected_png = std::fs::read(&expected).expect("read golden");
    let result = image_compare::compare_images(&actual_png, &expected_png, tolerance);

    if result.diff_percent > tolerance {
        if let Some(ref diff_img) = result.diff_image {
            image_compare::save_diff_image(name, diff_img);
        }
    }

    // Optionally update golden file
    if std::env::var("LABELIZE_UPDATE_GOLDEN").is_ok() && result.diff_percent > 0.0 {
        std::fs::write(&expected, &actual_png).expect("update golden file");
        return;
    }

    assert!(
        result.diff_percent <= tolerance,
        "ZPL golden test '{}' FAILED: {:.2}% pixel diff (tolerance: {:.2}%), dims: actual={:?}, expected={:?}",
        name,
        result.diff_percent,
        tolerance,
        result.actual_dims,
        result.expected_dims,
    );
}

/// Run a golden-file comparison for an EPL test case.
fn golden_epl(name: &str) {
    golden_epl_with_tolerance(name, MIGRATED_TOLERANCE);
}

fn golden_epl_with_tolerance(name: &str, tolerance: f64) {
    let dir = testdata_dir();
    let input = dir.join(format!("{}.epl", name));
    let expected = dir.join(format!("{}.png", name));

    if !input.exists() || !expected.exists() {
        eprintln!("SKIP {}: missing input or golden file", name);
        return;
    }

    let content = std::fs::read_to_string(&input).expect("read input");
    let actual_png = render_helpers::render_epl_to_png(&content, render_helpers::default_options());
    let expected_png = std::fs::read(&expected).expect("read golden");
    let result = image_compare::compare_images(&actual_png, &expected_png, tolerance);

    if result.diff_percent > tolerance {
        if let Some(ref diff_img) = result.diff_image {
            image_compare::save_diff_image(name, diff_img);
        }
    }

    if std::env::var("LABELIZE_UPDATE_GOLDEN").is_ok() && result.diff_percent > 0.0 {
        std::fs::write(&expected, &actual_png).expect("update golden file");
        return;
    }

    assert!(
        result.diff_percent <= tolerance,
        "EPL golden test '{}' FAILED: {:.2}% pixel diff (tolerance: {:.2}%), dims: actual={:?}, expected={:?}",
        name,
        result.diff_percent,
        tolerance,
        result.actual_dims,
        result.expected_dims,
    );
}

// ── ZPL golden tests ──────────────────────────────────────────────
// Tolerances are per-label ceilings (current diff + headroom).
// See docs/DIFF_THRESHOLDS.md for rationale.

#[test]
fn golden_amazon() {
    golden_zpl_with_tolerance("amazon", 4.0);
}
#[test]
fn golden_aztec_ec() {
    golden_zpl_with_tolerance("aztec_ec", 9.0);
}
#[test]
fn golden_barcode128_default_width() {
    golden_zpl_with_tolerance("barcode128_default_width", 2.0);
}
#[test]
fn golden_barcode128_line() {
    golden_zpl_with_tolerance("barcode128_line", 2.0);
}
#[test]
fn golden_barcode128_line_above() {
    golden_zpl_with_tolerance("barcode128_line_above", 2.0);
}
#[test]
fn golden_barcode128_mode_a() {
    golden_zpl_with_tolerance("barcode128_mode_a", 2.0);
}
#[test]
fn golden_barcode128_mode_d() {
    golden_zpl_with_tolerance("barcode128_mode_d", 2.0);
}
#[test]
fn golden_barcode128_mode_n() {
    golden_zpl_with_tolerance("barcode128_mode_n", 2.0);
}
#[test]
fn golden_barcode128_mode_n_cba_sets() {
    golden_zpl_with_tolerance("barcode128_mode_n_cba_sets", 2.0);
}
#[test]
fn golden_barcode128_mode_u() {
    golden_zpl_with_tolerance("barcode128_mode_u", 3.0);
}
#[test]
fn golden_barcode128_rotated() {
    golden_zpl_with_tolerance("barcode128_rotated", 2.0);
}
#[test]
fn golden_bstc() {
    golden_zpl_with_tolerance("bstc", 1.0);
}
#[test]
fn golden_dbs() {
    golden_zpl_with_tolerance("dbs", 6.0);
}
#[test]
fn golden_dhlecommercetr() {
    golden_zpl_with_tolerance("dhlecommercetr", 6.0);
}
#[test]
fn golden_dhlpaket() {
    golden_zpl_with_tolerance("dhlpaket", 4.0);
}
#[test]
fn golden_dhlparceluk() {
    golden_zpl_with_tolerance("dhlparceluk", 5.5);
}
#[test]
fn golden_dpdpl() {
    golden_zpl_with_tolerance("dpdpl", 8.0);
}
#[test]
fn golden_ean13() {
    golden_zpl_with_tolerance("ean13", 3.0);
}
#[test]
fn golden_encodings_013() {
    golden_zpl_with_tolerance("encodings_013", 3.0);
}
#[test]
fn golden_fedex() {
    golden_zpl_with_tolerance("fedex", 9.0);
}
#[test]
fn golden_gd_thin_r() {
    golden_zpl_with_tolerance("gd_thin_r", 2.0);
}
#[test]
fn golden_gd_thin_l() {
    golden_zpl_with_tolerance("gd_thin_l", 2.0);
}
#[test]
fn golden_gd_thick() {
    golden_zpl_with_tolerance("gd_thick", 4.5);
}
#[test]
fn golden_gd_default_params() {
    golden_zpl_with_tolerance("gd_default_params", 1.0);
}
#[test]
fn golden_gb_0_height() {
    golden_zpl_with_tolerance("gb_0_height", 1.0);
}
#[test]
fn golden_gb_0_width() {
    golden_zpl_with_tolerance("gb_0_width", 1.0);
}
#[test]
fn golden_gb_normal() {
    golden_zpl_with_tolerance("gb_normal", 1.0);
}
#[test]
fn golden_gb_rounded() {
    golden_zpl_with_tolerance("gb_rounded", 1.0);
}
#[test]
fn golden_glscz() {
    golden_zpl_with_tolerance("glscz", 5.0);
}
#[test]
fn golden_glsdk_return() {
    golden_zpl_with_tolerance("glsdk_return", 7.0);
}
#[test]
fn golden_gs() {
    golden_zpl_with_tolerance("gs", 2.0);
}
#[test]
fn golden_icapaket() {
    golden_zpl_with_tolerance("icapaket", 6.0);
}
#[test]
fn golden_jcpenney() {
    golden_zpl_with_tolerance("jcpenney", 8.0);
}
#[test]
fn golden_kmart() {
    golden_zpl_with_tolerance("kmart", 9.0);
}
#[test]
fn golden_labelary() {
    golden_zpl_with_tolerance("labelary", 5.5);
}
#[test]
fn golden_pnldpd() {
    golden_zpl_with_tolerance("pnldpd", 14.5);
}
#[test]
fn golden_pocztex() {
    golden_zpl_with_tolerance("pocztex", 5.5);
}
#[test]
fn golden_porterbuddy() {
    golden_zpl_with_tolerance("porterbuddy", 10.0);
}
#[test]
fn golden_posten() {
    golden_zpl_with_tolerance("posten", 5.0);
}
#[test]
fn golden_qr_code_ft_manual() {
    golden_zpl_with_tolerance("qr_code_ft_manual", 4.0);
}
#[test]
fn golden_qr_code_offset() {
    golden_zpl_with_tolerance("qr_code_offset", 3.0);
}
#[test]
fn golden_return_qrcode() {
    golden_zpl_with_tolerance("return_qrcode", 7.0);
}
#[test]
fn golden_reverse_qr() {
    golden_zpl_with_tolerance("reverse_qr", 5.0);
}
#[test]
fn golden_reverse() {
    golden_zpl_with_tolerance("reverse", 2.0);
}
#[test]
fn golden_swisspost() {
    golden_zpl_with_tolerance("swisspost", 3.0);
}
#[test]
fn golden_templating() {
    golden_zpl_with_tolerance("templating", 3.0);
}
#[test]
fn golden_text_fallback_default() {
    golden_zpl_with_tolerance("text_fallback_default", 5.0);
}
#[test]
fn golden_text_fo_b() {
    golden_zpl_with_tolerance("text_fo_b", 1.0);
}
#[test]
fn golden_text_fo_i() {
    golden_zpl_with_tolerance("text_fo_i", 1.0);
}
#[test]
fn golden_text_fo_n() {
    golden_zpl_with_tolerance("text_fo_n", 1.0);
}
#[test]
fn golden_text_fo_r() {
    golden_zpl_with_tolerance("text_fo_r", 1.0);
}
#[test]
fn golden_text_ft_auto_pos() {
    golden_zpl_with_tolerance("text_ft_auto_pos", 3.0);
}
#[test]
fn golden_text_ft_b() {
    golden_zpl_with_tolerance("text_ft_b", 1.0);
}
#[test]
fn golden_text_ft_i() {
    golden_zpl_with_tolerance("text_ft_i", 1.0);
}
#[test]
fn golden_text_ft_n() {
    golden_zpl_with_tolerance("text_ft_n", 1.0);
}
#[test]
fn golden_text_ft_r() {
    golden_zpl_with_tolerance("text_ft_r", 1.0);
}
#[test]
fn golden_text_multiline() {
    golden_zpl_with_tolerance("text_multiline", 2.0);
}
#[test]
fn golden_ups_surepost() {
    golden_zpl_with_tolerance("ups_surepost", 12.0);
}
#[test]
fn golden_ups() {
    golden_zpl_with_tolerance("ups", 9.0);
}
#[test]
fn golden_usps() {
    golden_zpl_with_tolerance("usps", 6.0);
}

// ── New Carrier Labels (March 2026) ────────────────────────────────

#[test]
fn golden_tnt_express() {
    golden_zpl_with_tolerance("tnt_express", 6.0);
}
#[test]
fn golden_royalmail() {
    golden_zpl_with_tolerance("royalmail", 5.5);
}
#[test]
fn golden_canadapost() {
    golden_zpl_with_tolerance("canadapost", 6.5);
}
#[test]
fn golden_auspost() {
    golden_zpl_with_tolerance("auspost", 6.0);
}
#[test]
fn golden_colissimo() {
    golden_zpl_with_tolerance("colissimo", 5.0);
}
#[test]
fn golden_postnl() {
    golden_zpl_with_tolerance("postnl", 5.5);
}
#[test]
fn golden_bpost() {
    golden_zpl_with_tolerance("bpost", 5.5);
}
#[test]
fn golden_correos() {
    golden_zpl_with_tolerance("correos", 5.5);
}
#[test]
fn golden_dbschenker() {
    golden_zpl_with_tolerance("dbschenker", 6.0);
}
#[test]
fn golden_evri() {
    golden_zpl_with_tolerance("evri", 5.0);
}
#[test]
fn golden_dpdde() {
    golden_zpl_with_tolerance("dpdde", 5.5);
}
#[test]
fn golden_ontrac() {
    golden_zpl_with_tolerance("ontrac", 6.0);
}
#[test]
fn golden_seur() {
    golden_zpl_with_tolerance("seur", 5.5);
}
#[test]
fn golden_purolator() {
    golden_zpl_with_tolerance("purolator", 4.5);
}
#[test]
fn golden_inpost() {
    golden_zpl_with_tolerance("inpost", 7.0);
}
#[test]
fn golden_yodel() {
    golden_zpl_with_tolerance("yodel", 5.5);
}
#[test]
fn golden_pdf417_basic() {
    golden_zpl_with_tolerance("pdf417_basic", 1.0);
}

// ── Italian carrier golden tests ─────────────────────────────────
// Anonymized real-world labels from Italian e-commerce shipping.
// Labelary API used as reference renderer for expected PNGs.

#[test]
fn golden_dhlparcelit() {
    // DHL Parcel Italy: ^A0I dominant, ~DG/^XG stored graphics (DHL logo),
    // Code128 barcodes, ^FH hex encoding
    golden_zpl_with_tolerance("dhlparcelit", 12.0);
}

#[test]
fn golden_brtit() {
    // BRT (Bartolini) Italy: ^POI orientation, ~DG000.GRF logo,
    // ^A0B rotated text, ^FR reverse video, Code128
    golden_zpl_with_tolerance("brtit", 8.0);
}

#[test]
fn golden_posteit() {
    // Poste Italiane: DataMatrix ^BX, ^GFA Z64 compressed logo,
    // ^BCB bottom-up barcode, ^FH hex in all fields
    golden_zpl_with_tolerance("posteit", 8.0);
}

#[test]
fn golden_amazonshipping() {
    // Amazon Shipping (MXP5): ^BXN/B/I/R DataMatrix in all 4 orientations,
    // ^FR field reverse, ^GFA inline graphics, ^FH hex in all fields
    golden_zpl_with_tolerance("amazonshipping", 10.0);
}

// ── EPL golden tests ──────────────────────────────────────────────

#[test]
fn golden_dpduk_epl() {
    golden_epl_with_tolerance("dpduk", 6.5);
}
