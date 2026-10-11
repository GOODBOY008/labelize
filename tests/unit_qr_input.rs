use labelize::elements::barcode_qr::QrCharacterMode::*;
use labelize::elements::barcode_qr::QrErrorCorrectionLevel::*;
use labelize::parsers::zpl::qr_payload::parse_qr_fd_payload;
use proptest::prelude::*;

macro_rules! rejects_without_panicking {
    ($name:ident, $data:expr) => {
        #[test]
        fn $name() {
            assert!(parse_qr_fd_payload($data).is_err());
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
fn empty_field_is_a_skip_not_an_error() {
    // `^FD^FS` produces an empty payload the renderer skips (Labelary
    // behavior) rather than a parse error.
    assert_eq!(
        parse_qr_fd_payload("").unwrap(),
        (String::new(), H, Automatic)
    );
}

#[test]
fn valid_inputs_keep_content_level_and_mode() {
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
        ("QM,XHELLO", "HELLO", Q, Automatic),
    ] {
        assert_eq!(
            parse_qr_fd_payload(input).unwrap(),
            (content.to_string(), level, mode),
            "{input:?}"
        );
    }
}

proptest! {
    #[test]
    fn arbitrary_utf8_never_panics(data in any::<String>()) {
        let _ = parse_qr_fd_payload(&data);
        let _ = parse_qr_fd_payload(&format!("QM,{data}"));
        let _ = parse_qr_fd_payload(&format!("QM,B{data}"));
    }
}
