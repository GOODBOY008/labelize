use labelize::elements::barcode_qr::{
    BarcodeQr, BarcodeQrWithData, QrCharacterMode, QrErrorCorrectionLevel,
};
use labelize::elements::label_position::LabelPosition;
use labelize::elements::reverse_print::ReversePrint;
use proptest::prelude::*;

fn qr(data: &str) -> BarcodeQrWithData {
    BarcodeQrWithData {
        reverse_print: ReversePrint { value: false },
        barcode: BarcodeQr { magnification: 1 },
        height: 0,
        position: LabelPosition::default(),
        data: data.to_string(),
    }
}

macro_rules! rejects_without_panicking {
    ($name:ident, $data:expr) => {
        #[test]
        fn $name() {
            assert!(qr($data).get_input_data().is_err());
        }
    };
}

rejects_without_panicking!(binary_length_splits_utf8, "QM,B0001ä");
rejects_without_panicking!(emoji_in_prefix, "😀");
rejects_without_panicking!(emoji_after_level, "Q😀");
rejects_without_panicking!(multibyte_manual_indicator, "QM,äABCDE");
rejects_without_panicking!(multibyte_binary_length, "QM,B0😀X");
rejects_without_panicking!(multibyte_at_prefix_boundary, "00®");

#[test]
fn valid_inputs_keep_content_level_and_mode() {
    use QrCharacterMode::*;
    use QrErrorCorrectionLevel::*;
    for (input, content, level, mode) in [
        ("QA,HELLO", "HELLO", Q, Automatic),
        ("QA,ä😀", "ä😀", Q, Automatic),
        ("HM,N123", "123", H, Numeric),
        ("MM,AHELLO", "HELLO", M, Alphanumeric),
        ("LM,K漢字", "漢字", L, Kanji),
        ("QM,B0002ä", "ä", Q, Binary),
        ("QM,B0004😀", "😀", Q, Binary),
        ("QM,B0003ABCDEF", "ABC", Q, Binary),
        // Preserve the existing truncation policy for overlong lengths.
        ("QM,B9999ä", "ä", Q, Binary),
        ("QM,B0003A|B", "A|B", Q, Binary),
        ("QA,A|B", "AB", Q, Automatic),
        ("XA,HELLO", "HELLO", H, Automatic),
    ] {
        assert_eq!(
            qr(input).get_input_data().unwrap(),
            (content.to_string(), level, mode),
            "{input:?}"
        );
    }
}

proptest! {
    #[test]
    fn arbitrary_utf8_never_panics(data in any::<String>()) {
        let _ = qr(&data).get_input_data();
        let _ = qr(&format!("QM,{data}")).get_input_data();
        let _ = qr(&format!("QM,B{data}")).get_input_data();
    }
}
