use labelize::barcodes::datamatrix;
use labelize::{DrawerOptions, EplParser, Renderer, ZplParser};
use rxing::common::{BitMatrix, DecoderRXingResult};
use rxing::datamatrix::decoder::Decoder;
use std::io::Cursor;

fn decode_image(image: &image::RgbaImage, scale: u32) -> DecoderRXingResult {
    let mut bits = BitMatrix::new(image.width() / scale, image.height() / scale).unwrap();
    for y in 0..bits.getHeight() {
        for x in 0..bits.getWidth() {
            let pixel = image.get_pixel(x * scale, y * scale);
            bits.set_bool(x, y, pixel[0] == 0 && pixel[3] != 0);
        }
    }
    Decoder::new()
        .decode(&bits)
        .expect("independent RXing decode")
}

fn encode(data: &[u8], escape: u8) -> DecoderRXingResult {
    decode_image(&datamatrix::encode_zpl(data, 1, 0, 0, escape).unwrap(), 1)
}

#[test]
fn independent_decoder_sees_gs1_identifier_and_internal_fnc1() {
    // The previous renderer used the literal API: escapes survived as text.
    // Keep that API literal for EPL and callers, but route ZPL through encode_zpl.
    let literal = decode_image(
        &datamatrix::encode("_1010950110153000310ABC_121XYZ", 1, 0, 0).unwrap(),
        1,
    );
    assert_eq!(literal.getText(), "_1010950110153000310ABC_121XYZ");
    assert_eq!(literal.getSymbologyModifier(), 1);
    let decoded = encode(b"_1010950110153000310ABC_121XYZ", b'_');
    assert_eq!(
        decoded.getText().as_bytes(),
        b"010950110153000310ABC\x1d21XYZ"
    );
    assert_eq!(
        decoded.getSymbologyModifier(),
        2,
        "GS1 DataMatrix must report ]d2"
    );
    let words = decoded.getRawBytes();
    assert_eq!(words[0], 232);
    assert_eq!(words.iter().filter(|&&word| word == 232).count(), 2);
}

#[test]
fn decimal_gs_does_not_impersonate_fnc1() {
    let decoded = encode(b"_d029ABC_d029XYZ", b'_');
    assert_eq!(decoded.getText().as_bytes(), b"\x1dABC\x1dXYZ");
    assert_eq!(
        decoded.getSymbologyModifier(),
        1,
        "literal GS is not a GS1 marker"
    );
    let gs1 = encode(b"_1ABC_d029XYZ", b'_');
    assert_eq!(gs1.getText().as_bytes(), b"ABC\x1dXYZ");
    assert_eq!(gs1.getSymbologyModifier(), 2);
}

#[test]
fn custom_escape_literal_escape_and_control_bytes_decode() {
    let decoded = encode(b"#1A##B#@#G#d029#199", b'#');
    assert_eq!(decoded.getText().as_bytes(), b"A#B\0\x07\x1d\x1d99");
    assert_eq!(decoded.getSymbologyModifier(), 2);
    let literal = encode(b"A__B", b'_');
    assert_eq!(literal.getText(), "A_B");
}

#[test]
fn plain_data_uses_existing_auto_encoder_unchanged() {
    for data in ["AB12", "12345678901234567890", "hello world"] {
        assert_eq!(
            datamatrix::encode_zpl(data.as_bytes(), 4, 0, 0, b'_').unwrap(),
            datamatrix::encode(data, 4, 0, 0).unwrap()
        );
    }
}

fn render_zpl(source: &str) -> image::RgbaImage {
    let label = ZplParser::new().parse(source.as_bytes()).unwrap().remove(0);
    let mut png = Cursor::new(Vec::new());
    let options = DrawerOptions {
        label_width_mm: 50.0,
        label_height_mm: 25.0,
        dpmm: 8,
        ..Default::default()
    };
    Renderer::new()
        .draw_label_as_png(&label, &mut png, options)
        .unwrap();
    let image = image::load_from_memory(png.get_ref()).unwrap().to_rgba8();
    crop_symbol(&image)
}

