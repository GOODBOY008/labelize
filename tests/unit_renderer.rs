mod common;

use common::render_helpers;
use labelize::elements::barcode_128::{Barcode128, Barcode128WithData, BarcodeMode};
use labelize::elements::barcode_qr::{BarcodeQr, BarcodeQrWithData};
use labelize::elements::drawer_options::DrawerOptions;
use labelize::elements::field_alignment::FieldAlignment;
use labelize::elements::field_orientation::FieldOrientation;
use labelize::elements::font::FontInfo;
use labelize::elements::graphic_box::GraphicBox;
use labelize::elements::graphic_circle::GraphicCircle;
use labelize::elements::graphic_ellipse::GraphicEllipse;
use labelize::elements::label_element::LabelElement;
use labelize::elements::label_info::LabelInfo;
use labelize::elements::label_position::LabelPosition;
use labelize::elements::line_color::LineColor;
use labelize::elements::reverse_print::ReversePrint;
use labelize::elements::text_field::TextField;

fn default_options() -> DrawerOptions {
    render_helpers::default_options()
}

fn empty_label() -> LabelInfo {
    LabelInfo {
        print_width: 0,
        inverted: false,
        elements: vec![],
    }
}

fn render_label(label: &LabelInfo, options: DrawerOptions) -> Vec<u8> {
    render_helpers::render_label_to_png(label, options)
}

fn decode_png(png: &[u8]) -> image::RgbaImage {
    image::load_from_memory(png).expect("decode png").to_rgba8()
}

// --- Canvas dimensions ---

#[test]
fn empty_label_produces_correct_canvas_dimensions() {
    let opts = DrawerOptions {
        label_width_mm: 50.0,
        label_height_mm: 30.0,
        dpmm: 8,
        enable_inverted_labels: false,
    };
    let png = render_label(&empty_label(), opts.clone());
    let img = decode_png(&png);
    let expected_w = (50.0 * 8.0_f64).ceil() as u32;
    let expected_h = (30.0 * 8.0_f64).ceil() as u32;
    assert_eq!(img.width(), expected_w);
    assert_eq!(img.height(), expected_h);
}

#[test]
fn dpmm_6_scales_canvas() {
    let opts = DrawerOptions {
        label_width_mm: 100.0,
        label_height_mm: 150.0,
        dpmm: 6,
        enable_inverted_labels: false,
    };
    let png = render_label(&empty_label(), opts);
    let img = decode_png(&png);
    assert_eq!(img.width(), 600);
    assert_eq!(img.height(), 900);
}

// --- GraphicBox rendering ---

#[test]
fn graphic_box_renders_black_pixels_in_box_region() {
    let label = LabelInfo {
        print_width: 0,
        inverted: false,
        elements: vec![LabelElement::GraphicBox(GraphicBox {
            reverse_print: ReversePrint { value: false },
            position: LabelPosition {
                x: 10,
                y: 10,
                calculate_from_bottom: false,
                automatic_position: false,
            },
            width: 100,
            height: 50,
            border_thickness: 3,
            corner_rounding: 0,
            line_color: LineColor::Black,
        })],
    };
    let png = render_label(&label, default_options());
    let img = decode_png(&png);

    // Check that some pixels inside the box border are black
    let pixel = img.get_pixel(10, 10);
    assert!(
        pixel[0] < 128,
        "expected black pixel at box border, got {:?}",
        pixel
    );

    // Check that a pixel far outside the box is white
    let outside = img.get_pixel(img.width() - 1, img.height() - 1);
    assert!(
        outside[0] > 128,
        "expected white pixel outside box, got {:?}",
        outside
    );
}

// --- GraphicCircle rendering ---

#[test]
fn graphic_circle_renders_non_white_pixels() {
    let label = LabelInfo {
        print_width: 0,
        inverted: false,
        elements: vec![LabelElement::GraphicCircle(GraphicCircle {
            reverse_print: ReversePrint { value: false },
            position: LabelPosition {
                x: 50,
                y: 50,
                calculate_from_bottom: false,
                automatic_position: false,
            },
            circle_diameter: 80,
            border_thickness: 3,
            line_color: LineColor::Black,
        })],
    };
    let png = render_label(&label, default_options());
    let img = decode_png(&png);

    // At least some non-white pixels should exist
    let has_dark = img.pixels().any(|p| p[0] < 128);
    assert!(has_dark, "expected non-white pixels for circle");
}

