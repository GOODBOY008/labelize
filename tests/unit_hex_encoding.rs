use labelize::encodings;
use labelize::hex;

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
fn graphic_whitespace_preserves_repeat_counts_and_row_padding() {
    // H repeats the F twice, the comma pads the row, and the colon copies it.
    let bytes = hex::decode_graphic_field_data("H \tF , \r\n: ", 2).unwrap();
    assert_eq!(bytes, [0xFF, 0x00, 0xFF, 0x00]);
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

#[test]
fn cp850_0xa6_is_feminine_ordinal() {
    let raw = hex::decode_escaped_string("_A6", b'_').expect("decode failed");
    let unicode = encodings::to_unicode_text(&raw, 0).expect("conversion failed");
    assert_eq!(
        unicode, "\u{00AA}",
        "CP850 0xA6 should be ª (feminine ordinal indicator)"
    );
}

#[test]
fn cp850_0xa7_is_masculine_ordinal() {
    let raw = hex::decode_escaped_string("_A7", b'_').expect("decode failed");
    let unicode = encodings::to_unicode_text(&raw, 0).expect("conversion failed");
    assert_eq!(
        unicode, "\u{00BA}",
        "CP850 0xA7 should be º (masculine ordinal indicator)"
    );
}

#[test]
fn cp850_0xa8_is_inverted_question_mark() {
    let raw = hex::decode_escaped_string("_A8", b'_').expect("decode failed");
    let unicode = encodings::to_unicode_text(&raw, 0).expect("conversion failed");
    assert_eq!(
        unicode, "\u{00BF}",
        "CP850 0xA8 should be ¿ (inverted question mark)"
    );
}

#[test]
fn cp850_0xa9_is_registered_sign() {
    // In CP850 (charset 0), byte 0xA9 = ® (REGISTERED SIGN, U+00AE)
    // This is the character emitted by ^FH_^FD_A9^FS in ZPL
    let raw = hex::decode_escaped_string("_A9", b'_').expect("decode failed");
    let unicode = encodings::to_unicode_text(&raw, 0).expect("conversion failed");
    assert_eq!(
        unicode, "\u{00AE}",
        "CP850 0xA9 should be ® (registered sign)"
    );
}

#[test]
fn cp850_0xaa_is_not_sign() {
    let raw = hex::decode_escaped_string("_AA", b'_').expect("decode failed");
    let unicode = encodings::to_unicode_text(&raw, 0).expect("conversion failed");
    assert_eq!(unicode, "\u{00AC}", "CP850 0xAA should be ¬ (not sign)");
}

#[test]
fn cp850_0xab_is_one_half() {
    let raw = hex::decode_escaped_string("_AB", b'_').expect("decode failed");
    let unicode = encodings::to_unicode_text(&raw, 0).expect("conversion failed");
    assert_eq!(
        unicode, "\u{00BD}",
        "CP850 0xAB should be ½ (vulgar fraction one half)"
    );
}

#[test]
fn cp850_0xac_is_one_quarter() {
    let raw = hex::decode_escaped_string("_AC", b'_').expect("decode failed");
    let unicode = encodings::to_unicode_text(&raw, 0).expect("conversion failed");
    assert_eq!(
        unicode, "\u{00BC}",
        "CP850 0xAC should be ¼ (vulgar fraction one quarter)"
    );
}

#[test]
fn cp850_0xad_is_inverted_exclamation() {
    let raw = hex::decode_escaped_string("_AD", b'_').expect("decode failed");
    let unicode = encodings::to_unicode_text(&raw, 0).expect("conversion failed");
    assert_eq!(
        unicode, "\u{00A1}",
        "CP850 0xAD should be ¡ (inverted exclamation mark)"
    );
}

#[test]
fn cp850_0xae_is_left_double_angle_quote() {
    let raw = hex::decode_escaped_string("_AE", b'_').expect("decode failed");
    let unicode = encodings::to_unicode_text(&raw, 0).expect("conversion failed");
    assert_eq!(
        unicode, "\u{00AB}",
        "CP850 0xAE should be « (left double angle quotation mark)"
    );
}

#[test]
fn cp850_0xaf_is_right_double_angle_quote() {
    let raw = hex::decode_escaped_string("_AF", b'_').expect("decode failed");
    let unicode = encodings::to_unicode_text(&raw, 0).expect("conversion failed");
    assert_eq!(
        unicode, "\u{00BB}",
        "CP850 0xAF should be » (right double angle quotation mark)"
    );
}
