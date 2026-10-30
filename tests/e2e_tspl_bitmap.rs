//! BITMAP compatibility is checked against explicit pixels, not a second
//! renderer with the same parser. See docs/TSPL_BASELINES.md for provenance.
use image::GrayImage;
use labelize::{render_png, Language, RenderOptions};

fn render(source: &[u8]) -> GrayImage {
    let output = render_png(
        source,
        RenderOptions {
            language: Some(Language::Tspl),
            ..Default::default()
        },
    )
    .unwrap();
    image::load_from_memory(&output.bytes).unwrap().to_luma8()
}

fn bitmap_source(width_bytes: usize, height: usize, payload: &[u8]) -> Vec<u8> {
    let mut source = format!(
        "SIZE {} dot,{height} dot\nDIRECTION 1\nCLS\nBITMAP 0,0,{width_bytes},{height},0,",
        width_bytes * 8
    )
    .into_bytes();
    source.extend_from_slice(payload);
    source.extend_from_slice(b"\nPRINT 1\n");
    source
}

#[test]
fn full_size_issue_108_hex_and_binary_render_all_eight_rows() {
    let hex = include_bytes!("../testdata/unit/tspl_bitmap_hex.tspl");
    let pixels = render(hex);
    let raw = [0xff, 0, 0xff, 0].repeat(8);
    assert_eq!(pixels, render(&bitmap_source(4, 8, &raw)));
    assert_eq!(pixels.dimensions(), (32, 8));
    for (x, _, pixel) in pixels.enumerate_pixels() {
        assert_eq!(pixel[0], if (x / 8) % 2 == 0 { 255 } else { 0 });
    }
}

#[test]
fn tsc_manual_bitmap_table_matches_the_printed_arrow() {
    // TSC manual, printed pp.44-45 (PDF pp.56-57): data table and the
    // separately printed cell grid. The latter supplies the expected pixels.
    let raw = [
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0xff, 0x03, 0xff, 0x11, 0xff, 0x18, 0xff, 0x1c,
        0x7f, 0x1e, 0x3f, 0x1f, 0x1f, 0x1f, 0x8f, 0x1f, 0xc7, 0x1f, 0xe3, 0x1f, 0xf7, 0x1f, 0xff,
        0x1f, 0xff,
    ];
    let expected = [
        "################",
        "################",
        "################",
        "#####...........",
        "######..........",
        "###.###.........",
        "###..###........",
        "###...###.......",
        "###....###......",
        "###.....###.....",
        "###......###....",
        "###.......###...",
        "###........###..",
        "###.........#...",
        "###.............",
        "###.............",
    ];
    let pixels = render(&bitmap_source(2, 16, &raw));
    assert_eq!(pixels.dimensions(), (16, 16));
    assert_eq!(
        pixels,
        render(include_bytes!("../testdata/unit/tspl_bitmap_manual.tspl"))
    );
    for (y, row) in expected.iter().enumerate() {
        for (x, ink) in row.bytes().enumerate() {
            assert_eq!(
                pixels.get_pixel(x as u32, y as u32)[0],
                if ink == b'#' { 0 } else { 255 },
                "({x}, {y})"
            );
        }
    }
}

#[test]
fn bitmap_modes_keep_the_same_pixels_for_hex_and_binary() {
    for mode in 0..=2 {
        let prefix =
            format!("SIZE 16 dot,1 dot\nDIRECTION 1\nCLS\nBAR 0,0,8,1\nBITMAP 0,0,2,1,{mode},");
        let mut raw = prefix.as_bytes().to_vec();
        raw.extend_from_slice(&[0xf0, 0x0f]);
        raw.extend_from_slice(b"\nPRINT 1\n");
        let hex = format!("{prefix}F00F\nPRINT 1\n");
        let pixels = render(&raw);
        assert_eq!(pixels, render(hex.as_bytes()));
    }
}
