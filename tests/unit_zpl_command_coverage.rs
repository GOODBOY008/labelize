use labelize::parsers::zpl::format::format_zpl;
use labelize::{elements::LabelElement, ZplParser};

fn texts(source: &[u8]) -> Vec<Vec<String>> {
    ZplParser::new()
        .parse(source)
        .unwrap()
        .iter()
        .map(|label| {
            label
                .elements
                .iter()
                .filter_map(|el| match el {
                    LabelElement::Text(t) => Some(t.text.clone()),
                    _ => None,
                })
                .collect()
        })
        .collect()
}

#[test]
fn prefixes_delimiters_and_binary_data_survive_formatting_and_separate_jobs() {
    let source=b"~CC!!XA!CD;!FO100;150!FDfirst,second;third!FS~CT??DGR:A;1;1;FF!IMR:A!FS!GFB;4;4;1;!;?\n!FS!XZ";
    for job in [source.to_vec(), format_zpl(source)] {
        let labels = ZplParser::new().parse(&job).unwrap();
        let el = &labels[0].elements;
        assert!(
            matches!(&el[0], LabelElement::Text(t) if t.position.x==100 && t.position.y==150 && t.text=="first,second;third")
        );
        assert!(matches!(&el[1], LabelElement::GraphicField(g) if g.data==[255]));
        assert!(matches!(&el[2], LabelElement::GraphicField(g) if g.data==b"!;?\n"));
    }
    let mut p = ZplParser::new();
    p.parse(b"~CC!~CD;").unwrap();
    let page = p.parse(b"!XA!FO5;7!FDx!FS!XZ").unwrap();
    assert!(matches!(&page[0].elements[0],LabelElement::Text(t) if t.position.y==7));
}

#[test]
fn batch_fields_use_each_serial_for_its_replica_group() {
    let serials = texts(b"^XA^SNID-009X,-2,Y^FS^FDBL99-9^SFAAdd%d,1%1^FS^PQ5,0,2^XZ");
    assert_eq!(
        serials,
        [
            ["ID-009X", "BL99-9"],
            ["ID-009X", "BL99-9"],
            ["ID-007X", "BM01-0"],
            ["ID-007X", "BM01-0"],
            ["ID-005X", "BM02-1"]
        ]
    );
    assert_eq!(texts(b"^XA^SN001,1,N^FS^PQ2^XZ"), [["1"], ["2"]]);
    assert_eq!(
        texts(b"^XA^DFR:S.ZPL^FS^FN1^FS^SN008,1,Y^FS^XZ^XA^XFR:S.ZPL^FN1^FDName^FS^PQ2^XZ"),
        [["Name", "008"], ["Name", "009"]]
    );
}

#[test]
fn references_substrings_clock_offsets_and_remapping() {
    let pages=texts(b"^XA^ST11,18,2026,10,05,09,M^SO2,0,1^FN2^FDBATCH123^FS^FE#^FD#2#/#2,b,1,3#/#2,f,2,2#^FS^FC%,@^FD%Y-%m-%d %H:%M:%S / @d^FS^CI13,66,65,65,66^FDAB^FS^XZ");
    assert_eq!(
        pages,
        [["BATCH123/123/AT", "2026-11-18 10:05:09 / 19", "BA"]]
    );
}

#[test]
fn height_limited_text_rotates_wraps_and_discards_overflow() {
    use labelize::elements::field_orientation::FieldOrientation;
    let labels = ZplParser::new()
        .parse(b"^XA^A0N,20,20^TBR,55,40^FDFIRST SECOND THIRD^FS^XZ")
        .unwrap();
    let LabelElement::Text(t) = &labels[0].elements[0] else {
        panic!("missing text block")
    };
    let b = t.block.as_ref().unwrap();
    assert_eq!(
        (b.max_width, b.max_lines, t.font.orientation),
        (55, 2, FieldOrientation::Rotated90)
    );
    assert!(!b.overwrite_last_line);
}

#[test]
fn map_clear_preserves_background_without_accumulating_variable_fields() {
    let labels = ZplParser::new()
        .parse(b"^XA^MCN^GB16,8,8^FS^XZ^XA^FVONE^FS^XZ^XA^FVTWO^FS^XZ^XA^MCY^FDEND^FS^XZ")
        .unwrap();
    assert_eq!(
        labels.iter().map(|l| l.elements.len()).collect::<Vec<_>>(),
        [1, 2, 2, 1]
    );
    assert!(matches!(
        &labels[2].elements[0],
        LabelElement::RasterLabel(_)
    ));
}
fn pixels(job: impl AsRef<[u8]>) -> image::GrayImage {
    let out = labelize::render_png(
        job.as_ref(),
        labelize::RenderOptions {
            width_mm: Some(100.0),
            height_mm: Some(50.0),
            language: Some(labelize::Language::Zpl),
            ..Default::default()
        },
    )
    .unwrap();
    image::load_from_memory(&out.bytes).unwrap().to_luma8()
}

