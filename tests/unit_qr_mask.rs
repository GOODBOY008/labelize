use labelize::barcodes::qrcode::encode;
use labelize::elements::barcode_qr::QrErrorCorrectionLevel as Ec;

#[test]
fn five_isolated_labelary_matrices_improve_without_changing_codewords() {
    use qrcode::{Color, EcLevel, QrCode};
    let reference = image::open("testdata/unit/qr_mask_selection.png")
        .unwrap()
        .to_rgba8();
    for (data, x0, y0) in [
        ("abcabaa", 24, 34),
        ("abcucaa", 200, 34),
        ("abccdaa", 376, 34),
        ("abcndaa", 24, 210),
        ("abcaeaa", 200, 210),
    ] {
        let expected: Vec<Vec<bool>> = (0..21)
            .map(|y| {
                (0..21)
                    .map(|x| reference.get_pixel(x0 + x * 4 + 2, y0 + y * 4 + 2)[0] < 128)
                    .collect()
            })
            .collect();
        let baseline = QrCode::with_error_correction_level(data.as_bytes(), EcLevel::H).unwrap();
        assert_eq!(baseline.width(), 21);
        let before: Vec<Vec<bool>> = (0..21)
            .map(|y| (0..21).map(|x| baseline[(x, y)] == Color::Dark).collect())
            .collect();
        assert_ne!(
            before, expected,
            "fixture must expose the old mask difference"
        );
        let corrected = encode(data, 1, Ec::H).unwrap();
        let after: Vec<Vec<bool>> = (0..21)
            .map(|y| {
                (0..21)
                    .map(|x| corrected.get_pixel(x + 4, y + 4)[3] != 0)
                    .collect()
            })
            .collect();
        assert_eq!(
            after, expected,
            "{data}: complete matrix must match Labelary"
        );
        let decode = |matrix: &Vec<Vec<bool>>| {
            rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(matrix).unwrap()
        };
        let old = decode(&before);
        let new = decode(&after);
        let labelary = decode(&expected);
        assert_eq!(new.getText(), data);
        assert_eq!(new.getRawBytes(), old.getRawBytes());
        assert_eq!(new.getRawBytes(), labelary.getRawBytes());
        assert_eq!(new.getECLevel(), labelary.getECLevel());
        assert_eq!(new.getByteSegments(), &vec![data.as_bytes().to_vec()]);
    }
}

#[test]
fn standard_mask_matches_independent_toolkit_matrices() {
    for name in ["mask-24", "mask-37", "mask-46", "mask-64", "mask-907"] {
        let text = std::fs::read_to_string(format!("testdata/qr-mask/{name}.txt"))
            .expect("required independent QR mask fixture");
        let mut lines = text.lines();
        let data = lines.next().unwrap();
        let ec = match lines.next().unwrap() {
            "L" => Ec::L,
            "M" => Ec::M,
            "Q" => Ec::Q,
            "H" => Ec::H,
            _ => unreachable!(),
        };
        let mask = lines.next().unwrap();
        let _independent_scores = lines.next().unwrap();
        let expected: Vec<Vec<bool>> = lines
            .map(|r| r.bytes().map(|v| v == b'1').collect())
            .collect();
        {
            let img = encode(data, 1, ec).unwrap();
            assert_eq!(img.width() as usize, expected.len() + 8);
            let actual: Vec<Vec<bool>> = (0..expected.len())
                .map(|y| {
                    (0..expected.len())
                        .map(|x| img.get_pixel(x as u32 + 4, y as u32 + 4)[3] != 0)
                        .collect()
                })
                .collect();
            assert!(
                actual == expected,
                "{data} {ec:?}: must select standard mask {mask}"
            );
            let decoded =
                rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(&actual).unwrap();
            assert_eq!(decoded.getText(), data);
            assert_eq!(decoded.getRawBytes()[0] >> 4, 4);
            assert_eq!(decoded.getByteSegments(), &vec![data.as_bytes().to_vec()]);
        }
    }
}

#[test]
fn mask_change_preserves_auto_segments_codewords_correction_size_and_scaling() {
    use qrcode::{Color, EcLevel, QrCode};
    for (ec, original_ec) in [
        (Ec::L, EcLevel::L),
        (Ec::M, EcLevel::M),
        (Ec::Q, EcLevel::Q),
        (Ec::H, EcLevel::H),
    ] {
        for data in [
            "mask-24",
            "12345678901234567890",
            "ABC-123",
            "ABC12345678901234567890abc",
            "ä😀",
        ] {
            let original =
                QrCode::with_error_correction_level(data.as_bytes(), original_ec).unwrap();
            let bits: Vec<Vec<bool>> = (0..original.width())
                .map(|y| {
                    (0..original.width())
                        .map(|x| original[(x, y)] == Color::Dark)
                        .collect()
                })
                .collect();
            let expected =
                rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(&bits).unwrap();
            let unscaled = encode(data, 1, ec).unwrap();
            for mag in [1, 2, 4] {
                let actual = encode(data, mag, ec).unwrap();
                let matrix: Vec<Vec<bool>> = (0..original.width())
                    .map(|y| {
                        (0..original.width())
                            .map(|x| {
                                actual.get_pixel(
                                    (x as u32 + 4) * mag as u32,
                                    (y as u32 + 4) * mag as u32,
                                )[3] != 0
                            })
                            .collect()
                    })
                    .collect();
                let decoded =
                    rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(&matrix).unwrap();
                assert_eq!(decoded.getRawBytes(), expected.getRawBytes());
                assert_eq!(decoded.getByteSegments(), expected.getByteSegments());
                assert_eq!(decoded.getECLevel(), expected.getECLevel());
                assert_eq!(
                    actual.dimensions(),
                    (
                        unscaled.width() * mag as u32,
                        unscaled.height() * mag as u32
                    )
                );
                for (x, y, pixel) in actual.enumerate_pixels() {
                    let (mx, my) = (x / mag as u32, y / mag as u32);
                    assert_eq!(pixel, unscaled.get_pixel(mx, my));
                    if mx < 4
                        || my < 4
                        || mx >= original.width() as u32 + 4
                        || my >= original.width() as u32 + 4
                    {
                        assert_eq!(pixel[3], 0, "quiet zone");
                    }
                }
            }
        }
    }
}
