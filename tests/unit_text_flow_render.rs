//! Placement assertions independent of the substitute font's exact outline.
mod common;

use common::render_helpers;
use image::{GrayImage, Luma};
use labelize::DrawerOptions;

fn render(font: &str, orientation: char, command: &str, value: &str, block: &str) -> GrayImage {
    let sizes = if font == "A" { "36,20" } else { "40,32" };
    let source = format!(
        "^XA^PW640^LL640^CI28^FO320,180^A{font}{orientation},{sizes}{command}{block}^FD{value}^FS^XZ"
    );
    let bytes = render_helpers::render_zpl_to_png(
        &source,
        DrawerOptions {
            label_width_mm: 80.0,
            label_height_mm: 80.0,
            dpmm: 8,
            ..Default::default()
        },
    );
    let mut image = image::load_from_memory(&bytes).expect("PNG").to_luma8();
    for value in image.pixels_mut() {
        *value = Luma([if value[0] < 128 { 0 } else { 255 }]);
    }
    image
}

fn runs(image: &GrayImage, horizontal: bool) -> Vec<(u32, u32)> {
    let length = if horizontal {
        image.width()
    } else {
        image.height()
    };
    let other = if horizontal {
        image.height()
    } else {
        image.width()
    };
    let mut start = None;
    let mut ranges = Vec::new();
    for coordinate in 0..=length {
        let occupied = coordinate < length
            && (0..other).any(|p| {
                let (x, y) = if horizontal {
                    (coordinate, p)
                } else {
                    (p, coordinate)
                };
                image.get_pixel(x, y)[0] == 0
            });
        if occupied && start.is_none() {
            start = Some(coordinate);
        } else if !occupied {
            if let Some(begin) = start.take() {
                ranges.push((begin, coordinate));
            }
        }
    }
    ranges
}

fn crop_ink(image: &GrayImage) -> GrayImage {
    let xs = runs(image, true);
    let ys = runs(image, false);
    let (x, y) = (xs.first().expect("ink").0, ys.first().expect("ink").0);
    let (right, bottom) = (xs.last().unwrap().1, ys.last().unwrap().1);
    image::imageops::crop_imm(image, x, y, right - x, bottom - y).to_image()
}

fn components(image: &GrayImage, horizontal: bool) -> Vec<GrayImage> {
    runs(image, horizontal)
        .into_iter()
        .map(|(start, end)| {
            let crop = if horizontal {
                image::imageops::crop_imm(image, start, 0, end - start, image.height())
            } else {
                image::imageops::crop_imm(image, 0, start, image.width(), end - start)
            };
            crop_ink(&crop.to_image())
        })
        .collect()
}

#[test]
fn horizontal_spacing_moves_existing_glyph_pixels_by_exact_requested_dots() {
    for font in ["0", "A"] {
        let baseline = render(font, 'N', "", "IIIII", "");
        assert!(baseline == render(font, 'N', "^FPH,0", "IIIII", ""));
        let glyphs = runs(&baseline, true);
        assert_eq!(glyphs.len(), 5);
        for gap in [5, 15, 30] {
            let actual = render(font, 'N', &format!("^FPH,{gap}"), "IIIII", "");
            let mut expected = GrayImage::from_pixel(640, 640, Luma([255]));
            for (index, &(start, end)) in glyphs.iter().enumerate() {
                for x in start..end {
                    for y in 0..640 {
                        expected.put_pixel(x + index as u32 * gap, y, *baseline.get_pixel(x, y));
                    }
                }
            }
            assert!(actual == expected, "font {font}, gap {gap}");
        }
    }
}

#[test]
fn reverse_flow_reverses_order_without_mirroring_glyphs() {
    for font in ["0", "A"] {
        let actual = render(font, 'N', "^FPR,30", "FLP", "");
        let actual = components(&actual, true);
        let expected: Vec<_> = ["P", "L", "F"]
            .map(|value| crop_ink(&render(font, 'N', "", value, "")))
            .into_iter()
            .collect();
        assert_eq!(actual, expected, "font {font}");
    }
}

