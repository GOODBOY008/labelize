mod common;
#[path = "common/fixture_test_dir.rs"]
mod fixture_test_dir;

use common::image_compare;
#[path = "common/golden_fixture.rs"]
mod golden_fixture;
use common::render_helpers;

/// Maximum allowed pixel-difference percentage for carrier label tests.
const LABEL_TOLERANCE: f64 = 15.0;
/// Tolerance for unit/synthetic tests — compared against Labelary reference at 813×1626.
const UNIT_TOLERANCE: f64 = 8.0;

fn testdata_dir() -> std::path::PathBuf {
    render_helpers::testdata_dir()
}

/// Run a golden-file comparison for a ZPL test case.
fn golden_zpl(name: &str) {
    golden_zpl_with_tolerance(name, LABEL_TOLERANCE);
}

fn effective_zpl_tolerance(is_unit: bool, requested: f64) -> f64 {
    if is_unit {
        requested.min(UNIT_TOLERANCE)
    } else {
        requested
    }
}

#[test]
fn unit_golden_tolerance_honors_stricter_requested_limit() {
    assert_eq!(effective_zpl_tolerance(true, 0.0), 0.0);
    assert_eq!(effective_zpl_tolerance(true, 1.0), 1.0);
    assert_eq!(effective_zpl_tolerance(true, 15.0), UNIT_TOLERANCE);
    assert_eq!(effective_zpl_tolerance(false, 15.0), 15.0);
}

