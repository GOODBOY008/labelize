#[cfg(feature = "cli")]
use std::fs;
#[cfg(feature = "cli")]
use std::io::Cursor;
#[cfg(feature = "cli")]
use std::path::{Path, PathBuf};

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand, ValueEnum};
#[cfg(feature = "cli")]
use labelize::{
    DrawerOptions, EplParser, LabelInfo, Renderer, TsplParsedLabel, TsplParser, ZplParser,
};

#[cfg(feature = "cli")]
#[derive(Parser)]
#[command(
    name = "labelize",
    version,
    about = "Turn ZPL/EPL/TSPL into pixels — label rendering, simplified."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[cfg(feature = "cli")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum InputFormat {
    Zpl,
    Epl,
    Tspl,
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
    /// Convert a ZPL/EPL/TSPL file to PNG or PDF
    Convert {
        /// Input file path (.zpl, .epl, or .tspl)
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

        /// Label width override in mm
        #[arg(long)]
        width: Option<f64>,

        /// Label height override in mm
        #[arg(long)]
        height: Option<f64>,

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
        "tspl" => InputFormat::Tspl,
        _ => InputFormat::Zpl,
    }
}

#[cfg(feature = "cli")]
fn detect_format_from_content_type(content_type: &str) -> InputFormat {
    let content_type = content_type.to_ascii_lowercase();
    if content_type.contains("tspl") {
        InputFormat::Tspl
    } else if content_type.contains("epl") {
        InputFormat::Epl
    } else {
        InputFormat::Zpl
    }
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

#[cfg(feature = "cli")]
fn parse_labels(content: &[u8], format: InputFormat) -> Result<Vec<LabelInfo>, String> {
    match format {
        InputFormat::Epl => EplParser::new().parse(content),
        InputFormat::Tspl => TsplParser::new().parse(content),
        InputFormat::Zpl => ZplParser::new().parse(content),
    }
}

#[cfg(feature = "cli")]
fn parse_labels_with_options(
    content: &[u8],
    format: InputFormat,
    base_options: DrawerOptions,
    width_override: Option<f64>,
    height_override: Option<f64>,
) -> Result<Vec<TsplParsedLabel>, String> {
    match format {
        InputFormat::Tspl => Ok(TsplParser::new()
            .parse_with_options(content, base_options)?
            .into_iter()
            .map(|mut parsed| {
                parsed.drawer_options = apply_dimension_overrides(
                    parsed.drawer_options,
                    width_override,
                    height_override,
                );
                parsed
            })
            .collect()),
        _ => Ok(parse_labels(content, format)?
            .into_iter()
            .map(|label| TsplParsedLabel {
                label,
                drawer_options: apply_dimension_overrides(
                    base_options.clone(),
                    width_override,
                    height_override,
                ),
            })
            .collect()),
    }
}

#[cfg(feature = "cli")]
fn apply_dimension_overrides(
    mut options: DrawerOptions,
    width_override: Option<f64>,
    height_override: Option<f64>,
) -> DrawerOptions {
    if let Some(width) = width_override {
        options.label_width_mm = width;
    }
    if let Some(height) = height_override {
        options.label_height_mm = height;
    }
    options
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
    width: Option<f64>,
    height: Option<f64>,
    dpmm: i32,
) -> Result<(), String> {
    let content = fs::read(input).map_err(|e| format!("Failed to read input file: {}", e))?;

    let fmt = detect_format(input, format);
    let base_options = DrawerOptions {
        label_width_mm: width.unwrap_or_else(default_width),
        label_height_mm: height.unwrap_or_else(default_height),
        dpmm,
        ..Default::default()
    };
    let labels = parse_labels_with_options(&content, fmt, base_options, width, height)?;

    if labels.is_empty() {
        return Err("No labels found in input".to_string());
    }

    let multi = labels.len() > 1;
    for (i, parsed) in labels.iter().enumerate() {
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

        let data = render_label(&parsed.label, &parsed.drawer_options, output_type)?;
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
    }

    async fn convert_handler(
        headers: HeaderMap,
        Query(params): Query<ConvertParams>,
        body: Bytes,
    ) -> impl IntoResponse {
        // Detect format from Content-Type header
        let content_type = headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let format = detect_format_from_content_type(content_type);
        let base_options = DrawerOptions {
            label_width_mm: params.width.unwrap_or_else(default_width),
            label_height_mm: params.height.unwrap_or_else(default_height),
            dpmm: params.dpmm,
            ..Default::default()
        };
        let labels =
            parse_labels_with_options(&body, format, base_options, params.width, params.height);

        let labels = match labels {
            Ok(l) => l,
            Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
        };

        let parsed = match labels.into_iter().next() {
            Some(l) => l,
            None => {
                return (StatusCode::BAD_REQUEST, "No labels found".to_string()).into_response()
            }
        };

        let want_pdf = params.output.as_deref() == Some("pdf");

        let renderer = Renderer::new();
        let mut buf = Cursor::new(Vec::new());
        if let Err(e) =
            renderer.draw_label_as_png(&parsed.label, &mut buf, parsed.drawer_options.clone())
        {
            return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
        }

        if want_pdf {
            let img = match image::load_from_memory(&buf.into_inner()) {
                Ok(img) => img.to_rgba8(),
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("image decode: {}", e),
                    )
                        .into_response()
                }
            };
            let mut pdf_buf = Cursor::new(Vec::new());
            match labelize::encode_pdf(&img, &parsed.drawer_options, &mut pdf_buf) {
                Ok(_) => (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/pdf")],
                    pdf_buf.into_inner(),
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("pdf encode: {}", e),
                )
                    .into_response(),
            }
        } else {
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "image/png")],
                buf.into_inner(),
            )
                .into_response()
        }
    }

    let app = Router::new()
        .route("/", get(playground_page))
        .route("/health", get(health))
        .route("/convert", post(convert_handler));

    let addr = format!("{}:{}", host, port);
    println!("Starting server on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");
    axum::serve(listener, app).await.expect("Server failed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_format_supports_tspl_extension_and_override() {
        assert_eq!(
            detect_format(Path::new("label.tspl"), None),
            InputFormat::Tspl
        );
        assert_eq!(
            detect_format(Path::new("label.txt"), Some(InputFormat::Tspl)),
            InputFormat::Tspl
        );
    }

    #[test]
    fn content_type_detection_supports_tspl() {
        assert_eq!(
            detect_format_from_content_type("application/tspl; charset=utf-8"),
            InputFormat::Tspl
        );
        assert_eq!(
            detect_format_from_content_type("text/vnd.tspl"),
            InputFormat::Tspl
        );
        assert_eq!(
            detect_format_from_content_type("APPLICATION/TSPL"),
            InputFormat::Tspl
        );
        assert_eq!(
            detect_format_from_content_type("application/epl"),
            InputFormat::Epl
        );
    }

    #[test]
    fn explicit_dimensions_override_tspl_size_per_axis() {
        let options = DrawerOptions {
            label_width_mm: 50.0,
            label_height_mm: 25.0,
            dpmm: 8,
            enable_inverted_labels: true,
        };

        let resolved = apply_dimension_overrides(options, Some(100.0), None);
        assert_eq!(resolved.label_width_mm, 100.0);
        assert_eq!(resolved.label_height_mm, 25.0);

        let resolved = apply_dimension_overrides(resolved, None, Some(75.0));
        assert_eq!(resolved.label_width_mm, 100.0);
        assert_eq!(resolved.label_height_mm, 75.0);
    }
}
