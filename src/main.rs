#[cfg(feature = "cli")]
use std::fs;
#[cfg(feature = "cli")]
use std::io::Cursor;
#[cfg(feature = "cli")]
use std::path::{Path, PathBuf};

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand, ValueEnum};
#[cfg(any(feature = "cli", feature = "serve"))]
use labelize::{DrawerOptions, EplParser, LabelInfo, Renderer, ZplParser};

#[cfg(feature = "serve")]
mod manage {
    use std::sync::atomic::AtomicU64;
    use std::time::Instant;

    pub struct Metrics {
        pub total_requests: AtomicU64,
        pub total_renders: AtomicU64,
        pub pdf_renders: AtomicU64,
        pub failed_renders: AtomicU64,
        pub total_render_time_us: AtomicU64,
    }

    impl Default for Metrics {
        fn default() -> Self {
            Self {
                total_requests: AtomicU64::new(0),
                total_renders: AtomicU64::new(0),
                pdf_renders: AtomicU64::new(0),
                failed_renders: AtomicU64::new(0),
                total_render_time_us: AtomicU64::new(0),
            }
        }
    }

    pub struct AppState {
        pub metrics: Metrics,
        pub started_at: Instant,
    }

    pub fn app_state() -> AppState {
        AppState {
            metrics: Metrics::default(),
            started_at: Instant::now(),
        }
    }
}

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(
    name = "labelize",
    version,
    about = "Turn ZPL/EPL into pixels — label rendering, simplified."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Clone, Copy, ValueEnum)]
enum InputFormat {
    Zpl,
    Epl,
}

#[cfg(feature = "cli")]
#[derive(Clone, Copy, ValueEnum)]
enum OutputType {
    Png,
    Pdf,
}

#[cfg(feature = "cli")]
#[derive(Subcommand)]
enum Commands {
    /// Convert a ZPL/EPL file to PNG or PDF
    Convert {
        /// Input file path (.zpl or .epl)
        input: PathBuf,

        /// Output file path (default: input stem + .png/.pdf)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Input format (auto-detected from extension if omitted)
        #[arg(short, long)]
        format: Option<InputFormat>,

        /// Output type
        #[arg(short = 't', long = "type", default_value = "png")]
        output_type: OutputType,

        /// Label width in mm
        #[arg(long, default_value_t = 102.0)]
        width: f64,

        /// Label height in mm
        #[arg(long, default_value_t = 152.0)]
        height: f64,

        /// Dots per mm (6, 8, 12, or 24)
        #[arg(long, default_value_t = 8)]
        dpmm: i32,
    },

    /// Start HTTP server for label conversion
    #[cfg(feature = "serve")]
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Port to listen on
        #[arg(short, long, default_value_t = 8080)]
        port: u16,
    },
}

#[cfg(feature = "cli")]
fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert {
            input,
            output,
            format,
            output_type,
            width,
            height,
            dpmm,
        } => {
            if let Err(e) = convert_file(
                &input,
                output.as_deref(),
                format,
                output_type,
                width,
                height,
                dpmm,
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        #[cfg(feature = "serve")]
        Commands::Serve { host, port } => {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            rt.block_on(serve(host, port));
        }
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("CLI not available. Rebuild with: cargo build --features cli");
    std::process::exit(1);
}

#[cfg(feature = "cli")]
fn detect_format(path: &Path, override_fmt: Option<InputFormat>) -> InputFormat {
    if let Some(fmt) = override_fmt {
        return fmt;
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_lowercase().as_str() {
        "epl" => InputFormat::Epl,
        _ => InputFormat::Zpl,
    }
}

#[cfg(feature = "cli")]
fn parse_labels(content: &[u8], format: InputFormat, dpmm: i32) -> Result<Vec<LabelInfo>, String> {
    match format {
        InputFormat::Epl => EplParser::new().parse(content),
        // dpmm up front: ^MU may express coordinates in mm/inches.
        InputFormat::Zpl => ZplParser::with_dpmm(dpmm).parse(content),
    }
}

#[cfg(feature = "cli")]
fn output_extension(output_type: OutputType) -> &'static str {
    match output_type {
        OutputType::Png => "png",
        OutputType::Pdf => "pdf",
    }
}

#[cfg(feature = "cli")]
fn default_output_path(input: &Path, output_type: OutputType, index: Option<usize>) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let ext = output_extension(output_type);
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    match index {
        Some(i) => parent.join(format!("{}_{}.{}", stem, i + 1, ext)),
        None => parent.join(format!("{}.{}", stem, ext)),
    }
}

#[cfg(feature = "cli")]
fn render_label(
    label: &LabelInfo,
    options: &DrawerOptions,
    output_type: OutputType,
) -> Result<Vec<u8>, String> {
    let renderer = Renderer::new();
    let mut buf = Cursor::new(Vec::new());
    match output_type {
        OutputType::Png => renderer.draw_label_as_png(label, &mut buf, options.clone())?,
        OutputType::Pdf => {
            renderer.draw_label_as_png(label, &mut buf, options.clone())?;
            let img = image::load_from_memory(&buf.into_inner())
                .map_err(|e| format!("Failed to decode rendered image: {}", e))?
                .to_rgba8();
            let mut pdf_buf = Cursor::new(Vec::new());
            labelize::encode_pdf(&img, options, &mut pdf_buf)
                .map_err(|e| format!("Failed to encode PDF: {}", e))?;
            return Ok(pdf_buf.into_inner());
        }
    }
    Ok(buf.into_inner())
}

