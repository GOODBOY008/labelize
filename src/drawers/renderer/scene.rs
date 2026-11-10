//! A reusable drawing scene in source printer dots.
//!
//! Parsing resolution belongs to the caller. Output backends scale this scene
//! without reinterpreting input coordinates or font metrics.

use super::*;
use crate::error::LabelizeError;
use std::sync::Arc;
use tiny_skia::{FillRule, Path, PathBuilder, Rect, Transform};

mod glyphs;
mod pdf;
mod raster;

pub use pdf::encode_scenes_pdf;
#[cfg(test)]
pub(crate) use pdf::outline_control;

const MAX_SOURCE_PIXELS: u64 = 16 * 1024 * 1024;
const MAX_OUTPUT_PIXELS: f64 = 64.0 * 1024.0 * 1024.0;
const MAX_DIMENSION: f64 = 32_000.0;

/// A local raster operation needed to preserve pixel-defined compositing.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct RasterFallback {
    /// Zero-based source element index.
    pub element: usize,
    /// Why pixel-defined compositing is needed instead of ordinary primitives.
    pub reason: String,
    /// Affected rectangle in source dots, before whole-page transforms.
    pub bounds: [u32; 4],
}

/// Statistics describing the actual scene, including any raster fallbacks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SceneStats {
    pub glyphs: usize,
    pub paths: usize,
    pub images: usize,
    pub fallbacks: Vec<RasterFallback>,
}

/// Settings for rasterizing an already laid-out scene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenePngOptions {
    /// Output pixels per source dot. Does not change parsing resolution.
    pub scale: f64,
    /// Preserve grayscale edge coverage; otherwise threshold to black/white.
    pub antialias: bool,
}

impl Default for ScenePngOptions {
    fn default() -> Self {
        Self {
            scale: 1.0,
            antialias: false,
        }
    }
}

#[derive(Clone, Debug)]
enum Op {
    Path {
        path: Path,
        color: u8,
        rule: FillRule,
    },
    Text {
        face: FontFace,
        glyphs: Vec<text_layout::PlacedGlyph>,
        logical_text: Option<String>,
    },
    Image {
        image: Arc<RgbaImage>,
        x: f32,
        y: f32,
    },
    Group {
        transform: Transform,
        clip: Option<Rect>,
        ops: Vec<Op>,
    },
}

/// A label after layout, reusable across PDF and multiple PNG resolutions.
#[derive(Debug, Clone)]
pub struct LabelScene {
    width: u32,
    height: u32,
    width_mm: f64,
    height_mm: f64,
    dpmm: i32,
    ops: Vec<Op>,
    stats: SceneStats,
}

