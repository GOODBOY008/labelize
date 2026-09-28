use labelize::barcodes::qrcode::encode_with_mode;
use labelize::elements::barcode_qr::{QrCharacterMode as Mode, QrErrorCorrectionLevel as Ec};

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
        for mode in [Mode::Automatic, Mode::Binary] {
            let img = encode_with_mode(data, 1, ec, mode).unwrap();
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
                "{data} {ec:?} {mode:?}: must select standard mask {mask}"
            );
            let decoded =
                rxing::qrcode::decoder::qrcode_decoder::decode_bool_array(&actual).unwrap();
            assert_eq!(decoded.getText(), data);
            assert_eq!(decoded.getRawBytes()[0] >> 4, 4);
            assert_eq!(decoded.getByteSegments(), &vec![data.as_bytes().to_vec()]);
        }
    }
}
