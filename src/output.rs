//! Explicit output selection. Existing `render_png`/`render_pdf` keep their defaults.
use crate::{DrawerOptions, LabelInfo, LabelizeError, RenderOptions, Rendered, Renderer};
use std::io::Cursor;

/// Rendering backend. `Vector` uses shared geometry for both PDF and PNG.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputMode {
    #[default]
    Raster,
    Vector,
}

impl std::str::FromStr for OutputMode {
    type Err = LabelizeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "raster" => Ok(Self::Raster),
            "vector" => Ok(Self::Vector),
            _ => Err(LabelizeError::Parse(
                "output mode must be raster or vector".into(),
            )),
        }
    }
}

/// Additional output settings, separate from source printer resolution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutputOptions {
    pub mode: OutputMode,
    /// PNG pixels per source dot. PDF is resolution independent and requires 1.
    pub scale: f64,
    /// Combine all labels in one vector PDF. Otherwise render the first label.
    pub all_pages: bool,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            mode: OutputMode::Raster,
            scale: 1.0,
            all_pages: false,
        }
    }
}

impl OutputOptions {
    pub fn validate(self, pdf: bool) -> Result<(), LabelizeError> {
        let invalid = |s: &str| Err(LabelizeError::Parse(s.into()));
        if !self.scale.is_finite() || self.scale <= 0.0 {
            return invalid("scale must be finite and positive");
        }
        if (pdf || self.mode == OutputMode::Raster) && self.scale != 1.0 {
            return invalid("scale is only available for vector PNG output");
        }
        if self.all_pages && (!pdf || self.mode != OutputMode::Vector) {
            return invalid("all pages requires vector PDF output");
        }
        if self.mode == OutputMode::Vector && !cfg!(feature = "vector") {
            return invalid("vector output requires a build with the vector feature");
        }
        Ok(())
    }
}

/// Render the first label to PNG with an explicit output backend.
pub fn render_png_with_output(
    bytes: &[u8],
    options: RenderOptions,
    output: OutputOptions,
) -> Result<Rendered, LabelizeError> {
    render(bytes, options, output, false)
}

/// Render to PDF; `all_pages` combines every parsed label with its own size.
pub fn render_pdf_with_output(
    bytes: &[u8],
    options: RenderOptions,
    output: OutputOptions,
) -> Result<Rendered, LabelizeError> {
    render(bytes, options, output, true)
}

fn render(
    bytes: &[u8],
    options: RenderOptions,
    output: OutputOptions,
    pdf: bool,
) -> Result<Rendered, LabelizeError> {
    output.validate(pdf)?;
    if options.dpmm <= 0
        || [options.width_mm, options.height_mm]
            .into_iter()
            .flatten()
            .any(|v| !v.is_finite() || v <= 0.0)
    {
        return Err(LabelizeError::Parse(
            "dimensions and dpmm must be finite and positive".into(),
        ));
    }
    let language = options
        .language
        .unwrap_or_else(|| crate::detect_language(bytes));
    let labels = crate::parse_labels(language, bytes, options.dpmm)?;
    let first = labels
        .first()
        .ok_or_else(|| LabelizeError::Parse("No labels found".into()))?;
    let drawer = |label: &LabelInfo| DrawerOptions {
        label_width_mm: options
            .width_mm
            .or(label.label_width_mm)
            .unwrap_or(crate::DEFAULT_LABEL_WIDTH_MM),
        label_height_mm: options
            .height_mm
            .or(label.label_height_mm)
            .unwrap_or(crate::DEFAULT_LABEL_HEIGHT_MM),
        dpmm: options.dpmm,
        antialias: options.antialias,
        ..Default::default()
    };
    let bytes = if output.all_pages {
        #[cfg(feature = "vector")]
        {
            let scenes = labels
                .iter()
                .map(|label| {
                    let drawer = drawer(label);
                    validate_canvas(&drawer, 1.0)?;
                    crate::LabelScene::new(label, drawer)
                })
                .collect::<Result<Vec<_>, LabelizeError>>()?;
            crate::encode_scenes_pdf(&scenes)?
        }
        #[cfg(not(feature = "vector"))]
        {
            unreachable!("validated feature availability")
        }
    } else {
        render_label_with_output(first, drawer(first), output, pdf)?
    };
    Ok(Rendered {
        language,
        labels: labels.len(),
        bytes,
    })
}

