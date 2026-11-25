//! The independent anyd detector locates symbols directly in the rendered PNG.
use anyd::{codes::aztec::scan, GrayFrame};
use image::{imageops, GrayImage};
use labelize::{detect_language, format_source, parse_labels, render_png, Language, RenderOptions};

fn render(command: &[u8], dpmm: i32) -> GrayImage {
    let rendered = render_png(
        command,
        RenderOptions {
            dpmm,
            width_mm: Some(35.0),
            height_mm: Some(28.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(rendered.language, Language::Tspl);
    image::load_from_memory(&rendered.bytes)
        .unwrap()
        .into_luma8()
}

fn scan_image(image: &GrayImage) -> Vec<u8> {
    let frame = GrayFrame::new(
        image.as_raw(),
        image.width() as usize,
        image.height() as usize,
    )
    .unwrap();
    scan(&frame)
        .expect("independent Aztec scanner")
        .payload_bytes()
}

fn symbol_area(image: &GrayImage, x: u32, y: u32, side: u32) -> GrayImage {
    let ink = image.enumerate_pixels().filter(|(_, _, p)| p[0] == 0);
    let mut bounds = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, _) in ink {
        bounds = (
            bounds.0.min(x),
            bounds.1.min(y),
            bounds.2.max(x),
            bounds.3.max(y),
        );
    }
    assert_eq!(bounds, (x, y, x + side - 1, y + side - 1));
    imageops::crop_imm(image, x, y, side, side).to_image()
}

#[test]
fn quoted_symbols_scan_at_the_requested_origin_scale_and_rotation() {
    for dpmm in [8, 12] {
        let report = render(
            b"SIZE 50 mm,30 mm\nCLS\nAZTEC 10,10,0,3,0,0,0,0,0,\"ABC123\"\nPRINT 1,1",
            dpmm,
        );
        assert_eq!(scan_image(&report), b"ABC123");
        let upright = symbol_area(&report, 10, 10, 15 * 3);
        for (rotation, expected) in [
            (90, imageops::rotate90(&upright)),
            (180, imageops::rotate180(&upright)),
            (270, imageops::rotate270(&upright)),
        ] {
            let cmd = format!("AZTEC 10,10,{rotation},3,\"ABC123\"");
            let image = render(cmd.as_bytes(), dpmm);
            assert_eq!(scan_image(&image), b"ABC123");
            assert_eq!(symbol_area(&image, 10, 10, 15 * 3), expected);
        }
        for (params, size, expected) in [
            ("", 15 * 6, b"ABC123".as_slice()),
            ("4,103,", 23 * 4, b"ABC123"),
            ("4,202,", 23 * 4, b"ABC123"),
            ("4,300,", 11 * 4, &[42][..]),
        ] {
            let data = if params == "4,300," { "42" } else { "ABC123" };
            let cmd = format!("aztec 18,20,0,{params}\"{data}\"");
            let image = render(cmd.as_bytes(), dpmm);
            assert_eq!(scan_image(&image), expected);
            symbol_area(&image, 18, 20, size);
        }
    }
}

#[test]
fn counted_payloads_preserve_bytes_and_resume_at_the_next_command() {
    let payload = b"\"\x80\xff,\0\r\nPRINT 1\r\n ";
    let mut source = format!("  aztec 10,12,0,4,25,0,0,1,0,{},", payload.len()).into_bytes();
    source.extend_from_slice(payload);
    source.extend_from_slice(b"\r\nPRINT 1\r\nBAR 0,0,2,2\r\nPRINT 1\r\n");
    assert_eq!(detect_language(&source), Language::Tspl);
    let (_, formatted) = format_source(&source, None);
    for dpmm in [8, 12] {
        for data in [&source, &formatted] {
            assert_eq!(parse_labels(Language::Tspl, data, dpmm).unwrap().len(), 2);
            assert_eq!(scan_image(&render(data, dpmm)), payload);
        }
    }
}

#[test]
fn reversed_output_complements_every_module_and_handles_counted_input() {
    for dpmm in [8, 12] {
        let normal = render(b"AZTEC 13,17,0,5,101,0,0,1,0,\"1234567890\"", dpmm);
        let inverse = render(b"AZTEC 13,17,0,5,101,0,0,1,1,10,1234567890", dpmm);
        let mut area = symbol_area(&inverse, 13, 17, 15 * 5);
        imageops::invert(&mut area);
        assert_eq!(area, symbol_area(&normal, 13, 17, 15 * 5));
        assert_eq!(scan_image(&normal), b"1234567890");
    }
}

#[test]
fn quoted_escapes_and_optional_delimiters_in_counted_form_preserve_data() {
    let cases: &[(&[u8], &[u8])] = &[
        (br#"AZTEC 18,20,0," A,B\"C\["]D ""#, b" A,B\"C\"D "),
        (br#"AZTEC 18,20,0,"A\\["]B""#, b"A\\\"B"),
        (br#"AZTEC 18,20,0,"A\""#, br"A\"),
        (b"AZTEC 18,20,0,4,0,1,\"A\x1b\x1bB\"", b"A\x1bB"),
        (b"AZTEC 18,20,0,4,0,0,0,1,0,3,\"ABC\"", b"ABC"),
        (b"AZTEC 18,20,0,4,0,0,0,1,0,3,\"A\"", b"\"A\""),
    ];
    for &(input, expected) in cases {
        let (_, formatted) = format_source(input, None);
        assert_eq!(scan_image(&render(&formatted, 8)), expected);
    }
}

#[test]
fn counted_quotes_keep_payload_line_endings_and_the_following_print() {
    let cases: &[(&[u8], &[u8])] = &[
        (b"2,\"A\n\"", b"A\n"),
        (b"2,\"A\r\"  \t", b"A\r"),
        (b"3,\"A\r\n\"", b"A\r\n"),
        (b"3,\"A\"", b"\"A\""),
        (b"3,A \t", b"A \t"),
    ];
    for &(field, payload) in cases {
        let command = [b"AZTEC 18,20,0,4,0,0,0,1,0,".as_slice(), field].concat();
        let lines = [
            command.as_slice(),
            b"PRINT 1",
            b"CLS",
            b"BAR 1,1,2,2",
            b"PRINT 1",
            b"",
        ];
        let source = lines.join(b"\r\n".as_slice());
        let (_, formatted) = format_source(&source, None);
        assert_eq!(formatted, lines.join(b"\n".as_slice()));
        for dpmm in [8, 12] {
            for job in [&source, &formatted] {
                let labels = parse_labels(Language::Tspl, job, dpmm).unwrap();
                assert_eq!(labels.len(), 2, "following PRINT flushes the first label");
                assert_eq!(scan_image(&render(job, dpmm)), payload);
            }
        }
    }
}

#[test]
fn formatting_a_truncated_counted_payload_preserves_bytes_and_error() {
    let prefix = b"AZTEC 18,20,0,4,0,0,0,1,0,";
    for short in [b"2,A".as_slice(), b"40,\xff\0\r\nPRINT 1\n\r \t"] {
        let source = [prefix.as_slice(), short].concat();
        let error = parse_labels(Language::Tspl, &source, 8).unwrap_err();
        assert!(error.to_string().contains("truncated counted payload"));
        let (_, formatted) = format_source(&source, None);
        assert_eq!(formatted, source);
        let formatted_error = parse_labels(Language::Tspl, &formatted, 8).unwrap_err();
        assert_eq!(formatted_error.to_string(), error.to_string());
    }
}

#[test]
fn malformed_headers_and_unsupported_special_modes_are_reported() {
    for (command, error) in [
        ("AZTEC 0,0,0,4,0,0,0,1,0,9,xx", "truncated"),
        ("AZTEC 0,0,0,4,0,0,0,1,0,1,xx", "unexpected bytes"),
        ("AZTEC 0,0,0,\"open", "closing quote"),
        ("AZTEC 0,0,0,\"closed\"extra", "unexpected bytes"),
        ("AZTEC 0,0,0,21,\"a\"", "module size"),
        ("AZTEC 0,0,45,\"a\"", "rotation"),
        ("AZTEC 0,0,0,3,105,\"a\"", "error correction"),
        ("AZTEC 0,0,0,3,0,1,\"\x1b0\"", "FLG"),
        ("AZTEC 0,0,0,3,0,0,1,\"a\"", "menu"),
        ("AZTEC 0,0,0,3,0,0,0,2,\"a\"", "multi-symbol"),
    ] {
        let failure = parse_labels(Language::Tspl, command.as_bytes(), 8)
            .unwrap_err()
            .to_string();
        assert!(failure.contains(error), "{command:?}: {failure}");
    }
}
