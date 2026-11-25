//! Studio printer byte fixtures and the application-facing fill/preview workflow.
use labelize::{Language, PrintTemplate, TemplateValues};
use std::{fs, path::PathBuf};

const LANGUAGES: [Language; 5] = [
    Language::Zpl,
    Language::Tspl,
    Language::Epl,
    Language::Cpcl,
    Language::Dpl,
];

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/templates")
        .join(name)
}

fn printer(stem: &str, language: Language) -> PathBuf {
    fixture(&format!("printers/{stem}.{}", language.name()))
}

fn record(name: &str) -> TemplateValues {
    serde_json::from_slice(&fs::read(fixture(&format!("{name}.json"))).unwrap()).unwrap()
}

#[test]
fn repeated_fills_match_the_reference_printer_jobs() {
    for language in LANGUAGES {
        let bytes = fs::read(printer("shipping.tpl", language)).unwrap();
        let template = PrintTemplate::load(language, &bytes).unwrap();
        let fields = template.fields();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "order_no");
        assert_eq!(
            fields[0]
                .uses
                .iter()
                .map(|u| (u.object, u.kind.as_str()))
                .collect::<Vec<_>>(),
            [(1, "text"), (3, "code128"), (4, "qr")]
        );
        assert_eq!(fields[1].name, "product");
        assert_eq!(fields[1].uses[0].object, 2);
        for name in ["first", "second"] {
            let values = record(name);
            assert!(template.validate(&values).is_empty());
            assert_eq!(
                template.fill(&values).unwrap(),
                fs::read(printer(name, language)).unwrap(),
                "{language:?} {name}"
            );
        }
    }
}

#[test]
fn validation_identifies_missing_unencodable_and_bad_check_digit_fields() {
    let tspl = PrintTemplate::load(
        Language::Tspl,
        &fs::read(printer("shipping.tpl", Language::Tspl)).unwrap(),
    )
    .unwrap();
    let mut values = TemplateValues::new();
    values.insert("product".into(), "保温杯".into());
    let problems = tspl.validate(&values);
    assert_eq!(problems.len(), 4);
    for (index, (problem, field)) in problems
        .iter()
        .zip(["order_no", "product", "order_no", "order_no"])
        .enumerate()
    {
        assert_eq!(problem.usage.object, index + 1);
        assert_eq!(problem.fields, [field]);
    }
    assert!(tspl.fill(&values).is_err());

    let ean = PrintTemplate::load(
        Language::Zpl,
        b"^XA\n^FXLabelize field ean13^FS\n^FO8,8^BEN,40,N,N^FH^FD{{gtin}}^FS\n^XZ\n",
    )
    .unwrap();
    let mut values = TemplateValues::new();
    values.insert("gtin".into(), "5901234123458".into());
    let errors = ean.validate(&values);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].fields, ["gtin"]);
    assert_eq!(errors[0].usage.kind, "ean13");
    assert!(errors[0].message.to_lowercase().contains("check digit"));
    values.insert("gtin".into(), "5901234123457".into());
    assert!(ean.validate(&values).is_empty());
}

#[cfg(feature = "cli")]
mod cli {
    use super::*;
    use std::{path::Path, process::Command};