fn assert_zpl_comparison(name: &str, result: &image_compare::CompareResult, tolerance: f64) {
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

#[test]
fn unit_golden_rejects_two_percent_pixel_diff_at_one_percent_limit() {
    let expected = image::RgbaImage::from_pixel(100, 1, image::Rgba([255, 255, 255, 255]));
    let mut actual = expected.clone();
    actual.put_pixel(0, 0, image::Rgba([0, 0, 0, 255]));
    actual.put_pixel(1, 0, image::Rgba([0, 0, 0, 255]));
    let encode = |img: image::RgbaImage| {
        let mut bytes = std::io::Cursor::new(Vec::new());
        img.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        bytes.into_inner()
    };
    let limit = effective_zpl_tolerance(true, 1.0);
    let result = image_compare::compare_images(&encode(actual), &encode(expected), limit);
    assert_eq!(result.diff_percent, 2.0);
    let failure = std::panic::catch_unwind(|| assert_zpl_comparison("two_pixels", &result, limit))
        .expect_err("2% difference must fail a requested 1% limit");
    let message = failure.downcast_ref::<String>().unwrap();
    assert!(message.contains("tolerance: 1.00%"), "{message}");
    assert_zpl_comparison("boundary", &result, effective_zpl_tolerance(true, 2.0));
    assert_zpl_comparison("default", &result, effective_zpl_tolerance(true, 15.0));
}

fn golden_zpl_with_tolerance(name: &str, tolerance: f64) {
    golden_zpl_in(&testdata_dir(), name, tolerance);
}

fn golden_zpl_in(dir: &std::path::Path, name: &str, tolerance: f64) {
    let (input, is_unit) =
        golden_fixture::resolve_input(dir, name, "zpl").unwrap_or_else(|e| panic!("{e}"));
    let fixture = golden_fixture::load_fixture(&input).unwrap_or_else(|e| panic!("{e}"));
    let options = render_helpers::default_options();
    let effective_tolerance = effective_zpl_tolerance(is_unit, tolerance);
    let content = fixture.content;
    let actual_png = render_helpers::render_zpl_to_png(&content, options);
    let expected_png = fixture.reference;
    let result = image_compare::compare_images(&actual_png, &expected_png, effective_tolerance);

    if result.diff_percent > effective_tolerance {
        if let Some(ref diff_img) = result.diff_image {
            image_compare::save_diff_image(name, diff_img);
        }
    }

    assert_zpl_comparison(name, &result, effective_tolerance);
}

/// Run a golden-file comparison for an EPL test case.
fn golden_epl(name: &str) {
    golden_epl_with_tolerance(name, LABEL_TOLERANCE);
}

fn golden_epl_with_tolerance(name: &str, tolerance: f64) {
    golden_epl_in(&testdata_dir(), name, tolerance);
}

fn golden_epl_in(dir: &std::path::Path, name: &str, tolerance: f64) {
    let (input, _) =
        golden_fixture::resolve_input(dir, name, "epl").unwrap_or_else(|e| panic!("{e}"));
    let fixture = golden_fixture::load_fixture(&input).unwrap_or_else(|e| panic!("{e}"));
    let content = fixture.content;
    let actual_png = render_helpers::render_epl_to_png(&content, render_helpers::default_options());
    let expected_png = fixture.reference;
    let result = image_compare::compare_images(&actual_png, &expected_png, tolerance);

    if result.diff_percent > tolerance {
        if let Some(ref diff_img) = result.diff_image {
            image_compare::save_diff_image(name, diff_img);
        }
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

#[test]
fn golden_contract_missing_input_or_reference_fails_for_zpl_and_epl() {
    type Compare = fn(&std::path::Path, &str, f64);
    for (extension, compare) in [
        ("zpl", golden_zpl_in as Compare),
        ("epl", golden_epl_in as Compare),
    ] {
        let dir = fixture_test_dir::TestDir::new();
        assert!(std::panic::catch_unwind(|| compare(&dir.0, "missing", 8.0)).is_err());
        let input = dir.0.join(format!("missing.{extension}"));
        std::fs::write(&input, "input").unwrap();
        assert!(std::panic::catch_unwind(|| compare(&dir.0, "missing", 8.0)).is_err());
        assert!(!input.with_extension("png").exists());
    }
}

#[test]
fn golden_contract_update_environment_cannot_replace_references() {
    const CHILD_ROOT: &str = "LABELIZE_GOLDEN_CONTRACT_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let root = std::path::PathBuf::from(root);
        type Compare = fn(&std::path::Path, &str, f64);
        for (extension, compare) in [
            ("zpl", golden_zpl_in as Compare),
            ("epl", golden_epl_in as Compare),
        ] {
            let name = format!("readonly_{extension}");
            let reference = root.join(format!("{name}.png"));
            let before = std::fs::read(&reference).unwrap();
            assert!(
                std::panic::catch_unwind(|| compare(&root, &name, 0.0)).is_err(),
                "mismatched reference must fail"
            );
            assert_eq!(std::fs::read(reference).unwrap(), before);
        }
        return;
    }
    let dir = fixture_test_dir::TestDir::new();
    std::fs::write(
        dir.0.join("readonly_zpl.zpl"),
        "^XA^FO10,10^GB20,10,10^FS^XZ",
    )
    .unwrap();
    std::fs::write(dir.0.join("readonly_epl.epl"), "N\nLO10,10,20,10\nP1\n").unwrap();
    for extension in ["zpl", "epl"] {
        std::fs::write(
            dir.0.join(format!("readonly_{extension}.png")),
            fixture_test_dir::png(),
        )
        .unwrap();
    }
    // A child process isolates the old update flag and any failure diff artifacts.
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "golden_contract_update_environment_cannot_replace_references",
            "--nocapture",
        ])
        .env(CHILD_ROOT, &dir.0)
        .env("LABELIZE_UPDATE_GOLDEN", "1")
        .current_dir(&dir.0)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

// ── ZPL golden tests ──────────────────────────────────────────────
// Tolerances are per-label ceilings (current diff + headroom).
// See docs/DIFF_THRESHOLDS.md for rationale.

#[test]
fn golden_amazon() {
    golden_zpl_with_tolerance("amazon", 3.5);
}
#[test]
fn golden_aztec_ec() {
    golden_zpl_with_tolerance("aztec_ec", 7.5);
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
fn golden_code128_mode_d_fnc1() {
    golden_zpl_with_tolerance("code128_mode_d_fnc1", 1.0);
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
    golden_zpl_with_tolerance("barcode128_mode_u", 2.0);
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
    golden_zpl_with_tolerance("dbs", 5.0);
}
#[test]
fn golden_dhlecommercetr() {
    golden_zpl_with_tolerance("dhlecommercetr", 4.5);
}
#[test]
fn golden_dhlpaket() {
    golden_zpl_with_tolerance("dhlpaket", 3.5);
}
#[test]
fn golden_dhlparceluk() {
    golden_zpl_with_tolerance("dhlparceluk", 4.5);
}
#[test]
fn golden_dpdpl() {
    golden_zpl_with_tolerance("dpdpl", 7.5);
}
#[test]
fn golden_ean13() {
    golden_zpl_with_tolerance("ean13", 2.0);
}
#[test]
fn golden_cp850_hex_chars() {
    golden_zpl("cp850_hex_chars");
}
#[test]
fn golden_encodings_013() {
    golden_zpl_with_tolerance("encodings_013", 2.5);
}
#[test]
fn golden_fedex() {
    golden_zpl_with_tolerance("fedex", 7.0);
}
#[test]
fn golden_fedex_express() {
    golden_zpl_with_tolerance("fedex_express", 7.0);
}
#[test]
fn golden_fedex_ground() {
    golden_zpl_with_tolerance("fedex_ground", 6.0);
}
#[test]
fn golden_font_p() {
    golden_zpl("font_p");
}
#[test]
fn golden_font_q() {
    golden_zpl("font_q");
}
#[test]
fn golden_font_s() {
    golden_zpl("font_s");
}
#[test]
fn golden_font_r() {
    golden_zpl("font_r");
}
#[test]
fn golden_font_t() {
    golden_zpl("font_t");
}
#[test]
fn golden_font_u() {
    golden_zpl("font_u");
}
#[test]
fn golden_font_v() {
    golden_zpl("font_v");
}
#[test]
fn golden_gd_thin_r() {
    golden_zpl_with_tolerance("gd_thin_r", 1.0);
}
#[test]
fn golden_gd_thin_l() {
    golden_zpl_with_tolerance("gd_thin_l", 1.0);
}
#[test]
fn golden_gd_thick() {
    golden_zpl_with_tolerance("gd_thick", 1.0);
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
    golden_zpl_with_tolerance("glscz", 3.5);
}
#[test]
fn golden_glsdk_return() {
    golden_zpl_with_tolerance("glsdk_return", 5.5);
}
#[test]
fn golden_gs() {
    golden_zpl_with_tolerance("gs", 2.0);
}
#[test]
fn golden_icapaket() {
    golden_zpl_with_tolerance("icapaket", 5.5);
}
#[test]
fn golden_jcpenney() {
    golden_zpl_with_tolerance("jcpenney", 6.0);
}
#[test]
fn golden_kmart() {
    golden_zpl_with_tolerance("kmart", 8.0);
}
#[test]
fn golden_labelary() {
    golden_zpl_with_tolerance("labelary", 4.5);
}
#[test]
fn golden_mu_millimeters() {
    golden_zpl_with_tolerance("mu_millimeters", 8.0);
}
#[test]
fn golden_mu_dpi_conversion() {
    golden_zpl_with_tolerance("mu_dpi_conversion", 2.0);
}
#[test]
fn golden_pnldpd() {
    golden_zpl_with_tolerance("pnldpd", 11.5);
}
#[test]
fn golden_pocztex() {
    golden_zpl_with_tolerance("pocztex", 4.5);
}
#[test]
fn golden_porterbuddy() {
    golden_zpl_with_tolerance("porterbuddy", 7.0);
}
#[test]
fn golden_posten() {
    golden_zpl_with_tolerance("posten", 3.0);
}
#[test]
fn golden_qr_code_ft_manual() {
    golden_zpl_with_tolerance("qr_code_ft_manual", 1.0);
}
#[test]
fn golden_upce() {
    golden_zpl_with_tolerance("upce", 2.0);
}
#[test]
fn golden_lt_ls() {
    golden_zpl_with_tolerance("lt_ls", 2.0);
}
#[test]
fn golden_ge_ellipse() {
    golden_zpl_with_tolerance("ge_ellipse", 2.0);
}
#[test]
fn golden_ean8_upca() {
    golden_zpl_with_tolerance("ean8_upca", 2.0);
}
#[test]
fn golden_qr_code_offset() {
    golden_zpl_with_tolerance("qr_code_offset", 1.0);
}
#[test]
fn golden_return_qrcode() {
    golden_zpl_with_tolerance("return_qrcode", 4.0);
}
#[test]
fn golden_reverse_qr() {
    golden_zpl_with_tolerance("reverse_qr", 1.5);
}
#[test]
fn golden_reverse() {
    golden_zpl_with_tolerance("reverse", 1.5);
}
#[test]
fn golden_swisspost() {
    golden_zpl_with_tolerance("swisspost", 2.5);
}
#[test]
fn golden_templating() {
    golden_zpl_with_tolerance("templating", 2.5);
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
    golden_zpl_with_tolerance("text_ft_auto_pos", 2.5);
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
    golden_zpl_with_tolerance("text_multiline", 1.5);
}
#[test]
fn golden_ups_surepost() {
    golden_zpl_with_tolerance("ups_surepost", 10.0);
}
#[test]
fn golden_ups() {
    golden_zpl_with_tolerance("ups", 8.0);
}
#[test]
fn golden_ups_import_control() {
    golden_zpl_with_tolerance("ups_import_control", 4.5);
}
#[test]
fn golden_usps() {
    golden_zpl_with_tolerance("usps", 5.0);
}
#[test]
fn golden_usps_apo() {
    golden_zpl_with_tolerance("usps_apo", 4.0);
}
#[test]
fn golden_usps_intl() {
    golden_zpl_with_tolerance("usps_intl", 4.0);
}

// ── New Carrier Labels (March 2026) ────────────────────────────────

#[test]
fn golden_tnt_express() {
    golden_zpl_with_tolerance("tnt_express", 5.0);
}
#[test]
fn golden_royalmail() {
    golden_zpl_with_tolerance("royalmail", 4.5);
}
#[test]
fn golden_canadapost() {
    golden_zpl_with_tolerance("canadapost", 5.0);
}
#[test]
fn golden_auspost() {
    golden_zpl_with_tolerance("auspost", 5.0);
}
#[test]
fn golden_colissimo() {
    golden_zpl_with_tolerance("colissimo", 4.5);
}
#[test]
fn golden_postnl() {
    golden_zpl_with_tolerance("postnl", 5.0);
}
#[test]
fn golden_bpost() {
    golden_zpl_with_tolerance("bpost", 4.5);
}
#[test]
fn golden_correos() {
    golden_zpl_with_tolerance("correos", 5.0);
}
#[test]
fn golden_dbschenker() {
    golden_zpl_with_tolerance("dbschenker", 5.5);
}
#[test]
fn golden_evri() {
    golden_zpl_with_tolerance("evri", 4.5);
}
#[test]
fn golden_dpdde() {
    golden_zpl_with_tolerance("dpdde", 4.5);
}
#[test]
fn golden_ontrac() {
    golden_zpl_with_tolerance("ontrac", 4.5);
}
#[test]
fn golden_seur() {
    golden_zpl_with_tolerance("seur", 4.5);
}
#[test]
fn golden_purolator() {
    golden_zpl_with_tolerance("purolator", 4.0);
}
#[test]
fn golden_inpost() {
    golden_zpl_with_tolerance("inpost", 5.5);
}
#[test]
fn golden_yodel() {
    golden_zpl_with_tolerance("yodel", 4.5);
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
    golden_zpl_with_tolerance("dhlparcelit", 3.5);
}

#[test]
fn golden_brtit() {
    // BRT (Bartolini) Italy: ^POI orientation, ~DG000.GRF logo,
    // ^A0B rotated text, ^FR reverse video, Code128
    golden_zpl_with_tolerance("brtit", 2.0);
}

#[test]
fn golden_posteit() {
    // Poste Italiane: DataMatrix ^BX, ^GFA Z64 compressed logo,
    // ^BCB bottom-up barcode, ^FH hex in all fields
    golden_zpl_with_tolerance("posteit", 7.5);
}

#[test]
fn golden_amazonshipping() {
    // Amazon Shipping (MXP5): ^BXN/B/I/R DataMatrix in all 4 orientations,
    // ^FR field reverse, ^GFA inline graphics, ^FH hex in all fields
    golden_zpl_with_tolerance("amazonshipping", 4.0);
}

// ── Additional unit golden tests ──────────────────────────────────

#[test]
fn golden_ups_maxicode() {
    golden_zpl_with_tolerance("ups_maxicode", 5.0);
}
#[test]
fn golden_maxicode_mode4() {
    golden_zpl_with_tolerance("maxicode_mode4", 1.0);
}
#[test]
fn golden_maxicode_default_mode2() {
    golden_zpl_with_tolerance("maxicode_default_mode2", 1.0);
}
#[test]
fn golden_aztec_ec_1_ec23() {
    golden_zpl_with_tolerance("aztec_ec_1_ec23", 7.5);
}
#[test]
fn golden_aztec_ec_2_ec45() {
    golden_zpl_with_tolerance("aztec_ec_2_ec45", 7.5);
}
#[test]
fn golden_aztec_ec_3_ec70() {
    golden_zpl_with_tolerance("aztec_ec_3_ec70", 7.5);
}
#[test]
fn golden_aztec_ec_4_ec95() {
    golden_zpl_with_tolerance("aztec_ec_4_ec95", 7.5);
}
#[test]
fn golden_dhlparceluk_dhl_text() {
    golden_zpl_with_tolerance("dhlparceluk_dhl_text", 5.5);
}
#[test]
fn golden_dhlparceluk_ver() {
    golden_zpl_with_tolerance("dhlparceluk_ver", 5.5);
}
#[test]
fn golden_postnl_qr() {
    golden_zpl_with_tolerance("postnl_qr", 5.0);
}
#[test]
fn golden_edi_triangle() {
    golden_zpl_with_tolerance("edi_triangle", 2.0);
}
#[test]
fn golden_qr_ft_by100() {
    golden_zpl_with_tolerance("qr_ft_by100", 1.0);
}
#[test]
fn golden_qr_ft_600() {
    golden_zpl_with_tolerance("qr_ft_600", 1.0);
}
#[test]
fn golden_qr_ft_test() {
    golden_zpl_with_tolerance("qr_ft_test", 1.0);
}
#[test]
fn golden_cf_font_designator() {
    golden_zpl_with_tolerance("cf_font_designator", 5.0);
}
#[test]
fn golden_cf_font_no_orientation() {
    golden_zpl_with_tolerance("cf_font_no_orientation", 5.0);
}
#[test]
fn golden_fo_lenient_coord() {
    golden_zpl_with_tolerance("fo_lenient_coord", 5.0);
}

// ── EPL golden tests ──────────────────────────────────────────────

#[test]
fn golden_dpduk_epl() {
    golden_epl_with_tolerance("dpduk", 6.5);
}

/// Warehouse pick-ticket snippet exercising the EPL2 Table 1 bar code types
/// (`1`/`3`/`E30`/`2`/`UA0`), the `b` 2-D command (Q/D/A/P/M), `GW` raw
/// binary data (payload contains a newline byte), `X`/`LS`/`LW`/`LO` and the
/// ignored printer-setup commands (Q/R/S/D/ZB/JF/O/C). Labelary does not
/// render EPL, so the reference is the renderer baseline (0.00% at rest);
/// the tolerance only guards against regressions in the covered paths.
#[test]
fn golden_epl2_showcase() {
    golden_epl_with_tolerance("epl2_showcase", 2.0);
}
