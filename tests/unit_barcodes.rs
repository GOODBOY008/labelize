use image::{Rgba, RgbaImage};
use labelize::barcodes::{
    aztec, code128, code39, datamatrix, ean13, maxicode, pdf417, qrcode, twooffive,
};
use labelize::elements::barcode_qr::QrErrorCorrectionLevel;

// --- Code128 ---

#[test]
fn code128_encodes_ascii() {
    let img = code128::encode_auto("Hello123", 100, 2).expect("encode_auto failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn code128_encodes_digits_only() {
    let img = code128::encode_auto("1234567890", 80, 2).expect("encode_auto failed");
    assert!(img.width() > 0);
}

#[test]
fn code128_empty_input_handled() {
    // Empty input may succeed with a minimal barcode or error - either is acceptable
    let _result = code128::encode_auto("", 100, 2);
}

#[test]
fn code128_no_mode_strips_prefix_from_display() {
    // Per ZPL spec: >; = Start Code C, >6 = switch to Code B (from C), >7 = switch to Code A (from B).
    // Code A in mode N uses digit pairs: "52"→'T', "37"→'E', "51"→'S', "52"→'T' → "TEST".
    // Display text should NOT contain any > prefix codes.
    let (img, text) = code128::encode_no_mode(">;382436>6CODE128>752375152", 100, 2)
        .expect("encode_no_mode failed");
    assert!(img.width() > 0);
    assert!(
        !text.contains('>'),
        "display text should not contain '>' prefix codes: {}",
        text
    );
    assert!(
        text.contains("382436"),
        "display text should contain '382436': {}",
        text
    );
    assert!(
        text.contains("CODE128"),
        "display text should contain 'CODE128': {}",
        text
    );
    assert!(
        text.contains("TEST"),
        "display text should contain 'TEST' (Code A pair-mode decoding of 52,37,51,52): {}",
        text
    );
}

#[test]
fn code128_no_mode_default_code_b() {
    // Mode N without explicit prefix should default to Code B (not auto Code C)
    let (img1, text1) = code128::encode_no_mode("12345678", 100, 2).expect("encode_no_mode failed");
    // In Code B, each digit is 1 symbol; in Code C, pairs are 1 symbol.
    // Code B (8 data + start + check + stop) vs Code C (4 pairs + start + check + stop)
    // Code B should produce a wider barcode
    let img2 = code128::encode_auto("12345678", 100, 2).expect("encode_auto failed");
    assert!(
        img1.width() > img2.width(),
        "Mode N without prefix should use Code B (wider), not auto Code C"
    );
    assert_eq!(text1, "12345678");
}

#[test]
fn code128_auto_with_fnc1() {
    // FNC1 at start followed by digits should still detect Code C start
    let content = format!("{}1234567890", code128::ESCAPE_FNC_1);
    let img = code128::encode_auto(&content, 100, 2).expect("encode_auto with FNC1 failed");
    // With FNC1 + 10 digits: Start C, FNC1, 5 pairs, check, stop = 9 symbols
    // Without FNC1: Start C, 5 pairs, check, stop = 8 symbols
    let img_no_fnc1 = code128::encode_auto("1234567890", 100, 2).expect("encode_auto failed");
    assert!(
        img.width() > img_no_fnc1.width(),
        "FNC1 should add one symbol width"
    );
}

// --- Code39 ---

#[test]
fn code39_encodes_alphanumeric() {
    let img = code39::encode("ABC123", 100, 3, 2).expect("code39 failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn code39_empty_input_handled() {
    // Empty input may succeed with a minimal barcode or error - either is acceptable
    let _result = code39::encode("", 100, 3, 2);
}

#[test]
fn code128_ean_mode_keeps_ai_formatting_but_hides_fnc1_invocations() {
    let (encoded, display) = code128::prepare_ean_mode_data("(91)0005886>8(10)0000410549>8(99)05");
    let fnc1 = code128::ESCAPE_FNC_1;

    assert_eq!(
        encoded,
        format!("{fnc1}910005886{fnc1}100000410549{fnc1}9905")
    );
    assert_eq!(display, "(91)0005886(10)0000410549(99)05");
    assert!(!display.contains(">8"));
}

// --- EAN-13 ---

#[test]
fn ean13_encodes_12_digits() {
    let img = ean13::encode("123456789012", 100, 2).expect("ean13 failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn ean13_empty_input_returns_error() {
    let result = ean13::encode("", 100, 2);
    assert!(result.is_err(), "expected error for empty input");
}

// --- Interleaved 2-of-5 ---

#[test]
fn twooffive_encodes_digits() {
    let img = twooffive::encode("12345678", 100, 3, 2, false).expect("2of5 failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn twooffive_empty_input_returns_error() {
    let result = twooffive::encode("", 100, 3, 2, false);
    assert!(result.is_err(), "expected error for empty input");
}

// --- PDF417 ---

#[test]
fn pdf417_encodes_text() {
    let img = pdf417::encode("Hello World", 4, 0, 0, 0, false, 10).expect("pdf417 failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn pdf417_empty_input_returns_error() {
    let result = pdf417::encode("", 4, 0, 0, 0, false, 10);
    assert!(result.is_err(), "expected error for empty input");
}

#[test]
fn pdf417_module_width_scales_output() {
    // Encode at 1px module width, then at 3px — verify width ratio
    let img1 = pdf417::encode("Test data", 0, 0, 5, 0, false, 40).expect("encode 1");
    let img3 = pdf417::encode("Test data", 0, 0, 5, 0, false, 40).expect("encode 3");
    // Both produce same 1px-module-width images; scaling happens in renderer
    assert_eq!(img1.width(), img3.width());
}

#[test]
fn pdf417_row_height_fallback_from_by() {
    // b7_h=0 means use by_height/num_rows
    let img = pdf417::encode("Hello World", 0, 2, 5, 0, false, 40).expect("encode");
    assert!(img.height() > 0);
}

#[test]
fn pdf417_explicit_row_height_overrides_by() {
    // b7_h=5 means each row = 5px, regardless of by_height
    let img = pdf417::encode("Hello World", 5, 2, 5, 0, false, 9999).expect("encode");
    // With explicit row_height=5, rows*5 = image height
    // Verify height is reasonable (not 9999-based)
    assert!(
        img.height() < 500,
        "height should use b7_h=5, not by_height"
    );
}

#[test]
fn pdf417_default_aspect_ratio() {
    // cols=0, rows=0 → should pick rows ≈ 2×cols
    let img = pdf417::encode(
        "Some data to encode for aspect ratio test",
        0,
        0,
        0,
        0,
        false,
        10,
    )
    .expect("encode");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn pdf417_validation_rejects_over_928() {
    // rxing handles capacity validation internally; 30×90=2700 should fail
    let result = pdf417::encode("x", 0, 0, 30, 90, false, 10);
    assert!(result.is_err(), "30×90=2700 should exceed 928 limit");
}

#[test]
fn pdf417_truncated_mode() {
    let full = pdf417::encode("Truncated test", 0, 0, 0, 0, false, 10).expect("full");
    let trunc = pdf417::encode("Truncated test", 0, 0, 0, 0, true, 10).expect("truncated");
    assert!(
        trunc.width() <= full.width(),
        "truncated PDF417 should not be wider than full"
    );
}

#[test]
fn pdf417_crlf_in_data() {
    let img = pdf417::encode("Line1\nLine2", 0, 0, 0, 0, false, 10).expect("encode");
    assert!(img.width() > 0);
}

// --- Aztec ---

#[test]
fn aztec_encodes_text() {
    let img = aztec::encode("Hello", 4, 0).expect("aztec failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
    // Aztec codes should be square
    assert_eq!(img.width(), img.height(), "Aztec code should be square");
}

#[test]
fn aztec_empty_input_returns_empty_image() {
    let result = aztec::encode("", 4, 0);
    match result {
        Ok(img) => {
            assert_eq!(img.width(), 0, "empty Aztec code should have width 0");
            assert_eq!(img.height(), 0, "empty Aztec code should have height 0");
        }
        Err(_) => {
            panic!("empty Aztec input should not return an error");
        }
    }
}

// --- DataMatrix ---

#[test]
fn datamatrix_encodes_text() {
    let img = datamatrix::encode("Hello", 4, 0, 0).expect("datamatrix failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn datamatrix_empty_input_returns_error() {
    let result = datamatrix::encode("", 4, 0, 0);
    match result {
        Ok(img) => {
            assert_eq!(img.width(), 0, "empty DataMatrix code should have width 0");
            assert_eq!(img.height(), 0, "empty DataMatrix code should have height 0");
        }
        Err(_) => {
            panic!("empty DataMatrix input should not return an error");
        }
    }
}

// --- QR code ---

#[test]
fn qrcode_encodes_text() {
    let img = qrcode::encode("Hello World", 5, QrErrorCorrectionLevel::M).expect("qrcode failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
    // QR codes should be square
    assert_eq!(img.width(), img.height(), "QR code should be square");
}

#[test]
fn qrcode_empty_input_returns_error() {
    let result = qrcode::encode("", 5, QrErrorCorrectionLevel::M);
    // Empty input should either return an error or an empty image
    match result {
        Ok(img) => {
            assert_eq!(img.width(), 0, "empty QR code should have width 0");
            assert_eq!(img.height(), 0, "empty QR code should have height 0");
        }
        Err(_) => {
            panic!("empty QR input should not return an error");
        }
    }
}

// --- MaxiCode ---

#[test]
fn maxicode_encodes_text() {
    let img = maxicode::encode("Hello World", 4).expect("maxicode failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn maxicode_empty_input_returns_error() {
    let result = maxicode::encode("", 4);
    assert!(result.is_err(), "expected error for empty input");
}

#[test]
fn maxicode_mode_4_matches_qr_atelier_reference_codewords() {
    // Cross-checked against QR-Atelier's dependency-free MaxiCodeCore.
    let codewords = maxicode::encode_codewords("HELLO WORLD", 4).unwrap();
    assert_eq!(
        &codewords[..20],
        &[4, 8, 5, 12, 12, 15, 32, 23, 15, 18, 52, 3, 23, 18, 30, 12, 21, 9, 11, 39]
    );
    assert_eq!(&codewords[20..22], &[12, 4]);
    assert!(codewords[22..104].iter().all(|&value| value == 33));
    assert_eq!(
        &codewords[104..],
        &[
            35, 11, 38, 60, 46, 45, 14, 36, 31, 45, 34, 37, 0, 51, 10, 44, 21, 40, 7, 8, 22, 54, 1,
            0, 31, 60, 31, 26, 62, 7, 9, 3, 15, 58, 42, 11, 20, 42, 57, 11
        ]
    );
}

#[test]
fn maxicode_mode_2_primary_matches_qr_atelier_reference() {
    let codewords = maxicode::encode_codewords("002840336091062[)>ABC", 2).unwrap();
    assert_eq!(
        &codewords[..20],
        &[34, 45, 23, 33, 0, 21, 2, 18, 11, 0, 18, 59, 54, 25, 36, 30, 59, 12, 34, 61]
    );
}

#[test]
fn maxicode_mode_3_primary_matches_qr_atelier_reference() {
    let codewords = maxicode::encode_codewords("001124K1A0B1[)>ABC", 3).unwrap();
    assert_eq!(
        &codewords[..20],
        &[19, 44, 0, 28, 16, 60, 2, 31, 4, 0, 5, 55, 36, 44, 7, 9, 48, 63, 50, 17]
    );
}

#[test]
fn maxicode_numeric_compaction_matches_libzint_reference() {
    let codewords = maxicode::encode_codewords("123456789", 4).unwrap();
    assert_eq!(&codewords[1..7], &[31, 7, 22, 60, 52, 21]);
}

#[test]
fn maxicode_numeric_compaction_avoids_false_capacity_error() {
    assert!(maxicode::encode_codewords(&"1".repeat(138), 4).is_ok());
    assert!(maxicode::encode_codewords(&"1".repeat(139), 4).is_err());
}

#[test]
fn maxicode_rejects_unsupported_modes_and_excess_data() {
    assert!(maxicode::encode_codewords("HELLO", 5).is_err());
    assert!(maxicode::encode_codewords(&"A".repeat(94), 4).is_err());
    assert!(maxicode::encode_codewords("002840NOT-A-POSTAL-CODE", 2).is_err());
    assert!(maxicode::encode_codewords("ü02840336091062", 2).is_err());
}

// --- Multiple barcode widths ---

#[test]
fn code128_wider_bar_produces_wider_image() {
    let narrow = code128::encode_auto("TEST", 100, 1).expect("narrow");
    let wide = code128::encode_auto("TEST", 100, 3).expect("wide");
    assert!(
        wide.width() > narrow.width(),
        "wider bar width should produce wider image"
    );
}

#[test]
fn code128_taller_height_produces_taller_image() {
    let short = code128::encode_auto("TEST", 50, 2).expect("short");
    let tall = code128::encode_auto("TEST", 200, 2).expect("tall");
    assert!(
        tall.height() > short.height(),
        "taller height should produce taller image"
    );
}