#[test]
fn graphic_ellipse_renders_non_white_ring_pixels() {
    let label = LabelInfo {
        print_width: 0,
        inverted: false,
        elements: vec![LabelElement::GraphicEllipse(GraphicEllipse {
            reverse_print: ReversePrint { value: false },
            position: LabelPosition {
                x: 40,
                y: 50,
                calculate_from_bottom: false,
                automatic_position: false,
            },
            width: 80,
            height: 30,
            border_thickness: 3,
            line_color: LineColor::Black,
        })],
    };
    let png = render_label(&label, default_options());
    let img = decode_png(&png);

    let top = img.get_pixel(80, 50);
    assert!(
        top[0] < 128,
        "expected dark top ellipse pixel, got {:?}",
        top
    );

    let center = img.get_pixel(80, 65);
    assert!(
        center[0] > 128,
        "expected white center for ellipse ring, got {:?}",
        center
    );
}

// --- Text rendering ---

#[test]
fn text_renders_non_white_pixels() {
    let zpl = "^XA^FO50,50^A0N,40,40^FDRendered Text^FS^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    let has_dark = img.pixels().any(|p| p[0] < 128);
    assert!(has_dark, "expected non-white pixels for text");
}

#[test]
fn field_alignment_center_positions_text_around_anchor() {
    let label = LabelInfo {
        print_width: 0,
        inverted: false,
        elements: vec![LabelElement::Text(TextField {
            reverse_print: ReversePrint { value: false },
            font: FontInfo {
                name: "0".to_string(),
                width: 30.0,
                height: 30.0,
                orientation: FieldOrientation::Normal,
            },
            position: LabelPosition {
                x: 100,
                y: 50,
                calculate_from_bottom: false,
                automatic_position: false,
            },
            alignment: FieldAlignment::Center,
            text: "HI".to_string(),
            block: None,
        })],
    };
    let png = render_label(&label, default_options());
    let img = decode_png(&png);

    let min_dark_x = img
        .enumerate_pixels()
        .filter_map(|(x, _, p)| if p[0] < 128 { Some(x) } else { None })
        .min()
        .expect("expected text pixels");
    assert!(
        min_dark_x < 100,
        "center-aligned text should start left of anchor x=100, got {}",
        min_dark_x
    );
}

// --- Barcode rendering ---

#[test]
fn barcode_renders_at_position() {
    let zpl = "^XA^FO100,100^BCN,100,Y,N,N^FD123456^FS^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    // Check there are dark pixels in the barcode region
    let mut dark_in_region = false;
    for y in 100..200u32 {
        for x in 100..300u32 {
            if x < img.width() && y < img.height() && img.get_pixel(x, y)[0] < 128 {
                dark_in_region = true;
                break;
            }
        }
        if dark_in_region {
            break;
        }
    }
    assert!(dark_in_region, "expected barcode pixels in region");
}

// --- Label inversion ---

#[test]
fn inverted_label_produces_different_output() {
    let zpl = "^XA^FO50,50^A0N,40,40^FDInvert^FS^XZ";
    let opts_normal = DrawerOptions {
        enable_inverted_labels: true,
        ..default_options()
    };

    let mut parser = labelize::ZplParser::new();
    let labels = parser.parse(zpl.as_bytes()).expect("parse");

    let normal_png = render_helpers::render_label_to_png(&labels[0], opts_normal.clone());

    // Create an inverted version
    let mut inverted_label = labels[0].clone();
    inverted_label.inverted = true;
    let inverted_png = render_helpers::render_label_to_png(&inverted_label, opts_normal);

    // The two renders should be different
    assert_ne!(
        normal_png, inverted_png,
        "inverted should differ from normal"
    );
}

// --- Print width centering ---

#[test]
fn print_width_smaller_than_label_width() {
    let zpl = "^XA^PW400^FO10,10^GB380,50,3^FS^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);
    // Image should still be full label width (101.625mm × 8 dpmm = 813px)
    let expected_w = (101.625 * 8.0_f64).ceil() as u32;
    assert_eq!(img.width(), expected_w);
}

// ====================================================================
// Issue-specific regression tests
// ====================================================================

// --- Issue #1: Label inversion (^POI) should flip the image ---

