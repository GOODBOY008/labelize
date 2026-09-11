use labelize::barcodes::{datamatrix_legacy, BitMatrix};
use labelize::{DrawerOptions, Renderer, ZplParser};
use std::io::Cursor;

#[test]
fn placement_matches_all_21_fixed_norm_grids_and_reuses_immutable_cache() {
    let mut count = 0;
    for line in include_str!("../testdata/legacy/placement-grids.txt").lines() {
        let (size, values) = line.split_once(':').unwrap();
        let size = size.parse::<usize>().unwrap();
        let expected: Vec<usize> = values.split(',').map(|v| v.parse().unwrap()).collect();
        let actual = datamatrix_legacy::placement_for_size(size).unwrap();
        assert_eq!(actual, expected, "symbol size {size}");
        assert!(std::ptr::eq(
            actual,
            datamatrix_legacy::placement_for_size(size).unwrap()
        ));
        let mut sorted = actual.to_vec();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..actual.len()).collect::<Vec<_>>());
        count += actual.len();
    }
    assert_eq!(count, 18_389);
}

fn check_matrices(fixtures: &str) {
    let normalized = fixtures.replace("\r\n", "\n");
    for fixture in normalized.trim().split("\n\n") {
        let mut lines = fixture.lines();
        let header: Vec<_> = lines.next().unwrap().split('|').collect();
        let offset = usize::from(header.len() == 4);
        let quality = if offset == 1 {
            header[0].parse().unwrap()
        } else {
            0
        };
        let format = header[offset].parse().unwrap();
        let size = header[offset + 1].parse().unwrap();
        let bytes: Vec<u8> = header[offset + 2]
            .as_bytes()
            .chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let matrix =
            datamatrix_legacy::encode_with_ecc(&bytes, format, quality, Some(size)).unwrap();
        // Fixtures either request the maximum size or record automatic sizing.
        if size != 49 {
            assert_eq!(
                datamatrix_legacy::encode_with_ecc(&bytes, format, quality, None)
                    .unwrap()
                    .width(),
                size
            );
        }
        let rows: Vec<_> = lines.collect();
        assert_eq!(matrix.width(), rows.len());
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(matrix.width(), row.len());
            for (x, value) in row.bytes().enumerate() {
                assert_eq!(
                    matrix.get(x, y),
                    value == b'1',
                    "ECC {quality}, size {size}, format {format}, ({x},{y})"
                );
            }
        }
    }
}

#[test]
fn ecc050_matches_the_published_annex_q_matrix() {
    check_matrices(include_str!("../testdata/legacy/ecc050-annex-q-matrix.txt"));
}

#[test]
fn convolutional_matrices_match_same_source_js_port_vectors() {
    check_matrices(include_str!(
        "../testdata/legacy/convolution-js-matrices.txt"
    ));
}

#[test]
fn convolutional_byte_capacity_and_size_limits_are_explicit() {
    for (quality, minimum, maximum_bytes) in
        [(50, 11, 200), (80, 13, 176), (100, 13, 131), (140, 17, 63)]
    {
        for size in (9..minimum).step_by(2) {
            assert!(datamatrix_legacy::encode_with_ecc(b"1", 1, quality, Some(size)).is_err());
        }
        assert_eq!(
            datamatrix_legacy::encode_with_ecc(b"1", 1, quality, None)
                .unwrap()
                .width(),
            minimum
        );
        let data = vec![0; maximum_bytes];
        assert_eq!(
            datamatrix_legacy::encode_with_ecc(&data, 6, quality, None)
                .unwrap()
                .width(),
            49
        );
        assert!(
            datamatrix_legacy::encode_with_ecc(&vec![0; maximum_bytes + 1], 6, quality, None)
                .is_err()
        );
        assert!(datamatrix_legacy::encode_with_ecc(&data, 6, quality, Some(47)).is_err());
    }
    for quality in [1, 49, 51, 200, u16::MAX] {
        assert!(datamatrix_legacy::encode_with_ecc(b"A", 6, quality, None).is_err());
    }
}

