#[path = "common/fixture_test_dir.rs"]
mod fixture_test_dir;
#[path = "common/golden_fixture.rs"]
mod golden_fixture;
#[path = "common/reference_generation.rs"]
mod reference_generation;

use fixture_test_dir::{png, TestDir};
use golden_fixture::{discover_inputs, load_fixture, resolve_input};
use reference_generation::write_labelary_reference;

#[test]
fn missing_inputs_and_references_are_errors_without_creating_files() {
    let dir = TestDir::new();
    for ext in ["zpl", "epl"] {
        let input = dir.0.join(format!("missing.{ext}"));
        let error = resolve_input(&dir.0, "missing", ext).unwrap_err();
        assert!(error.contains(&input.display().to_string()), "{error}");
        assert!(load_fixture(&input)
            .err()
            .unwrap()
            .contains("cannot read golden input"));
        std::fs::write(&input, "input").unwrap();
        let reference = input.with_extension("png");
        let error = load_fixture(&input).err().unwrap();
        assert!(error.contains(&reference.display().to_string()), "{error}");
        assert!(!reference.exists());
    }
}

#[test]
fn corrupt_reference_and_unreadable_input_identify_the_path() {
    let dir = TestDir::new();
    let input = dir.0.join("case.zpl");
    std::fs::write(&input, "^XA^XZ").unwrap();
    let reference = input.with_extension("png");
    std::fs::write(&reference, b"not a PNG").unwrap();
    let error = load_fixture(&input).err().unwrap();
    assert!(
        error.contains("invalid golden reference")
            && error.contains(&reference.display().to_string()),
        "{error}"
    );
    // A directory in place of a file fails on Windows and Unix, including as root.
    let unreadable = dir.0.join("directory.zpl");
    std::fs::create_dir(&unreadable).unwrap();
    let error = load_fixture(&unreadable).err().unwrap();
    assert!(error.contains(&unreadable.display().to_string()), "{error}");
    assert_eq!(std::fs::read(&reference).unwrap(), b"not a PNG");
}

#[test]
fn valid_fixture_loading_preserves_inputs_and_references() {
    let dir = TestDir::new();
    std::fs::create_dir(dir.0.join("unit")).unwrap();
    for ext in ["zpl", "epl"] {
        let input = dir.0.join("unit").join(format!("case.{ext}"));
        std::fs::write(&input, "content").unwrap();
        std::fs::write(input.with_extension("png"), png()).unwrap();
        let (resolved, unit) = resolve_input(&dir.0, "case", ext).unwrap();
        assert_eq!(resolved, input);
        assert!(unit);
        let fixture = load_fixture(&resolved).unwrap();
        assert_eq!(fixture.content, "content");
        assert_eq!(fixture.reference, png());
        assert_eq!(std::fs::read(input.with_extension("png")).unwrap(), png());
    }
}

#[test]
fn discovery_rejects_missing_or_empty_directories() {
    let dir = TestDir::new();
    assert!(discover_inputs(std::slice::from_ref(&dir.0))
        .unwrap_err()
        .contains("no ZPL/EPL"));
    let missing = dir.0.join("missing");
    assert!(discover_inputs(std::slice::from_ref(&missing))
        .unwrap_err()
        .contains(&missing.display().to_string()));
    std::fs::write(dir.0.join("b.epl"), "N").unwrap();
    std::fs::write(dir.0.join("a.zpl"), "^XA^XZ").unwrap();
    assert_eq!(
        discover_inputs(std::slice::from_ref(&dir.0)).unwrap(),
        vec![dir.0.join("a.zpl"), dir.0.join("b.epl")]
    );
}

#[test]
fn failed_reference_fetch_never_creates_or_replaces_a_png() {
    let dir = TestDir::new();
    let input = dir.0.join("case.zpl");
    let output = input.with_extension("png");
    std::fs::write(&input, "^XA^XZ").unwrap();
    let error = write_labelary_reference(&input, false, |_| None).unwrap_err();
    assert!(error.contains("Labelary reference unavailable"), "{error}");
    assert!(!output.exists());
    std::fs::write(&output, png()).unwrap();
    assert!(write_labelary_reference(&input, true, |_| None).is_err());
    assert_eq!(std::fs::read(output).unwrap(), png());
}

#[test]
fn invalid_download_never_creates_or_replaces_a_reference() {
    let dir = TestDir::new();
    let input = dir.0.join("case.zpl");
    let output = input.with_extension("png");
    std::fs::write(&input, "^XA^XZ").unwrap();
    for existing in [false, true] {
        if existing {
            std::fs::write(&output, png()).unwrap();
        }
        let error = write_labelary_reference(&input, true, |_| Some(b"HTTP error page".to_vec()))
            .unwrap_err();
        assert!(error.contains("invalid Labelary PNG"), "{error}");
        if existing {
            assert_eq!(std::fs::read(&output).unwrap(), png());
        } else {
            assert!(!output.exists());
        }
    }
}

#[test]
fn explicit_generation_writes_valid_png_and_retains_existing_without_fetch() {
    let dir = TestDir::new();
    let input = dir.0.join("case.zpl");
    std::fs::write(&input, "^XA^XZ").unwrap();
    assert!(write_labelary_reference(&input, false, |zpl| {
        assert_eq!(zpl, "^XA^XZ");
        Some(png())
    })
    .unwrap());
    assert_eq!(std::fs::read(input.with_extension("png")).unwrap(), png());
    assert!(!write_labelary_reference(&input, false, |_| panic!(
        "existing reference must not be fetched"
    ))
    .unwrap());
}
