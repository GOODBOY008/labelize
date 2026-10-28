//! Persisted reverse-rendering fixtures. Expected pixels come from Labelary
//! and are checked against an independent raster oracle when captured.
//! Ordinary tests never generate or replace references and run offline.

use crate::common::{image_compare, render_helpers};
use labelize::{
    encode_image_bytes, DrawerOptions, ReverseLanguage, ReverseOptions, TsplParser, ZplParser,
};

pub struct Case {
    pub name: &'static str,
    source: &'static str,
    width: u32,
    height: u32,
    dpmm: i32,
    threshold: u8,
}

pub const CASES: &[Case] = &[
    Case {
        name: "shipping",
        source: "labels/amazon.png",
        width: 813,
        height: 1626,
        dpmm: 8,
        threshold: 128,
    },
    Case {
        name: "rgba",
        source: "reverse/rgba_input.png",
        width: 205,
        height: 137,
        dpmm: 8,
        threshold: 128,
    },
    Case {
        name: "threshold",
        source: "reverse/rgba_input.png",
        width: 205,
        height: 137,
        dpmm: 12,
        threshold: 200,
    },
    Case {
        name: "field_block",
        source: "unit/fb_single_line_overflow.png",
        width: 813,
        height: 1626,
        dpmm: 8,
        threshold: 128,
    },
];

pub fn compare(case: &Case, language: ReverseLanguage) -> image_compare::CompareResult {
    let root = render_helpers::testdata_dir();
    let options = ReverseOptions {
        dpmm: case.dpmm,
        threshold: case.threshold,
    };
    let input = std::fs::read(root.join(case.source)).expect("read reverse input fixture");
    let code = encode_image_bytes(&input, &options, language).expect("encode reverse fixture");
    let snapshot =
        root.join("reverse")
            .join(format!("{}.{}", case.name, language.file_extension()));
    let expected_code = std::fs::read(&snapshot).expect("read committed printer-code snapshot");
    // Do not dump hundreds of kilobytes of binary payload into a failed CI log.
    assert!(code == expected_code,
        "{} changed: generated {} bytes, expected {}; first differing byte {:?}. Review and explicitly regenerate the fixtures.",
        snapshot.display(), code.len(), expected_code.len(),
        code.iter().zip(&expected_code).position(|(a, b)| a != b));

    let labels = match language {
        ReverseLanguage::Zpl => ZplParser::new().parse(&code),
        ReverseLanguage::Tspl => TsplParser::new().parse(&code),
    }
    .expect("parse freshly encoded fixture");
    assert_eq!(labels.len(), 1);
    let label = &labels[0];
    let (width_mm, height_mm) = match language {
        ReverseLanguage::Zpl => (
            case.width as f64 / case.dpmm as f64,
            case.height as f64 / case.dpmm as f64,
        ),
        ReverseLanguage::Tspl => (
            label.label_width_mm.expect("TSPL SIZE width"),
            label.label_height_mm.expect("TSPL SIZE height"),
        ),
    };
    let actual = render_helpers::render_label_to_png(
        label,
        DrawerOptions {
            label_width_mm: width_mm,
            label_height_mm: height_mm,
            dpmm: case.dpmm,
            ..Default::default()
        },
    );
    let expected = std::fs::read(root.join("reverse").join(format!("{}.png", case.name)))
        .expect("read committed Labelary reference; run testdata/reverse/regenerate.py explicitly if missing");
    image_compare::compare_images(&actual, &expected, 0.0)
}
