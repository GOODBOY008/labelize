//! Fixed vector-output baselines. Normal tests never create or replace references.
#![allow(dead_code)] // Shared by the library golden and process e2e test targets.

use labelize::{Language, OutputMode, OutputOptions, RenderOptions};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn cases() -> Vec<Value> {
    serde_json::from_str(include_str!("../../testdata/vector/cases.json")).unwrap()
}

pub fn name(case: &Value) -> &str {
    case["name"].as_str().unwrap()
}

pub fn source(case: &Value) -> PathBuf {
    root().join(case["source"].as_str().unwrap())
}

pub fn options(case: &Value, antialias: bool) -> RenderOptions {
    RenderOptions {
        language: Some(Language::from_name(case["language"].as_str().unwrap()).unwrap()),
        width_mm: case["size_mm"][0].as_f64(),
        height_mm: case["size_mm"][1].as_f64(),
        dpmm: 8,
        antialias,
    }
}

pub fn output(scale: f64) -> OutputOptions {
    OutputOptions {
        mode: OutputMode::Vector,
        scale,
        all_pages: false,
    }
}

pub fn artifact_dir() -> PathBuf {
    let path = root().join("output/vector-golden");
    std::fs::create_dir_all(&path).unwrap();
    path
}

pub fn assert_png(actual: &[u8], expected: &Path, context: &str) {
    let actual = image::load_from_memory(actual).unwrap().to_luma8();
    let expected = image::open(expected)
        .unwrap_or_else(|e| panic!("{}: {e}; see testdata/vector/README.md", expected.display()))
        .to_luma8();
    assert_eq!(actual.dimensions(), expected.dimensions(), "{context}");
    let changed = actual
        .as_raw()
        .iter()
        .zip(expected.as_raw())
        .filter(|(a, b)| a != b)
        .count();
    assert_eq!(changed, 0, "{context}: {changed} golden pixels changed");
}

pub fn golden_png(case: &Value, scale: u32) -> PathBuf {
    root().join(format!(
        "testdata/vector/golden/{}-{scale}x.png",
        name(case)
    ))
}

pub fn assert_pdf(bytes: &[u8], pages: &[(f64, f64, &str)]) {
    let pdf = lopdf::Document::load_mem(bytes).unwrap();
    assert_eq!(pdf.get_pages().len(), pages.len());
    for ((number, id), &(width, height, text)) in pdf.get_pages().iter().zip(pages) {
        let bounds = pdf
            .get_dictionary(*id)
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap();
        for (bound, mm) in bounds[2..].iter().zip([width, height]) {
            assert!((bound.as_float().unwrap() as f64 - mm * 72.0 / 25.4).abs() < 0.001);
        }
        let extracted = pdf.extract_text(&[*number]).unwrap().replace('\n', "");
        assert!(
            extracted.contains(text),
            "page {number}: missing {text:?} in {extracted:?}"
        );
    }
}

pub fn export(case: &Value) {
    let data = std::fs::read(source(case)).unwrap();
    for scale in [1, 3] {
        let png = labelize::render_png_with_output(
            &data,
            options(case, scale == 3),
            output(scale as f64),
        )
        .unwrap();
        assert_eq!(
            png.labels,
            1,
            "{} must be a single label fixture",
            name(case)
        );
        let image = image::load_from_memory(&png.bytes).unwrap();
        assert_eq!(
            (image.width(), image.height()),
            (
                case["dimensions"][0].as_u64().unwrap() as u32 * scale,
                case["dimensions"][1].as_u64().unwrap() as u32 * scale,
            )
        );
        std::fs::write(
            artifact_dir().join(format!("{}-{scale}x.png", name(case))),
            &png.bytes,
        )
        .unwrap();
    }
    let pdf = labelize::render_pdf_with_output(&data, options(case, false), output(1.0)).unwrap();
    assert_pdf(
        &pdf.bytes,
        &[(
            case["size_mm"][0].as_f64().unwrap(),
            case["size_mm"][1].as_f64().unwrap(),
            "",
        )],
    );
    std::fs::write(
        artifact_dir().join(format!("{}.pdf", name(case))),
        &pdf.bytes,
    )
    .unwrap();
    if case["native_probe"] == true {
        // Negative control: the same source exported as a raster PDF. The PDF
        // structure checker must reject it even if searchable text is added.
        let raster =
            labelize::render_pdf_with_output(&data, options(case, false), OutputOptions::default())
                .unwrap();
        std::fs::write(
            artifact_dir().join("native-content-raster.pdf"),
            raster.bytes,
        )
        .unwrap();
    }
}

pub fn compare(index: usize) {
    let case = &cases()[index];
    export(case);
    for scale in [1, 3] {
        let actual =
            std::fs::read(artifact_dir().join(format!("{}-{scale}x.png", name(case)))).unwrap();
        assert_png(&actual, &golden_png(case, scale), name(case));
    }
}
