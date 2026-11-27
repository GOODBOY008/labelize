//! HTTP e2e for POST /encode and POST /template/fill over a real
//! `labelize serve` process. Requires the `serve` feature (CI runs this file
//! in a dedicated step); without it the file compiles to nothing so the
//! default `cargo test --test e2e_*` run is unaffected.
//!
//! The image round-trip contract matches `e2e_reverse.rs`: the encoded code is
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
    start_server_with_playground(true)
}

fn start_server_with_playground(enabled: bool) -> (ServerGuard, reqwest::blocking::Client, String) {
    let port = free_port();
    let child = std::process::Command::new(env!("CARGO_BIN_EXE_labelize"))
        .args(["serve", "--host", "127.0.0.1", "--port", &port.to_string()])
        .env("LABELIZE_PLAYGROUND_ENABLED", enabled.to_string())
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

fn template_request(language: &str, source: &[u8], values: &serde_json::Value) -> Vec<u8> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    serde_json::to_vec(&serde_json::json!({
        "language": language,
        "template_base64": STANDARD.encode(source),
        "values": values,
    }))
    .unwrap()
}

#[test]
fn http_template_fill_returns_native_jobs_for_every_language_and_renders_them() {
    let (_guard, client, base) = start_server();
    let fixtures = render_helpers::testdata_dir().join("templates");
    for record in ["first", "second"] {
        let values = serde_json::from_slice(
            &std::fs::read(fixtures.join(format!("{record}.json"))).unwrap(),
        )
        .unwrap();
        for language in [
            Language::Zpl,
            Language::Tspl,
            Language::Epl,
            Language::Cpcl,
            Language::Dpl,
        ] {
            let name = language.name();
            let printers = fixtures.join("printers");
            let source = std::fs::read(printers.join(format!("shipping.tpl.{name}"))).unwrap();
            let expected = std::fs::read(printers.join(format!("{record}.{name}"))).unwrap();
            let response = client
                .post(format!("{base}/template/fill"))
                .header("Content-Type", "Application/JSON; charset=utf-8")
                .body(template_request(name, &source, &values))
                .send()
                .unwrap();
            assert_eq!(response.status(), 200, "{record} {name}");
            assert_eq!(
                response.headers()["content-type"],
                "application/octet-stream"
            );
            assert_eq!(
                response.headers()["content-disposition"],
                format!(r#"attachment; filename="job.{name}""#)
            );
            let job = response.bytes().unwrap();
            assert_eq!(job.as_ref(), expected, "{record} {name}");

            // The DPL example contains a binary bitmap. Reuse the exact HTTP
            // response for the ordinary conversion route without transcoding.
            if language == Language::Dpl {
                let preview = client
                    .post(format!("{base}/convert?width=100&height=50&dpmm=8"))
                    .header("Content-Type", "application/dpl")
                    .body(job)
                    .send()
                    .unwrap();
                assert_eq!(preview.status(), 200);
                let sdk = render_png(
                    &expected,
                    RenderOptions {
                        language: Some(language),
                        width_mm: Some(100.0),
                        height_mm: Some(50.0),
                        ..Default::default()
                    },
                )
                .unwrap();
                assert_eq!(preview.bytes().unwrap().as_ref(), sdk.bytes);
            }
        }
    }
}

#[test]
fn http_template_fill_reports_input_errors_and_enforces_size_limits() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let (_guard, client, base) = start_server();
    let post = |body: Vec<u8>, media_type: &str| {
        client
            .post(format!("{base}/template/fill"))
            .header("Content-Type", media_type)
            .body(body)
            .send()
            .unwrap()
    };
    assert_eq!(post(b"{}".to_vec(), "text/plain").status(), 415);
    let source = b"^XA\n^FXLabelize field text^FS\n^FO0,0^A0N,10,10^FH^FD{{order_no}}^FS\n^XZ\n";
    for (body, message) in [
        (b"{".to_vec(), "JSON"),
        (
            serde_json::to_vec(
                &serde_json::json!(["zpl", STANDARD.encode(source), {"order_no":"000123"}]),
            )
            .unwrap(),
            "JSON object",
        ),
        (
            template_request("unknown", source, &serde_json::json!({})),
            "language",
        ),
        (
            br#"{"language":"dpl","template_base64":"!","values":{}}"#.to_vec(),
            "template_base64",
        ),
        (
            template_request("zpl", b"garbage", &serde_json::json!({})),
            "template",
        ),
        (
            template_request("zpl", source, &serde_json::json!({"order_no":123})),
            "string",
        ),
        (
            template_request("zpl", source, &serde_json::json!({})),
            "order_no",
        ),
    ] {
        let response = post(body, "application/json");
        assert_eq!(response.status(), 400);
        assert!(response.text().unwrap().contains(message), "{message}");
    }
    let two_fields = b"^XA\n^FXLabelize field text^FS\n^FH^FD{{first}}^FS\n^FXLabelize field text^FS\n^FH^FD{{second}}^FS\n^XZ\n";
    let response = post(
        template_request("zpl", two_fields, &serde_json::json!({})),
        "application/json",
    );
    assert_eq!(response.status(), 400);
    let error = response.text().unwrap();
    assert!(error.contains("first"));
    assert!(
        !error.contains("second"),
        "HTTP stops at the first invalid field"
    );
    let oversized = vec![b' '; labelize::template::MAX_TEMPLATE_REQUEST_BYTES + 1];
    assert_eq!(post(oversized, "application/json").status(), 413);

    // Exactly 512 occurrences of a 4,096-byte value fill the 2 MiB
    // output allowance. A single trailing fixed byte must still count.
    let mut directory = String::from("LABELIZE-TPL/1 tspl US-ASCII\n");
    for i in 0..512 {
        use std::fmt::Write as _;
        writeln!(directory, "{} 5 1 text tspl -", i * 5).unwrap();
    }
    directory.push('\n');
    directory.push_str(&"{{v}}".repeat(512));
    let values = serde_json::json!({"v": "X".repeat(4096)});
    let exact = post(
        template_request("tspl", directory.as_bytes(), &values),
        "application/json",
    );
    assert_eq!(exact.status(), 200);
    assert_eq!(
        exact.bytes().unwrap().len(),
        labelize::template::MAX_TEMPLATE_OUTPUT_BYTES
    );
    directory.push('!');
    let too_large = post(
        template_request("tspl", directory.as_bytes(), &values),
        "application/json",
    );
    assert_eq!(too_large.status(), 413);
    assert_eq!(
        too_large.text().unwrap(),
        "Filled printer job exceeds 2 MiB"
    );
}

