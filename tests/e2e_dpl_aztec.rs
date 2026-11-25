//! Scan the public PNG output to verify DPL Aztec field boundaries.
use anyd::{codes::aztec::scan, GrayFrame};
use labelize::{format_source, render_png, Language, RenderOptions};

fn check_record(kind: char, count: &str, payload: &[u8]) {
    // Honeywell W1f/W1F: [byte count], ECI mode 0, EC/size 000, then business data.
    let header = format!("\x02L\rm\rD11\r1W1{kind}5500001000100{count}0000");
    // The following box sits at x=30..35 mm, well clear of the barcode at x=10 mm.
    let source = [
        header.as_bytes(),
        payload,
        b"\r1X1100000100300B050050010010\rE\r",
    ]
    .concat();
    let (_, formatted) = format_source(&source, Some(Language::Dpl));
    for dpmm in [8, 12] {
        for (form, job) in [("original", &source), ("formatted", &formatted)] {
            let rendered = render_png(
                job,
                RenderOptions {
                    width_mm: Some(40.0),
                    height_mm: Some(40.0),
                    dpmm,
                    ..Default::default()
                },
            )
            .unwrap();
            let image = image::load_from_memory(&rendered.bytes)
                .unwrap()
                .into_luma8();
            let dots = dpmm as u32;
            assert_eq!(
                image.get_pixel(30 * dots + dots / 2, 36 * dots)[0],
                0,
                "following box is visible ({form}, {dpmm} dpmm)"
            );
            let frame = GrayFrame::new(
                image.as_raw(),
                image.width() as usize,
                image.height() as usize,
            )
            .unwrap();
            let decoded = scan(&frame).expect("independent Aztec scan");
            assert_eq!(decoded.payload_bytes(), payload, "{form}, {dpmm} dpmm");
        }
    }
}

#[test]
fn variable_records_strip_options_and_preserve_business_prefixes() {
    check_record('f', "", b"SO-00123");
    check_record('f', "", b"0000SO-00123");
}

#[test]
fn counted_records_strip_count_and_options_and_preserve_business_prefixes() {
    check_record('F', "0012", b"SO-00123");
    check_record('F', "0016", b"0000SO-00123");
}

#[test]
fn counted_record_carries_binary_through_the_record_delimiter() {
    check_record('F', "0015", b"AZTEC\r\n\0\x80\xff ");
}
