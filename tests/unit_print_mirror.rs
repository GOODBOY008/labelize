use image::RgbaImage;
use labelize::{DrawerOptions, LabelInfo, Renderer, ZplParser};

const FIELD: &str = "^FO10,20^GB20,10,10^FS";

fn options() -> DrawerOptions {
    DrawerOptions {
        label_width_mm: 25.375,
        label_height_mm: 12.625,
        dpmm: 8,
        ..Default::default()
    }
}

fn render(label: &LabelInfo, opts: DrawerOptions) -> RgbaImage {
    let mut png = Vec::new();
    Renderer::new()
        .draw_label_as_png(label, &mut png, opts)
        .unwrap();
    image::load_from_memory(&png).unwrap().to_rgba8()
}

fn labels(source: &str) -> Vec<LabelInfo> {
    ZplParser::new().parse(source.as_bytes()).unwrap()
}

fn bounds(image: &RgbaImage) -> (u32, u32, u32, u32) {
    let mut result = (image.width(), image.height(), 0, 0);
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[0] < 128 {
            result.0 = result.0.min(x);
            result.1 = result.1.min(y);
            result.2 = result.2.max(x);
            result.3 = result.3.max(y);
        }
    }
    result
}

#[test]
fn mirror_and_orientation_match_labelary_with_print_width() {
    // Independently measured using Labelary's 8dpmm, 1 x 0.5 inch canvas.
    // A 100-dot print width leaves odd margins: centering precedes mirroring.
    for (commands, expected) in [
        ("", (10, 20, 29, 29)),
        ("^PMY", (173, 20, 192, 29)),
        ("^PW100", (61, 20, 80, 29)),
        ("^PW100^PMY", (122, 20, 141, 29)),
        ("^PW100^POI", (122, 71, 141, 80)),
        ("^PW100^PMY^POI", (61, 71, 80, 80)),
        ("^PW300^PMY", (173, 20, 192, 29)),
    ] {
        let label = &labels(&format!("^XA{commands}{FIELD}^XZ"))[0];
        assert_eq!(bounds(&render(label, options())), expected, "{commands}");
    }
}

#[test]
fn mirror_persists_until_explicitly_disabled_across_labels_and_parse_calls() {
    let mut parser = ZplParser::new();
    let source = format!("^XA^PMY{FIELD}^XZ^XA{FIELD}^XZ^XA^PMN{FIELD}^XZ^XA{FIELD}^XZ");
    let parsed = parser.parse(source.as_bytes()).unwrap();
    assert_eq!(parsed.len(), 4);
    for (label, x) in parsed.iter().zip([173, 173, 10, 10]) {
        assert_eq!(bounds(&render(label, options())).0, x);
    }
    parser.parse(b"^XA^PMY^XZ").unwrap();
    let next = parser.parse(format!("^XA{FIELD}^XZ").as_bytes()).unwrap();
    assert_eq!(bounds(&render(&next[0], options())).0, 173);
    let fresh = labels(&format!("^XA{FIELD}^XZ"));
    assert_eq!(bounds(&render(&fresh[0], options())).0, 10);
}

#[test]
fn missing_and_invalid_parameters_leave_printer_state_unchanged() {
    for enabled in [false, true] {
        for command in ["^PM", "^PMX", "^PM1", "^PMYES"] {
            let initial = if enabled { "^PMY" } else { "^PMN" };
            let parsed = labels(&format!("^XA{initial}{command}{FIELD}^XZ"));
            assert_eq!(
                bounds(&render(&parsed[0], options())).0,
                if enabled { 173 } else { 10 },
                "{initial}{command}"
            );
        }
    }
}

#[test]
fn last_mirror_command_applies_to_the_entire_label_and_all_copies() {
    for commands in ["^PMY", "^PMN^PMY"] {
        let parsed = labels(&format!("^XA{FIELD}{commands}^PQ3^XZ"));
        assert_eq!(parsed.len(), 3);
        for label in parsed {
            assert_eq!(bounds(&render(&label, options())).0, 173);
        }
    }
    let parsed = labels(&format!("^XA^PMY{FIELD}^PMN^XZ"));
    assert_eq!(bounds(&render(&parsed[0], options())).0, 10);
}

