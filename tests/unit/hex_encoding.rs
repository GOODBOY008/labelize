use labelize::hex;
use labelize::encodings;

// --- Hex escape decoding ---

#[test]
fn decode_escaped_string_basic() {
    // _2D = '-'
    let result = hex::decode_escaped_string("Hello_2DWorld", b'_').expect("decode failed");
    assert_eq!(result, "Hello-World");
}

#[test]
fn decode_escaped_string_no_escapes() {
    let result = hex::decode_escaped_string("plain text", b'_').expect("decode failed");
    assert_eq!(result, "plain text");
}

#[test]
fn decode_escaped_string_multiple_escapes() {
    // _41 = 'A', _42 = 'B'
    let result = hex::decode_escaped_string("_41_42C", b'_').expect("decode failed");
    assert_eq!(result, "ABC");
}

// --- Graphic field data ---

#[test]
fn decode_graphic_field_simple_hex() {
    let data = "FFFF0000";
    let bytes = hex::decode_graphic_field_data(data, 2).expect("decode failed");
    assert_eq!(bytes, vec![0xFF, 0xFF, 0x00, 0x00]);
}

#[test]
fn decode_graphic_field_with_compression() {
    // 'H' = repeat count 2, followed by 'F' means "FF"
    let data = "HF";
    let bytes = hex::decode_graphic_field_data(data, 1).expect("decode failed");
    assert_eq!(bytes, vec![0xFF]);
}

#[test]
fn decode_graphic_field_comma_pads_with_zeros() {
    // comma fills remaining row bytes with '0'
    let data = "FF,";
    let bytes = hex::decode_graphic_field_data(data, 2).expect("decode failed");
    assert_eq!(bytes.len(), 2);
    assert_eq!(bytes[0], 0xFF);
    assert_eq!(bytes[1], 0x00);
}

// --- Z64 decoding via decode_graphic_field_data ---

#[test]
fn decode_z64_through_graphic_field() {
    // Create Z64 data: zlib-compress then base64-encode
    use std::io::Write;
    let original = vec![0xFF, 0x00, 0xFF, 0x00];

    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&original).expect("compress");
    let compressed = encoder.finish().expect("finish");

    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &compressed);
    let z64_data = format!(":Z64:{}:CRC", b64);

    let decoded = hex::decode_graphic_field_data(&z64_data, 2).expect("z64 decode failed");
    assert_eq!(decoded, original);
}

// --- Encoding conversion ---

#[test]
fn charset_0_ascii_passthrough() {
    let result = encodings::to_unicode_text("Hello", 0).expect("conversion failed");
    assert_eq!(result, "Hello");
}

#[test]
fn charset_27_windows1252() {
    let result = encodings::to_unicode_text("Hello", 27).expect("conversion failed");
    assert_eq!(result, "Hello");
}

#[test]
fn unknown_charset_passthrough() {
    let result = encodings::to_unicode_text("Test", 99).expect("conversion failed");
    assert_eq!(result, "Test");
}
