use labelize::elements::field_alignment::FieldAlignment;
use labelize::elements::field_orientation::FieldOrientation;
use labelize::elements::graphic_field::GraphicFieldMode;
use labelize::elements::label_element::LabelElement;
use labelize::{DrawerOptions, TsplParser};

fn base_options() -> DrawerOptions {
    DrawerOptions {
        label_width_mm: 102.0,
        label_height_mm: 152.0,
        dpmm: 8,
        enable_inverted_labels: true,
    }
}

fn parse_with_options(tspl: &[u8]) -> Vec<labelize::parsers::TsplParsedLabel> {
    TsplParser::new()
        .parse_with_options(tspl, base_options())
        .expect("TSPL parse failed")
}

#[test]
fn size_updates_drawer_options_without_touching_label_info() {
    let labels = parse_with_options(
        br#"SIZE 100 mm,50 mm
CLS
TEXT 10,20,"3",0,1,1,"Hello"
PRINT 1
"#,
    );

    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0].label.print_width, 0);
    assert!((labels[0].drawer_options.label_width_mm - 100.0).abs() < f64::EPSILON);
    assert!((labels[0].drawer_options.label_height_mm - 50.0).abs() < f64::EPSILON);
}

#[test]
fn size_supports_inches_and_dots() {
    let labels = parse_with_options(
        br#"SIZE 2,1
CLS
TEXT 0,0,"1",0,1,1,"A"
PRINT 1
SIZE 812 dot,406 dot
CLS
TEXT 0,0,"1",0,1,1,"B"
PRINT 1
"#,
    );

    assert_eq!(labels.len(), 2);
    assert!((labels[0].drawer_options.label_width_mm - 50.8).abs() < 0.001);
    assert!((labels[0].drawer_options.label_height_mm - 25.4).abs() < 0.001);
    assert!((labels[1].drawer_options.label_width_mm - 101.5).abs() < 0.001);
    assert!((labels[1].drawer_options.label_height_mm - 50.75).abs() < 0.001);
}

#[test]
fn text_maps_font_rotation_alignment_and_reference_offsets() {
    let labels = parse_with_options(
        br#"SIZE 4,2
REFERENCE 10,20
SHIFT 3,4
CLS
TEXT 100,50,"3",90,2,3,2,"Aligned"
PRINT 1
"#,
    );

    let text = match &labels[0].label.elements[0] {
        LabelElement::Text(text) => text,
        other => panic!("expected Text, got {:?}", other),
    };
    assert_eq!(text.text, "Aligned");
    assert_eq!(text.position.x, 113);
    assert_eq!(text.position.y, 74);
    assert_eq!(text.font.orientation, FieldOrientation::Rotated90);
    assert_eq!(text.font.width, 32.0);
    assert_eq!(text.font.height, 72.0);
    assert_eq!(text.alignment, FieldAlignment::Center);
}

#[test]
fn text_uses_tspl_resident_bitmap_font_sizes() {
    let labels = parse_with_options(
        br#"SIZE 4,2
CLS
TEXT 0,0,"1",0,1,1,"F1"
TEXT 0,20,"5",0,2,3,"F5"
TEXT 0,80,"8",0,1,2,"F8"
PRINT 1
"#,
    );

    let text = |idx| match &labels[0].label.elements[idx] {
        LabelElement::Text(text) => text,
        other => panic!("expected Text, got {:?}", other),
    };

    assert_eq!(text(0).font.name, "TSPL_1");
    assert_eq!(text(0).font.width, 8.0);
    assert_eq!(text(0).font.height, 12.0);
    assert!(text(0).font.is_bitmap_font());

    assert_eq!(text(1).font.name, "TSPL_5");
    assert_eq!(text(1).font.width, 64.0);
    assert_eq!(text(1).font.height, 144.0);
    assert!(text(1).font.is_bitmap_font());

    assert_eq!(text(2).font.name, "TSPL_8");
    assert_eq!(text(2).font.width, 14.0);
    assert_eq!(text(2).font.height, 50.0);
    assert!(text(2).font.is_bitmap_font());
}

