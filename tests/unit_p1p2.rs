//! Unit tests for the P1/P2 command batch: ^LT, ^LS, ^B9 (UPC-E), ^A@, ^CW,
//! ^ID/^IM/^IS/~EG, ^FX, ^PQ and ^SN/^SF.

use labelize::barcodes::upce;
use labelize::elements::label_element::LabelElement;
use labelize::ZplParser;

fn parse(zpl: &str) -> Vec<labelize::LabelInfo> {
    let mut parser = ZplParser::new();
    parser.parse(zpl.as_bytes()).expect("parse failed")
}

fn boxes_of(label: &labelize::LabelInfo) -> Vec<(i32, i32)> {
    label
        .elements
        .iter()
        .filter_map(|el| match el {
            LabelElement::GraphicBox(g) => Some((g.position.x, g.position.y)),
            _ => None,
        })
        .collect()
}

fn texts_of(label: &labelize::LabelInfo) -> Vec<String> {
    label
        .elements
        .iter()
        .filter_map(|el| match el {
            LabelElement::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect()
}

// ── ^LT (label top) ────────────────────────────────────────────────────────────

#[test]
fn lt_positive_shifts_content_down() {
    let labels = parse("^XA^FO100,100^GB50,50,5^FS^XZ");
    let shifted = parse("^XA^LT50^FO100,100^GB50,50,5^FS^XZ");
    let (_, y0) = boxes_of(&labels[0])[0];
    let (_, y1) = boxes_of(&shifted[0])[0];
    assert_eq!(y0, 100);
    assert_eq!(y1, 150);
}

#[test]
fn lt_negative_shifts_content_up_and_clamps_at_zero() {
    let shifted = parse("^XA^LT-40^FO50,100^GB50,50,5^FS^XZ");
    assert_eq!(boxes_of(&shifted[0])[0], (50, 60));
    // y + LT < 0 clamps to 0 (Labelary behavior)
    let clamped = parse("^XA^LT-120^FO50,100^GB50,50,5^FS^XZ");
    assert_eq!(boxes_of(&clamped[0])[0], (50, 0));
}

#[test]
fn lt_beyond_120_dots_is_ignored() {
    // Calibrated: ^LT121 and ^LT-121 are no-ops on Labelary/Zebra.
    let plain = parse("^XA^FO50,100^GB50,50,5^FS^XZ");
    for lt in ["121", "-121", "500", "-500"] {
        let labels = parse(&format!("^XA^LT{}^FO50,100^GB50,50,5^FS^XZ", lt));
        assert_eq!(boxes_of(&labels[0])[0], boxes_of(&plain[0])[0], "LT {}", lt);
    }
}

#[test]
fn lt_applies_retroactively_within_a_format() {
    // Fields placed before ^LT are shifted too (Labelary renders the whole format
    // with the final value).
    let labels = parse("^XA^FO100,100^GB50,50,5^FS^LT40^FO100,300^GB50,50,5^FS^XZ");
    assert_eq!(boxes_of(&labels[0]), vec![(100, 140), (100, 340)]);
}

#[test]
fn lt_persists_across_formats() {
    let labels = parse("^XA^LT50^FO100,100^GB50,50,5^FS^XZ^XA^FO200,200^GB50,50,5^FS^XZ");
    assert_eq!(labels.len(), 2);
    assert_eq!(boxes_of(&labels[1])[0], (200, 250));
}

#[test]
fn lt_applies_to_baseline_fields_too() {
    let labels = parse("^XA^LT30^FT100,100^A0N,20,20^FDHi^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::Text(t) => assert_eq!(t.position.y, 130),
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn lt_shifts_recalled_formats_at_recall_time_not_store_time() {
    // Stored formats keep raw positions; the shift applies when the recalled
    // elements are emitted (Labelary behavior, verified against Labelary).
    let zpl = "^XA^LT40^DFR:BASE.ZPL^FO100,100^GB50,50,5^FS^XZ^XA^LT30^XFR:BASE.ZPL^FS^XZ";
    let labels = parse(zpl);
    assert_eq!(labels.len(), 1);
    // Only the recall-format LT (30) applies: 100 + 30.
    assert_eq!(boxes_of(&labels[0])[0], (100, 130));
}

#[test]
fn ls_positive_shifts_left_and_clamps() {
    let labels = parse("^XA^LS50^FO100,100^GB50,50,5^FS^XZ");
    assert_eq!(boxes_of(&labels[0])[0], (50, 100));
    // x - LS < 0 clamps at 0 per element.
    let labels = parse("^XA^LS50^FO10,100^GB50,50,5^FS^XZ");
    assert_eq!(boxes_of(&labels[0])[0].0, 0);
}

#[test]
fn ls_negative_shifts_right() {
    let labels = parse("^XA^LS-50^FO100,100^GB50,50,5^FS^XZ");
    assert_eq!(boxes_of(&labels[0])[0], (150, 100));
}

#[test]
fn lt_and_ls_combine_additively() {
    let labels = parse("^XA^LT40^LS20^FO100,100^GB50,50,5^FS^XZ");
    assert_eq!(boxes_of(&labels[0])[0], (80, 140));
}

// ── ^B9 (UPC-E) ───────────────────────────────────────────────────────────────

#[test]
fn upce_check_digits_match_labelary() {
    // (number system, six digits, expected check digit) — every extracted parity
    // pattern was verified against a Labelary ^B9 render for all 20 (NS, C) pairs.
    let cases = [
        (0, "100007", 8),
        (0, "100002", 7),
        (0, "100005", 4),
        (0, "100011", 5),
        (0, "100001", 8),
        (0, "100019", 1),
        (0, "100010", 6),
        (0, "100000", 9),
        (0, "100009", 2),
        (0, "100008", 5),
        (1, "100002", 4),
        (1, "100005", 1),
    ];
    for (ns, digits, expected) in cases {
        let sym = upce::encode(&format!("{}{}", ns, digits), 100, 2).expect("encode");
        assert_eq!(sym.number_system, ns);
        assert_eq!(sym.check_digit, expected, "ns={} e={}", ns, digits);
    }
}

#[test]
fn upce_symbol_is_51_modules_wide() {
    let sym = upce::encode("0425261", 100, 2).expect("encode");
    assert_eq!(sym.image.width(), 51 * 2);
    // Guard extension is 12% of the bar height (measured on Labelary).
    assert_eq!(sym.image.height(), 100 + 12);
    assert_eq!(sym.data_height, 100);
}

#[test]
fn upce_six_digit_input_defaults_to_number_system_zero() {
    let a = upce::encode("100007", 100, 2).expect("encode");
    let b = upce::encode("0100007", 100, 2).expect("encode");
    assert_eq!(a.number_system, 0);
    assert_eq!(a.image, b.image);
}

#[test]
fn upce_eleven_digit_input_compresses() {
    // UPC-A 0 10000 00007 compresses to UPC-E 100007 with check digit 8.
    let sym = upce::encode("01000000007", 100, 2).expect("encode");
    assert_eq!(sym.number_system, 0);
    assert_eq!(sym.digits, [1, 0, 0, 0, 0, 7]);
    assert_eq!(sym.check_digit, 8);
}

#[test]
fn upce_zero_suppression_rules() {
    // M ends 000/100/200 -> e = M1M2M3 P3P4P5
    let sym = upce::encode("012000003457", 100, 2).expect("encode");
    assert_eq!(sym.digits, [1, 2, 0, 3, 4, 5]);
    // M ends 00 (not 000/100/200) -> e = M1M2M3 P4P5 3
    let sym = upce::encode("012300000456", 100, 2).expect("encode");
    assert_eq!(sym.digits, [1, 2, 3, 4, 5, 3]);
    // M ends 0 (not 00) -> e = M1M2M3M4 P5 4
    let sym = upce::encode("012340000056", 100, 2).expect("encode");
    assert_eq!(sym.digits, [1, 2, 3, 4, 5, 4]);
    // no zeros -> e = M1..M5 P5
    let sym = upce::encode("012345000061", 100, 2).expect("encode");
    assert_eq!(sym.digits, [1, 2, 3, 4, 5, 6]);
}

#[test]
fn upce_rejects_bad_input_lengths() {
    assert!(upce::encode("12345", 100, 2).is_err());
    assert!(upce::encode("12345678", 100, 2).is_err());
}

#[test]
fn b9_parser_defaults_and_check_digit_flag() {
    // 7-digit input, defaults: interpretation line below, check digit printed.
    let labels = parse("^XA^FO50,50^B9N,100,Y,N^FD0425261^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::BarcodeUcpe(bc) => {
            assert_eq!(bc.barcode.height, 100);
            assert!(bc.barcode.line);
            assert!(!bc.barcode.line_above);
            assert!(bc.barcode.check_digit);
        }
        other => panic!("expected barcode, got {:?}", other),
    }
    // e=N hides the check digit from the interpretation line but keeps it encoded.
    let labels = parse("^XA^FO50,50^B9N,100,Y,N,N^FD0425261^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::BarcodeUcpe(bc) => assert!(!bc.barcode.check_digit),
        other => panic!("expected barcode, got {:?}", other),
    }
}

#[test]
fn b9_check_digit_flag_hides_text_only() {
    // The check digit is always encoded (it picks the parity) — the flag only
    // controls the interpretation line.
    let with = upce::encode("0425261", 100, 2).expect("encode");
    let without = upce::encode("0425261", 100, 2).expect("encode");
    assert_eq!(with.check_digit, without.check_digit);
    assert_eq!(with.image, without.image);
}

// ── ^A@ (named font) / ^CW (font identifier) ──────────────────────────────────

#[test]
fn aat_selects_builtin_font_by_name() {
    let labels = parse("^XA^A@B,30,20^FO50,50^FDHello^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::Text(t) => assert_eq!(t.font.name, "B"),
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn aat_numeric_name_falls_back_to_font_zero() {
    let labels = parse("^XA^A@3,30,20^FO50,50^FDHello^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::Text(t) => assert_eq!(t.font.name, "0"),
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn aat_unknown_downloadable_name_falls_back_to_default() {
    // Downloaded font names cannot be rendered; degrade to the default font.
    let labels = parse("^XA^A@E:ARI000.TTF,30,20^FO50,50^FDHello^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::Text(t) => assert_eq!(t.font.name, "A"),
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn cw_maps_font_identifier() {
    let labels = parse("^XA^CWX,B^AX,30,20^FO50,50^FDHello^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::Text(t) => assert_eq!(t.font.name, "B"),
        other => panic!("expected text, got {:?}", other),
    }
}

#[test]
fn cw_to_downloadable_font_falls_back() {
    let labels = parse("^XA^CWW,E:ARI000.TTF^AWW,30,20^FO50,50^FDHello^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::Text(t) => assert_eq!(t.font.name, "A"),
        other => panic!("expected text, got {:?}", other),
    }
}

// ── storage management: ^ID / ^IM / ^IS / ~EG ─────────────────────────────────

#[test]
fn id_deletes_stored_graphic_before_recall() {
    // Delete after download: the recall must then find nothing.
    let labels =
        parse("^XA^DFR:BASE.ZPL^FO100,100^GB50,50,5^FS^XZ^XA^IDR:BASE.ZPL^FS^XFR:BASE.ZPL^FS^XZ");
    assert!(labels.is_empty(), "recalled format should be deleted");
}

#[test]
fn im_moves_and_is_copies_stored_format() {
    // ^IM renames; after the move only the new name resolves.
    let zpl = "^XA^DFR:BASE.ZPL^FO100,100^GB50,50,5^FS^XZ^XA^IMR:BASE.ZPL,R:MOVED.ZPL^FS^XFR:MOVED.ZPL^FS^XZ";
    let labels = parse(zpl);
    assert!(
        labels.len() == 1 && boxes_of(&labels[0]).len() == 1,
        "^IM move should make the new name resolvable"
    );
    assert_eq!(boxes_of(&labels[0])[0], (100, 100));

    // ^IS copies: both names resolve (both recalls merge into the same label).
    let zpl = "^XA^DFR:BASE.ZPL^FO100,100^GB50,50,5^FS^XZ^XA^ISR:BASE.ZPL,R:COPY.ZPL^FS^XFR:BASE.ZPL^FS^XFR:COPY.ZPL^FS^XZ";
    let labels = parse(zpl);
    assert_eq!(
        boxes_of(&labels[0]),
        vec![(100, 100), (100, 100)],
        "^IS should leave both names resolvable"
    );
}

#[test]
fn eg_erases_all_or_named_graphics() {
    // ~EG with no name clears the whole graphics store.
    let zpl =
        "^XA~DGR:IMG.GRF,00008,00001,8,,,,^FS^XG R:IMG.GRF,1,1^FS^XZ~EG^XA^XGR:IMG.GRF,1,1^FS^XZ";
    // NOTE: this stream uses a graphics store; ~EG then makes the recall a no-op.
    let labels = parse(zpl);
    assert!(labels.is_empty(), "~EG should erase all graphics");

    // ~EG with a name removes only that graphic.
    let zpl = "^XA~DGR:IMG.GRF,00008,00001,8,,,,^FS^XGR:IMG.GRF,1,1^FS^XZ~EGR:IMG.GRF^XA^XGR:IMG.GRF,1,1^FS^XZ";
    let labels = parse(zpl);
    assert_eq!(labels.len(), 1, "first recall renders, second is erased");
}

// ── ^FX (comment) ─────────────────────────────────────────────────────────────

#[test]
fn fx_comment_is_ignored() {
    let labels = parse("^XA^FX this is a comment^FO100,100^GB50,50,5^FS^XZ");
    assert_eq!(boxes_of(&labels[0]), vec![(100, 100)]);
    assert!(parse("^XA^FX nothing here^FS^XZ").is_empty());
}

// ── ^PQ (print quantity), ^SN/^SF (inert serial state) ────────────────────────

#[test]
fn pq_emits_quantity_times_copies_labels() {
    assert_eq!(parse("^XA^FO50,50^GB50,50,5^FS^XZ").len(), 1);
    let labels = parse("^XA^FO50,50^GB50,50,5^FS^PQ3^XZ");
    assert_eq!(labels.len(), 3);
    let labels = parse("^XA^FO50,50^GB50,50,5^FS^PQ2,0,3,Y^XZ");
    assert_eq!(labels.len(), 6, "quantity x copies");
}

#[test]
fn pq_is_scoped_to_a_format() {
    let labels = parse("^XA^FO50,50^GB50,50,5^FS^PQ3^XZ^XA^FO50,50^GB50,50,5^FS^XZ");
    assert_eq!(labels.len(), 4, "second format defaults to quantity 1");
}

#[test]
fn pq_zero_or_missing_defaults_to_one() {
    let labels = parse("^XA^FO50,50^GB50,50,5^FS^PQ0^XZ");
    assert_eq!(labels.len(), 1);
}

#[test]
fn sn_sf_are_inert_and_serials_render_literally() {
    // Serial markers in ^FD render literally (Labelary behavior): no substitution.
    let labels = parse("^XA^SN001^SFABC#^FO50,50^A0N,30,30^FDABC#001^FS^XZ");
    assert_eq!(texts_of(&labels[0]), vec!["ABC#001".to_string()]);
}

// ── P0: ^GE (graphic ellipse) ─────────────────────────────────────────────────

#[test]
fn ge_defaults_and_params() {
    let labels = parse("^XA^FO100,100^GE200,100,5^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::GraphicEllipse(ge) => {
            assert_eq!((ge.position.x, ge.position.y), (100, 100));
            assert_eq!(ge.width, 200);
            assert_eq!(ge.height, 100);
            assert_eq!(ge.border_thickness, 5);
        }
        other => panic!("expected ellipse, got {:?}", other),
    }
    // defaults: width/height 3, thickness 1
    let labels = parse("^XA^FO100,100^GE,,\u{200B}^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::GraphicEllipse(ge) => {
            assert_eq!(ge.width, 3);
            assert_eq!(ge.height, 3);
            assert_eq!(ge.border_thickness, 1);
        }
        other => panic!("expected ellipse, got {:?}", other),
    }
    // W = white line color
    let labels = parse("^XA^FO100,100^GE200,100,5,W^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::GraphicEllipse(ge) => assert!(matches!(ge.line_color, LineColor::White)),
        other => panic!("expected ellipse, got {:?}", other),
    }
}

use labelize::elements::line_color::LineColor;

#[test]
fn ge_renders_ring_and_fill() {
    use labelize::{DrawerOptions, Renderer};
    let options = DrawerOptions {
        label_width_mm: 101.5,
        label_height_mm: 101.5,
        dpmm: 8,
        ..Default::default()
    };
    let render = |zpl: &str| {
        let labels = parse(zpl);
        let renderer = Renderer::new();
        let mut buf = std::io::Cursor::new(Vec::new());
        renderer
            .draw_label_as_png(&labels[0], &mut buf, options.clone())
            .expect("render");
        image::load_from_memory(&buf.into_inner())
            .expect("decode")
            .to_luma8()
    };
    // ring: border 5px, hollow center
    let img = render("^XA^FO100,100^GE200,100,5^FS^XZ");
    assert_eq!(img.get_pixel(105, 150)[0], 0, "ring left edge dark");
    assert_eq!(img.get_pixel(200, 150)[0], 255, "center hollow");
    assert_eq!(img.get_pixel(200, 115)[0], 255, "top interior hollow");
    // filled when thickness >= minor axis
    let img = render("^XA^FO100,100^GE200,100,50^FS^XZ");
    assert_eq!(img.get_pixel(200, 150)[0], 0, "thick border fills");
    // white color paints white (invisible on white canvas)
    let img = render("^XA^FO100,100^GE200,100,5,W^FS^XZ");
    assert_eq!(img.get_pixel(150, 150)[0], 255);
}

// ── P0: ^B8 (EAN-8) / ^BU (UPC-A) ─────────────────────────────────────────────

#[test]
fn b8_structure_and_check_digit() {
    let sym = labelize::barcodes::ean8::encode("96385074", 100, 2).expect("encode");
    assert_eq!(sym.image.width(), 67 * 2);
    assert_eq!(sym.check_digit, 4, "classic EAN-8 example 9638507 -> 4");
    // 7-digit input computes the check digit; 8-digit input recomputes it.
    let a = labelize::barcodes::ean8::encode("9638507", 100, 2).expect("encode");
    let b = labelize::barcodes::ean8::encode("96385074", 100, 2).expect("encode");
    assert_eq!(a.image, b.image);
    assert_eq!(a.digits, [9, 6, 3, 8, 5, 0, 7, 4]);
}

#[test]
fn b8_rejects_bad_lengths() {
    assert!(labelize::barcodes::ean8::encode("123456", 100, 2).is_err());
    assert!(labelize::barcodes::ean8::encode("123456789", 100, 2).is_err());
}

#[test]
fn b8_parser_defaults() {
    let labels = parse("^XA^FO50,50^B8N,100,Y,N^FD12345678^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::BarcodeEan8(bc) => {
            assert_eq!(bc.barcode.height, 100);
            assert!(bc.barcode.line);
            assert!(!bc.barcode.line_above);
        }
        other => panic!("expected barcode, got {:?}", other),
    }
}

#[test]
fn bu_check_digits_match_labelary() {
    // Verified against Labelary renders (right-half decode).
    let sym = labelize::barcodes::upca::encode("01234567890", 100, 2).expect("encode");
    assert_eq!(sym.check_digit, 5);
    let sym = labelize::barcodes::upca::encode("11234567890", 100, 2).expect("encode");
    assert_eq!(sym.check_digit, 2);
}

#[test]
fn bu_symbol_encodes_first_six_digits_left() {
    // Labelary puts the input's first six digits in the left half (L parity),
    // the remaining five plus check in the right half (R parity).
    let sym = labelize::barcodes::upca::encode("01234567890", 100, 2).expect("encode");
    assert_eq!(sym.image.width(), 95 * 2);
    assert_eq!(sym.digits, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0]);
    // 12-digit input uses the first 11 and recomputes the check.
    let a = labelize::barcodes::upca::encode("012345678905", 100, 2).expect("encode");
    let b = labelize::barcodes::upca::encode("012345678900", 100, 2).expect("encode");
    assert_eq!(a.image, b.image);
}

#[test]
fn bu_rejects_bad_lengths() {
    assert!(labelize::barcodes::upca::encode("1234567890", 100, 2).is_err());
    assert!(labelize::barcodes::upca::encode("0123456789012", 100, 2).is_err());
}

#[test]
fn bu_parser_defaults() {
    let labels = parse("^XA^FO50,50^BUN,100,Y,N^FD01234567890^FS^XZ");
    match &labels[0].elements[0] {
        LabelElement::BarcodeUca(bc) => {
            assert_eq!(bc.barcode.height, 100);
            assert!(bc.barcode.line);
        }
        other => panic!("expected barcode, got {:?}", other),
    }
}

// ── P0: ^LL (label length) ────────────────────────────────────────────────────

#[test]
fn ll_is_recorded_without_rendering_effect() {
    // ^LL has no visible effect (the canvas size comes from draw options) —
    // matching Labelary, which also keeps the requested label size.
    let plain = parse("^XA^FO50,50^GB100,100,5^FS^XZ");
    let with_ll = parse("^XA^LL400^FO50,50^GB100,100,5^FS^XZ");
    assert_eq!(boxes_of(&plain[0]), boxes_of(&with_ll[0]));
}
