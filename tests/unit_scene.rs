#![cfg(feature = "vector")]

use labelize::{
    encode_scenes_pdf, parse_labels, DrawerOptions, LabelScene, Language, ScenePngOptions,
};

fn scene(source: &str, width: f64, height: f64) -> LabelScene {
    let label = parse_labels(Language::Zpl, source.as_bytes(), 8)
        .unwrap()
        .remove(0);
    LabelScene::new(
        &label,
        DrawerOptions {
            label_width_mm: width,
            label_height_mm: height,
            ..Default::default()
        },
    )
    .unwrap()
}

fn mapped_text(pdf: &lopdf::Document) -> String {
    // lopdf adds a newline at every ET, including separately positioned
    // glyphs on the same baseline. Keep spaces and all Unicode characters;
    // logical line extraction is also checked with PDFium and Poppler by
    // e2e/vector/controls.py.
    pdf.extract_text(&[1]).unwrap().replace('\n', "")
}

#[test]
fn pdf_contains_subset_font_and_extractable_unicode_text() {
    let scene = scene(
        "^XA^CI28^FO20,20^A0N,30,26^FDCafé Straße 123^FS^XZ",
        80.0,
        50.0,
    );
    assert_eq!(scene.stats().images, 0);
    let bytes = scene.to_pdf().unwrap();
    let pdf = lopdf::Document::load_mem(&bytes).unwrap();
    let text = mapped_text(&pdf);
    assert!(text.contains("Café Straße 123"), "{text:?}");
    // The prior full-font prototype exceeds 24 KB for this font alone.
    assert!(bytes.len() < 20_000, "{} bytes", bytes.len());
    assert_eq!(pdf.get_pages().len(), 1);
}

#[test]
fn blocks_and_all_rotations_have_native_glyphs() {
    for orientation in ["N", "R", "I", "B"] {
        let source = format!(
            "^XA^FO160,160^A0{orientation},26,24^FB180,3,0,L,0^FDOne two three four five^FS^XZ"
        );
        let scene = scene(&source, 80.0, 80.0);
        assert!(scene.stats().glyphs > 10);
        assert_eq!(scene.stats().images, 0);
        assert!(scene.stats().fallbacks.is_empty());
        let pdf = lopdf::Document::load_mem(&scene.to_pdf().unwrap()).unwrap();
        assert!(mapped_text(&pdf).contains("One two"));
        let png = scene
            .to_png(ScenePngOptions {
                scale: 3.0,
                antialias: true,
            })
            .unwrap();
        assert_eq!(image::load_from_memory(&png).unwrap().width(), 1920);
    }
}

#[test]
fn png_scaling_writes_physical_resolution_and_rejects_unbounded_allocations() {
    let scene = scene("^XA^FO20,20^GB80,60,60^FS^XZ", 50.0, 40.0);
    let png = scene
        .to_png(ScenePngOptions {
            scale: 3.0,
            antialias: false,
        })
        .unwrap();
    let decoder = png::Decoder::new(std::io::Cursor::new(png));
    let reader = decoder.read_info().unwrap();
    assert_eq!((reader.info().width, reader.info().height), (1200, 960));
    let dims = reader.info().pixel_dims.unwrap();
    assert_eq!(dims.xppu, 24000);
    assert_eq!(dims.yppu, 24000);
    assert_eq!(dims.unit, png::Unit::Meter);
    for scale in [0.0, -1.0, f64::NAN, f64::INFINITY, 1e100] {
        assert!(scene
            .to_png(ScenePngOptions {
                scale,
                antialias: true
            })
            .is_err());
    }
}

