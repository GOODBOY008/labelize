#[cfg(feature = "cli")]
use std::fs;
#[cfg(feature = "cli")]
use std::io::Cursor;
#[cfg(feature = "cli")]
use std::path::{Path, PathBuf};

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand, ValueEnum};
#[cfg(any(feature = "cli", feature = "serve"))]
use labelize::{parse_labels, DrawerOptions, LabelInfo, Language, Renderer, ReverseLanguage};

/// Historical default label dimensions, used when neither an explicit
/// override nor the label's own declared size (TSPL SIZE) is available.
#[cfg(any(feature = "cli", feature = "serve"))]
const DEFAULT_LABEL_WIDTH_MM: f64 = 102.0;
#[cfg(any(feature = "cli", feature = "serve"))]
const DEFAULT_LABEL_HEIGHT_MM: f64 = 152.0;

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(
    name = "labelize",
    version,
    about = "Turn ZPL/EPL/TSPL/CPCL/DPL into pixels — label rendering, simplified."
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
    Tspl,
    Cpcl,
    Dpl,
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
    /// Convert a ZPL/EPL/TSPL/CPCL/DPL file to PNG or PDF
    Convert {
        /// Input file path (.zpl, .epl, .tspl, .cpcl, or .dpl)
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

        /// Label width override in mm (defaults to the label's own SIZE
        /// for TSPL, else 102)
        #[arg(long)]
        width: Option<f64>,

        /// Label height override in mm (defaults to the label's own SIZE
        /// for TSPL, else 152)
        #[arg(long)]
        height: Option<f64>,

        /// Dots per mm (6, 8, 12, or 24)
        #[arg(long, default_value_t = 8)]
        dpmm: i32,

        /// Emit 8-bit grayscale output preserving antialiasing (default: 1-bit)
        #[arg(long)]
        antialias: bool,
    },

    /// Convert a raster image into ZPL or TSPL printer code (reverse rendering)
    Encode {
        /// Input image file path (PNG, JPEG, GIF, BMP, …)
        input: PathBuf,

        /// Output file path (default: input stem + .zpl/.tspl)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Target printer language (default: from the output extension, else ZPL)
        #[arg(short, long)]
        language: Option<ReverseLanguage>,

        /// Positive dots per millimetre; one image pixel = one dot
        #[arg(long, default_value_t = 8)]
        dpmm: i32,

        /// Luminance threshold (0-255): pixels darker than this print as ink
        #[arg(long, default_value_t = 128)]
        threshold: u8,
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
            antialias,
        } => {
            if let Err(e) = convert_file(
                &input,
                output.as_deref(),
                format,
                output_type,
                width,
                height,
                dpmm,
                antialias,
            ) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Encode {
            input,
            output,
            language,
            dpmm,
            threshold,
        } => {
            if let Err(e) = encode_file(&input, output.as_deref(), language, dpmm, threshold) {
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
impl InputFormat {
    fn language(self) -> Language {
        match self {
            InputFormat::Zpl => Language::Zpl,
            InputFormat::Epl => Language::Epl,
            InputFormat::Tspl => Language::Tspl,
            InputFormat::Cpcl => Language::Cpcl,
            InputFormat::Dpl => Language::Dpl,
        }
    }
}

#[cfg(feature = "cli")]
fn detect_format(path: &Path, override_fmt: Option<InputFormat>, content: &[u8]) -> InputFormat {
    if let Some(fmt) = override_fmt {
        return fmt;
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext.to_lowercase().as_str() {
        "zpl" => InputFormat::Zpl,
        "epl" => InputFormat::Epl,
        "tspl" => InputFormat::Tspl,
        "cpcl" => InputFormat::Cpcl,
        "dpl" => InputFormat::Dpl,
        // Unknown extension: sniff the content instead of assuming ZPL.
        _ => match labelize::detect_language(content) {
            Language::Epl => InputFormat::Epl,
            Language::Tspl => InputFormat::Tspl,
            Language::Cpcl => InputFormat::Cpcl,
            Language::Dpl => InputFormat::Dpl,
            Language::Zpl => InputFormat::Zpl,
        },
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
fn default_output_path(input: &Path, ext: &str, index: Option<usize>) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    match index {
        Some(i) => parent.join(format!("{}_{}.{}", stem, i + 1, ext)),
        None => parent.join(format!("{}.{}", stem, ext)),
    }
}

#[cfg(feature = "cli")]
fn encode_file(
    input: &Path,
    output: Option<&Path>,
    language: Option<ReverseLanguage>,
    dpmm: i32,
    threshold: u8,
) -> Result<(), String> {
    let image_bytes = fs::read(input).map_err(|e| format!("Failed to read input file: {}", e))?;

    // Language resolves from the flag, else the output extension, else ZPL.
    let language = language.or_else(|| {
        let ext = output
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        match ext.as_str() {
            "tspl" | "tsc" => Some(ReverseLanguage::Tspl),
            "zpl" => Some(ReverseLanguage::Zpl),
            _ => None,
        }
    });
    let language = language.unwrap_or(ReverseLanguage::Zpl);
    let options = labelize::ReverseOptions { dpmm, threshold };
    let data = labelize::encode_image_bytes(&image_bytes, &options, language)
        .map_err(|e| e.to_string())?;
    let out_path = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_output_path(input, language.file_extension(), None));

    fs::write(&out_path, data).map_err(|e| format!("Failed to write output file: {}", e))?;
    println!("Encoded {} -> {}", input.display(), out_path.display());
    Ok(())
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
#[allow(clippy::too_many_arguments)]
fn convert_file(
    input: &Path,
    output: Option<&Path>,
    format: Option<InputFormat>,
    output_type: OutputType,
    width: Option<f64>,
    height: Option<f64>,
    dpmm: i32,
    antialias: bool,
) -> Result<(), String> {
    let content = fs::read(input).map_err(|e| format!("Failed to read input file: {}", e))?;

    let fmt = detect_format(input, format, &content);
    let labels = parse_labels(fmt.language(), &content, dpmm).map_err(|e| e.to_string())?;

    if labels.is_empty() {
        return Err("No labels found in input".to_string());
    }

    let multi = labels.len() > 1;
    for (i, label) in labels.iter().enumerate() {
        let options = DrawerOptions {
            label_width_mm: width
                .or(label.label_width_mm)
                .unwrap_or(DEFAULT_LABEL_WIDTH_MM),
            label_height_mm: height
                .or(label.label_height_mm)
                .unwrap_or(DEFAULT_LABEL_HEIGHT_MM),
            dpmm,
            antialias,
            ..Default::default()
        };

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
            None => default_output_path(
                input,
                output_extension(output_type),
                if multi { Some(i) } else { None },
            ),
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
        extract::Query,
        http::{header, HeaderMap, StatusCode},
        response::IntoResponse,
        routing::{get, post},
        Router,
    };

    // Playground deployment config (LABELIZE_* env vars; see README).
    let config = labelize::playground::PlaygroundConfig::from_env();
    // Resolve the page once at startup; Box::leak hands the router a
    // &'static str without copying the default (borrowed) page.
    let page: &'static str = match labelize::playground::page_html(&config) {
        std::borrow::Cow::Borrowed(html) => html,
        std::borrow::Cow::Owned(html) => Box::leak(html.into_boxed_str()),
    };
    let playground_html: Option<&'static str> = config.enabled.then_some(page);
    // Canonical origin for robots.txt/sitemap.xml (LABELIZE_PUBLIC_BASE_URL).
    let base: &'static str = Box::leak(config.public_base_url.clone().into_boxed_str());

    let robots = move || async move {
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
                (header::CACHE_CONTROL, "max-age=86400"),
            ],
            labelize::playground::robots_txt(base),
        )
            .into_response()
    };

    let sitemap = move || async move {
        (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/xml; charset=utf-8"),
                (header::CACHE_CONTROL, "max-age=86400"),
            ],
            labelize::playground::sitemap_xml(base),
        )
            .into_response()
    };

    let root = move || async move {
        match playground_html {
            Some(html) => (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                    (header::CACHE_CONTROL, "no-cache"),
                ],
                html,
            )
                .into_response(),
            // API-only mode: no web UI, just a self-describing endpoint list.
            None => (
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, "application/json"),
                    (header::CACHE_CONTROL, "no-cache"),
                ],
                format!(
                    "{{\"service\":\"labelize\",\"version\":\"{}\",\"playground\":false,\
                     \"endpoints\":{{\"health\":\"GET /health\",\"convert\":\"POST /convert\",\
                     \"validate\":\"POST /validate\",\"format\":\"POST /format\",\
                     \"encode\":\"POST /encode\"}}}}",
                    env!("CARGO_PKG_VERSION")
                ),
            )
                .into_response(),
        }
    };

    async fn health() -> impl IntoResponse {
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"status":"ok"}"#,
        )
    }

    #[derive(serde::Deserialize)]
    struct ConvertParams {
        #[serde(default)]
        width: Option<f64>,
        #[serde(default)]
        height: Option<f64>,
        #[serde(default = "default_dpmm")]
        dpmm: i32,
        #[serde(default)]
        output: Option<String>,
        /// Emit 8-bit grayscale output preserving antialiasing.
        #[serde(default)]
        antialias: bool,
    }

    fn default_dpmm() -> i32 {
        8
    }

    async fn convert_handler(
        headers: HeaderMap,
        Query(params): Query<ConvertParams>,
        body: Bytes,
    ) -> impl IntoResponse {
        // Language comes from the Content-Type when it names one, otherwise
        // the body itself is sniffed (inside render, when left unset).
        let content_type = headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let language = if content_type.contains("epl") {
            Some(Language::Epl)
        } else if content_type.contains("tspl") {
            Some(Language::Tspl)
        } else if content_type.contains("cpcl") {
            Some(Language::Cpcl)
        } else if content_type.contains("dpl") {
            Some(Language::Dpl)
        } else if content_type.contains("zpl") {
            Some(Language::Zpl)
        } else {
            None
        };

        let options = labelize::RenderOptions {
            language,
            width_mm: params.width,
            height_mm: params.height,
            dpmm: params.dpmm,
            antialias: params.antialias,
        };

        // Parse failures are user input (400); render/encode failures are
        // engine-side (500) — LabelizeError's variants carry that split.
        let rendered = if params.output.as_deref() == Some("pdf") {
            labelize::render_pdf(&body, options)
        } else {
            labelize::render_png(&body, options)
        };

        match rendered {
            Ok(out) => {
                let content = if params.output.as_deref() == Some("pdf") {
                    "application/pdf"
                } else {
                    "image/png"
                };
                (StatusCode::OK, [(header::CONTENT_TYPE, content)], out.bytes).into_response()
            }
            Err(labelize::LabelizeError::Parse(msg)) => {
                (StatusCode::BAD_REQUEST, msg).into_response()
            }
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    }

    #[derive(serde::Deserialize)]
    struct LanguageParams {
        /// Force the language (`zpl` | `epl` | `tspl`); absent = auto-detect
        /// from the body, matching `/convert`'s Content-Type sniffing.
        #[serde(default)]
        language: Option<String>,
        #[serde(default = "default_dpmm")]
        dpmm: i32,
    }

    // Shared `?language=` resolution for /validate and /format: absent means
    // auto-detect, an unknown name is a 400.
    fn forced_language(name: Option<&str>) -> Result<Option<Language>, String> {
        match name {
            None => Ok(None),
            Some(name) => Language::from_name(name).map(Some).ok_or_else(|| {
                format!("unknown language {name:?} — expected zpl, epl, tspl, cpcl or dpl")
            }),
        }
    }

    async fn validate_handler(
        Query(params): Query<LanguageParams>,
        body: Bytes,
    ) -> impl IntoResponse {
        let forced = match forced_language(params.language.as_deref()) {
            Ok(forced) => forced,
            Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
        };
        let report = labelize::validate(&body, params.dpmm, forced);
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            serde_json::to_string(&report).unwrap_or_default(),
        )
            .into_response()
    }

    async fn format_handler(
        Query(params): Query<LanguageParams>,
        body: Bytes,
    ) -> impl IntoResponse {
        let forced = match forced_language(params.language.as_deref()) {
            Ok(forced) => forced,
            Err(msg) => return (StatusCode::BAD_REQUEST, msg).into_response(),
        };
        // Formatting never fails and never changes what the source renders;
        // unknown commands are reflowed, not rejected.
        let (_language, formatted) = labelize::format_source(&body, forced);
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
            formatted,
        )
            .into_response()
    }

    #[derive(serde::Deserialize)]
    struct EncodeParams {
        #[serde(default = "default_dpmm")]
        dpmm: i32,
        #[serde(default = "default_threshold")]
        threshold: u8,
        /// Target language: `zpl` (ASCII text) or `tspl` (binary bytes)
        #[serde(default = "default_encode_language")]
        language: String,
    }

    fn default_threshold() -> u8 {
        128
    }

    fn default_encode_language() -> String {
        "zpl".to_string()
    }

    // Reverse rendering: raster image in, printer code out. Image-decode
    // and option failures are user input → 400.
    async fn encode_handler(Query(params): Query<EncodeParams>, body: Bytes) -> impl IntoResponse {
        let language = match params.language.parse::<ReverseLanguage>() {
            Ok(language) => language,
            Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        };
        let options = labelize::ReverseOptions {
            dpmm: params.dpmm,
            threshold: params.threshold,
        };
        let result = tokio::task::spawn_blocking(move || {
            labelize::encode_image_bytes(&body, &options, language)
        })
        .await;
        match result {
            Ok(Ok(bytes)) => (
                StatusCode::OK,
                [(header::CONTENT_TYPE, language.content_type())],
                bytes,
            )
                .into_response(),
            Ok(Err(e)) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        }
    }

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/robots.txt", get(robots))
        .route("/sitemap.xml", get(sitemap))
        .route("/convert", post(convert_handler))
        .route("/validate", post(validate_handler))
        .route("/format", post(format_handler))
        .route("/encode", post(encode_handler));

    let addr = format!("{}:{}", host, port);
    println!("Starting server on {}", addr);
    if config.enabled {
        println!(
            "Playground: enabled (Labelary compare: {})",
            if config.labelary_compare {
                "enabled"
            } else {
                "disabled"
            }
        );
    } else {
        println!("Playground: disabled — API-only mode (GET /health, POST /convert, POST /validate, POST /format, POST /encode)");
    }
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");
    axum::serve(listener, app).await.expect("Server failed");
}
