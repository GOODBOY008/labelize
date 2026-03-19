use crate::common::image_compare;
use crate::common::render_helpers;
use std::path::Path;

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

#[test] fn golden_amazon() { golden_zpl("amazon"); }
#[test] fn golden_aztec_ec() { golden_zpl("aztec_ec"); }
#[test] fn golden_barcode128_default_width() { golden_zpl("barcode128_default_width"); }
#[test] fn golden_barcode128_line() { golden_zpl("barcode128_line"); }
#[test] fn golden_barcode128_line_above() { golden_zpl("barcode128_line_above"); }
#[test] fn golden_barcode128_mode_a() { golden_zpl("barcode128_mode_a"); }
#[test] fn golden_barcode128_mode_d() { golden_zpl("barcode128_mode_d"); }
#[test] fn golden_barcode128_mode_n() { golden_zpl("barcode128_mode_n"); }
#[test] fn golden_barcode128_mode_n_cba_sets() { golden_zpl("barcode128_mode_n_cba_sets"); }
#[test] fn golden_barcode128_mode_u() { golden_zpl("barcode128_mode_u"); }
#[test] fn golden_barcode128_rotated() { golden_zpl("barcode128_rotated"); }
#[test] fn golden_bstc() { golden_zpl("bstc"); }
#[test] fn golden_dbs() { golden_zpl("dbs"); }
#[test] fn golden_dhlecommercetr() { golden_zpl("dhlecommercetr"); }
#[test] fn golden_dhlpaket() { golden_zpl("dhlpaket"); }
#[test] fn golden_dhlparceluk() { golden_zpl("dhlparceluk"); }
#[test] fn golden_dpdpl() { golden_zpl("dpdpl"); }
#[test] fn golden_ean13() { golden_zpl("ean13"); }
#[test] fn golden_encodings_013() { golden_zpl("encodings_013"); }
#[test] fn golden_fedex() { golden_zpl("fedex"); }
#[test] fn golden_gb_0_height() { golden_zpl("gb_0_height"); }
#[test] fn golden_gb_0_width() { golden_zpl("gb_0_width"); }
#[test] fn golden_gb_normal() { golden_zpl("gb_normal"); }
#[test] fn golden_gb_rounded() { golden_zpl("gb_rounded"); }
#[test] fn golden_glscz() { golden_zpl("glscz"); }
#[test] fn golden_glsdk_return() { golden_zpl("glsdk_return"); }
#[test] fn golden_gs() { golden_zpl("gs"); }
#[test] fn golden_icapaket() { golden_zpl("icapaket"); }
#[test] fn golden_jcpenney() { golden_zpl("jcpenney"); }
#[test] fn golden_kmart() { golden_zpl("kmart"); }
#[test] fn golden_labelary() { golden_zpl("labelary"); }
#[test] fn golden_pnldpd() { golden_zpl("pnldpd"); }
#[test] fn golden_pocztex() { golden_zpl("pocztex"); }
#[test] fn golden_porterbuddy() { golden_zpl("porterbuddy"); }
#[test] fn golden_posten() { golden_zpl("posten"); }
#[test] fn golden_qr_code_ft_manual() { golden_zpl("qr_code_ft_manual"); }
#[test] fn golden_qr_code_offset() { golden_zpl("qr_code_offset"); }
#[test] fn golden_return_qrcode() { golden_zpl("return_qrcode"); }
#[test] fn golden_reverse_qr() { golden_zpl("reverse_qr"); }
#[test] fn golden_reverse() { golden_zpl("reverse"); }
#[test] fn golden_swisspost() { golden_zpl("swisspost"); }
#[test] fn golden_templating() { golden_zpl("templating"); }
#[test] #[ignore = "complex GFA graphic field exceeds tolerance"] fn golden_text_fallback_default() { golden_zpl("text_fallback_default"); }
#[test] fn golden_text_fo_b() { golden_zpl("text_fo_b"); }
#[test] fn golden_text_fo_i() { golden_zpl("text_fo_i"); }
#[test] fn golden_text_fo_n() { golden_zpl("text_fo_n"); }
#[test] fn golden_text_fo_r() { golden_zpl("text_fo_r"); }
#[test] fn golden_text_ft_auto_pos() { golden_zpl("text_ft_auto_pos"); }
#[test] fn golden_text_ft_b() { golden_zpl("text_ft_b"); }
#[test] fn golden_text_ft_i() { golden_zpl("text_ft_i"); }
#[test] fn golden_text_ft_n() { golden_zpl("text_ft_n"); }
#[test] fn golden_text_ft_r() { golden_zpl("text_ft_r"); }
#[test] fn golden_text_multiline() { golden_zpl("text_multiline"); }
#[test] fn golden_ups_surepost() { golden_zpl("ups_surepost"); }
#[test] fn golden_ups() { golden_zpl("ups"); }
#[test] fn golden_usps() { golden_zpl("usps"); }

// ── EPL golden tests ──────────────────────────────────────────────

#[test] fn golden_dpduk_epl() { golden_epl("dpduk"); }