#[test]
fn text_supports_tspl_chinese_bitmap_fonts() {
    let labels = parse_with_options(
        r#"SIZE 4,2
CLS
TEXT 71,360,"TSS24.BF2",0,1,1,"张三 13888885555"
TEXT 71,390,"TST16.BF2",0,2,3,"李四"
TEXT 71,430,"TTT24.BF2",90,1,1,"王五"
TEXT 71,470,"TST24.BF2",0,1,1,"赵六"
TEXT 71,500,"TTT16.BF2",0,1,1,"钱七"
TEXT 71,530,"TSS16.BF2",0,1,1,"孙八"
PRINT 1
"#
        .as_bytes(),
    );

    let text = |idx| match &labels[0].label.elements[idx] {
        LabelElement::Text(text) => text,
        other => panic!("expected Text, got {:?}", other),
    };

    assert_eq!(text(0).font.name, "TSS24.BF2");
    assert_eq!(text(0).font.width, 24.0);
    assert_eq!(text(0).font.height, 24.0);
    assert_eq!(text(0).text, "张三 13888885555");

    assert_eq!(text(1).font.name, "TST16.BF2");
    assert_eq!(text(1).font.width, 32.0);
    assert_eq!(text(1).font.height, 48.0);

    assert_eq!(text(2).font.name, "TTT24.BF2");
    assert_eq!(text(2).font.orientation, FieldOrientation::Rotated90);

    assert_eq!(text(3).font.name, "TST24.BF2");
    assert_eq!(text(3).font.width, 24.0);
    assert_eq!(text(3).font.height, 24.0);

    assert_eq!(text(4).font.name, "TTT16.BF2");
    assert_eq!(text(4).font.width, 16.0);
    assert_eq!(text(4).font.height, 16.0);

    assert_eq!(text(5).font.name, "TSS16.BF2");
    assert_eq!(text(5).font.width, 16.0);
    assert_eq!(text(5).font.height, 16.0);
}

#[test]
fn quoted_text_can_span_lines() {
    let labels = parse_with_options(
        br#"SIZE 4,2
CLS
TEXT 25,25,"3",0,1,1,"FORMFEED COMMAND
TEXT"
PRINT 1
"#,
    );

    let text = match &labels[0].label.elements[0] {
        LabelElement::Text(text) => text,
        other => panic!("expected Text, got {:?}", other),
    };
    assert_eq!(text.text, "FORMFEED COMMAND\nTEXT");
}

#[test]
fn graphics_commands_map_to_renderable_elements() {
    let labels = parse_with_options(
        br#"SIZE 4,2
CLS
BAR 1,2,30,4
BOX 10,20,110,70,5,4
CIRCLE 40,50,20,3
ELLIPSE 60,70,80,30,2
ERASE 5,6,7,8
REVERSE 9,10,11,12
PRINT 1
"#,
    );

    assert_eq!(labels[0].label.elements.len(), 6);
    match &labels[0].label.elements[0] {
        LabelElement::GraphicBox(bar) => {
            assert_eq!(bar.position.x, 1);
            assert_eq!(bar.position.y, 2);
            assert_eq!(bar.width, 30);
            assert_eq!(bar.height, 4);
            assert_eq!(bar.border_thickness, 4);
        }
        other => panic!("expected BAR GraphicBox, got {:?}", other),
    }
    match &labels[0].label.elements[1] {
        LabelElement::GraphicBox(b) => {
            assert_eq!(b.width, 100);
            assert_eq!(b.height, 50);
            assert_eq!(b.border_thickness, 5);
            assert_eq!(b.corner_rounding, 4);
        }
        other => panic!("expected BOX GraphicBox, got {:?}", other),
    }
    assert!(matches!(
        labels[0].label.elements[2],
        LabelElement::GraphicCircle(_)
    ));
    assert!(matches!(
        labels[0].label.elements[3],
        LabelElement::GraphicEllipse(_)
    ));
    match &labels[0].label.elements[4] {
        LabelElement::GraphicBox(erase) => {
            assert_eq!(erase.width, 7);
            assert_eq!(erase.height, 8);
        }
        other => panic!("expected ERASE GraphicBox, got {:?}", other),
    }
    match &labels[0].label.elements[5] {
        LabelElement::GraphicBox(reverse) => {
            assert!(reverse.reverse_print.value);
            assert_eq!(reverse.width, 11);
            assert_eq!(reverse.height, 12);
        }
        other => panic!("expected REVERSE GraphicBox, got {:?}", other),
    }
}

#[test]
fn bitmap_preserves_raw_bytes_and_mode() {
    let labels = parse_with_options(b"SIZE 2,1\nCLS\nBITMAP 8,9,1,2,0,\x80\x01\nPRINT 1\n");

    let bitmap = match &labels[0].label.elements[0] {
        LabelElement::GraphicField(bitmap) => bitmap,
        other => panic!("expected GraphicField, got {:?}", other),
    };
    assert_eq!(bitmap.position.x, 8);
    assert_eq!(bitmap.position.y, 9);
    assert_eq!(bitmap.row_bytes, 1);
    assert_eq!(bitmap.total_bytes, 2);
    assert_eq!(bitmap.data, vec![0x80, 0x01]);
    assert_eq!(bitmap.mode, GraphicFieldMode::Overwrite);
}