#[test]
fn poi_inverted_label_flips_content() {
    let zpl = "^XA^POI^FO10,10^GB100,50,50,B,0^FS^XZ";
    let opts = DrawerOptions {
        enable_inverted_labels: true,
        ..default_options()
    };
    let png = render_helpers::render_zpl_to_png(zpl, opts);
    let img = decode_png(&png);

    // The black box at (10,10) should appear rotated 180° into the bottom-right area.
    let top_left = img.get_pixel(10, 10);
    assert!(
        top_left[0] > 128,
        "top-left should be white in inverted label, got {:?}",
        top_left
    );

    let mut dark_in_lower_right = false;
    let w = img.width();
    let h = img.height();
    for y in (h - 100)..h {
        for x in (w - 200)..w {
            if img.get_pixel(x, y)[0] < 128 {
                dark_in_lower_right = true;
                break;
            }
        }
        if dark_in_lower_right {
            break;
        }
    }
    assert!(
        dark_in_lower_right,
        "expected dark pixels in lower-right for inverted label"
    );
}

// --- Issue #2: Barcode backgrounds should be transparent ---

#[test]
fn barcode_background_is_transparent_text_shows_through() {
    // Place text, then a barcode overlapping the same region.
    let zpl = "^XA\
        ^FO100,100^A0N,80,80^FDBIG^FS\
        ^FO100,100^BCN,80,N,N,N^FD12345^FS\
        ^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    let mut dark_count = 0u32;
    for y in 100..180u32 {
        for x in 100..300u32 {
            if x < img.width() && y < img.height() && img.get_pixel(x, y)[0] < 128 {
                dark_count += 1;
            }
        }
    }
    assert!(
        dark_count > 100,
        "expected dark pixels from both text and barcode, got {}",
        dark_count
    );
}

// --- Issue #3: ^FT positions text by baseline (using font ascent) ---

#[test]
fn ft_baseline_positions_text_with_margin_from_top() {
    // ^FT30,30 with font height 28: baseline at y=30.
    // Top of text = 30 - ascent, leaving margin from y=0.
    let zpl = "^XA^FT30,30^A0N,,28^FDGOGREEN^FS^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    // Row 0 and row 1 should be white: text should not touch the very top edge
    let mut row0_has_dark = false;
    let mut row1_has_dark = false;
    for x in 0..img.width() {
        if img.get_pixel(x, 0)[0] < 128 {
            row0_has_dark = true;
        }
        if img.get_pixel(x, 1)[0] < 128 {
            row1_has_dark = true;
        }
    }
    assert!(
        !row0_has_dark,
        "row 0 should be white: text should not touch the top edge"
    );
    assert!(
        !row1_has_dark,
        "row 1 should be white: there should be margin from top"
    );
}

// --- Issue #5: Diagonal line with thick border draws filled triangle ---

#[test]
fn diagonal_line_thick_border_draws_triangle() {
    // ^GD115,120,120,B,L — thickness (120) >= min(115,120)=115 → filled diagonal
    // With dual-triangle drawing: primary (B=Black) fills one half, opposite (White) fills the other.
    // L direction (top_to_bottom=true) = "\" direction (top-left to bottom-right):
    //   - Primary (Black): upper-right triangle [TL, TR, BR]
    //   - Opposite (White): lower-left triangle [TL, BL, BR]
    let zpl = "^XA^FO100,100^GD115,120,120,B,L^FS^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    // Top-right corner should be black (inside the primary upper-right triangle)
    let tr = img.get_pixel(210, 105);
    assert!(
        tr[0] < 128,
        "top-right of triangle should be black, got {:?}",
        tr
    );

    // Bottom-left corner should be white (inside the opposite lower-left triangle)
    let bl = img.get_pixel(105, 215);
    assert!(
        bl[0] > 128,
        "bottom-left should be white (opposite triangle), got {:?}",
        bl
    );

    // Upper-right region should be black
    let upper_right = img.get_pixel(200, 110);
    assert!(
        upper_right[0] < 128,
        "upper-right region should be black, got {:?}",
        upper_right
    );
}

// --- Issue #7: QR code should include quiet zone ---

#[test]
fn qr_code_has_quiet_zone() {
    // QR code per ZPL spec includes a 4-module quiet zone around the code.
    // With magnification=10, the quiet zone is 40 pixels wide.
    // ^FO0,0 with default ^BY height=10: QR modules start at (x=0, y=10).
    // The quiet zone extends to the left/top of the field origin (partially off-canvas).
    let zpl = "^XA^FO0,0^BQN,2,10^FDQA,Test Data^FS^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    // Before the QR modules (y=0..9 = before FO_y + by_height), canvas is white.
    let mut all_white_before_modules = true;
    for y in 0..10u32 {
        for x in 0..10u32 {
            if x < img.width() && y < img.height() && img.get_pixel(x, y)[0] < 128 {
                all_white_before_modules = false;
                break;
            }
        }
        if !all_white_before_modules {
            break;
        }
    }
    assert!(
        all_white_before_modules,
        "rows before QR module start should be white"
    );

    // Dark pixels (QR modules) should appear starting at y=10 (FO_y + by_height).
    let mut has_dark_at_modules = false;
    for y in 10..20u32 {
        for x in 0..20u32 {
            if x < img.width() && y < img.height() && img.get_pixel(x, y)[0] < 128 {
                has_dark_at_modules = true;
                break;
            }
        }
        if has_dark_at_modules {
            break;
        }
    }
    assert!(
        has_dark_at_modules,
        "QR should have dark modules starting at y=FO_y+by_height=10"
    );
}