impl LabelScene {
    /// Compile a language-neutral label without changing the source resolution.
    pub fn new(label: &LabelInfo, options: DrawerOptions) -> Result<Self, LabelizeError> {
        let options = options.with_defaults();
        let (width, height) = dimensions(&options)?;
        let content_width = if label.print_width > 0 {
            width.min(label.print_width as u32)
        } else {
            width
        };
        let mut scene = Self {
            width,
            height,
            width_mm: options.label_width_mm,
            height_mm: options.label_height_mm,
            dpmm: options.dpmm,
            ops: Vec::new(),
            stats: SceneStats::default(),
        };
        let mut state = DrawerState::new();
        let mut ops = Vec::new();
        let mut barcode_regions = Vec::new();
        let mut source_text_bounds = Vec::new();
        let needs_compositing = label.elements.iter().any(|element| {
            element.is_reverse_print()
                || subpixel_diagonal(element)
                || matches!(
                    element,
                    LabelElement::InvertRegion(_)
                        | LabelElement::GraphicField(GraphicField {
                            mode: GraphicFieldMode::Xor,
                            ..
                        })
                )
        });
        // Pixel-defined compositing needs the exact prior background. Allocate
        // this reference surface only for labels containing such operations.
        let mut background = needs_compositing
            .then(|| RgbaImage::from_pixel(content_width, height, Rgba([255, 255, 255, 255])));
        let renderer = Renderer::new();
        let mut background_state = DrawerState::new();
        for (index, element) in label.elements.iter().enumerate() {
            if matches!(element, LabelElement::InvertRegion(_)) {
                continue;
            }
            if let Some(background) = &mut background {
                if element.is_reverse_print() {
                    let mut mask = RgbaImage::new(content_width, height);
                    renderer
                        .draw_element(&mut mask, element, &options, &mut background_state)
                        .map_err(LabelizeError::Render)?;
                    images::reverse_print::reverse_print(&mask, background);
                    if let Some(bounds) = covered_bounds(&mask, 30) {
                        record_fallback(
                            &mut scene.stats,
                            index,
                            "background-dependent field inversion",
                            bounds,
                        );
                    }
                    state = background_state.clone();
                    continue;
                }
                renderer
                    .draw_element(background, element, &options, &mut background_state)
                    .map_err(LabelizeError::Render)?;
                if let LabelElement::GraphicField(g) = element {
                    if g.mode == GraphicFieldMode::Xor {
                        let mut mask = RgbaImage::new(content_width, height);
                        let mut ordinary = g.clone();
                        ordinary.mode = GraphicFieldMode::Or;
                        renderer
                            .draw_element(
                                &mut mask,
                                &LabelElement::GraphicField(ordinary),
                                &options,
                                &mut DrawerState::new(),
                            )
                            .map_err(LabelizeError::Render)?;
                        if let Some(bounds) = covered_bounds(&mask, 1) {
                            record_fallback(&mut scene.stats, index, "bitmap XOR", bounds);
                        }
                        continue;
                    }
                }
            }
            let adjusted = flip_from_bottom(element, height);
            let element = adjusted.as_ref().unwrap_or(element);
            match element {
                LabelElement::Text(text) => {
                    let layout =
                        text_layout::layout(text, &mut state).map_err(LabelizeError::Render)?;
                    let op = text_op(layout);
                    collect_text_bounds(
                        std::slice::from_ref(&op),
                        Transform::identity(),
                        None,
                        &mut source_text_bounds,
                    );
                    ops.push(op);
                }
                LabelElement::GraphicBox(g) => {
                    let w = g.width.max(g.border_thickness) as f32;
                    let h = g.height.max(g.border_thickness) as f32;
                    let border = g.border_thickness.max(0) as f32;
                    let radius = ((w.min(h) / 2.0) * (g.corner_rounding as f32 / 8.0)).round();
                    let mut path = PathBuilder::new();
                    rounded_rect(
                        &mut path,
                        g.position.x as f32,
                        g.position.y as f32,
                        w,
                        h,
                        radius,
                    );
                    if border < w / 2.0 && border < h / 2.0 {
                        rounded_rect(
                            &mut path,
                            g.position.x as f32 + border,
                            g.position.y as f32 + border,
                            w - 2.0 * border,
                            h - 2.0 * border,
                            (radius - border).max(0.0),
                        );
                    }
                    push_path(&mut ops, path, g.line_color, FillRule::EvenOdd);
                }
                LabelElement::GraphicCircle(g) => {
                    ellipse(
                        &mut ops,
                        g.position.x as f32,
                        g.position.y as f32,
                        g.circle_diameter as f32,
                        g.circle_diameter as f32,
                        g.border_thickness.max(1) as f32,
                        g.line_color,
                    );
                }
                LabelElement::GraphicEllipse(g) => {
                    ellipse(
                        &mut ops,
                        g.position.x as f32,
                        g.position.y as f32,
                        g.width.max(1) as f32,
                        g.height.max(1) as f32,
                        g.border_thickness.max(1) as f32,
                        g.line_color,
                    );
                }
                LabelElement::DiagonalLine(g) => {
                    if subpixel_diagonal(element) {
                        // The raster polygon includes its pixel outline. A
                        // subpixel band loses that ink when vector coverage is
                        // thresholded. Preserve just this region from the final
                        // canvas, including any later overpainting.
                        let mut mask = RgbaImage::new(content_width, height);
                        renderer.draw_diagonal_line(&mut mask, g);
                        if let Some(bounds) = covered_bounds(&mask, 1) {
                            record_fallback(
                                &mut scene.stats,
                                index,
                                "subpixel diagonal band",
                                bounds,
                            );
                        }
                        continue;
                    }
                    let (x, y, w, h) = (
                        g.position.x as f32,
                        g.position.y as f32,
                        g.width as f32,
                        g.height as f32,
                    );
                    let b = g.border_thickness.max(1) as f32;
                    let mut path = PathBuilder::new();
                    let (top, bottom) = if g.top_to_bottom {
                        (x, x + w)
                    } else {
                        (x + w, x)
                    };
                    if b == 1.0 {
                        // The raster renderer draws a one-dot line segment,
                        // not a horizontal band. Keep a full dot of width
                        // perpendicular to the segment, through pixel centers,
                        // with square caps that include both endpoint dots.
                        let dx = bottom - top;
                        let length = dx.hypot(h);
                        if length == 0.0 {
                            if let Some(rect) = Rect::from_xywh(top, y, 1.0, 1.0) {
                                path.push_rect(rect);
                            }
                        } else {
                            let (tx, ty) = (0.5 * dx / length, 0.5 * h / length);
                            let (nx, ny) = (-ty, tx);
                            let (ax, ay) = (top + 0.5 - tx, y + 0.5 - ty);
                            let (bx, by) = (bottom + 0.5 + tx, y + h + 0.5 + ty);
                            path.move_to(ax - nx, ay - ny);
                            path.line_to(bx - nx, by - ny);
                            path.line_to(bx + nx, by + ny);
                            path.line_to(ax + nx, ay + ny);
                            path.close();
                        }
                    } else {
                        path.move_to(top, y);
                        path.line_to(top + b, y);
                        path.line_to(bottom + b, y + h + 1.0);
                        path.line_to(bottom, y + h + 1.0);
                        path.close();
                    }
                    // Adjacent opaque lines form one ink region. Painting
                    // their coverage separately darkens crossing subpixel
                    // edges differently in PDF and PNG rasterizers.
                    if index > 0
                        && matches!(label.elements[index - 1], LabelElement::DiagonalLine(_))
                        && !label.elements[index - 1].is_reverse_print()
                    {
                        if let Some(Op::Path {
                            path: previous,
                            color,
                            rule: FillRule::Winding,
                        }) = ops.last_mut()
                        {
                            if *color == line_color_to_rgba(g.line_color)[0] {
                                path.push_path(previous);
                                if let Some(combined) = path.finish() {
                                    *previous = combined;
                                }
                                continue;
                            }
                        }
                    }
                    push_path(&mut ops, path, g.line_color, FillRule::Winding);
                }
                LabelElement::GraphicField(g) if g.mode != GraphicFieldMode::Xor => {
                    let mut image = RgbaImage::new(content_width, height);
                    Renderer::new().draw_graphic_field(&mut image, g);
                    if let Some((image, x, y)) = crop_covered(&image) {
                        ops.push(Op::Image {
                            image: Arc::new(image),
                            x: x as f32,
                            y: y as f32,
                        });
                    }
                }
                LabelElement::Maxicode(code) => {
                    if code.data.is_empty() {
                        continue;
                    }
                    use crate::barcodes::maxicode::{
                        encode_codewords, module_positions, BULLSEYE_RINGS, SYMBOL_HEIGHT,
                        SYMBOL_WIDTH,
                    };
                    let codewords = encode_codewords(&code.data, code.code.mode)
                        .map_err(LabelizeError::Render)?;
                    let pos = adjust_image_typeset_position(
                        &RgbaImage::new(SYMBOL_WIDTH, SYMBOL_HEIGHT),
                        &code.position,
                        FieldOrientation::Normal,
                        height,
                    );
                    if let Some(bounds) = Rect::from_xywh(
                        pos.x as f32,
                        pos.y as f32,
                        SYMBOL_WIDTH as f32,
                        SYMBOL_HEIGHT as f32,
                    ) {
                        barcode_regions.push((index, bounds));
                    }
                    let mut modules = PathBuilder::new();
                    for (x, y) in module_positions(&codewords) {
                        modules.push_circle(x as f32 + 0.5, y as f32 + 0.5, 3.0);
                    }
                    let mut symbol = Vec::new();
                    push_path(&mut symbol, modules, LineColor::Black, FillRule::Winding);
                    for (inner, outer) in BULLSEYE_RINGS {
                        let mut ring = PathBuilder::new();
                        ring.push_circle(
                            (SYMBOL_WIDTH / 2) as f32 + 0.5,
                            (SYMBOL_HEIGHT / 2) as f32 + 0.5,
                            outer as f32,
                        );
                        ring.push_circle(
                            (SYMBOL_WIDTH / 2) as f32 + 0.5,
                            (SYMBOL_HEIGHT / 2) as f32 + 0.5,
                            inner as f32,
                        );
                        push_path(&mut symbol, ring, LineColor::Black, FillRule::EvenOdd);
                    }
                    ops.push(Op::Group {
                        transform: Transform::from_translate(pos.x as f32, pos.y as f32),
                        clip: Rect::from_xywh(0.0, 0.0, SYMBOL_WIDTH as f32, SYMBOL_HEIGHT as f32),
                        ops: symbol,
                    });
                }
                LabelElement::Barcode128(_)
                | LabelElement::BarcodeEan13(_)
                | LabelElement::BarcodeEan8(_)
                | LabelElement::BarcodeUca(_)
                | LabelElement::Barcode2of5(_)
                | LabelElement::Barcode39(_)
                | LabelElement::BarcodePdf417(_)
                | LabelElement::BarcodeAztec(_)
                | LabelElement::BarcodeDatamatrix(_)
                | LabelElement::BarcodeQr(_)
                | LabelElement::BarcodeUcpe(_) => {
                    let barcode = barcode_layout::layout(element, height, &options)
                        .map_err(LabelizeError::Render)?;
                    for part in barcode.parts {
                        match part {
                            barcode_layout::BarcodePart::Symbol {
                                image,
                                position,
                                orientation,
                            } => {
                                let transform = rotation(
                                    orientation,
                                    image.width(),
                                    image.height(),
                                    position.x as f32,
                                    position.y as f32,
                                );
                                // Keep any encoder-supplied quiet zone in the
                                // overlap region: nearby text can otherwise
                                // change symbol detection without touching ink.
                                if let Some(bounds) = Rect::from_xywh(
                                    0.0,
                                    0.0,
                                    image.width() as f32,
                                    image.height() as f32,
                                )
                                .and_then(|r| r.transform(transform))
                                {
                                    barcode_regions.push((index, bounds));
                                }
                                ops.push(Op::Group {
                                    transform,
                                    clip: None,
                                    ops: symbol_paths(&image)?,
                                });
                            }
                            barcode_layout::BarcodePart::Text { layout, .. } => {
                                ops.push(text_op(layout))
                            }
                        }
                    }
                }
                LabelElement::Barcode128Config(_)
                | LabelElement::BarcodeEan13Config(_)
                | LabelElement::BarcodeEan8Config(_)
                | LabelElement::BarcodeUcaConfig(_)
                | LabelElement::Barcode2of5Config(_)
                | LabelElement::Barcode39Config(_)
                | LabelElement::BarcodePdf417Config(_)
                | LabelElement::BarcodeAztecConfig(_)
                | LabelElement::BarcodeDatamatrixConfig(_)
                | LabelElement::BarcodeQrConfig(_)
                | LabelElement::MaxicodeConfig(_)
                | LabelElement::BarcodeUcpeConfig(_)
                | LabelElement::GraphicSymbolConfig(_)
                | LabelElement::FieldBlockConfig(_)
                | LabelElement::StoredField(_)
                | LabelElement::RecalledFieldData(_)
                | LabelElement::RecalledField(_)
                | LabelElement::RecalledFormat(_) => {}
                LabelElement::InvertRegion(_) | LabelElement::GraphicField(_) => {
                    return Err(LabelizeError::Render(format!(
                        "invalid compositing state for element {index}"
                    )))
                }
            }
        }
        // Some real labels place text across barcode modules. Different font
        // rasterizers can change whether those already damaged symbols decode.
        // Preserve the original ink only inside such overlaps; leave the rest
        // of the label as searchable text and vector geometry.
        if !barcode_regions.is_empty() {
            let overlaps: Vec<_> = barcode_regions
                .into_iter()
                .filter(|(_, barcode)| {
                    source_text_bounds
                        .iter()
                        .any(|text| text.intersect(barcode).is_some())
                })
                .collect();
            if !overlaps.is_empty() {
                if background.is_none() {
                    let mut image =
                        RgbaImage::from_pixel(content_width, height, Rgba([255, 255, 255, 255]));
                    let mut state = DrawerState::new();
                    for element in &label.elements {
                        if !matches!(element, LabelElement::InvertRegion(_)) {
                            renderer
                                .draw_element(&mut image, element, &options, &mut state)
                                .map_err(LabelizeError::Render)?;
                        }
                    }
                    background = Some(image);
                }
                for (index, bounds) in overlaps {
                    let x0 = bounds.left().floor().max(0.0).min(content_width as f32) as u32;
                    let y0 = bounds.top().floor().max(0.0).min(height as f32) as u32;
                    let x1 = bounds.right().ceil().max(0.0).min(content_width as f32) as u32;
                    let y1 = bounds.bottom().ceil().max(0.0).min(height as f32) as u32;
                    if x1 > x0 && y1 > y0 {
                        record_fallback(
                            &mut scene.stats,
                            index,
                            "text overlaps barcode region",
                            [x0, y0, x1 - x0, y1 - y0],
                        );
                    }
                }
            }
        }
        if let Some(background) = &mut background {
            for (index, element) in label.elements.iter().enumerate() {
                if let LabelElement::InvertRegion(region) = element {
                    let x0 = (region.position.x as i64).clamp(0, content_width as i64) as u32;
                    let y0 = (region.position.y as i64).clamp(0, height as i64) as u32;
                    let x1 = (region.position.x as i64 + region.width as i64)
                        .clamp(0, content_width as i64) as u32;
                    let y1 = (region.position.y as i64 + region.height as i64)
                        .clamp(0, height as i64) as u32;
                    if x1 > x0 && y1 > y0 {
                        for y in y0..y1 {
                            for x in x0..x1 {
                                let p = background.get_pixel_mut(x, y);
                                p[0] = 255 - p[0];
                                p[1] = 255 - p[1];
                                p[2] = 255 - p[2];
                            }
                        }
                        record_fallback(
                            &mut scene.stats,
                            index,
                            "print-time region inversion",
                            [x0, y0, x1 - x0, y1 - y0],
                        );
                    }
                }
            }
        }
        // Overlay snapshots of the finished reference canvas. This also
        // preserves later overpainting inside an affected region, instead of
        // quantizing a temporary background before subsequent translucent ink.
        if let Some(background) = &background {
            for fallback in &scene.stats.fallbacks {
                push_fallback_image(&mut ops, background, fallback.bounds, options.antialias);
            }
        }
        let offset = ((width - content_width) / 2) as f32;
        let inverted = options.enable_inverted_labels && label.inverted;
        let transform = if inverted {
            Transform::from_row(-1.0, 0.0, 0.0, -1.0, width as f32 - offset, height as f32)
        } else {
            Transform::from_translate(offset, 0.0)
        };
        let transform = if label.mirrored {
            Transform::from_row(-1.0, 0.0, 0.0, 1.0, width as f32, 0.0).pre_concat(transform)
        } else {
            transform
        };
        scene.ops.push(Op::Group {
            transform,
            clip: Rect::from_xywh(0.0, 0.0, content_width as f32, height as f32),
            ops,
        });
        count_ops(&scene.ops, &mut scene.stats);
        Ok(scene)
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    pub fn size_mm(&self) -> (f64, f64) {
        (self.width_mm, self.height_mm)
    }
    pub fn stats(&self) -> &SceneStats {
        &self.stats
    }
    pub fn to_pdf(&self) -> Result<Vec<u8>, LabelizeError> {
        encode_scenes_pdf(std::slice::from_ref(self))
    }
    pub fn to_png(&self, options: ScenePngOptions) -> Result<Vec<u8>, LabelizeError> {
        raster::encode(self, options)
    }
}

fn dimensions(options: &DrawerOptions) -> Result<(u32, u32), LabelizeError> {
    let valid = options.dpmm > 0
        && options.label_width_mm.is_finite()
        && options.label_height_mm.is_finite()
        && options.label_width_mm > 0.0
        && options.label_height_mm > 0.0;
    if !valid {
        return Err(LabelizeError::Render(
            "invalid scene size or source resolution".into(),
        ));
    }
    let dots = |mm: f64| {
        let value = mm * options.dpmm as f64;
        if (value - value.round()).abs() <= value.abs() * f64::EPSILON * 4.0 {
            value.round()
        } else {
            value.ceil()
        }
    };
    let (w, h) = (dots(options.label_width_mm), dots(options.label_height_mm));
    if w < 1.0
        || h < 1.0
        || w > MAX_DIMENSION
        || h > MAX_DIMENSION
        || w * h > MAX_SOURCE_PIXELS as f64
    {
        return Err(LabelizeError::Render(
            "scene exceeds source pixel limit".into(),
        ));
    }
    Ok((w as u32, h as u32))
}

fn text_op(layout: text_layout::TextLayout) -> Op {
    let text = Op::Text {
        face: layout.face,
        glyphs: layout.glyphs,
        logical_text: layout.logical_text,
    };
    if let Some(buffer) = layout.buffer {
        Op::Group {
            transform: rotation(
                buffer.orientation,
                buffer.width,
                buffer.height,
                buffer.x as f32,
                buffer.y as f32,
            ),
            clip: Rect::from_xywh(0.0, 0.0, buffer.width as f32, buffer.height as f32),
            ops: vec![text],
        }
    } else {
        text
    }
}

fn rotation(orientation: FieldOrientation, w: u32, h: u32, x: f32, y: f32) -> Transform {
    match orientation {
        FieldOrientation::Normal => Transform::from_translate(x, y),
        FieldOrientation::Rotated90 => Transform::from_row(0.0, 1.0, -1.0, 0.0, h as f32 + x, y),
        FieldOrientation::Rotated180 => {
            Transform::from_row(-1.0, 0.0, 0.0, -1.0, w as f32 + x, h as f32 + y)
        }
        FieldOrientation::Rotated270 => Transform::from_row(0.0, -1.0, 1.0, 0.0, x, w as f32 + y),
    }
}

fn count_ops(ops: &[Op], stats: &mut SceneStats) {
    for op in ops {
        match op {
            Op::Path { .. } => stats.paths += 1,
            Op::Text { glyphs, .. } => stats.glyphs += glyphs.len(),
            Op::Image { .. } => stats.images += 1,
            Op::Group { ops, .. } => count_ops(ops, stats),
        }
    }
}

fn push_path(ops: &mut Vec<Op>, builder: PathBuilder, color: LineColor, rule: FillRule) {
    if let Some(path) = builder.finish() {
        ops.push(Op::Path {
            path,
            color: line_color_to_rgba(color)[0],
            rule,
        });
    }
}

fn rounded_rect(p: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, radius: f32) {
    let Some(rect) = Rect::from_xywh(x, y, w, h) else {
        return;
    };
    let r = radius.max(0.0).min(w / 2.0).min(h / 2.0);
    if r == 0.0 {
        p.push_rect(rect);
        return;
    }
    let c = r * 0.552_284_8;
    p.move_to(x + r, y);
    p.line_to(x + w - r, y);
    p.cubic_to(x + w - r + c, y, x + w, y + r - c, x + w, y + r);
    p.line_to(x + w, y + h - r);
    p.cubic_to(x + w, y + h - r + c, x + w - r + c, y + h, x + w - r, y + h);
    p.line_to(x + r, y + h);
    p.cubic_to(x + r - c, y + h, x, y + h - r + c, x, y + h - r);
    p.line_to(x, y + r);
    p.cubic_to(x, y + r - c, x + r - c, y, x + r, y);
    p.close();
}

#[allow(clippy::too_many_arguments)]
fn ellipse(ops: &mut Vec<Op>, x: f32, y: f32, w: f32, h: f32, border: f32, color: LineColor) {
    let mut path = PathBuilder::new();
    if let Some(rect) = Rect::from_xywh(x + 0.5, y + 0.5, w, h) {
        path.push_oval(rect);
    }
    if let Some(rect) = Rect::from_xywh(
        x + 0.5 + border,
        y + 0.5 + border,
        w - 2.0 * border,
        h - 2.0 * border,
    ) {
        path.push_oval(rect);
    }
    push_path(ops, path, color, FillRule::EvenOdd);
}

fn crop_covered(image: &RgbaImage) -> Option<(RgbaImage, u32, u32)> {
    covered_bounds(image, 1).map(|[x, y, w, h]| {
        (
            image::imageops::crop_imm(image, x, y, w, h).to_image(),
            x,
            y,
        )
    })
}

fn covered_bounds(image: &RgbaImage, alpha_threshold: u8) -> Option<[u32; 4]> {
    let (mut x0, mut y0, mut x1, mut y1) = (image.width(), image.height(), 0, 0);
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] >= alpha_threshold {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x + 1);
            y1 = y1.max(y + 1);
        }
    }
    (x0 < x1 && y0 < y1).then_some([x0, y0, x1.saturating_sub(x0), y1.saturating_sub(y0)])
}