#[test]
fn barcode_qrcode_and_pdf417_commands_map_to_existing_barcode_elements() {
    let labels = parse_with_options(
        br#"SIZE 4,2
CLS
BARCODE 10,20,"128",60,1,90,2,2,"ABC123"
BARCODE 15,25,"39C",70,0,180,2,6,"CODE39"
QRCODE 30,40,H,4,A,270,"QR DATA"
PDF417 50,60,400,200,90,E3,W4,H5,R10,C6,T1,"PDF DATA"
PRINT 1
"#,
    );

    match &labels[0].label.elements[0] {
        LabelElement::Barcode128(bc) => {
            assert_eq!(bc.data, "ABC123");
            assert_eq!(bc.position.x, 10);
            assert_eq!(bc.barcode.height, 60);
            assert_eq!(bc.barcode.orientation, FieldOrientation::Rotated90);
            assert!(bc.barcode.line);
            assert_eq!(bc.barcode.line_alignment, FieldAlignment::Left);
        }
        other => panic!("expected Barcode128, got {:?}", other),
    }
    match &labels[0].label.elements[1] {
        LabelElement::Barcode39(bc) => {
            assert_eq!(bc.data, "CODE39");
            assert_eq!(bc.barcode.orientation, FieldOrientation::Rotated180);
            assert!(bc.barcode.check_digit);
            assert!(!bc.barcode.line);
            assert_eq!(bc.width_ratio, 3.0);
            assert_eq!(bc.barcode.line_alignment, FieldAlignment::Left);
        }
        other => panic!("expected Barcode39, got {:?}", other),
    }
    match &labels[0].label.elements[2] {
        LabelElement::BarcodeQr(qr) => {
            assert_eq!(qr.data, "HA,QR DATA");
            assert_eq!(qr.barcode.magnification, 4);
            assert_eq!(qr.barcode.orientation, FieldOrientation::Rotated270);
        }
        other => panic!("expected BarcodeQr, got {:?}", other),
    }
    match &labels[0].label.elements[3] {
        LabelElement::BarcodePdf417(pdf) => {
            assert_eq!(pdf.data, "PDF DATA");
            assert_eq!(pdf.barcode.orientation, FieldOrientation::Rotated90);
            assert_eq!(pdf.barcode.security, 3);
            assert_eq!(pdf.barcode.module_width, 4);
            assert_eq!(pdf.barcode.row_height, 5);
            assert_eq!(pdf.barcode.by_height, 200);
            assert_eq!(pdf.barcode.rows, 10);
            assert_eq!(pdf.barcode.columns, 6);
            assert!(pdf.barcode.truncate);
        }
        other => panic!("expected BarcodePdf417, got {:?}", other),
    }
}

#[test]
fn barcode_human_readable_value_controls_line_alignment() {
    let labels = parse_with_options(
        br#"SIZE 4,2
CLS
BARCODE 10,20,"128",60,1,0,2,2,"LEFT"
BARCODE 10,100,"128",60,2,0,2,2,"CENTER"
BARCODE 10,180,"128",60,3,0,2,2,"RIGHT"
PRINT 1
"#,
    );

    let alignment = |idx| match &labels[0].label.elements[idx] {
        LabelElement::Barcode128(bc) => {
            assert!(bc.barcode.line);
            bc.barcode.line_alignment
        }
        other => panic!("expected Barcode128, got {:?}", other),
    };

    assert_eq!(alignment(0), FieldAlignment::Left);
    assert_eq!(alignment(1), FieldAlignment::Center);
    assert_eq!(alignment(2), FieldAlignment::Right);
}

#[test]
fn unsupported_visible_commands_return_an_error_but_device_commands_are_ignored() {
    let labels = parse_with_options(
        br#"SIZE 4,2
SPEED 4
DENSITY 10
SET TEAR ON
CLS
TEXT 0,0,"1",0,1,1,"A"
PRINT 1
"#,
    );
    assert_eq!(labels.len(), 1);

    let err = TsplParser::new()
        .parse_with_options(
            br#"SIZE 4,2
CLS
MPDF417 10,10,0,"ABC"
PRINT 1
"#,
            base_options(),
        )
        .expect_err("MPDF417 should be unsupported");
    assert!(err.contains("unsupported TSPL command MPDF417"));
}
