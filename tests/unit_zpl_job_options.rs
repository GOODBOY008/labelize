use labelize::{ZplParseOptions, ZplParser};

fn options() -> ZplParseOptions {
    ZplParseOptions {
        use_declared_dimensions: true,
        max_labels: Some(1000),
        require_complete_formats: true,
    }
}

#[test]
fn declared_media_is_opt_in_and_changes_per_label() {
    let source = b"^XA^PW800^LL1200^FO10,10^FDfirst^FS^XZ^XA^PW400^LL240^FO10,10^FDsecond^FS^XZ";
    let old = ZplParser::new().parse(source).unwrap();
    assert!(old
        .iter()
        .all(|l| l.label_width_mm.is_none() && l.label_height_mm.is_none()));
    let labels = ZplParser::with_dpmm(8)
        .parse_with_options(source, options())
        .unwrap();
    assert_eq!(
        (labels[0].label_width_mm, labels[0].label_height_mm),
        (Some(100.0), Some(150.0))
    );
    assert_eq!(
        (labels[1].label_width_mm, labels[1].label_height_mm),
        (Some(50.0), Some(30.0))
    );
}

#[test]
fn media_inherits_and_uses_the_configured_resolution() {
    let labels = ZplParser::with_dpmm(12)
        .parse_with_options(b"^PW1200^LL1800^XA^FDone^FS^XZ^XA^FDtwo^FS^XZ", options())
        .unwrap();
    assert_eq!(labels.len(), 2);
    for label in labels {
        assert_eq!(label.label_width_mm, Some(100.0));
        assert_eq!(label.label_height_mm, Some(150.0));
    }
    let missing = ZplParser::new()
        .parse_with_options(b"^XA^PW800^FDhi^FS^XZ", options())
        .unwrap();
    assert_eq!(missing[0].label_height_mm, None);
}

#[test]
fn copy_limit_is_cumulative_and_checked_before_expansion() {
    let mut opts = options();
    opts.max_labels = Some(5);
    let err = ZplParser::new()
        .parse_with_options(b"^XA^FDone^FS^PQ3^XZ^XA^FDtwo^FS^PQ3^XZ", opts)
        .unwrap_err();
    assert!(err.contains("5 label limit"));
    let oversized = b"^XA^FDhuge^FS^PQ100000,0,100000^XZ";
    assert!(ZplParser::new()
        .parse_with_options(oversized, opts)
        .is_err());
    assert!(ZplParser::new().parse(oversized).is_err());
    let labels = ZplParser::new()
        .parse_with_options(b"^XA^FDhi^FS^PQ2,0,2^XZ", opts)
        .unwrap();
    assert_eq!(labels.len(), 4);
}

#[test]
fn complete_jobs_reject_partial_or_nested_formats() {
    for source in [
        b"^XA^FDpartial^FS".as_slice(),
        b"^XA^XA^FDnested^FS^XZ",
        b"^XZ",
    ] {
        assert!(ZplParser::new()
            .parse_with_options(source, options())
            .is_err());
    }
    // Legacy consumers keep their permissive framing behavior.
    assert!(ZplParser::new().parse(b"^XA^FDpartial^FS").is_ok());
    // Prefix changes are handled by the existing lexer, not by a job-level regex.
    assert_eq!(
        ZplParser::new()
            .parse_with_options(b"^CC!!XA!PW800!LL1200!FO10,10!FDhello!FS!XZ", options())
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn length_after_first_field_applies_to_next_format() {
    let labels = labelize::ZplParser::with_dpmm(8)
        .parse_with_options(
            b"^XA^LL800,Y^FO0,0^FDone^FS^LL1200^XZ^XA^FO0,0^FDtwo^FS^XZ",
            labelize::ZplParseOptions {
                use_declared_dimensions: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(labels[0].label_height_mm, Some(100.0));
    assert_eq!(labels[1].label_height_mm, Some(150.0));
}