fn subpixel_diagonal(element: &LabelElement) -> bool {
    let LabelElement::DiagonalLine(line) = element else {
        return false;
    };
    let height = line.height as f32 + 1.0;
    line.border_thickness > 1
        && line.border_thickness as f32 * height < (line.width as f32).hypot(height)
}

fn record_fallback(stats: &mut SceneStats, element: usize, reason: &str, bounds: [u32; 4]) {
    stats.fallbacks.push(RasterFallback {
        element,
        reason: reason.into(),
        bounds,
    });
}

fn push_fallback_image(
    ops: &mut Vec<Op>,
    background: &RgbaImage,
    bounds: [u32; 4],
    antialias: bool,
) {
    let [x, y, w, h] = bounds;
    let mut image = image::imageops::crop_imm(background, x, y, w, h).to_image();
    if !antialias {
        for pixel in image.pixels_mut() {
            let value = if pixel[0] > 128 { 255 } else { 0 };
            *pixel = Rgba([value, value, value, 255]);
        }
    }
    ops.push(Op::Image {
        image: Arc::new(image),
        x: x as f32,
        y: y as f32,
    });
}

// The existing barcode encoders expose a binary module raster. Turn its runs
// into rectangle paths, joining identical runs vertically. No full-label
// rasterization, tracing of text, or image embedding is involved.
fn symbol_paths(image: &RgbaImage) -> Result<Vec<Op>, LabelizeError> {
    use std::collections::BTreeMap;
    type Run = (u32, u32, u8);
    let mut active = BTreeMap::<Run, (u32, u32)>::new();
    let mut paths = BTreeMap::<u8, PathBuilder>::new();
    let flush = |key: Run, bounds: (u32, u32), paths: &mut BTreeMap<u8, PathBuilder>| {
        if let Some(rect) = Rect::from_xywh(
            key.0 as f32,
            bounds.0 as f32,
            key.1 as f32,
            (bounds.1 - bounds.0) as f32,
        ) {
            paths.entry(key.2).or_default().push_rect(rect);
        }
    };
    for y in 0..image.height() {
        let mut next = BTreeMap::new();
        let mut x = 0;
        while x < image.width() {
            let p = *image.get_pixel(x, y);
            if p[3] == 0 {
                x += 1;
                continue;
            }
            if p[3] != 255 || p[0] != p[1] || p[0] != p[2] {
                return Err(LabelizeError::Render(
                    "non-monochrome barcode module data".into(),
                ));
            }
            let start = x;
            x += 1;
            while x < image.width() && *image.get_pixel(x, y) == p {
                x += 1;
            }
            let key = (start, x - start, p[0]);
            let from = active.remove(&key).map(|(from, _)| from).unwrap_or(y);
            next.insert(key, (from, y + 1));
        }
        for (key, bounds) in active {
            flush(key, bounds, &mut paths);
        }
        active = next;
    }
    for (key, bounds) in active {
        flush(key, bounds, &mut paths);
    }
    Ok(paths
        .into_iter()
        .filter_map(|(color, path)| {
            path.finish().map(|path| Op::Path {
                path,
                color,
                rule: FillRule::Winding,
            })
        })
        .collect())
}