#[test]
fn vertical_flow_stacks_upright_characters_and_ignores_additional_gap() {
    for font in ["0", "A"] {
        let baseline = render(font, 'N', "^FPV,0", "FLP", "");
        for gap in [5, 10, 30] {
            assert!(baseline == render(font, 'N', &format!("^FPV,{gap}"), "FLP", ""));
        }
        let expected: Vec<_> = ["F", "L", "P"]
            .map(|value| crop_ink(&render(font, 'N', "", value, "")))
            .into_iter()
            .collect();
        assert_eq!(components(&baseline, false), expected, "font {font}");
    }
}

#[test]
fn directed_flow_keeps_all_ink_when_the_field_is_rotated() {
    for command in ["^FPH,15", "^FPR,15", "^FPV,15"] {
        let count = |orientation| {
            render("A", orientation, command, "FLP", "")
                .pixels()
                .filter(|pixel| pixel[0] == 0)
                .count()
        };
        let expected = count('N');
        assert!(expected > 100);
        for orientation in ['R', 'I', 'B'] {
            assert_eq!(
                count(orientation),
                expected,
                "{command}, orientation {orientation}"
            );
        }
    }
}

#[test]
fn vertical_block_gap_affects_wrapping_but_not_horizontal_character_placement() {
    let block = "^FB100,5,0,L,0";
    let baseline = render("0", 'N', "^FPV,0", "ABC DE FG", block);
    let spaced = render("0", 'N', "^FPV,30", "ABC DE FG", block);
    assert!(baseline != spaced);
    assert!(crop_ink(&spaced).width() < 35);
    assert!(crop_ink(&spaced).height() > crop_ink(&baseline).height());
}

#[test]
fn directed_symbol_text_keeps_its_origin_after_an_unsupported_character() {
    use ab_glyph::{Font, FontRef, PxScale};
    use labelize::elements::{
        font::FontInfo,
        font_face::FontFace,
        label_element::LabelElement,
        label_position::LabelPosition,
        text_field::TextField,
        text_flow::{TextDirection, TextFlow},
    };
    use labelize::LabelInfo;

    let font = FontRef::try_from_slice(FontFace::Symbol.ttf_data()).unwrap();
    assert_eq!(font.glyph_id('H'), ab_glyph::GlyphId(0));
    assert!(font
        .outline_glyph(font.glyph_id('A').with_scale(PxScale::from(24.0)))
        .is_some());
    let render_at = |x, y, direction| {
        let label = LabelInfo {
            elements: vec![LabelElement::Text(TextField {
                font: FontInfo {
                    face: FontFace::Symbol,
                    width: 24.0,
                    height: 24.0,
                    ..Default::default()
                },
                text: "\u{10ffff}A".to_owned(),
                position: LabelPosition {
                    x,
                    y,
                    ..Default::default()
                },
                alignment: Default::default(),
                reverse_print: Default::default(),
                flow: TextFlow {
                    direction,
                    origin_width: 24.0,
                    ..Default::default()
                },
                block: None,
            })],
            ..Default::default()
        };
        let png = render_helpers::render_label_to_png(&label, render_helpers::unit_options());
        image::load_from_memory(&png).unwrap().to_luma8()
    };
    for direction in [TextDirection::Reverse, TextDirection::Vertical] {
        let reference = render_at(100, 100, direction);
        let moved = render_at(320, 180, direction);
        let reference_x = runs(&reference, true)[0].0;
        let reference_y = runs(&reference, false)[0].0;
        assert_eq!(runs(&moved, true)[0].0 - reference_x, 220);
        assert_eq!(runs(&moved, false)[0].0 - reference_y, 80);
        assert_eq!(crop_ink(&moved), crop_ink(&reference));
    }
}

#[test]
fn reverse_block_keeps_line_offsets_across_blank_and_missing_glyph_lines() {
    let block = "^FB500,5,0,L,0";
    let top = render("0", 'N', "^FPR,5", "CDE", block);
    let bottom = render("0", 'N', "^FPR,5", "AB", block);
    let top_row = image::imageops::crop_imm(&top, 0, 175, 640, 40).to_image();
    let bottom_row = image::imageops::crop_imm(&bottom, 0, 175, 640, 40).to_image();
    for value in ["AB\\&\\&CDE", "AB\\&\u{10ffff}\\&CDE"] {
        let actual = render("0", 'N', "^FPR,5", value, block);
        assert!(image::imageops::crop_imm(&actual, 0, 175, 640, 40).to_image() == top_row);
        assert!(image::imageops::crop_imm(&actual, 0, 255, 640, 40).to_image() == bottom_row);
    }
}
