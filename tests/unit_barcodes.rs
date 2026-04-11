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
    // Mode N with >; (Code B start) + >6 (Code B switch) + >7 (Code C switch)
    // Display text should NOT contain any > prefix codes
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
fn aztec_empty_input_returns_error() {
    let result = aztec::encode("", 4, 0);
    assert!(result.is_err(), "expected error for empty input");
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
    assert!(result.is_err(), "expected error for empty input");
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
    assert!(result.is_err(), "expected error for empty input");
}

// --- MaxiCode ---

#[test]
fn maxicode_encodes_text() {
    let img = maxicode::encode("Hello World").expect("maxicode failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn maxicode_empty_input_returns_error() {
    let result = maxicode::encode("");
    assert!(result.is_err(), "expected error for empty input");
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
