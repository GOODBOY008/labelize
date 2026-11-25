use image::imageops::crop_imm;
use labelize::{render_png_with_output, Language, OutputMode, OutputOptions, RenderOptions};

const TEXT: &str = "A保温杯B";
const FONTS: &[(Language, &str, u32)] = &[
    (Language::Cpcl, "GBUNSG16.CPF", 16),
    (Language::Cpcl, "GBUNSG24.CPF", 24),
    (Language::Tspl, "TSS16.BF2", 16),
    (Language::Tspl, "TSS24.BF2", 24),
];

fn job(language: Language, font: &str, dpmm: i32, fields: &[(u32, u32, &str)]) -> String {
    let dpi = if dpmm == 8 { 200 } else { 300 };
    let mut source = match language {
        Language::Cpcl => format!(
            "! 0 {dpi} {dpi} {} 1\r\nPAGE-WIDTH {}\r\nCOUNTRY UTF-8\r\n",
            30 * dpmm,
            50 * dpmm
        ),
        Language::Tspl => "SIZE 50 mm,30 mm\r\nDIRECTION 1\r\nCLS\r\nCODEPAGE UTF-8\r\n".into(),
        _ => unreachable!(),
    };
    for (x, y, text) in fields {
        source.push_str(&match language {
            Language::Cpcl => format!("TEXT {font} 0 {x} {y} {text}\r\n"),
            Language::Tspl => format!("TEXT {x},{y},\"{font}\",0,1,1,\"{text}\"\r\n"),
            _ => unreachable!(),
        });
    }
    source.push_str(if language == Language::Cpcl {
        "PRINT\r\n"
    } else {
        "PRINT 1\r\n"
    });
    source
}

fn options(language: Language, dpmm: i32) -> RenderOptions {
    RenderOptions {
        language: Some(language),
        dpmm,
        width_mm: Some(50.0),
        height_mm: Some(30.0),
        ..Default::default()
    }
}

fn check_cells(mode: OutputMode) {
    for &(language, font, cell) in FONTS {
        for dpmm in [8, 12] {
            // Place a second copy of each glyph at its expected pen position.
            // Comparing rows checks actual advances through the output backend.
            let first_han = 20 + cell / 2;
            let fields = [
                (20, 20, TEXT),
                (20, 80, "A"),
                (first_han, 80, "保"),
                (first_han + cell, 80, "温"),
                (first_han + 2 * cell, 80, "杯"),
                (first_han + 3 * cell, 80, "B"),
            ];
            let source = job(language, font, dpmm, &fields);
            let output = OutputOptions {
                mode,
                ..Default::default()
            };
            let png =
                render_png_with_output(source.as_bytes(), options(language, dpmm), output).unwrap();
            let pixels = image::load_from_memory(&png.bytes).unwrap().to_luma8();
            let row = |y| crop_imm(&pixels, 0, y, pixels.width(), cell).to_image();
            assert_eq!(row(20), row(80), "{font} at {dpmm} dpmm ({mode:?})");

            let glyphs: Vec<_> = (0..3)
                .map(|i| crop_imm(&pixels, first_han + i * cell, 20, cell, cell).to_image())
                .collect();
            assert!(
                glyphs.iter().all(|g| g.as_raw().iter().any(|&p| p < 128)),
                "{font} has a blank Chinese glyph at {dpmm} dpmm ({mode:?})"
            );
            assert!(
                glyphs[0] != glyphs[1] && glyphs[1] != glyphs[2] && glyphs[0] != glyphs[2],
                "{font} repeats a glyph at {dpmm} dpmm ({mode:?})"
            );
        }
    }
}

#[test]
fn raster_named_chinese_fonts_use_distinct_glyphs_and_device_cells() {
    check_cells(OutputMode::Raster);
}

#[cfg(feature = "vector")]
#[test]
fn vector_named_chinese_fonts_use_distinct_glyphs_and_device_cells() {
    check_cells(OutputMode::Vector);
}

#[cfg(feature = "vector")]
#[test]
fn named_chinese_fonts_have_searchable_unicode_pdf_text() {
    let output = OutputOptions {
        mode: OutputMode::Vector,
        ..Default::default()
    };
    for &(language, font, _) in FONTS {
        for dpmm in [8, 12] {
            let mixed = "A保温杯BΩβЖāǎě€𝚉";
            let source = job(language, font, dpmm, &[(20, 20, mixed)]);
            let rendered = labelize::render_pdf_with_output(
                source.as_bytes(),
                options(language, dpmm),
                output,
            )
            .unwrap();
            let pdf = lopdf::Document::load_mem(&rendered.bytes).unwrap();
            let extracted = pdf.extract_text(&[1]).unwrap().replace('\n', "");
            assert_eq!(extracted, mixed, "{font} at {dpmm} dpmm");
        }
    }
}

#[test]
fn cp936_symbols_have_glyphs_in_the_named_face() {
    use ab_glyph::{Font, FontRef};
    use labelize::elements::font_face::FontFace;

    let font = FontRef::try_from_slice(FontFace::BitmapSquare.ttf_data()).unwrap();
    let mut missing = std::collections::BTreeSet::new();
    for lead in 0x81..=0xfe {
        for trail in (0x40..=0x7e).chain(0x80..=0xfe) {
            let bytes = [lead, trail];
            if let Some(decoded) =
                encoding_rs::GBK.decode_without_bom_handling_and_without_replacement(&bytes)
            {
                for c in decoded.chars() {
                    // CP936's user-defined areas have no universal glyph.
                    if !('\u{e000}'..='\u{f8ff}').contains(&c) && font.glyph_id(c).0 == 0 {
                        missing.insert(c);
                    }
                }
            }
        }
    }
    assert!(missing.is_empty(), "missing CP936 characters: {missing:?}");
    assert_ne!(font.glyph_id('€').0, 0); // CP936's single-byte 0x80 extension
}