fn collect_text_bounds(ops: &[Op], transform: Transform, clip: Option<Rect>, out: &mut Vec<Rect>) {
    for op in ops {
        match op {
            Op::Text { face, glyphs, .. } => {
                let Ok(font) = FontRef::try_from_slice(face.ttf_data()) else {
                    continue;
                };
                for glyph in glyphs {
                    let Some(outline) = font.outline_glyph(glyph.glyph.clone()) else {
                        continue;
                    };
                    let bb = outline.px_bounds();
                    let Some(mut bounds) = Rect::from_xywh(
                        bb.min.x + glyph.x as f32,
                        bb.min.y + glyph.y as f32,
                        bb.width(),
                        bb.height(),
                    )
                    .and_then(|r| r.transform(transform)) else {
                        continue;
                    };
                    if let Some(clip) = clip {
                        let Some(clipped) = bounds.intersect(&clip) else {
                            continue;
                        };
                        bounds = clipped;
                    }
                    out.push(bounds);
                }
            }
            Op::Group {
                transform: local,
                clip: bounds,
                ops,
            } => {
                let transform = transform.pre_concat(*local);
                let local_clip = bounds.and_then(|r| r.transform(transform));
                let clip = match (clip, local_clip) {
                    (Some(a), Some(b)) => {
                        let Some(c) = a.intersect(&b) else {
                            continue;
                        };
                        Some(c)
                    }
                    (a, b) => a.or(b),
                };
                collect_text_bounds(ops, transform, clip, out);
            }
            _ => {}
        }
    }
}