#[test]
fn numeric_length_probes_preserve_explicit_generic_encoder_limits() {
    // A device's reported AUTO 500/501 boundary must not become a global cap.
    for size in [None, Some(49)] {
        for length in [500, 501, 511] {
            assert!(datamatrix_legacy::encode(&vec![b'1'; length], 1, size).is_ok());
        }
        for length in [512, 596, 597] {
            let data = vec![b'1'; length];
            let error = datamatrix_legacy::encode(&data, 1, size).unwrap_err();
            assert!(error.contains("extended record-length semantics are unresolved"));
            // Exercise the ZPL route as well: no truncation, wrap, or ECC200 fallback.
            let dimension = size.unwrap_or(0);
            let zpl = format!(
                "^XA^BXN,1,0,{dimension},{dimension},1^FD{}^FS^XZ",
                String::from_utf8(data).unwrap()
            );
            assert!(render(zpl.as_bytes())
                .unwrap_err()
                .contains("extended record-length semantics are unresolved"));
        }
    }
}

#[test]
fn zebra_maximum_field_table_matches_except_unresolved_long_numeric_ecc000() {
    // Zebra BX "Maximum Field Sizes", columns format IDs 1 through 6.
    let rows = [
        (0, [596, 452, 394, 413, 310, 271]),
        (50, [457, 333, 291, 305, 228, 200]),
        (80, [402, 293, 256, 268, 201, 176]),
        (100, [300, 218, 190, 200, 150, 131]),
        (140, [144, 105, 91, 96, 72, 63]),
    ];
    for (quality, capacities) in rows {
        for (index, capacity) in capacities.into_iter().enumerate() {
            let format = index as u8 + 1;
            let byte = if format == 1 { b'1' } else { b'A' };
            let data = vec![byte; capacity];
            if quality == 0 && format == 1 {
                let error = datamatrix_legacy::encode_with_ecc(&data, format, quality, Some(49))
                    .unwrap_err();
                assert!(error.contains("not yet supported"));
                continue;
            }
            assert!(
                datamatrix_legacy::encode_with_ecc(&data, format, quality, Some(49)).is_ok(),
                "ECC {quality}, format {format}"
            );
            assert!(
                datamatrix_legacy::encode_with_ecc(
                    &vec![byte; capacity + 1],
                    format,
                    quality,
                    Some(49)
                )
                .is_err(),
                "ECC {quality}, format {format}"
            );
        }
    }
}

#[test]
fn mixed_legacy_labels_reset_quality_and_convolution_state() {
    let qualities = [140, 50, 100, 80, 0, 140];
    let source: String = qualities
        .iter()
        .map(|quality| format!("^XA^FO20,20^BXN,2,{quality}^FDAB12^FS^XZ"))
        .collect();
    let labels = ZplParser::new().parse(source.as_bytes()).unwrap();
    assert_eq!(labels.len(), qualities.len());
    for (label, quality) in labels.iter().zip(qualities) {
        let mut output = Cursor::new(Vec::new());
        Renderer::new()
            .draw_label_as_png(label, &mut output, DrawerOptions::default())
            .unwrap();
        let actual = image::load_from_memory(output.get_ref())
            .unwrap()
            .to_rgba8();
        let standalone =
            render(format!("^XA^FO20,20^BXN,2,{quality}^FDAB12^FS^XZ").as_bytes()).unwrap();
        assert_eq!(actual, standalone, "quality {quality}");
    }
}

#[test]
fn legacy_field_symbols_match_recorded_ci13_and_ci27_photo_grids() {
    // Fixed sampled modules, not references regenerated by our encoder.
    // These scoped printer observations are not independent overall decoding.
    let fixtures =
        include_str!("../testdata/legacy/ci13-ci27-photo-grids.txt").replace("\r\n", "\n");
    let mut count = 0;
    for fixture in fixtures.trim().split("\n\n") {
        let mut lines = fixture.lines();
        let (id, hex) = lines.next().unwrap().split_once('|').unwrap();
        let payload: Vec<u8> = hex
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        let actual = datamatrix_legacy::encode(&payload, 6, Some(23)).unwrap();
        let rows: Vec<_> = lines.collect();
        assert_eq!(rows.len(), 23);
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), 23);
            for (x, bit) in row.bytes().enumerate() {
                assert!(matches!(bit, b'0' | b'1'));
                assert_eq!(actual.get(x, y), bit == b'1', "{id} ({x},{y})");
            }
        }
        count += 1;
    }
    assert_eq!(count, 12);
}