#[test]
fn each_extra_barcode_draws_a_symbol() {
    let cases = [
        ("^B1N,Y,50,N", "123-45"),
        ("^B4N,3,N,A", "AB123"),
        ("^B5N,50,N", "12345678901"),
        ("^BAN,50,N", "ABC123"),
        ("^BBN,8,Y,8,2,F", "BLOCK42"),
        ("^BFN,3,0", "ABC"),
        ("^BIN,50,N", "12345"),
        ("^BJN,50,N", "12345"),
        ("^BKN,N,50,N,N,A,B", "12345"),
        ("^BLN,50,N", "ITEM123"),
        ("^BMN,B,50,N", "12345"),
        ("^BPN,N,50,N", "01AC"),
        ("^BRN,1,2", "0001234567890"),
        ("^BSN,40,N", "12"),
        ("^BTN", "123456,SERIAL12"),
        ("^BZN,50,N,N,0", "12345"),
    ];
    for (cmd, data) in cases {
        let job = format!("^XA^FO40,40{cmd}^FD{data}^FS^XZ");
        let image = pixels(&job);
        assert!(image.as_raw().contains(&0), "{cmd} has no ink");
        assert_ne!(
            image,
            pixels(format!("^XA^FO40,40^FD{data}^FS^XZ")),
            "{cmd} fell through to text"
        );
    }
}

#[test]
fn downloaded_objects_and_true_type_faces_reach_the_renderer() {
    let font = include_bytes!("../src/assets/fonts/DejaVuSansMono.ttf");
    let hex: String = font.iter().map(|x| format!("{x:02x}")).collect();
    let field = b"^XA^CI28^FO10,10^A@N,34,34,R:SAMPLE.TTF^FDWide 123^FS^XZ";
    let mut raw = format!("~DYR:SAMPLE.TTF,B,T,{},,", font.len()).into_bytes();
    raw.extend(font);
    raw.extend(field);
    let reference = pixels(&raw);
    assert_ne!(reference, pixels(field));
    for command in ["~DT", "~DU"] {
        let mut zpl = format!("{command}R:SAMPLE.TTF,{},{}", font.len(), hex).into_bytes();
        zpl.extend(field);
        assert_eq!(pixels(zpl), reference);
    }
    let img = pixels(b"~DYR:DOT,B,G,3,1,^~\n^XA^FO8,8^IMR:DOT^FS^XZ");
    assert_eq!(img, pixels(b"^XA^FO8,8^GFA,3,3,1,5E7E0A^FS^XZ"));
}

#[test]
fn downloaded_bitmap_font_keeps_its_native_cells() {
    let src=b"~DBR:T.FNT,N,5,4,4,4,1,TEST,#0041.5.4.0.4.4.F090F09090^XA^CI28^FO10,10^A@N,5,4,R:T.FNT^FDA^FS^XZ";
    let image = pixels(src);
    let expected = pixels(b"^XA^FO10,10^GFA,5,5,1,F090F09090^FS^XZ");
    assert_eq!(image, expected);
}

#[test]
fn linked_font_supplies_missing_glyphs_and_can_be_unlinked() {
    let base = include_bytes!("../src/assets/fonts/ZplGSCustom.ttf");
    let link = include_bytes!("../src/assets/fonts/DejaVuSansMono.ttf");
    let mut resources = Vec::new();
    for (name, bytes) in [("SYMBOL", &base[..]), ("MONO", &link[..])] {
        resources.extend(format!("~DYR:{name}.TTF,B,T,{},,", bytes.len()).bytes());
        resources.extend(bytes);
    }
    let render = |command: &[u8]| {
        let mut job = resources.clone();
        job.extend(command);
        job.extend(b"^XA^CI28^FO8,8^A@N,36,36,R:SYMBOL.TTF^FDz^FS^XZ");
        pixels(job)
    };
    let missing = render(b"");
    let linked = render(b"^FLR:MONO.TTF,R:SYMBOL.TTF,1");
    assert_ne!(linked, missing);
    assert_eq!(
        render(b"^FLR:MONO.TTF,R:SYMBOL.TTF,1^FLR:MONO.TTF,R:SYMBOL.TTF,0"),
        missing
    );
}