#[test]
fn qr_code_rotation_changes_rendered_output() {
    let label = |orientation| LabelInfo {
        print_width: 0,
        inverted: false,
        elements: vec![LabelElement::BarcodeQr(BarcodeQrWithData {
            reverse_print: ReversePrint { value: false },
            barcode: BarcodeQr {
                magnification: 4,
                orientation,
            },
            height: 0,
            position: LabelPosition {
                x: 80,
                y: 80,
                calculate_from_bottom: false,
                automatic_position: false,
            },
            data: "HA,QR DATA".to_string(),
        })],
    };

    let normal_png = render_label(&label(FieldOrientation::Normal), default_options());
    let rotated_png = render_label(&label(FieldOrientation::Rotated90), default_options());

    assert_ne!(
        normal_png, rotated_png,
        "rotated QR render should differ from normal orientation"
    );
}

// --- Issue #8: Barcode interpretation line rotates with barcode ---

#[test]
fn barcode_interpretation_line_rotates_with_barcode() {
    // ^BCI = Code128, Rotated180, with interpretation line
    let zpl = "^XA^FO200,200^BCI,100,Y,N,N^FD12345^FS^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    // The barcode and interpretation line should produce dark pixels
    let has_dark_pixels = img.pixels().any(|p| p[0] < 128);
    assert!(
        has_dark_pixels,
        "barcode with rotated interpretation line should render"
    );

    // Compare: render same barcode with Normal orientation
    let zpl_normal = "^XA^FO200,200^BCN,100,Y,N,N^FD12345^FS^XZ";
    let png_normal = render_helpers::render_zpl_to_png(zpl_normal, default_options());

    // The rotated version should differ from normal
    assert_ne!(
        png, png_normal,
        "rotated barcode should differ from normal orientation"
    );
}

#[test]
fn barcode_interpretation_line_alignment_changes_text_position() {
    fn render_with_alignment(alignment: FieldAlignment) -> image::RgbaImage {
        let label = LabelInfo {
            print_width: 0,
            inverted: false,
            elements: vec![LabelElement::Barcode128(Barcode128WithData {
                reverse_print: ReversePrint { value: false },
                barcode: Barcode128 {
                    orientation: FieldOrientation::Normal,
                    height: 80,
                    line: true,
                    line_above: false,
                    check_digit: false,
                    mode: BarcodeMode::Automatic,
                    line_alignment: alignment,
                },
                width: 2,
                position: LabelPosition {
                    x: 120,
                    y: 100,
                    calculate_from_bottom: false,
                    automatic_position: false,
                },
                data: "123456".to_string(),
            })],
        };
        decode_png(&render_label(&label, default_options()))
    }

    fn text_bounds(img: &image::RgbaImage) -> (u32, u32) {
        let mut min_x = img.width();
        let mut max_x = 0;
        for y in 182..210 {
            for x in 0..img.width() {
                if img.get_pixel(x, y)[0] < 128 {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                }
            }
        }
        assert!(min_x < img.width(), "expected interpretation text pixels");
        (min_x, max_x)
    }

    let left = text_bounds(&render_with_alignment(FieldAlignment::Left));
    let center = text_bounds(&render_with_alignment(FieldAlignment::Center));
    let right = text_bounds(&render_with_alignment(FieldAlignment::Right));

    assert!(left.0 < center.0, "left text should start before center");
    assert!(center.0 < right.0, "center text should start before right");
    assert!(right.1 > center.1, "right text should end after center");
}

// --- Issue #9: EAN-13 guard bars should be taller than data bars ---

#[test]
fn ean13_guard_bars_taller_than_data_bars() {
    let img =
        labelize::barcodes::ean13::encode("123456789012", 200, 2).expect("ean13 encode failed");

    // Image should be taller than 200px to accommodate guard bar extension
    let total_height = img.height();
    assert!(
        total_height > 200,
        "EAN-13 image height ({}) should exceed barcode height (200) for guard bars",
        total_height
    );

    // Guard bars at start (modules 0-2) should have dark pixels in the extension area
    let bar_width = 2usize;
    let extended_row = 205u32; // in the guard bar extension area
    let has_guard_extension = (0..bar_width).any(|x| {
        x < img.width() as usize
            && extended_row < img.height()
            && img.get_pixel(x as u32, extended_row)[0] < 128
    });
    assert!(
        has_guard_extension,
        "guard bar at start should extend below data bar area"
    );
}