#[test]
fn recorded_printer_samples_match_module_for_module() {
    check_matrices(include_str!(
        "../testdata/legacy/ecc000-printer-matrices.txt"
    ));
}

#[test]
fn javascript_port_vectors_match_without_claiming_independent_validation() {
    check_matrices(include_str!(
        "../testdata/legacy/ecc000-js-port-vectors.txt"
    ));
}

fn render(zpl: &[u8]) -> Result<image::RgbaImage, String> {
    let labels = ZplParser::new().parse(zpl).unwrap();
    let mut output = Cursor::new(Vec::new());
    Renderer::new().draw_label_as_png(&labels[0], &mut output, DrawerOptions::default())?;
    Ok(image::load_from_memory(output.get_ref())
        .unwrap()
        .to_rgba8())
}

fn assert_drawn(matrix: &BitMatrix, orientation: &str, quality: u16) {
    let source = format!("^XA^FO20,20^BX{orientation},2,{quality},13,23,6^FDAB12^FS^XZ");
    let image = render(source.as_bytes()).unwrap();
    let expected = matrix.to_image(2, 2);
    let expected = match orientation {
        "R" => image::imageops::rotate90(&expected),
        "I" => image::imageops::rotate180(&expected),
        "B" => image::imageops::rotate270(&expected),
        _ => expected,
    };
    for y in 0..expected.height() {
        for x in 0..expected.width() {
            assert_eq!(
                image.get_pixel(20 + x, 20 + y)[0] == 0,
                expected.get_pixel(x, y)[3] != 0,
                "orientation {orientation}, ({x},{y})"
            );
        }
    }
}

#[test]
fn zpl_routes_all_legacy_qualities_dimensions_scaling_and_orientations() {
    for quality in [0, 50, 80, 100, 140] {
        let matrix = datamatrix_legacy::encode_with_ecc(b"AB12", 6, quality, Some(23)).unwrap();
        for orientation in ["N", "R", "I", "B"] {
            assert_drawn(&matrix, orientation, quality);
        }
    }
    let automatic = render(b"^XA^FO20,20^BXN,2,0^FDAB12^FS^XZ").unwrap();
    assert_eq!(
        automatic,
        render(b"^XA^FO20,20^BXN,2,0,50,51^FDAB12^FS^XZ").unwrap()
    );
}

#[test]
fn zpl_keeps_ecc200_escapes_literal_for_legacy_and_handles_ascii_fh() {
    let plain = render(b"^XA^FO20,20^BXN,2,0^FD_1ABC^FS^XZ").unwrap();
    assert_eq!(
        plain,
        render(b"^XA^FO20,20^BXN,2,0,0,0,6,!^FD_1ABC^FS^XZ").unwrap()
    );
    let fh = render(b"^XA^FO20,20^BXN,2,0^FH_^FDAB_1D12^FS^XZ").unwrap();
    let literal = render(b"^XA^FO20,20^BXN,2,0^FDAB\x1d12^FS^XZ").unwrap();
    assert_eq!(fh, literal);
}

#[test]
fn zpl_unsupported_or_invalid_legacy_fields_fail_explicitly() {
    for source in [
        "^XA^BXN,2,0,10^FDAB12^FS^XZ",
        "^XA^BXN,2,0,9,9^FDAB12^FS^XZ",
        "^XA^BXN,2,0,0,0,1^FDAB12^FS^XZ",
        "^XA^BXN,2,0,0,0,7^FDAB12^FS^XZ",
        "^XA^BXN,2,0,0,0,6,_,2^FDAB12^FS^XZ",
    ] {
        assert!(render(source.as_bytes()).is_err(), "{source}");
    }
}