#[test]
fn multipage_pdf_preserves_individual_page_sizes() {
    let a = scene("^XA^FO20,20^FDPage A^FS^XZ", 80.0, 50.0);
    let b = scene("^XA^FO20,20^FDPage B^FS^XZ", 40.0, 90.0);
    let bytes = encode_scenes_pdf(&[a, b]).unwrap();
    let pdf = lopdf::Document::load_mem(&bytes).unwrap();
    assert_eq!(pdf.get_pages().len(), 2);
    for ((_, id), (w, h)) in pdf.get_pages().iter().zip([(80.0, 50.0), (40.0, 90.0)]) {
        let bounds = pdf
            .get_dictionary(*id)
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap();
        assert!((bounds[2].as_float().unwrap() as f64 - w * 72.0 / 25.4).abs() < 0.001);
        assert!((bounds[3].as_float().unwrap() as f64 - h * 72.0 / 25.4).abs() < 0.001);
    }
    assert!(encode_scenes_pdf(&[]).is_err());
}

#[test]
fn pdf_page_boundaries_do_not_add_a_device_pixel_at_printer_resolution() {
    for (width, height) in [
        (100.0, 150.0),
        (50.0, 50.0),
        (40.0, 90.0),
        (101.625, 203.25),
    ] {
        let scene = scene("^XA^FO20,20^GB30,20,20^FS^XZ", width, height);
        let pdf = lopdf::Document::load_mem(&scene.to_pdf().unwrap()).unwrap();
        let page = *pdf.get_pages().get(&1).unwrap();
        let bounds = pdf
            .get_dictionary(page)
            .unwrap()
            .get(b"MediaBox")
            .unwrap()
            .as_array()
            .unwrap();
        for (point, dots) in [
            (&bounds[2], scene.dimensions().0),
            (&bounds[3], scene.dimensions().1),
        ] {
            for scale in [1.0, 3.0] {
                let pixels = (point.as_float().unwrap() as f64 * 8.0 * 25.4 / 72.0 * scale).ceil();
                assert_eq!(
                    pixels,
                    dots as f64 * scale,
                    "{width} x {height} mm at {scale}x"
                );
            }
        }
    }
}

