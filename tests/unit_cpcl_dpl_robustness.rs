use labelize::{format_source, parse_labels, Language};
use proptest::prelude::*;

proptest! {
    #[test]
    fn arbitrary_records_and_formatting_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        for (language, prefix) in [(Language::Cpcl, b"! 0 203 203 100 1\n".as_slice()), (Language::Dpl, b"\x02L\r1".as_slice())] {
            let mut source = prefix.to_vec();
            source.extend_from_slice(&bytes);
            let _ = parse_labels(language, &source, 8);
            let (_, formatted) = format_source(&source, Some(language));
            let _ = parse_labels(language, &formatted, 8);
        }
    }
}