#[cfg(feature = "cli")]
fn convert_file(
    input: &Path,
    output: Option<&Path>,
    format: Option<InputFormat>,
    output_type: OutputType,
    width: f64,
    height: f64,
    dpmm: i32,
) -> Result<(), String> {
    let content = fs::read(input).map_err(|e| format!("Failed to read input file: {}", e))?;

    let fmt = detect_format(input, format);
    let labels = parse_labels(&content, fmt, dpmm)?;

    if labels.is_empty() {
        return Err("No labels found in input".to_string());
    }

    let options = DrawerOptions {
        label_width_mm: width,
        label_height_mm: height,
        dpmm,
        ..Default::default()
    };

    let multi = labels.len() > 1;
    for (i, label) in labels.iter().enumerate() {
        let out_path = match output {
            Some(p) if !multi => p.to_path_buf(),
            Some(p) => {
                let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
                let ext = p
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or(output_extension(output_type));
                let parent = p.parent().unwrap_or_else(|| Path::new("."));
                parent.join(format!("{}_{}.{}", stem, i + 1, ext))
            }
            None => default_output_path(input, output_type, if multi { Some(i) } else { None }),
        };

        let data = render_label(label, &options, output_type)?;
        fs::write(&out_path, data).map_err(|e| format!("Failed to write output file: {}", e))?;
        println!("Converted {} -> {}", input.display(), out_path.display());
    }

    Ok(())
}