    fn template_command(operation: &str, input: &Path, data: &Path, output: &Path) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_labelize"));
        cmd.args(["template", operation])
            .arg(input)
            .arg("--data")
            .arg(data)
            .arg("-o")
            .arg(output);
        cmd
    }

    #[test]
    fn cli_failure_preserves_previous_job_and_preview() {
        let tmp = tempfile::tempdir().unwrap();
        let input = printer("shipping.tpl", Language::Epl);
        let data = tmp.path().join("order.json");
        let output = tmp.path().join("previous.png");
        fs::write(&output, b"previous output").unwrap();
        for (operation, json, diagnostic) in [
            ("fill", r#"{"product":"ok"}"#, "order_no"),
            (
                "render",
                r#"{"order_no":"000123","product":"保温杯"}"#,
                "product",
            ),
            ("fill", r#"{"order_no":123,"product":"ok"}"#, "string"),
            ("render", r#"["000123"]"#, "object"),
        ] {
            fs::write(&data, json).unwrap();
            let result = template_command(operation, &input, &data, &output)
                .output()
                .unwrap();
            assert!(!result.status.success());
            let stderr = String::from_utf8_lossy(&result.stderr);
            assert!(stderr.contains(diagnostic), "{operation}: {stderr}");
            assert_eq!(fs::read(&output).unwrap(), b"previous output");
        }
        let result = template_command("render", &input, &fixture("first.json"), &output)
            .args(["--dpmm", "0"])
            .output()
            .unwrap();
        assert!(!result.status.success());
        assert_eq!(fs::read(&output).unwrap(), b"previous output");
    }

    #[cfg(feature = "vector")]
    fn run(cmd: &mut Command) {
        let result = cmd.output().unwrap();
        assert!(
            result.status.success(),
            "{cmd:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[cfg(feature = "vector")]
    #[test]
    fn utf8_device_profiles_print_and_preview_the_chinese_product_name() {
        let tmp = tempfile::tempdir().unwrap();
        let values = record("utf8");
        let product = &values["product"];
        let json = fixture("utf8.json");
        let size = ["--width", "100", "--height", "50", "--dpmm", "8"];
        for language in [Language::Tspl, Language::Cpcl] {
            let input = printer("utf8.tpl", language);
            let template = PrintTemplate::load(language, &fs::read(&input).unwrap()).unwrap();
            let bytes = template.fill(&values).unwrap();
            assert!(bytes
                .windows(product.len())
                .any(|v| v == product.as_bytes()));
            let native = tmp.path().join(format!("filled.{}", language.name()));
            fs::write(&native, bytes).unwrap();

            let preview = tmp.path().join("order.pdf");
            let converted = tmp.path().join("converted.pdf");
            run(template_command("render", &input, &json, &preview).args(size));
            run(Command::new(env!("CARGO_BIN_EXE_labelize"))
                .arg("convert")
                .arg(&native)
                .args(size)
                .args(["-t", "pdf", "-o"])
                .arg(&converted));
            assert!(
                fs::read(&preview).unwrap() == fs::read(converted).unwrap(),
                "default PDF: {language:?}"
            );

            run(template_command("render", &input, &json, &preview)
                .args(size)
                .args(["--mode", "vector"]));
            let pdf = lopdf::Document::load(preview).unwrap();
            let text = pdf.extract_text(&[1]).unwrap().replace('\n', "");
            assert!(
                text.contains("Order: 000123") && text.contains(product),
                "{language:?}: {text:?}"
            );

            let png = tmp.path().join("order.png");
            run(template_command("render", &input, &json, &png).args(size));
            let page = image::open(png).unwrap().to_luma8();
            // Both exported device fonts use 24-dot square cells at this resolution.
            let glyphs =
                [32, 56, 80].map(|x| image::imageops::crop_imm(&page, x, 112, 24, 24).to_image());
            assert!(glyphs.iter().all(|glyph| glyph.as_raw().contains(&0)));
            assert!(
                glyphs[0] != glyphs[1] && glyphs[1] != glyphs[2] && glyphs[0] != glyphs[2],
                "{language:?}: expected three distinct product glyphs"
            );
        }
    }

    #[cfg(feature = "vector")]
    #[test]
    fn cli_previews_match_native_jobs_and_scan_at_both_resolutions() {
        use rxing::{helpers::detect_in_luma_slice, BarcodeFormat};

        let tmp = tempfile::tempdir().unwrap();
        for language in LANGUAGES {
            let input = printer("shipping.tpl", language);
            let native = tmp.path().join(format!("filled.{}", language.name()));
            let mut previous_titles = std::collections::BTreeMap::new();
            for name in ["first", "second"] {
                let json = fixture(&format!("{name}.json"));
                let values = record(name);
                run(&mut template_command("fill", &input, &json, &native));
                assert_eq!(
                    fs::read(&native).unwrap(),
                    fs::read(printer(name, language)).unwrap()
                );
                for dpmm in [8, 12] {
                    let density = dpmm.to_string();
                    let size = ["--width", "100", "--height", "50", "--dpmm", &density];
                    for (extension, mode) in [("png", "raster"), ("pdf", "vector")] {
                        let direct = tmp.path().join(format!("preview.{extension}"));
                        let ordinary = tmp.path().join(format!("converted.{extension}"));
                        run(template_command("render", &input, &json, &direct)
                            .args(size)
                            .args(["--mode", mode]));
                        run(Command::new(env!("CARGO_BIN_EXE_labelize"))
                            .arg("convert")
                            .arg(&native)
                            .args(size)
                            .args(["--mode", mode, "-t", extension])
                            .arg("-o")
                            .arg(&ordinary));
                        let bytes = fs::read(&direct).unwrap();
                        let context = format!("{language:?} {name} {dpmm}dpmm {extension}");
                        assert!(
                            bytes == fs::read(&ordinary).unwrap(),
                            "preview differs from convert: {context}"
                        );
                        if extension == "pdf" {
                            let pdf = lopdf::Document::load_mem(&bytes).unwrap();
                            let text = pdf.extract_text(&[1]).unwrap().replace('\n', "");
                            let order = format!("Order: {}", values["order_no"]);
                            assert!(text.contains(&order), "{context}: {text:?}");
                            assert!(text.contains(&values["product"]), "{context}: {text:?}");
                        } else {
                            let page = image::load_from_memory(&bytes).unwrap().to_luma8();
                            assert_eq!(page.dimensions(), (100 * dpmm, 50 * dpmm), "{context}");
                            for format in [BarcodeFormat::CODE_128, BarcodeFormat::QR_CODE] {
                                let barcode = detect_in_luma_slice(
                                    page.as_raw(),
                                    page.width(),
                                    page.height(),
                                    Some(format),
                                )
                                .unwrap_or_else(|e| panic!("{context} {format:?}: {e:?}"));
                                assert_eq!(
                                    barcode.getText(),
                                    values["order_no"],
                                    "{context} {format:?}"
                                );
                            }
                            // Only the visible order title varies in these top 100 rows;
                            // all product fields and symbols start below this band.
                            let title = image::imageops::crop_imm(&page, 0, 0, page.width(), 100)
                                .to_image();
                            if let Some(first) = previous_titles.insert(dpmm, title.clone()) {
                                assert!(first != title, "order title did not change: {context}");
                            }
                        }
                    }
                }
            }
        }
    }
}
