//! Reproducible all-label corpus export; see e2e/vector/README.md.
use labelize::{parse_labels, DrawerOptions, LabelScene, Language, ScenePngOptions};
use serde_json::json;
use std::{fs, io::Cursor, path::PathBuf, time::Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "output/pdf/vector-audit".into()),
    );
    fs::create_dir_all(&out)?;
    let mut results = Vec::new();
    let mut failures = Vec::new();
    let mut excluded = Vec::new();
    for (dir, input_dir) in [
        ("labels", "testdata/labels"),
        ("unit", "testdata/unit"),
        ("regression", "testdata/vector/fixtures/regression"),
    ] {
        let mut paths = fs::read_dir(input_dir)?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !["zpl", "epl", "tspl", "cpcl", "dpl", "clp"].contains(&ext) {
                continue;
            }
            if ext == "clp" {
                excluded.push(json!({"source":path,"reason":"CLP is not one of the five supported input languages; existing parser produces no labels"}));
                continue;
            }
            let data = fs::read(&path)?;
            let language =
                Language::from_name(ext).unwrap_or_else(|| labelize::detect_language(&data));
            let labels = match parse_labels(language, &data, 8) {
                Ok(labels) => labels,
                Err(e) => {
                    failures.push(json!({"source":path,"stage":"parse","error":e.to_string()}));
                    continue;
                }
            };
            if labels.is_empty() {
                failures.push(json!({"source":path,"stage":"parse","error":"empty labels"}));
            }
            for (index, label) in labels.iter().enumerate() {
                let name = format!(
                    "{dir}-{}-{:03}",
                    path.file_stem().unwrap().to_string_lossy(),
                    index + 1
                );
                let options = DrawerOptions {
                    label_width_mm: label.label_width_mm.unwrap_or(if dir == "labels" {
                        101.625
                    } else {
                        101.5
                    }),
                    label_height_mm: label.label_height_mm.unwrap_or(if dir == "labels" {
                        203.25
                    } else {
                        203.0
                    }),
                    ..Default::default()
                };
                let exported = (|| -> Result<serde_json::Value, Box<dyn std::error::Error>> {
                    let start = Instant::now();
                    let mut reference = Cursor::new(Vec::new());
                    labelize::Renderer::new().draw_label_as_png(
                        label,
                        &mut reference,
                        options.clone(),
                    )?;
                    let legacy_ms = start.elapsed().as_secs_f64() * 1000.0;
                    fs::write(out.join(format!("{name}-legacy.png")), reference.get_ref())?;
                    let legacy_pdf_start = Instant::now();
                    let image = image::load_from_memory(reference.get_ref())?.to_rgba8();
                    let mut legacy_pdf = Cursor::new(Vec::new());
                    labelize::encode_pdf(&image, &options, &mut legacy_pdf)?;
                    let legacy_pdf_ms =
                        legacy_ms + legacy_pdf_start.elapsed().as_secs_f64() * 1000.0;
                    fs::write(out.join(format!("{name}-legacy.pdf")), legacy_pdf.get_ref())?;
                    let start = Instant::now();
                    let scene = LabelScene::new(label, options)?;
                    let layout_ms = start.elapsed().as_secs_f64() * 1000.0;
                    let start = Instant::now();
                    let pdf = scene.to_pdf()?;
                    let pdf_ms = start.elapsed().as_secs_f64() * 1000.0;
                    fs::write(out.join(format!("{name}.pdf")), &pdf)?;
                    let start = Instant::now();
                    let png = scene.to_png(ScenePngOptions::default())?;
                    let png_ms = start.elapsed().as_secs_f64() * 1000.0;
                    fs::write(out.join(format!("{name}.png")), &png)?;
                    fs::write(
                        out.join(format!("{name}-3x.png")),
                        scene.to_png(ScenePngOptions {
                            scale: 3.0,
                            antialias: true,
                        })?,
                    )?;
                    Ok(
                        json!({"name":name,"source":path,"label":index,"labels":labels.len(),
                        "language":language.name(),"dimensions":scene.dimensions(),"size_mm":scene.size_mm(),
                        "stats":scene.stats(),"legacy_png_bytes":reference.get_ref().len(),
                        "legacy_pdf_bytes":legacy_pdf.get_ref().len(),"pdf_bytes":pdf.len(),"png_bytes":png.len(),
                        "legacy_ms":legacy_ms,"legacy_pdf_ms":legacy_pdf_ms,"layout_ms":layout_ms,"pdf_ms":pdf_ms,"png_ms":png_ms}),
                    )
                })();
                match exported {
                    Ok(record) => {
                        eprintln!("{name}");
                        results.push(record);
                    }
                    Err(e) => failures.push(
                        json!({"source":path,"label":index,"stage":"render","error":e.to_string()}),
                    ),
                }
            }
        }
    }
    fs::write(
        out.join("manifest.json"),
        serde_json::to_vec_pretty(
            &json!({"pages":results,"failures":failures,"excluded":excluded}),
        )?,
    )?;
    eprintln!(
        "Exported {} pages; {} failures",
        results.len(),
        failures.len()
    );
    if !failures.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