#[cfg(feature = "serve")]
async fn serve(host: String, port: u16) {
    use axum::{
        body::Bytes,
        extract::{Query, State},
        http::{header, HeaderMap, StatusCode},
        response::IntoResponse,
        routing::{get, post},
        Router,
    };
    use manage::AppState;

    async fn playground_page() -> impl IntoResponse {
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            labelize::playground::PLAYGROUND_HTML,
        )
    }

    async fn health() -> impl IntoResponse {
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"status":"ok"}"#,
        )
    }

    async fn manage_health() -> impl IntoResponse {
        let (disk_total, disk_free) = {
            #[cfg(unix)]
            {
                use std::ffi::CString;
                let path = CString::new("/").unwrap();
                let mut buf = unsafe { std::mem::zeroed::<libc::statvfs>() };
                let rc = unsafe { libc::statvfs(path.as_ptr(), &mut buf) };
                if rc == 0 {
                    let total = buf.f_blocks as u64 * buf.f_frsize as u64;
                    let free = buf.f_bavail as u64 * buf.f_frsize as u64;
                    (total, free)
                } else {
                    (0, 0)
                }
            }
            #[cfg(not(unix))]
            {
                (0, 0)
            }
        };
        let disk_up = disk_free > 10 * 1024 * 1024;
        let body = serde_json::json!({
            "status": "UP",
            "components": {
                "diskSpace": {
                    "status": if disk_up { "UP" } else { "DOWN" },
                    "details": {
                        "total": disk_total,
                        "free": disk_free,
                        "threshold": 10485760,
                        "exists": true
                    }
                }
            }
        });
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body.to_string(),
        )
    }

    async fn manage_info() -> impl IntoResponse {
        let tz = std::env::var("TZ").unwrap_or_else(|_| "UTC".to_string());
        let body = serde_json::json!({
            "app": {
                "name": "labelize-service",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "build": {
                "rustc": "rustc",
                "target": std::env::consts::ARCH,
                "os": std::env::consts::OS,
            },
            "labelize": {
                "version": env!("CARGO_PKG_VERSION"),
            },
            "environment": {
                "timezone": tz,
                "lang": std::env::var("LANG").unwrap_or_default(),
            }
        });
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body.to_string(),
        )
    }

    async fn manage_env() -> impl IntoResponse {
        let env_vars: Vec<(String, String)> = std::env::vars()
            .filter(|(k, _)| {
                let ku = k.to_uppercase();
                if ku.contains("TOKEN")
                    || ku.contains("KEY")
                    || ku.contains("PASSWORD")
                    || ku.contains("SECRET")
                    || ku.contains("CREDENTIAL")
                {
                    return false;
                }
                ku.starts_with("LABELIZE_")
                    || ku.starts_with("RUST_")
                    || ku == "PATH"
                    || ku == "HOME"
                    || ku == "HOSTNAME"
                    || ku == "LANG"
                    || ku == "LANGUAGE"
                    || ku == "TZ"
                    || ku == "LC_ALL"
                    || ku == "LC_CTYPE"
                    || ku == "TERM"
                    || ku == "USER"
                    || ku == "SHELL"
            })
            .collect();
        let mut env_map: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        for (k, v) in env_vars {
            env_map.insert(k, v);
        }
        let body = serde_json::json!({
            "activeProfiles": [],
            "propertySources": [{
                "name": "systemEnvironment",
                "properties": env_map,
            }]
        });
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body.to_string(),
        )
    }

    async fn manage_metrics(State(state): State<std::sync::Arc<AppState>>) -> impl IntoResponse {
        use std::sync::atomic::Ordering;
        let m = &state.metrics;
        let total_renders = m.total_renders.load(Ordering::Relaxed);
        let total_time_ms = m.total_render_time_us.load(Ordering::Relaxed) as f64 / 1000.0;
        let avg_time_ms = if total_renders > 0 {
            total_time_ms / total_renders as f64
        } else {
            0.0
        };
        let uptime_s = state.started_at.elapsed().as_secs();

        let body = serde_json::json!({
            "labelize.render.total": total_renders,
            "labelize.render.pdf": m.pdf_renders.load(Ordering::Relaxed),
            "labelize.render.failed": m.failed_renders.load(Ordering::Relaxed),
            "labelize.requests.total": m.total_requests.load(Ordering::Relaxed),
            "labelize.render.time.total_ms": total_time_ms.round() as u64,
            "labelize.render.time.avg_ms": avg_time_ms.round() as u64,
            "labelize.uptime.seconds": uptime_s,
        });
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body.to_string(),
        )
    }

    #[derive(serde::Deserialize)]
    struct ConvertParams {
        #[serde(default = "default_width")]
        width: f64,
        #[serde(default = "default_height")]
        height: f64,
        #[serde(default = "default_dpmm")]
        dpmm: i32,
        #[serde(default)]
        output: Option<String>,
        #[serde(default)]
        antialias: bool,
    }

    fn default_width() -> f64 {
        102.0
    }
    fn default_height() -> f64 {
        152.0
    }
    fn default_dpmm() -> i32 {
        8
    }

    async fn convert_handler(
        State(state): State<std::sync::Arc<AppState>>,
        headers: HeaderMap,
        Query(params): Query<ConvertParams>,
        body: Bytes,
    ) -> impl IntoResponse {
        use std::sync::atomic::Ordering;

        state.metrics.total_requests.fetch_add(1, Ordering::Relaxed);
        let render_start = std::time::Instant::now();
        // Detect format from Content-Type header
        let content_type = headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let labels = if content_type.contains("epl") {
            EplParser::new().parse(&body)
        } else {
            ZplParser::with_dpmm(params.dpmm).parse(&body)
        };

        let labels = match labels {
            Ok(l) => l,
            Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
        };

        let label = match labels.into_iter().next() {
            Some(l) => l,
            None => {
                return (StatusCode::BAD_REQUEST, "No labels found".to_string()).into_response()
            }
        };

        let options = DrawerOptions {
            label_width_mm: params.width,
            label_height_mm: params.height,
            dpmm: params.dpmm,
            antialias: params.antialias,
            ..Default::default()
        };

        let want_pdf = params.output.as_deref() == Some("pdf");

        let renderer = Renderer::new();

        let mut buf = Cursor::new(Vec::new());
        if let Err(e) = renderer.draw_label_as_png(&label, &mut buf, options.clone()) {
            state.metrics.failed_renders.fetch_add(1, Ordering::Relaxed);
            return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
        }

        if want_pdf {
            state.metrics.pdf_renders.fetch_add(1, Ordering::Relaxed);
            state.metrics.total_renders.fetch_add(1, Ordering::Relaxed);
            state
                .metrics
                .total_render_time_us
                .fetch_add(render_start.elapsed().as_micros() as u64, Ordering::Relaxed);

            let img = match image::load_from_memory(&buf.into_inner()) {
                Ok(img) => img.to_rgba8(),
                Err(e) => {
                    state.metrics.failed_renders.fetch_add(1, Ordering::Relaxed);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("image decode: {}", e),
                    )
                        .into_response();
                }
            };
            let mut pdf_buf = Cursor::new(Vec::new());
            match labelize::encode_pdf(&img, &options, &mut pdf_buf) {
                Ok(_) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/pdf")],
                    pdf_buf.into_inner(),
                )
                    .into_response(),
                Err(e) => {
                    state.metrics.failed_renders.fetch_add(1, Ordering::Relaxed);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("pdf encode: {}", e),
                    )
                        .into_response()
                }
            }
        } else {
            state.metrics.total_renders.fetch_add(1, Ordering::Relaxed);
            state
                .metrics
                .total_render_time_us
                .fetch_add(render_start.elapsed().as_micros() as u64, Ordering::Relaxed);
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "image/png")],
                buf.into_inner(),
            )
                .into_response()
        }
    }

    let shared_state = std::sync::Arc::new(manage::app_state());

    let app = Router::new()
        .route("/", get(playground_page))
        .route("/health", get(health))
        .route("/convert", post(convert_handler))
        .route("/manage/health", get(manage_health))
        .route("/manage/info", get(manage_info))
        .route("/manage/env", get(manage_env))
        .route("/manage/metrics", get(manage_metrics))
        .with_state(shared_state);

    let addr = format!("{}:{}", host, port);
    println!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");
    axum::serve(listener, app).await.expect("Server failed");
}