/// Encode a parsed label; use `LabelScene` directly to inspect fallback diagnostics.
pub fn render_label_with_output(
    label: &LabelInfo,
    drawer: DrawerOptions,
    output: OutputOptions,
    pdf: bool,
) -> Result<Vec<u8>, LabelizeError> {
    output.validate(pdf)?;
    validate_canvas(&drawer, output.scale)?;
    if output.mode == OutputMode::Vector {
        #[cfg(feature = "vector")]
        {
            let scene = crate::LabelScene::new(label, drawer.clone())?;
            return if pdf {
                scene.to_pdf()
            } else {
                scene.to_png(crate::ScenePngOptions {
                    scale: output.scale,
                    antialias: drawer.antialias,
                })
            };
        }
    }
    let mut png = Cursor::new(Vec::new());
    Renderer::new()
        .draw_label_as_png(label, &mut png, drawer.clone())
        .map_err(LabelizeError::Render)?;
    if !pdf {
        return Ok(png.into_inner());
    }
    let image = image::load_from_memory(png.get_ref())
        .map_err(|e| LabelizeError::Render(e.to_string()))?
        .to_rgba8();
    let mut bytes = Cursor::new(Vec::new());
    crate::encode_pdf(&image, &drawer, &mut bytes)?;
    Ok(bytes.into_inner())
}

fn validate_canvas(drawer: &DrawerOptions, scale: f64) -> Result<(), LabelizeError> {
    let w = (drawer.label_width_mm * drawer.dpmm as f64).ceil();
    let h = (drawer.label_height_mm * drawer.dpmm as f64).ceil();
    let ow = (w * scale).ceil();
    let oh = (h * scale).ceil();
    if drawer.dpmm <= 0
        || !w.is_finite()
        || !h.is_finite()
        || w < 1.0
        || h < 1.0
        || w > 32000.0
        || h > 32000.0
        || w * h > 16.0 * 1024.0 * 1024.0
        || !ow.is_finite()
        || !oh.is_finite()
        || ow > 32000.0
        || oh > 32000.0
        || ow * oh > 64.0 * 1024.0 * 1024.0
    {
        return Err(LabelizeError::Parse(
            "canvas exceeds source or output pixel limits".into(),
        ));
    }
    Ok(())
}

#[cfg(all(test, feature = "vector"))]
mod diagnostics {
    /// Independent-viewer control: retain exactly the same positioned glyphs,
    /// but replace font text with its outlines to isolate viewer font hinting.
    #[test]
    #[ignore = "writes PDF outline controls for external viewer verification"]
    fn export_pdf_outline_controls() {
        let out = std::path::Path::new("output/pdf/vector-outline-control");
        std::fs::create_dir_all(out).unwrap();
        for (dir, name) in [
            ("labels", "fedex"),
            ("labels", "fedex_ground"),
            ("labels", "glscz"),
            ("unit", "barcode128_default_width"),
            ("unit", "barcode128_mode_a"),
            ("unit", "barcode128_mode_d"),
            ("unit", "barcode128_mode_n"),
            ("unit", "barcode128_mode_u"),
        ] {
            let bytes = std::fs::read(format!("testdata/{dir}/{name}.zpl")).unwrap();
            let label = crate::parse_labels(crate::Language::Zpl, &bytes, 8)
                .unwrap()
                .remove(0);
            let scene = crate::LabelScene::new(
                &label,
                crate::DrawerOptions {
                    label_width_mm: if dir == "labels" { 101.625 } else { 101.5 },
                    label_height_mm: if dir == "labels" { 203.25 } else { 203.0 },
                    ..Default::default()
                },
            )
            .unwrap();
            let bytes = crate::drawers::renderer::scene::outline_control(&[scene]).unwrap();
            std::fs::write(out.join(format!("{dir}-{name}-001.pdf")), bytes).unwrap();
        }
    }
}
