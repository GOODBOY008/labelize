use labelize::barcodes::{code128, code39, ean13, twooffive, pdf417, aztec, datamatrix, qrcode, maxicode};

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
    let img = pdf417::encode("Hello World", 4, 0, 0, false).expect("pdf417 failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn pdf417_empty_input_returns_error() {
    let result = pdf417::encode("", 4, 0, 0, false);
    assert!(result.is_err(), "expected error for empty input");
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
    let img = datamatrix::encode("Hello", 4).expect("datamatrix failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
}

#[test]
fn datamatrix_empty_input_returns_error() {
    let result = datamatrix::encode("", 4);
    assert!(result.is_err(), "expected error for empty input");
}

// --- QR code ---

#[test]
fn qrcode_encodes_text() {
    let img = qrcode::encode("Hello World", 5).expect("qrcode failed");
    assert!(img.width() > 0);
    assert!(img.height() > 0);
    // QR codes should be square
    assert_eq!(img.width(), img.height(), "QR code should be square");
}

#[test]
fn qrcode_empty_input_returns_error() {
    let result = qrcode::encode("", 5);
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
