use labelize::elements::drawer_options::DrawerOptions;
use labelize::elements::label_element::LabelElement;
use labelize::elements::label_info::LabelInfo;
use labelize::elements::graphic_box::GraphicBox;
use labelize::elements::graphic_circle::GraphicCircle;
use labelize::elements::label_position::LabelPosition;
use labelize::elements::line_color::LineColor;
use labelize::elements::reverse_print::ReversePrint;
use crate::common::render_helpers;

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

// --- Text rendering ---

#[test]
fn text_renders_non_white_pixels() {
    let zpl = "^XA^FO50,50^A0N,40,40^FDRendered Text^FS^XZ";
    let png = render_helpers::render_zpl_to_png(zpl, default_options());
    let img = decode_png(&png);

    let has_dark = img.pixels().any(|p| p[0] < 128);
    assert!(has_dark, "expected non-white pixels for text");
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
    // Image should still be full label width
    let expected_w = (102.0 * 8.0_f64).ceil() as u32;
    assert_eq!(img.width(), expected_w);
}