#[test]
fn binary_compositing_avoids_pdf_image_resampling() {
    let label = parse_labels(
        Language::Zpl,
        b"^XA^FO20,20^GB100,50,50^FS^FO25,25^FR^A0N,25,20^FD00^FS^XZ",
        8,
    )
    .unwrap()
    .remove(0);
    let scene = LabelScene::new(
        &label,
        DrawerOptions {
            label_width_mm: 40.0,
            label_height_mm: 30.0,
            antialias: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!scene.stats().fallbacks.is_empty());
    let pdf = lopdf::Document::load_mem(&scene.to_pdf().unwrap()).unwrap();
    let has_image = pdf.objects.values().any(|object| {
        object
            .as_stream()
            .ok()
            .and_then(|s| s.dict.get(b"Subtype").ok())
            .and_then(|s| s.as_name().ok())
            == Some(b"Image")
    });
    assert!(!has_image);
}

#[test]
fn repeated_opaque_diagonal_does_not_darken_antialiased_edges() {
    let draw = |source: &[u8]| {
        let label = parse_labels(Language::Epl, source, 8).unwrap().remove(0);
        LabelScene::new(
            &label,
            DrawerOptions {
                label_width_mm: 30.0,
                label_height_mm: 20.0,
                ..Default::default()
            },
        )
        .unwrap()
        .to_png(ScenePngOptions {
            scale: 3.0,
            antialias: true,
        })
        .unwrap()
    };
    assert_eq!(
        draw(b"N\nLS20,20,2,180,30\nP1\n"),
        draw(b"N\nLS20,20,2,180,30\nLS20,20,2,180,30\nP1\n")
    );
}

#[test]
fn one_dot_diagonals_keep_ink_along_their_full_length() {
    for (x0, y0, x1, y1) in [
        (20_u32, 20_u32, 320_u32, 23_u32),
        (20, 23, 320, 20),
        (20, 20, 23, 320),
        (23, 20, 20, 320),
        (20, 20, 320, 320),
        (20, 320, 320, 20),
    ] {
        let source =
            format!("! 0 203 203 400 1\nPAGE-WIDTH 400\nLINE {x0} {y0} {x1} {y1} 1\nPRINT\n");
        let label = parse_labels(Language::Cpcl, source.as_bytes(), 8)
            .unwrap()
            .remove(0);
        let scene = LabelScene::new(
            &label,
            DrawerOptions {
                label_width_mm: 50.0,
                label_height_mm: 50.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(scene.stats().images, 0);
        for scale in [1, 3] {
            let image = image::load_from_memory(
                &scene
                    .to_png(ScenePngOptions {
                        scale: scale as f64,
                        antialias: false,
                    })
                    .unwrap(),
            )
            .unwrap()
            .to_luma8();
            let horizontal = x0.abs_diff(x1) >= y0.abs_diff(y1);
            let (start, end) = if horizontal { (x0, x1) } else { (y0, y1) };
            for major in start.min(end) * scale..(start.max(end) + 1) * scale {
                assert!(
                    (0..400 * scale).any(|minor| {
                        let (x, y) = if horizontal {
                            (major, minor)
                        } else {
                            (minor, major)
                        };
                        image.get_pixel(x, y)[0] == 0
                    }),
                    "missing ink at {major}, scale {scale}: {source}"
                );
            }
        }
    }
}

#[test]
fn shallow_multi_dot_diagonals_preserve_raster_ink() {
    for thickness in [2, 3, 8] {
        for rise in [1, 3, 20] {
            for reverse in [false, true] {
                let (y0, y1) = if reverse {
                    (20 + rise, 20)
                } else {
                    (20, 20 + rise)
                };
                let source = format!(
                    "! 0 203 203 100 1\nPAGE-WIDTH 400\nLINE 20 {y0} 320 {y1} {thickness}\nPRINT\n"
                );
                let label = parse_labels(Language::Cpcl, source.as_bytes(), 8)
                    .unwrap()
                    .remove(0);
                let options = DrawerOptions {
                    label_width_mm: 50.0,
                    label_height_mm: 12.5,
                    ..Default::default()
                };
                let mut legacy = Vec::new();
                labelize::Renderer::new()
                    .draw_label_as_png(&label, &mut legacy, options.clone())
                    .unwrap();
                let legacy = image::load_from_memory(&legacy).unwrap().to_luma8();
                assert!(legacy.pixels().filter(|p| p[0] == 0).count() >= 300);
                let scene = LabelScene::new(&label, options).unwrap();
                for scale in [1, 3] {
                    let expected = image::imageops::resize(
                        &legacy,
                        400 * scale,
                        100 * scale,
                        image::imageops::FilterType::Nearest,
                    );
                    let actual = image::load_from_memory(
                        &scene
                            .to_png(ScenePngOptions {
                                scale: scale as f64,
                                antialias: false,
                            })
                            .unwrap(),
                    )
                    .unwrap()
                    .to_luma8();
                    let differences = actual
                        .pixels()
                        .zip(expected.pixels())
                        .filter(|(a, b)| a != b)
                        .count();
                    assert_eq!(differences, 0, "scale {scale}: {source}");
                }
                assert_eq!(scene.stats().fallbacks.len(), 1);
                assert_eq!(scene.stats().fallbacks[0].reason, "subpixel diagonal band");
            }
        }
    }
}

fn compare_compositing(language: Language, source: &[u8]) {
    let label = parse_labels(language, source, 8).unwrap().remove(0);
    let options = DrawerOptions {
        label_width_mm: 50.0,
        label_height_mm: 40.0,
        ..Default::default()
    };
    let scene = LabelScene::new(&label, options.clone()).unwrap();
    assert!(
        !scene.stats().fallbacks.is_empty(),
        "{language:?}: {:?}",
        String::from_utf8_lossy(source)
    );
    assert!(scene
        .stats()
        .fallbacks
        .iter()
        .all(|f| f.bounds[2] < 400 && f.bounds[3] < 320));
    let mut legacy = Vec::new();
    labelize::Renderer::new()
        .draw_label_as_png(&label, &mut legacy, options)
        .unwrap();
    let png = scene.to_png(ScenePngOptions::default()).unwrap();
    assert_eq!(
        image::load_from_memory(&legacy).unwrap().to_luma8(),
        image::load_from_memory(&png).unwrap().to_luma8()
    );
    assert!(scene.to_pdf().unwrap().starts_with(b"%PDF"));
}

#[test]
fn local_compositing_preserves_draw_order_and_exact_binary_pixels() {
    compare_compositing(
        Language::Cpcl,
        b"! 0 203 203 320 1\nPAGE-WIDTH 400\nLINE 20 20 320 23 2\nBOX 100 0 120 50 20\nPRINT\n",
    );
    compare_compositing(
        Language::Zpl,
        b"^XA^FO20,20^GB100,60,60^FS^FO40,30^FR^GB40,20,20^FS^FO50,35^GB10,10,10^FS^FO45,32^A0N,12,10^FDx^FS^XZ",
    );
    compare_compositing(
        Language::Tspl,
        b"SIZE 50 mm,40 mm\nDIRECTION 1\nCLS\nREVERSE 20,20,40,20\nBAR 20,20,20,20\nPRINT 1\n",
    );
    compare_compositing(Language::Tspl, b"SIZE 50 mm,40 mm\nDIRECTION 1\nCLS\nBAR 20,20,4,4\nBITMAP 20,20,1,4,2,\x0f\x0f\x0f\x0f\nBAR 22,21,2,2\nPRINT 1\n");
}

#[test]
fn narrow_print_width_inversion_mirroring_and_clipping_match_legacy() {
    let mut label = parse_labels(
        Language::Zpl,
        b"^XA^PW201^FO190,20^GB40,25,25^FS^FO10,60^GB30,20,20^FS^XZ",
        8,
    )
    .unwrap()
    .remove(0);
    label.inverted = true;
    label.mirrored = true;
    let options = DrawerOptions {
        label_width_mm: 50.0,
        label_height_mm: 40.0,
        ..Default::default()
    };
    let scene = LabelScene::new(&label, options.clone()).unwrap();
    let mut legacy = Vec::new();
    labelize::Renderer::new()
        .draw_label_as_png(&label, &mut legacy, options)
        .unwrap();
    assert_eq!(
        image::load_from_memory(&legacy).unwrap().to_luma8(),
        image::load_from_memory(&scene.to_png(Default::default()).unwrap())
            .unwrap()
            .to_luma8()
    );
}

#[test]
fn corrected_resident_zeros_and_justified_spaces_remain_searchable() {
    let scene = scene("^XA^FO20,20^AEN,56,30^FD100200^FS^FO20,100^A0N,26,24^FB180,3,0,J,0^FDOne two three four five six^FS^XZ", 80.0, 60.0);
    let pdf = lopdf::Document::load_mem(&scene.to_pdf().unwrap()).unwrap();
    let text = mapped_text(&pdf);
    assert!(text.contains("100200"), "{text:?}");
    assert!(text.contains("One two"), "{text:?}");
    assert_eq!(scene.stats().images, 0);
    assert!(scene.stats().fallbacks.is_empty());
    scene
        .to_png(ScenePngOptions {
            scale: 3.0,
            antialias: true,
        })
        .unwrap();
}

#[test]
fn explicit_output_options_keep_raster_defaults_and_validate_before_rendering() {
    use labelize::{
        render_pdf_with_output, render_png_with_output, OutputMode, OutputOptions, RenderOptions,
    };
    let source = b"^XA^FO20,20^A0N,30,20^FDOne^FS^XZ^XA^FDTwo^FS^XZ";
    let legacy = labelize::render_png(source, Default::default())
        .unwrap()
        .bytes;
    assert_eq!(
        legacy,
        render_png_with_output(source, Default::default(), Default::default())
            .unwrap()
            .bytes
    );
    let output = OutputOptions {
        mode: OutputMode::Vector,
        ..Default::default()
    };
    let pdf = render_pdf_with_output(
        source,
        Default::default(),
        OutputOptions {
            all_pages: true,
            ..output
        },
    )
    .unwrap();
    assert_eq!(
        lopdf::Document::load_mem(&pdf.bytes)
            .unwrap()
            .get_pages()
            .len(),
        2
    );
    for bad in [
        OutputOptions {
            scale: f64::NAN,
            ..output
        },
        OutputOptions {
            scale: 100.0,
            ..output
        },
        OutputOptions {
            all_pages: true,
            ..output
        },
    ] {
        assert!(matches!(
            render_png_with_output(source, Default::default(), bad),
            Err(labelize::LabelizeError::Parse(_))
        ));
    }
    assert!(render_png_with_output(
        source,
        RenderOptions {
            width_mm: Some(f64::INFINITY),
            ..Default::default()
        },
        output
    )
    .is_err());
    assert!(render_pdf_with_output(
        source,
        Default::default(),
        OutputOptions {
            scale: 2.0,
            ..output
        }
    )
    .is_err());
}

#[test]
fn barcode_interpretation_is_native_but_overlapping_source_text_is_local() {
    for source in [
        "^XA^FO20,20^BEN,60,Y,N^FD590123412345^FS^XZ",
        "^XA^FO20,20^BY2^BCN,60,Y,N^FD1234567890^FS^XZ",
    ] {
        let scene = scene(source, 80.0, 40.0);
        assert!(scene.stats().glyphs > 0);
        assert!(scene.stats().paths > 0);
        assert_eq!(scene.stats().images, 0);
        assert!(scene.stats().fallbacks.is_empty());
    }
    let scene = scene(
        "^XA^FO20,20^BQN,2,4^FDQA,OVERLAP^FS^FO20,20^A0N,20,20^FDX^FS^XZ",
        50.0,
        40.0,
    );
    assert!(scene
        .stats()
        .fallbacks
        .iter()
        .any(|f| f.reason == "text overlaps barcode region"));
    assert!(scene
        .stats()
        .fallbacks
        .iter()
        .all(|f| f.bounds[2] < 400 && f.bounds[3] < 320));
}

#[test]
fn local_fallback_keeps_source_black_white_pixels_when_png_edges_are_antialiased() {
    let label = parse_labels(
        Language::Zpl,
        b"^XA^FO20,20^A0N,22,20^FD123abc^FS^FO20,20^BQN,2,4^FDQA,OVERLAP^FS^XZ",
        8,
    )
    .unwrap()
    .remove(0);
    let options = DrawerOptions {
        label_width_mm: 50.0,
        label_height_mm: 40.0,
        ..Default::default()
    };
    let scene = LabelScene::new(&label, options.clone()).unwrap();
    let [x, y, w, h] = scene
        .stats()
        .fallbacks
        .iter()
        .find(|f| f.reason == "text overlaps barcode region")
        .unwrap()
        .bounds;
    let mut bytes = Vec::new();
    labelize::Renderer::new()
        .draw_label_as_png(&label, &mut bytes, options)
        .unwrap();
    let original = image::load_from_memory(&bytes).unwrap().to_luma8();
    let enlarged = image::load_from_memory(
        &scene
            .to_png(ScenePngOptions {
                scale: 3.0,
                antialias: true,
            })
            .unwrap(),
    )
    .unwrap()
    .to_luma8();
    for py in y..y + h {
        for px in x..x + w {
            assert_eq!(
                original.get_pixel(px, py),
                enlarged.get_pixel(px * 3 + 1, py * 3 + 1),
                "source pixel {px},{py}"
            );
        }
    }
}