fn crop_symbol(image: &image::RgbaImage) -> image::RgbaImage {
    let dark: Vec<_> = image
        .enumerate_pixels()
        .filter(|(_, _, p)| p[0] == 0 && p[3] != 0)
        .collect();
    let min_x = dark.iter().map(|(x, _, _)| *x).min().unwrap();
    let min_y = dark.iter().map(|(_, y, _)| *y).min().unwrap();
    let max_x = dark.iter().map(|(x, _, _)| *x).max().unwrap();
    let max_y = dark.iter().map(|(_, y, _)| *y).max().unwrap();
    image::imageops::crop_imm(image, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}

#[test]
fn field_hex_runs_before_escape_processing_and_preserves_gs() {
    // ^FH creates both the FNC1 escape and a literal GS, in that order.
    let source = "^XA^FO20,20^BXN,4,200^FH_^FD_5F1AB12_1DXYZ^FS^XZ";
    let decoded = decode_image(&render_zpl(source), 4);
    assert_eq!(decoded.getText().as_bytes(), b"AB12\x1dXYZ");
    assert_eq!(decoded.getSymbologyModifier(), 2);
}

#[test]
fn custom_escape_and_modern_default_are_field_local() {
    for parameters in ["N,4,200", "N,4,200,0,0,6,_", "N,4,200,0,0,6,"] {
        let source = format!("^XA^FO20,20^BX{parameters}^FD_1AB12^FS^XZ");
        assert_eq!(
            decode_image(&render_zpl(&source), 4).getSymbologyModifier(),
            2
        );
    }
    let source = "^XA^FO20,20^BXN,4,200,0,0,6,#^FD#1AB12^FS^XZ";
    assert_eq!(
        decode_image(&render_zpl(source), 4).getSymbologyModifier(),
        2
    );
    let labels = ZplParser::new()
        .parse(b"^XA^FO20,20^BXN,4,200,0,0,6,#^FD#1AB12^FS^XZ^XA^FO20,20^BXN,4,200^FD_1AB12^FS^XZ")
        .unwrap();
    for (label, escape) in labels.iter().zip([b'#', b'_']) {
        match &label.elements[0] {
            labelize::elements::label_element::LabelElement::BarcodeDatamatrix(bc) => {
                assert_eq!(bc.barcode.escape, escape)
            }
            _ => panic!("expected DataMatrix"),
        }
    }
}

#[test]
fn epl_disables_zpl_escape_processing_even_when_quality_is_ecc200() {
    let mut label = EplParser::new()
        .parse(b"N\nb20,20,D,h4,\"A_1B~1\"\nP1\n")
        .unwrap()
        .remove(0);
    match &mut label.elements[0] {
        labelize::elements::label_element::LabelElement::BarcodeDatamatrix(bc) => {
            assert_eq!(bc.barcode.escape, 0);
            bc.barcode.quality = 200; // Also covers the explicit EPL quality in PR #54.
        }
        _ => panic!("expected DataMatrix"),
    }
    let mut output = Cursor::new(Vec::new());
    Renderer::new()
        .draw_label_as_png(&label, &mut output, DrawerOptions::default())
        .unwrap();
    let image = image::load_from_memory(output.get_ref())
        .unwrap()
        .to_rgba8();
    let decoded = decode_image(&crop_symbol(&image), 4);
    assert_eq!(decoded.getText(), "A_1B~1");
    assert_eq!(decoded.getSymbologyModifier(), 1);
}

#[test]
fn only_the_configured_escape_is_active() {
    let underscore = encode(b"_1AB~1CD", b'_');
    assert_eq!(underscore.getText(), "AB~1CD");
    let tilde = encode(b"~1AB_1CD", b'~');
    assert_eq!(tilde.getText(), "AB_1CD");
    assert_eq!(tilde.getSymbologyModifier(), 2);
    // Free the tilde from its role as the ZPL command prefix with ^CT.
    let source = "^CT!^XA^FO20,20^BXN,4,200,0,0,6,~^FD~1AB12^FS^XZ";
    let decoded = decode_image(&render_zpl(source), 4);
    assert_eq!(decoded.getText(), "AB12");
    assert_eq!(decoded.getSymbologyModifier(), 2);
}

#[test]
fn upper_shift_numeric_pairs_and_padding_survive_independent_decoding() {
    for size in [0, 12, 16, 22, 32] {
        let image = datamatrix::encode_zpl(b"_1_d128_d25599", 1, size, size, b'_').unwrap();
        let decoded = decode_image(&image, 1);
        assert_eq!(decoded.getSymbologyModifier(), 2);
        assert_eq!(&decoded.getRawBytes()[..6], &[232, 235, 1, 235, 128, 229]);
    }
}

#[test]
fn terminal_explicit_pad_starts_randomized_padding_immediately() {
    // Decoded text alone misses duplicate PAD: readers stop at the first 129.
    for (size, expected) in [
        (10, vec![66, 129, 70]),
        (12, vec![66, 129, 70, 220, 115]),
        (14, vec![66, 129, 70, 220, 115, 11, 161, 56]),
    ] {
        for (data, escape) in [(&b"A_0"[..], b'_'), (&b"A#0"[..], b'#')] {
            let image = datamatrix::encode_zpl(data, 1, size, size, escape).unwrap();
            let decoded = decode_image(&image, 1);
            assert_eq!(decoded.getText(), "A");
            assert_eq!(decoded.getRawBytes(), &expected);
        }
    }
    // Explicit PAD at exact capacity must not grow the symbol.
    let exact = datamatrix::encode_zpl(b"AB_0", 1, 10, 10, b'_').unwrap();
    assert_eq!(exact.width(), 10);
    assert_eq!(decode_image(&exact, 1).getRawBytes(), &[66, 67, 129]);
    // Without an explicit PAD, the initial unrandomized PAD is still required.
    let implicit = datamatrix::encode_zpl(b"_1A", 1, 12, 12, b'_').unwrap();
    assert_eq!(
        decode_image(&implicit, 1).getRawBytes(),
        &[232, 66, 129, 220, 115]
    );
}
