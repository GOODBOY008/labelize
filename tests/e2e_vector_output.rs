//! Exercise real CLI processes and HTTP routes against fixed output baselines.
#![cfg(all(feature = "cli", feature = "vector"))]

#[path = "common/vector_golden.rs"]
mod vector_golden;

use std::{path::PathBuf, process::Command};
use vector_golden as golden;

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "labelize-vector-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn convert(
    input: &std::path::Path,
    output: &std::path::Path,
    args: &[&str],
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_labelize"))
        .arg("convert")
        .arg(input)
        .arg("--output")
        .arg(output)
        .args(args)
        .output()
        .unwrap()
}

fn successful(result: std::process::Output) {
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn cli_vector_png_matches_goldens_for_all_five_languages() {
    let dir = Scratch::new();
    for case in golden::cases() {
        let output = dir.0.join("output.png");
        for scale in [1, 3] {
            let width = case["size_mm"][0].to_string();
            let height = case["size_mm"][1].to_string();
            let scale_arg = scale.to_string();
            let mut args = vec![
                "--mode", "vector", "--scale", &scale_arg, "--width", &width, "--height", &height,
            ];
            if scale == 3 {
                args.push("--antialias");
            }
            successful(convert(&golden::source(&case), &output, &args));
            let png = std::fs::read(&output).unwrap();
            golden::assert_png(&png, &golden::golden_png(&case, scale), golden::name(&case));
            let reader = png::Decoder::new(std::io::Cursor::new(&png))
                .read_info()
                .unwrap();
            let physical = reader.info().pixel_dims.unwrap();
            assert_eq!(
                (physical.xppu, physical.yppu, physical.unit),
                (8000 * scale, 8000 * scale, png::Unit::Meter)
            );
        }
    }
}

#[test]
fn cli_pdf_preserves_mixed_sizes_quantity_order_and_numbered_outputs() {
    let dir = Scratch::new();
    let input = golden::root().join("testdata/vector/mixed_sizes.tspl");
    let output = dir.0.join("batch.pdf");
    let expected = [(40.0, 30.0, "PAGE ONE"), (50.0, 20.0, "PAGE TWO")];
    successful(convert(
        &input,
        &output,
        &["--mode", "vector", "--type", "pdf", "--all-pages"],
    ));
    golden::assert_pdf(&std::fs::read(&output).unwrap(), &expected);
    successful(convert(
        &input,
        &output,
        &["--mode", "vector", "--type", "pdf"],
    ));
    for (index, page) in expected.into_iter().enumerate() {
        golden::assert_pdf(
            &std::fs::read(dir.0.join(format!("batch_{}.pdf", index + 1))).unwrap(),
            &[page],
        );
    }
    successful(convert(
        &input,
        &output,
        &[
            "--mode",
            "vector",
            "--type",
            "pdf",
            "--all-pages",
            "--width",
            "60",
            "--height",
            "25",
        ],
    ));
    golden::assert_pdf(
        &std::fs::read(&output).unwrap(),
        &[(60.0, 25.0, "PAGE ONE"), (60.0, 25.0, "PAGE TWO")],
    );
    successful(convert(
        &golden::root().join("testdata/vector/quantity.zpl"),
        &output,
        &[
            "--mode",
            "vector",
            "--type",
            "pdf",
            "--all-pages",
            "--width",
            "40",
            "--height",
            "30",
        ],
    ));
    golden::assert_pdf(
        &std::fs::read(&output).unwrap(),
        &[
            (40.0, 30.0, "COPY ONE"),
            (40.0, 30.0, "COPY ONE"),
            (40.0, 30.0, "COPY TWO"),
        ],
    );
}

#[test]
fn cli_rejects_invalid_vector_options_without_overwriting_output() {
    let dir = Scratch::new();
    let input = golden::source(&golden::cases()[9]);
    let output = dir.0.join("existing.pdf");
    std::fs::write(&output, b"keep this file").unwrap();
    for args in [
        vec!["--mode", "bogus"],
        vec!["--mode", "vector", "--scale", "0"],
        vec!["--mode", "vector", "--scale", "NaN"],
        vec!["--mode", "vector", "--scale", "10000"],
        vec!["--mode", "vector", "--type", "pdf", "--scale", "2"],
        vec!["--mode", "raster", "--scale", "2"],
        vec!["--mode", "vector", "--all-pages"],
        vec!["--mode", "raster", "--type", "pdf", "--all-pages"],
        vec!["--mode", "vector", "--dpmm", "0"],
        vec!["--mode", "vector", "--width", "inf"],
    ] {
        assert!(
            !convert(&input, &output, &args).status.success(),
            "{args:?}"
        );
        assert_eq!(
            std::fs::read(&output).unwrap(),
            b"keep this file",
            "{args:?}"
        );
    }
}

#[cfg(feature = "serve")]
mod http {
    use super::*;
    use std::{
        process::{Child, Stdio},
        time::{Duration, Instant},
    };

    struct Server {
        child: Child,
        client: reqwest::blocking::Client,
        base: String,
    }
    impl Drop for Server {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
    impl Server {
        fn start() -> Self {
            let port = std::net::TcpListener::bind("127.0.0.1:0")
                .unwrap()
                .local_addr()
                .unwrap()
                .port();
            let child = Command::new(env!("CARGO_BIN_EXE_labelize"))
                .args(["serve", "--host", "127.0.0.1", "--port", &port.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let mut server = Self {
                child,
                base: format!("http://127.0.0.1:{port}"),
                client: reqwest::blocking::Client::builder()
                    .no_proxy()
                    .timeout(Duration::from_secs(10))
                    .build()
                    .unwrap(),
            };
            let start = Instant::now();
            loop {
                if server
                    .client
                    .get(format!("{}/health", server.base))
                    .send()
                    .is_ok_and(|r| r.status().is_success())
                {
                    return server;
                }
                assert!(
                    server.child.try_wait().unwrap().is_none(),
                    "server exited before readiness"
                );
                assert!(
                    start.elapsed() < Duration::from_secs(20),
                    "server did not start"
                );
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        fn post(&self, source: &[u8], language: &str, query: &str) -> reqwest::blocking::Response {
            self.client
                .post(format!("{}/convert?{query}", self.base))
                .header("Content-Type", format!("application/{language}"))
                .body(source.to_vec())
                .send()
                .unwrap()
        }
    }

    #[test]
    fn http_vector_png_and_pdf_match_library_goldens_for_all_five_languages() {
        let server = Server::start();
        for case in golden::cases() {
            let data = std::fs::read(golden::source(&case)).unwrap();
            let language = case["language"].as_str().unwrap();
            let size = format!("width={}&height={}", case["size_mm"][0], case["size_mm"][1]);
            for scale in [1, 3] {
                let response = server.post(
                    &data,
                    language,
                    &format!("mode=vector&scale={scale}&antialias={}&{size}", scale == 3),
                );
                assert_eq!(response.status(), 200, "{}", golden::name(&case));
                assert_eq!(response.headers()["content-type"], "image/png");
                golden::assert_png(
                    &response.bytes().unwrap(),
                    &golden::golden_png(&case, scale),
                    golden::name(&case),
                );
            }
            let response = server.post(&data, language, &format!("mode=vector&output=pdf&{size}"));
            assert_eq!(response.status(), 200);
            assert_eq!(response.headers()["content-type"], "application/pdf");
            let actual = response.bytes().unwrap();
            let expected = labelize::render_pdf_with_output(
                &data,
                golden::options(&case, false),
                golden::output(1.0),
            )
            .unwrap();
            assert!(
                actual.as_ref() == expected.bytes,
                "{}: HTTP PDF differs from library output",
                golden::name(&case)
            );
        }
    }

    #[test]
    fn http_pdf_selects_first_or_all_pages_and_applies_size_overrides() {
        let server = Server::start();
        let source =
            std::fs::read(golden::root().join("testdata/vector/mixed_sizes.tspl")).unwrap();
        for query in [
            "mode=vector&output=pdf",
            "mode=vector&output=pdf&pages=first",
        ] {
            let response = server.post(&source, "tspl", query);
            assert_eq!(response.status(), 200);
            golden::assert_pdf(&response.bytes().unwrap(), &[(40.0, 30.0, "PAGE ONE")]);
        }
        let response = server.post(&source, "tspl", "mode=vector&output=pdf&pages=all");
        assert_eq!(response.status(), 200);
        golden::assert_pdf(
            &response.bytes().unwrap(),
            &[(40.0, 30.0, "PAGE ONE"), (50.0, 20.0, "PAGE TWO")],
        );
        let response = server.post(
            &source,
            "tspl",
            "mode=vector&output=pdf&pages=all&width=60&height=25",
        );
        assert_eq!(response.status(), 200);
        golden::assert_pdf(
            &response.bytes().unwrap(),
            &[(60.0, 25.0, "PAGE ONE"), (60.0, 25.0, "PAGE TWO")],
        );
        let source = std::fs::read(golden::root().join("testdata/vector/quantity.zpl")).unwrap();
        let response = server.post(
            &source,
            "zpl",
            "mode=vector&output=pdf&pages=all&width=40&height=30",
        );
        assert_eq!(response.status(), 200);
        golden::assert_pdf(
            &response.bytes().unwrap(),
            &[
                (40.0, 30.0, "COPY ONE"),
                (40.0, 30.0, "COPY ONE"),
                (40.0, 30.0, "COPY TWO"),
            ],
        );
    }

    #[test]
    fn http_rejects_invalid_vector_requests_with_400() {
        let server = Server::start();
        for query in [
            "mode=bogus",
            "mode=vector&scale=0",
            "mode=vector&scale=NaN",
            "mode=vector&scale=10000",
            "mode=vector&output=pdf&scale=2",
            "mode=raster&scale=2",
            "pages=all",
            "mode=vector&pages=all",
            "mode=vector&width=inf",
            "mode=vector&dpmm=0",
            "output=svg",
            "pages=last",
        ] {
            assert_eq!(
                server
                    .post(b"^XA^FO20,20^FDTest^FS^XZ", "zpl", query)
                    .status(),
                400,
                "{query}"
            );
        }
    }
}
