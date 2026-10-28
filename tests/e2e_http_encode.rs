//! HTTP e2e for POST /encode: image bytes in, printer code out, over a real
//! `labelize serve` process. Requires the `serve` feature (CI runs this file
//! in a dedicated step); without it the file compiles to nothing so the
//! default `cargo test --test e2e_*` run is unaffected.
//!
//! The round-trip contract matches `e2e_reverse.rs`: the encoded code is
//! rendered through the forward pipeline and must reproduce the binarized
//! source pixel-for-pixel — this time crossing the axum route, query-param
//! parsing and content-type negotiation on the way.

#![cfg(feature = "serve")]

mod common;

use common::render_helpers;
use common::reverse_check::assert_round_trip;
use labelize::{render_png, Language, RenderOptions};

struct ServerGuard {
    child: std::process::Child,
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Grab an OS-assigned port, then release it for the server to bind. The
/// tiny TOCTOU window is the standard trick and fine for a test.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .expect("bind for port probe")
        .local_addr()
        .expect("local addr")
        .port()
}

fn start_server() -> (ServerGuard, reqwest::blocking::Client, String) {
    let port = free_port();
    let child = std::process::Command::new(env!("CARGO_BIN_EXE_labelize"))
        .args(["serve", "--host", "127.0.0.1", "--port", &port.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn labelize serve (requires --features serve)");
    let guard = ServerGuard { child };
    let base = format!("http://127.0.0.1:{port}");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("http client");

    for _ in 0..150 {
        if let Ok(resp) = client.get(format!("{base}/health")).send() {
            if resp.status().is_success() {
                return (guard, client, base);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("labelize serve did not become healthy on {base}");
}

fn fixture_png() -> Vec<u8> {
    std::fs::read(render_helpers::testdata_dir().join("labels/amazon.png"))
        .expect("amazon.png fixture")
}

#[test]
fn http_encode_zpl_round_trip_is_pixel_identical() {
    let (_guard, client, base) = start_server();
    let source = image::load_from_memory(&fixture_png())
        .expect("decode fixture")
        .to_rgba8();

    let resp = client
        .post(format!("{base}/encode?dpmm=8&threshold=128&language=zpl"))
        .header("Content-Type", "application/octet-stream")
        .body(fixture_png())
        .send()
        .expect("POST /encode");
    assert_eq!(resp.status(), 200);
    assert!(resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .starts_with("text/plain"));
    let zpl = resp.text().expect("zpl body");
    assert!(
        zpl.contains("^GFA,"),
        "response must be a ^GFA graphic field"
    );

    // The generated ZPL leaves the size unset: render at the image's physical size (the
    // documented contract), exactly like the playground does after encoding.
    let out = render_png(
        zpl.as_bytes(),
        RenderOptions {
            language: Some(Language::Zpl),
            width_mm: Some(101.625), // 813 px @ 8 dpmm
            height_mm: Some(203.25), // 1626 px @ 8 dpmm
            dpmm: 8,
            antialias: false,
        },
    )
    .expect("render generated ZPL");
    assert_round_trip(&source, &out.bytes, "HTTP /encode zpl round trip");
}

#[test]
fn http_encode_tspl_round_trip_is_pixel_identical() {
    let (_guard, client, base) = start_server();
    let source = image::load_from_memory(&fixture_png())
        .expect("decode fixture")
        .to_rgba8();

    let resp = client
        .post(format!("{base}/encode?language=tspl"))
        .header("Content-Type", "application/octet-stream")
        .body(fixture_png())
        .send()
        .expect("POST /encode tspl");
    assert_eq!(resp.status(), 200);
    assert!(resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .starts_with("application/octet-stream"));
    let tspl = resp.bytes().expect("tspl body");
    assert!(tspl.starts_with(b"SIZE "), "tspl must start with SIZE");

    // The generated SIZE declares the canvas, so defaults suffice.
    let out = render_png(
        &tspl,
        RenderOptions {
            language: Some(Language::Tspl),
            dpmm: 8,
            ..RenderOptions::default()
        },
    )
    .expect("render generated TSPL");
    assert_round_trip(&source, &out.bytes, "HTTP /encode tspl round trip");
}

#[test]
fn http_encode_rejects_bad_input_with_400() {
    let (_guard, client, base) = start_server();

    let resp = client
        .post(format!("{base}/encode"))
        .header("Content-Type", "application/octet-stream")
        .body(b"definitely not an image".to_vec())
        .send()
        .expect("POST /encode garbage");
    assert_eq!(resp.status(), 400);

    let resp = client
        .post(format!("{base}/encode?language=epl"))
        .header("Content-Type", "application/octet-stream")
        .body(b"\x89PNG\r\n\x1a\nminimal".to_vec())
        .send()
        .expect("POST /encode bad language");
    assert_eq!(resp.status(), 400);
    let body = resp.text().expect("error body");
    assert!(body.contains("expected zpl or tspl"), "got: {body}");

    let resp = client
        .post(format!("{base}/encode?dpmm=0"))
        .header("Content-Type", "application/octet-stream")
        .body(b"\x89PNG\r\n\x1a\nminimal".to_vec())
        .send()
        .expect("POST /encode dpmm=0");
    assert_eq!(resp.status(), 400);
}

#[test]
fn http_encode_validates_all_options_and_preserves_threshold_extremes() {
    let (_guard, client, base) = start_server();
    let source = image::RgbaImage::from_pixel(1, 1, image::Rgba([127, 127, 127, 255]));
    let mut png = Vec::new();
    labelize::encode_png(&source, &mut png).unwrap();
    for query in [
        "dpmm=0",
        "dpmm=-1",
        "dpmm=8junk",
        "dpmm=1.5",
        "dpmm=2147483648",
        "dpmm=",
        "threshold=-0",
        "threshold=-1",
        "threshold=256",
        "threshold=2.5",
        "threshold=abc",
        "threshold=",
        "language=",
        "language=epl",
        "dpmm=8&dpmm=12",
        "language=zpl&language=tspl",
    ] {
        let response = client
            .post(format!("{base}/encode?{query}"))
            .body(png.clone())
            .send()
            .unwrap();
        assert_eq!(response.status(), 400, "{query}");
    }
    for (threshold, expected) in [(0, "00"), (255, "80")] {
        let response = client
            .post(format!("{base}/encode?threshold={threshold}&language=ZPL"))
            .body(png.clone())
            .send()
            .unwrap();
        assert_eq!(response.status(), 200);
        assert!(response
            .text()
            .unwrap()
            .contains(&format!("\n{expected}\n")));
    }
    let response = client
        .post(format!("{base}/encode"))
        .body(b"P6\n4097 4096\n255\n".to_vec())
        .send()
        .unwrap();
    assert_eq!(response.status(), 400);
    assert!(response.text().unwrap().contains("16 Mi pixels"));
}
