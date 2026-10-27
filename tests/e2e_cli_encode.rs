#![cfg(feature = "cli")]

use labelize::{encode_image_bytes, ReverseLanguage, ReverseOptions};
use std::{path::Path, process::Command};

struct Scratch(std::path::PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn encode(input: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_labelize"))
        .arg("encode")
        .arg(input)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn cli_encode_selects_the_target_and_preserves_existing_output_on_errors() {
    let dir = Scratch(std::env::temp_dir().join(format!(
            "labelize-encode-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )));
    std::fs::create_dir(&dir.0).unwrap();
    let input = dir.0.join("image.png");
    let image = image::RgbaImage::from_pixel(13, 7, image::Rgba([127, 127, 127, 255]));
    let mut png = Vec::new();
    labelize::encode_png(&image, &mut png).unwrap();
    std::fs::write(&input, &png).unwrap();
    let expected =
        |language| encode_image_bytes(&png, &ReverseOptions::default(), language).unwrap();

    assert!(encode(&input, &[]).status.success());
    assert_eq!(
        std::fs::read(input.with_extension("zpl")).unwrap(),
        expected(ReverseLanguage::Zpl)
    );
    assert!(encode(&input, &["-l", "tspl"]).status.success());
    assert_eq!(
        std::fs::read(input.with_extension("tspl")).unwrap(),
        expected(ReverseLanguage::Tspl)
    );
    let output = dir.0.join("inferred.TSC");
    assert!(encode(&input, &["-o", output.to_str().unwrap()])
        .status
        .success());
    assert_eq!(
        std::fs::read(&output).unwrap(),
        expected(ReverseLanguage::Tspl)
    );
    assert!(
        encode(&input, &["-o", output.to_str().unwrap(), "-l", "zpl"])
            .status
            .success()
    );
    assert_eq!(
        std::fs::read(&output).unwrap(),
        expected(ReverseLanguage::Zpl)
    );

    for args in [
        vec!["--dpmm", "0"],
        vec!["--threshold", "256"],
        vec!["-l", "epl"],
    ] {
        let mut options = vec!["-o", output.to_str().unwrap()];
        options.extend(args);
        assert!(!encode(&input, &options).status.success());
        assert_eq!(
            std::fs::read(&output).unwrap(),
            expected(ReverseLanguage::Zpl)
        );
    }
    std::fs::write(&input, b"not an image").unwrap();
    assert!(!encode(&input, &["-o", output.to_str().unwrap()])
        .status
        .success());
    assert_eq!(
        std::fs::read(&output).unwrap(),
        expected(ReverseLanguage::Zpl)
    );
}