#[test]
fn mirror_flips_all_pixels_including_rotated_fields_barcodes_and_clipping() {
    let fields = "^FO10,12^A0R,18,14^FDF7>^FS^FO60,30^BQN,2,1^FDQA,ABC^FS^FO95,70^GB20,10,10^FS";
    for width in [100, 101, 203] {
        for orientation in ["^PON", "^POI"] {
            for antialias in [false, true] {
                let source = format!("^XA^PW{width}{orientation}{fields}^XZ");
                let mirrored_source = source.replace("^XZ", "^PMY^XZ");
                let mut opts = options();
                opts.antialias = antialias;
                let normal = render(&labels(&source)[0], opts.clone());
                let mirrored = render(&labels(&mirrored_source)[0], opts);
                assert!(
                    mirrored.as_raw() == image::imageops::flip_horizontal(&normal).as_raw(),
                    "width={width}, {orientation}, antialias={antialias}"
                );
            }
        }
    }
}

#[test]
fn disabling_inverted_labels_does_not_disable_mirroring() {
    let parsed = labels(&format!("^XA^PW100^PMY^POI{FIELD}^XZ"));
    let mut opts = options();
    opts.enable_inverted_labels = false;
    assert_eq!(bounds(&render(&parsed[0], opts)), (122, 20, 141, 29));
}

#[test]
fn stored_formats_replay_explicit_mirror_settings_without_capturing_defaults() {
    for (source, expected) in [
        (
            format!("^XA^DFR:M.ZPL^PMY{FIELD}^XZ^XA{FIELD}^XZ"),
            vec![10],
        ),
        (
            format!("^XA^DFR:M.ZPL^PMY{FIELD}^XZ^XA^PMN^XFR:M.ZPL^FS^XZ^XA{FIELD}^XZ"),
            vec![173, 173],
        ),
        (
            format!("^XA^DFR:M.ZPL^PMY{FIELD}^XZ^XA^XFR:M.ZPL^FS^PMN^XZ"),
            vec![10],
        ),
        (
            format!("^XA^PMY{FIELD}^XZ^XA^DFR:M.ZPL{FIELD}^XZ^XA^PMN^XFR:M.ZPL^FS^XZ"),
            vec![173, 10],
        ),
        (
            format!("^XA^DFR:M.ZPL{FIELD}^XZ^XA^PMY^XFR:M.ZPL^FS^XZ"),
            vec![173],
        ),
    ] {
        let parsed = labels(&source);
        assert_eq!(parsed.len(), expected.len());
        for (label, x) in parsed.iter().zip(expected) {
            assert_eq!(bounds(&render(label, options())).0, x, "{source}");
        }
    }
}

#[test]
fn stored_setting_only_formats_and_explicit_resets_replay_in_order() {
    let source = format!(
        "^XA^DFR:ON.ZPL^PMY^XZ^XA^DFR:OFF.ZPL^PMN^XZ\
         ^XA^XFR:ON.ZPL^FS{FIELD}^XZ\
         ^XA^XFR:ON.ZPL^FS^XFR:OFF.ZPL^FS{FIELD}^XZ\
         ^XA^PMY^DFR:BEFORE.ZPL{FIELD}^XZ^XA{FIELD}^XZ"
    );
    let parsed = labels(&source);
    assert_eq!(parsed.len(), 3);
    for (label, x) in parsed.iter().zip([173, 10, 10]) {
        assert_eq!(bounds(&render(label, options())).0, x);
    }
}

#[test]
fn labelary_graphic_fixtures_match_pixel_for_pixel() {
    for name in [
        "print_mirror",
        "print_mirror_width",
        "print_mirror_inverted",
    ] {
        let source = std::fs::read_to_string(format!("testdata/unit/{name}.zpl")).unwrap();
        let reference = image::open(format!("testdata/unit/{name}.png"))
            .unwrap()
            .to_rgba8();
        let opts = DrawerOptions {
            label_width_mm: 101.625,
            label_height_mm: 203.25,
            dpmm: 8,
            ..Default::default()
        };
        let actual = render(&labels(&source)[0], opts);
        assert_eq!(actual.dimensions(), reference.dimensions(), "{name}");
        assert!(
            actual.as_raw() == reference.as_raw(),
            "{name}: pixels differ from Labelary"
        );
    }
}