#[test]
fn composites_cover_databar_ean_and_gs1_128_with_both_input_styles() {
    for (variant, primary) in [
        (1, "12345678901"),
        (2, "12345678901"),
        (3, "12345678901"),
        (4, "12345678901"),
        (5, "12345678901"),
        (6, "(01)09012345678908(10)ABC123"),
        (7, "12345678901"),
        (8, "0123456"),
        (9, "590123412345"),
        (10, "1234567"),
        (11, "(01)04012345123456"),
        (12, "(01)04012345123456"),
    ] {
        for secondary in ["(99)1234567", "this is composite info"] {
            let job = format!("^XA^FO10,10^BRN,{variant},2,1,50^FD{primary}|{secondary}^FS^XZ");
            let result = labelize::render_png(
                job.as_bytes(),
                labelize::RenderOptions {
                    language: Some(labelize::Language::Zpl),
                    ..Default::default()
                },
            );
            assert!(
                result.is_ok(),
                "variant {variant}, {secondary}: {:?}",
                result.err()
            );
        }
    }
}

#[test]
fn physical_text_block_clips_a_partial_line_before_rotation() {
    for orientation in ["N", "R", "I", "B"] {
        let page = pixels(format!(
            "^XA^FO80,80^A0N,24,24^TB{orientation},80,6^FDONE TWO THREE^FS^XZ"
        ));
        let (w, h) = if matches!(orientation, "N" | "I") {
            (80, 6)
        } else {
            (6, 80)
        };
        let mut count = 0;
        for (x, y, p) in page.enumerate_pixels() {
            if p[0] < 128 {
                count += 1;
                assert!((80..80 + w).contains(&x) && (80..80 + h).contains(&y));
            }
        }
        assert!(count > 0, "{orientation}");
    }
}

#[test]
fn advanced_properties_shape_and_reorder_uploaded_unicode() {
    let font = include_bytes!("../src/assets/fonts/DejaVuSansMono.ttf");
    let mut base = format!("~DYR:S.TTF,B,T,{},,", font.len()).into_bytes();
    base.extend(font);
    let draw = |settings: &str| {
        let mut job = base.clone();
        job.extend(format!("^XA^CI28{settings}^FO10,10^A@N,40,40,R:S.TTF^FDسلام 123^FS^XZ").bytes());
        pixels(job)
    };
    assert_ne!(draw("^PA0,0,0,0"), draw("^PA0,1,1,1"));
}

#[test]
fn recall_assigns_serial_values_to_the_template_barcode() {
    let job =
        b"^XA^DFR:F.ZPL^FS^FO15,20^BAN,30,N^FN9^FS^XZ^XA^XFR:F.ZPL^FN9^SN007,1,Y^FS^PQ3,0,2^XZ";
    let pages = ZplParser::new().parse(job).unwrap();
    let values: Vec<_> = pages
        .iter()
        .map(|page| match &page.elements[0] {
            LabelElement::BarcodeSymbol(b) => {
                assert_eq!((b.position.x, b.position.y), (15, 20));
                b.data.clone()
            }
            el => panic!("unexpected field {el:?}"),
        })
        .collect();
    assert_eq!(values, [b"007", b"007", b"008"]);
}

#[test]
fn code93_shift_pairs_decode_before_encoding_and_remapping_uses_input_bytes() {
    let pages = ZplParser::new()
        .parse(b"^XA^BA^FD)A(B'V^FS^CI13,66,128^FD\x80^FS^XZ")
        .unwrap();
    assert!(matches!(&pages[0].elements[0],LabelElement::BarcodeSymbol(b) if b.data==b"a\x22@"));
    assert!(matches!(&pages[0].elements[1],LabelElement::Text(t) if t.text=="B"));
}

#[test]
fn png_downloads_validate_the_crc_before_replacing_an_existing_object() {
    const BODY: &str = "iVBORw0KGgoAAAANSUhEUgAAAAgAAAABCAAAAADGa7CfAAAADklEQVR4nGNk+M8AAowABwwBAp7YmwgAAAAASUVORK5CYII=";
    let command = |crc: &str| format!("~DYR:ICON,P,P,71,,:B64:{BODY}:{crc}");
    let recall = "^XA^FO12,12^IMR:ICON.PNG^FS^XZ";
    assert_eq!(
        pixels(format!("{}{recall}", command("AE00"))),
        pixels(b"^XA^FO12,12^GFA,1,1,1,81^FS^XZ")
    );
    let mut parser = ZplParser::new();
    parser.parse(command("AE00").as_bytes()).unwrap();
    assert!(parser
        .parse(command("AE01").as_bytes())
        .unwrap_err()
        .contains("CRC"));
    assert!(
        matches!(&parser.parse(recall.as_bytes()).unwrap()[0].elements[0], LabelElement::GraphicField(g) if g.data == [0x81])
    );
}

