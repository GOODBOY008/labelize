use clap::{Args, Subcommand};
use labelize::{Language, OutputMode, OutputOptions, PrintTemplate, RenderOptions, TemplateValues};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Args)]
pub(super) struct Record {
    /// Template exported from Labelize Studio
    input: PathBuf,
    /// JSON file containing one object of string values
    #[arg(long)]
    data: PathBuf,
    /// Destination printer file, PNG or PDF
    #[arg(short, long)]
    output: PathBuf,
    /// Printer language (default: infer from the template file)
    #[arg(short, long, visible_alias = "language", value_parser = parse_language)]
    format: Option<Language>,
}

#[derive(Subcommand)]
pub(super) enum TemplateCommand {
    /// Write the native printer bytes for one record
    Fill(Record),
    /// Preview one filled record as PNG or PDF, inferred from the output extension
    Render {
        #[command(flatten)]
        record: Record,
        /// Label width in millimetres
        #[arg(long)]
        width: Option<f64>,
        /// Label height in millimetres
        #[arg(long)]
        height: Option<f64>,
        /// Dots per millimetre at which to interpret the native job
        #[arg(long, default_value_t = 8)]
        dpmm: i32,
        /// Preserve grayscale antialiasing
        #[arg(long)]
        antialias: bool,
        /// Output backend (vector requires the vector feature)
        #[arg(long, default_value = "raster")]
        mode: OutputMode,
        /// PNG pixels per source dot with vector output
        #[arg(long, default_value_t = 1.0)]
        scale: f64,
        /// Combine all labels into one vector PDF
        #[arg(long)]
        all_pages: bool,
    },
}

fn parse_language(s: &str) -> Result<Language, String> {
    match s.to_ascii_lowercase().as_str() {
        "epl2" => Ok(Language::Epl),
        "tsc" => Ok(Language::Tspl),
        name => Language::from_name(name).ok_or_else(|| format!("Unknown template language {s}")),
    }
}

impl Record {
    fn fill(&self) -> Result<(Language, Vec<u8>), Box<dyn Error>> {
        let source = fs::read(&self.input)?;
        let ext = self
            .input
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let language = self
            .format
            .or_else(|| parse_language(ext).ok())
            .unwrap_or_else(|| {
                source
                    .strip_prefix(b"LABELIZE-TPL/1 ")
                    .and_then(|tail| tail.split(|&b| b == b' ').next())
                    .and_then(|name| std::str::from_utf8(name).ok())
                    .and_then(|name| parse_language(name).ok())
                    .unwrap_or(Language::Zpl)
            });
        let template = PrintTemplate::load(language, &source)?;
        let values: TemplateValues =
            serde_json::from_slice(&fs::read(&self.data)?).map_err(|e| {
                format!(
                    "{}: expected one JSON object of string values: {e}",
                    self.data.display()
                )
            })?;
        Ok((language, template.fill(&values)?))
    }
}

impl TemplateCommand {
    pub(super) fn run(self) -> Result<(), Box<dyn Error>> {
        let (path, bytes) = match self {
            Self::Fill(record) => {
                let (_, job) = record.fill()?;
                (record.output, job)
            }
            Self::Render {
                record,
                width,
                height,
                dpmm,
                antialias,
                mode,
                scale,
                all_pages,
            } => {
                let (language, job) = record.fill()?;
                let render = RenderOptions {
                    language: Some(language),
                    width_mm: width,
                    height_mm: height,
                    dpmm,
                    antialias,
                };
                let options = OutputOptions {
                    mode,
                    scale,
                    all_pages,
                };
                let ext = record
                    .output
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                let rendered = match ext.to_ascii_lowercase().as_str() {
                    "png" => labelize::render_png_with_output(&job, render, options)?,
                    "pdf" => labelize::render_pdf_with_output(&job, render, options)?,
                    _ => {
                        return Err(
                            "Template render output must have a .png or .pdf extension".into()
                        )
                    }
                };
                (record.output, rendered.bytes)
            }
        };
        write_output(&path, &bytes)?;
        Ok(())
    }
}

fn write_output(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    // Write on the destination filesystem so a short write never truncates
    // an existing order. New files honor the umask as other CLI outputs do.
    let directory = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut options = tempfile::Builder::new();
    options.prefix(".labelize-");
    #[cfg(unix)]
    let existing_mode = match fs::metadata(path) {
        Ok(file) => Some(file.permissions().mode() & 0o777),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            options.permissions(fs::Permissions::from_mode(0o666));
            None
        }
        Err(e) => return Err(e),
    };
    let mut tmp = options.tempfile_in(directory)?;
    tmp.write_all(bytes)?;
    #[cfg(unix)]
    if let Some(mode) = existing_mode {
        tmp.as_file()
            .set_permissions(fs::Permissions::from_mode(mode))?;
    }
    tmp.persist(path)?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn replacing_an_order_keeps_it_readable_by_the_print_service() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("order.zpl");
        fs::write(&path, b"old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        write_output(&path, b"new").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        assert_eq!(fs::read(path).unwrap(), b"new");
    }
}