// --- Issue #11: Code39 interpretation line should show asterisks ---

#[test]
fn code39_interpretation_line_shows_asterisks() {
    // ^B3N with interpretation line, data "ABC"
    // Should display *ABC* in the interpretation line
    // ^B3 params: orientation, check_digit, height, line, line_above
    let zpl = "^XA^FO100,100^B3N,,100,Y^FDABC^FS^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    // Interpretation line should render dark pixels below the barcode
    let mut text_dark_count = 0u32;
    for y in 202..220u32 {
        for x in 0..img.width() {
            if y < img.height() && img.get_pixel(x, y)[0] < 128 {
                text_dark_count += 1;
            }
        }
    }
    assert!(
        text_dark_count > 0,
        "interpretation line should render below barcode"
    );
}

// --- Issue #4 & #10: text block center alignment ---

#[test]
fn text_block_center_alignment_centers_text() {
    let zpl = "^XA^FO100,100^A0N,40,40^FB400,1,0,C,^FDCenter^FS^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    // Find leftmost and rightmost dark pixels in the text area
    let mut left_most = img.width();
    let mut right_most = 0u32;
    for y in 100..140u32 {
        for x in 100..500u32 {
            if x < img.width() && y < img.height() && img.get_pixel(x, y)[0] < 128 {
                left_most = left_most.min(x);
                right_most = right_most.max(x);
            }
        }
    }

    // Text center should be near block center (100 + 200 = 300)
    let text_center = (left_most + right_most) / 2;
    let diff = (text_center as i32 - 300i32).unsigned_abs();
    assert!(
        diff < 30,
        "text center ({}) should be near block center (300), diff={}",
        text_center,
        diff
    );
}

// --- Issue #6: Font "0" width ratio is approximately 60% ---

#[test]
fn font_0_width_ratio_produces_narrower_text() {
    // Render same text with font "0" — the text should be ~60% of font height wide per char.
    // Render "WWWWW" at height=40, width=40 → effective scale_x = 0.6*40/40 = 0.6
    let zpl = "^XA^FO10,10^A0N,40,40^FDWWWWW^FS^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    // Find the rightmost dark pixel to measure actual text width
    let mut right_most = 0u32;
    for y in 10..50u32 {
        for x in 10..img.width() {
            if y < img.height() && img.get_pixel(x, y)[0] < 128 {
                right_most = right_most.max(x);
            }
        }
    }
    let text_width = right_most - 10;
    // 5 chars at height=40 with 0.6 ratio: ~5*24=120 pixels; should not exceed 150
    // With ratio 1.0 it would be ~5*40=200 pixels
    assert!(
        text_width < 160,
        "font 0 text width ({}) should reflect ~60% width ratio, not full 1:1",
        text_width
    );
}

// --- Issue #10: Rotated90 ^FT text positions inside black box ---

#[test]
fn rotated90_ft_text_positions_within_expected_region() {
    // ^FWB = Rotated90, ^FT = baseline position, ^FB = field block centered
    // Text "HELLO" should be positioned so the rotated column spans upward from y=500
    let zpl = "^XA\
        ^FO400,100^GB80,400,80,B^FS\
        ^FWB^FT440,500^A0,,60^FB400,1,0,C,^FDHELLO^FS\
        ^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    // The text should be inside the black box region (y=100 to y=500)
    // With the fix, y_top = 500 - 400 = 100, so text spans y=100..500
    let mut dark_in_box = 0u32;
    for y in 100..500u32 {
        for x in 400..480u32 {
            if x < img.width() && y < img.height() && img.get_pixel(x, y)[0] < 128 {
                dark_in_box += 1;
            }
        }
    }
    assert!(
        dark_in_box > 50,
        "rotated text should have dark pixels inside the box region, got {}",
        dark_in_box
    );

    // Text should NOT appear significantly below y=500
    let mut dark_below = 0u32;
    for y in 550..700u32 {
        for x in 400..480u32 {
            if x < img.width() && y < img.height() && img.get_pixel(x, y)[0] < 128 {
                dark_below += 1;
            }
        }
    }
    assert!(
        dark_below < 10,
        "rotated text should not appear below the box, got {} dark pixels",
        dark_below
    );
}