#[test]
fn bitmap_downloads_accept_individually_encoded_glyphs() {
    let reference = pixels(b"^XA^FO10,10^GFA,5,5,1,F090F09090^FS^XZ");
    for glyph in [
        ":B64:8JDwkJA=:24E5",
        ":Z64:eJz7MOHDhAkAC3UDkQ==:191E",
        ":Z64:H4sIAAAAAAACE/sw4cOECQBqLNwHBQAAAA==:3BBF",
    ] {
        let job = format!("~DBR:T.FNT,N,5,4,4,4,1,TEST,#0041.5.4.0.4.4.{glyph}^XA^FO10,10^A@N,5,4,R:T.FNT^FDA^FS^XZ");
        assert_eq!(pixels(job), reference);
    }
}

#[test]
fn legacy_intellifont_storage_and_unsupported_outlines_are_distinct() {
    let mut printer = ZplParser::new();
    let pages = printer
        .parse(b"~DSR:LEGACY.FNT,4,11223344^XA^FDOK^FS^XZ")
        .unwrap();
    assert_eq!(pages.len(), 1);
    let error = printer
        .parse(b"^XA^A@N,30,30,R:LEGACY.FNT^FDTEXT^FS^XZ")
        .unwrap_err();
    assert!(error.contains("Intellifont"));
}

#[test]
fn gs1_parentheses_and_field_separators_preserve_the_same_payload() {
    let marked = "(01)09012345678906(10)BATCH(21)1234";
    let raw = "010901234567890610BATCH_1D211234";
    for (variant, suffix) in [
        (6, ""),
        (11, ""),
        (12, ""),
        (6, "|(99)1234567"),
        (11, "|(99)1234567"),
    ] {
        let label = |field: &str| {
            pixels(format!(
                "^XA^FO12,12^BRN,{variant},2,1,50^FH^FD{field}{suffix}^FS^XZ"
            ))
        };
        assert_eq!(label(marked), label(raw), "GS1 variant {variant}{suffix}");
    }
}

#[test]
fn code49_keeps_the_reference_modules_and_binding_bars() {
    // Zint's Code 49 module dump for ABC. The final two bits are byte padding.
    let rows = [
        [0xbf, 0x6c, 0xb2, 0x67, 0x33, 0xcb, 0xa2, 0x5f, 0xbc],
        [0xb3, 0xcb, 0xa1, 0x33, 0x3b, 0x88, 0xa7, 0x93, 0x3c],
    ];
    let page = pixels(b"^XA^FO20,20^BY1^B4N,3,N,A^FDABC^FS^XZ");
    for y in 0..9 {
        for x in 0..81u32 {
            let expected = match y {
                0 | 8 => true,
                4 => (10..80).contains(&x),
                _ if (10..80).contains(&x) => {
                    let i = (x - 10) as usize;
                    rows[usize::from(y > 4)][i / 8] & (128 >> (i % 8)) != 0
                }
                _ => false,
            };
            assert_eq!(
                page.get_pixel(20 + x, 20 + y)[0] == 0,
                expected,
                "dot ({x},{y})"
            );
        }
    }
}

#[test]
fn long_literal_composite_fields_select_larger_symbol_sizes() {
    let data = "abcDEF0123456789".repeat(16);
    for (variant, primary, lengths) in [
        (1, "12345678901", &[50, 110][..]),
        (11, "(01)04012345123456", &[50, 110][..]),
        (12, "(01)04012345123456", &[50, 110, 250][..]),
    ] {
        for &len in lengths {
            let source = format!(
                "^XA^FO10,10^BRN,{variant},2,1,50^FD{primary}|{}^FS^XZ",
                &data[..len]
            );
            assert!(
                pixels(source).pixels().any(|p| p[0] == 0),
                "empty symbol {variant}, {len} bytes"
            );
        }
    }
}

#[test]
fn stacked_codablock_height_respects_explicit_rows_and_overall_default() {
    for (by, height, expected_height) in [("^BY2,3,40", "", 40u32), ("^BY2,3,40", "8", 38)] {
        let page = pixels(format!(
            "^XA^FO10,20{by}^BBN,{height},Y,6,2,F^FDABCDE^FS^XZ"
        ));
        let ys: Vec<_> = (0..page.height())
            .filter(|&y| (10..page.width()).any(|x| page.get_pixel(x, y)[0] == 0))
            .collect();
        assert_eq!(ys.len(), expected_height as usize);
        assert_eq!(ys[0], 20);
        assert_eq!(ys.last(), Some(&(20 + expected_height - 1)));
    }
}

#[test]
fn tlc_without_a_serial_prints_the_six_digit_code39_alone() {
    let primary = pixels(b"^XA^FO20,20^BY2,2,40^B3N,N,40,N,N^FD123456^FS^XZ");
    for data in ["123456", "1234567"] {
        assert_eq!(pixels(format!("^XA^FO20,20^BTN^FD{data}^FS^XZ")), primary);
    }
}