#[test]
fn http_openapi_is_shared_with_the_console_and_available_without_the_playground() {
    let expected: serde_json::Value =
        serde_json::from_str(labelize::playground::OPENAPI_JSON).expect("shared OpenAPI");
    for enabled in [true, false] {
        let (_guard, client, base) = start_server_with_playground(enabled);
        let response = client
            .get(format!("{base}/openapi.json"))
            .send()
            .expect("GET /openapi.json");
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-type"], "application/json");
        let actual: serde_json::Value =
            serde_json::from_str(&response.text().unwrap()).expect("schema JSON");
        assert_eq!(actual, expected);
        let root = client.get(&base).send().unwrap().text().unwrap();
        if enabled {
            assert!(root.contains("id=\"nav-api\" href=\"#api\""));
            let embedded = root
                .split("<script id=\"api-data\" type=\"application/json\">")
                .nth(1)
                .unwrap()
                .split("</script>")
                .next()
                .unwrap();
            let embedded: serde_json::Value = serde_json::from_str(embedded).unwrap();
            assert_eq!(embedded, expected);
        } else {
            let listing: serde_json::Value = serde_json::from_str(&root).unwrap();
            assert_eq!(listing["playground"], false);
            assert_eq!(listing["endpoints"]["template_fill"], "POST /template/fill");
            assert_eq!(listing["endpoints"]["openapi"], "GET /openapi.json");
        }
    }
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
