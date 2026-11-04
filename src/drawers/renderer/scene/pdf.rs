use super::*;
use krilla::geom::{
    Path as PdfPath, PathBuilder as PdfPathBuilder, Point, Size, Transform as PdfTransform,
};
use krilla::paint::{Fill, FillRule as PdfFillRule};
use krilla::surface::Surface;
use krilla::tagging::{ContentTag, SpanTag};
use krilla::text::{Font, GlyphId, KrillaGlyph};
use krilla::{page::PageSettings, Document};

/// Encode one or more scenes into one PDF, preserving each page's physical size.
pub fn encode_scenes_pdf(scenes: &[LabelScene]) -> Result<Vec<u8>, LabelizeError> {
    encode(scenes, false)
}

#[cfg(test)]
pub(crate) fn outline_control(scenes: &[LabelScene]) -> Result<Vec<u8>, LabelizeError> {
    encode(scenes, true)
}

fn encode(scenes: &[LabelScene], outlined: bool) -> Result<Vec<u8>, LabelizeError> {
    if scenes.is_empty() {
        return Err(LabelizeError::Render("no scenes to encode".into()));
    }
    let mut document = Document::new();
    let mut fonts = Vec::<(FontFace, Font)>::new();
    for scene in scenes {
        let settings =
            PageSettings::from_wh(page_extent(scene.width_mm), page_extent(scene.height_mm))
                .ok_or_else(|| LabelizeError::Render("invalid PDF page dimensions".into()))?;
        let mut page = document.start_page_with(settings);
        let mut surface = page.surface();
        let scale = (72.0 / (25.4 * scene.dpmm as f64)) as f32;
        surface.push_transform(&PdfTransform::from_scale(scale, scale));
        let result = draw_ops(&scene.ops, &mut surface, &mut fonts, outlined);
        surface.pop();
        surface.finish();
        page.finish();
        result?;
    }
    document
        .finish()
        .map_err(|e| LabelizeError::Encode(format!("vector PDF: {e:?}")))
}

// Viewers ceil the physical page size to a device-pixel canvas. Rounding a
// mathematically exact dot boundary upwards can therefore add a whole pixel
// and stretch narrow barcode modules. Use the adjacent lower f32 only when
// nearest rounding would exceed the requested physical size.
fn page_extent(mm: f64) -> f32 {
    let exact = mm * 72.0 / 25.4;
    let rounded = exact as f32;
    if rounded as f64 > exact {
        rounded.next_down()
    } else {
        rounded
    }
}

fn pdf_transform(t: Transform) -> PdfTransform {
    PdfTransform::from_row(t.sx, t.ky, t.kx, t.sy, t.tx, t.ty)
}

fn pdf_path(path: &Path) -> Result<PdfPath, LabelizeError> {
    let mut builder = PdfPathBuilder::new();
    for segment in path.segments() {
        match segment {
            tiny_skia::PathSegment::MoveTo(p) => builder.move_to(p.x, p.y),
            tiny_skia::PathSegment::LineTo(p) => builder.line_to(p.x, p.y),
            tiny_skia::PathSegment::QuadTo(c, p) => builder.quad_to(c.x, c.y, p.x, p.y),
            tiny_skia::PathSegment::CubicTo(a, b, p) => {
                builder.cubic_to(a.x, a.y, b.x, b.y, p.x, p.y)
            }
            tiny_skia::PathSegment::Close => builder.close(),
        }
    }
    builder
        .finish()
        .ok_or_else(|| LabelizeError::Render("invalid drawing path".into()))
}

fn draw_ops(
    ops: &[Op],
    surface: &mut Surface<'_>,
    fonts: &mut Vec<(FontFace, Font)>,
    outlined: bool,
) -> Result<(), LabelizeError> {
    for op in ops {
        match op {
            Op::Path { path, color, rule } => {
                surface.set_fill(Some(Fill {
                    paint: krilla::color::rgb::Color::new(*color, *color, *color).into(),
                    rule: match rule {
                        FillRule::EvenOdd => PdfFillRule::EvenOdd,
                        _ => PdfFillRule::NonZero,
                    },
                    ..Fill::default()
                }));
                surface.draw_path(&pdf_path(path)?);
            }
            Op::Image { image, x, y } => {
                surface.push_transform(&PdfTransform::from_translate(*x, *y));
                if let Some(paths) = binary_image_paths(image) {
                    // Local image resampling can change entire rows/columns
                    // even with Interpolate=false. Bilevel pixels have exact
                    // geometry: paint their background once, then their ink.
                    surface.set_fill(Some(Fill {
                        paint: krilla::color::rgb::Color::new(255, 255, 255).into(),
                        ..Fill::default()
                    }));
                    surface.draw_path(&pdf_path(&PathBuilder::from_rect(
                        Rect::from_xywh(0.0, 0.0, image.width() as f32, image.height() as f32)
                            .ok_or_else(|| LabelizeError::Render("empty image".into()))?,
                    ))?);
                    surface.set_fill(Some(Fill::default()));
                    for path in paths {
                        surface.draw_path(&path);
                    }
                } else {
                    surface.draw_image(
                        krilla::image::Image::from_rgba8(
                            image.as_raw().clone(),
                            image.width(),
                            image.height(),
                        ),
                        Size::from_wh(image.width() as f32, image.height() as f32)
                            .ok_or_else(|| LabelizeError::Render("empty image".into()))?,
                    );
                }
                surface.pop();
            }
            Op::Group {
                transform,
                clip,
                ops,
            } => {
                surface.push_transform(&pdf_transform(*transform));
                if let Some(clip) = clip {
                    surface.push_clip_path(
                        &pdf_path(&PathBuilder::from_rect(*clip))?,
                        &PdfFillRule::NonZero,
                    );
                }
                let result = draw_ops(ops, surface, fonts, outlined);
                if clip.is_some() {
                    surface.pop();
                }
                surface.pop();
                result?;
            }
            Op::Text { face, glyphs } => {
                let font = if let Some((_, font)) = fonts.iter().find(|(f, _)| f == face) {
                    font.clone()
                } else {
                    let font = Font::new(face.ttf_data().into(), 0).ok_or_else(|| {
                        LabelizeError::Render("embedded font could not be loaded".into())
                    })?;
                    fonts.push((*face, font.clone()));
                    font
                };
                let metrics = FontRef::try_from_slice(face.ttf_data())
                    .map_err(|e| LabelizeError::Render(e.to_string()))?;
                let factor = metrics
                    .units_per_em()
                    .ok_or_else(|| LabelizeError::Render("font has no em size".into()))?
                    / metrics.height_unscaled();
                let mut rest = glyphs.as_slice();
                while !rest.is_empty() {
                    let count = rest
                        .iter()
                        .skip(1)
                        .position(|g| g.line_start)
                        .map(|i| i + 1)
                        .unwrap_or(rest.len());
                    let (line, tail) = rest.split_at(count);
                    rest = tail;
                    let text: String = line.iter().map(|g| g.character).collect();
                    surface.start_tagged(ContentTag::Span(
                        SpanTag::empty().with_actual_text(Some(&text)),
                    ));
                    let result = draw_line(line, &font, &metrics, factor, surface, outlined);
                    surface.end_tagged();
                    result?;
                }
            }
        }
    }
    Ok(())
}

// Bound the cost of representing binary pixels as rectangles. Dense/noisy
// source images and images with grayscale or alpha keep their image encoding.
// This does not reconstruct smooth outlines from raster text or graphics.
fn binary_image_paths(image: &RgbaImage) -> Option<Vec<PdfPath>> {
    use std::collections::BTreeMap;
    const MAX_RECTS: usize = 4096;
    if image
        .pixels()
        .any(|p| p[3] != 255 || p[0] != p[1] || p[1] != p[2] || !matches!(p[0], 0 | 255))
    {
        return None;
    }
    let mut active = BTreeMap::<(u32, u32), u32>::new();
    let mut builder = PdfPathBuilder::new();
    let mut count = 0;
    let mut flush = |(x, width): (u32, u32), from: u32, to: u32| {
        count += 1;
        // Bias binary pixel edges toward the interior by less than 1/1000
        // dot. This absorbs point/dot transform roundoff: Poppler otherwise
        // adds gray coverage to the neighboring white pixel at exact dot
        // boundaries, which can change local barcode binarization.
        const EDGE_GUARD: f32 = 0.000_1;
        let (x, y, right, bottom) = (
            x as f32 + EDGE_GUARD,
            from as f32 + EDGE_GUARD,
            (x + width) as f32 - EDGE_GUARD,
            to as f32 - EDGE_GUARD,
        );
        builder.move_to(x, y);
        builder.line_to(right, y);
        builder.line_to(right, bottom);
        builder.line_to(x, bottom);
        builder.close();
        count <= MAX_RECTS
    };
    for y in 0..image.height() {
        let mut next = BTreeMap::new();
        let mut x = 0;
        while x < image.width() {
            if image.get_pixel(x, y)[0] == 255 {
                x += 1;
                continue;
            }
            let start = x;
            while x < image.width() && image.get_pixel(x, y)[0] == 0 {
                x += 1;
            }
            let key = (start, x - start);
            next.insert(key, active.remove(&key).unwrap_or(y));
        }
        for (key, from) in active {
            if !flush(key, from, y) {
                return None;
            }
        }
        active = next;
    }
    for (key, from) in active {
        if !flush(key, from, image.height()) {
            return None;
        }
    }
    Some(builder.finish().into_iter().collect())
}

fn draw_line(
    glyphs: &[text_layout::PlacedGlyph],
    font: &Font,
    metrics: &FontRef,
    factor: f32,
    surface: &mut Surface<'_>,
    outlined: bool,
) -> Result<(), LabelizeError> {
    let mut rest = glyphs;
    while let Some(first) = rest.first() {
        let count = rest
            .iter()
            .take_while(|g| g.glyph.scale == first.glyph.scale && g.color == first.color)
            .count();
        let (run, tail) = rest.split_at(count);
        let corrected = run.iter().any(|glyph| glyph.dedot);
        rest = tail;
        let sx = first.glyph.scale.x * factor;
        let sy = first.glyph.scale.y * factor;
        if sx <= 0.0 || sy <= 0.0 {
            continue;
        }
        // Corrected resident zero outlines need a searchable text layer with
        // the original glyph mapping; its ink is replaced by the shared path.
        // Fill opacity leaves the text in the page stream (no form XObject).
        surface.set_fill(Some(Fill {
            paint: krilla::color::rgb::Color::new(first.color[0], first.color[1], first.color[2])
                .into(),
            opacity: if corrected {
                krilla::num::NormalizedF32::ZERO
            } else {
                krilla::num::NormalizedF32::ONE
            },
            ..Fill::default()
        }));
        surface.push_transform(&PdfTransform::from_scale(sx, sy));
        // CID widths are fractional PDF font units, but some viewers use
        // integer widths. Starting every glyph at its shared layout position
        // avoids accumulating that rounding across a long line, while the
        // enclosing ActualText span preserves the logical line for extraction.
        for g in run {
            let mut utf8 = [0; 4];
            let text = g.character.encode_utf8(&mut utf8);
            let positioned = KrillaGlyph {
                glyph_id: GlyphId::new(g.glyph.id.0 as u32),
                text_range: 0..text.len(),
                x_advance: metrics.h_advance_unscaled(g.glyph.id)
                    / metrics.units_per_em().unwrap_or(1.0),
                y_advance: 0.0,
                x_offset: 0.0,
                y_offset: 0.0,
                location: None,
            };
            surface.draw_glyphs(
                Point::from_xy(
                    (g.x as f32 + g.glyph.position.x) / sx,
                    (g.y as f32 + g.glyph.position.y) / sy,
                ),
                &[positioned],
                font.clone(),
                text,
                1.0,
                outlined,
            );
        }
        surface.pop();
        if corrected {
            surface.set_fill(Some(Fill::default()));
            for g in run {
                if let Some(path) = super::glyphs::path(metrics, g) {
                    let path = pdf_path(&path)?;
                    surface.push_transform(&PdfTransform::from_row(
                        g.glyph.scale.x / metrics.height_unscaled(),
                        0.0,
                        0.0,
                        -g.glyph.scale.y / metrics.height_unscaled(),
                        g.glyph.position.x + g.x as f32,
                        g.glyph.position.y + g.y as f32,
                    ));
                    surface.draw_path(&path);
                    surface.pop();
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grayscale_alpha_and_complex_images_keep_their_image_encoding() {
        for pixel in [Rgba([128, 128, 128, 255]), Rgba([0, 0, 0, 128])] {
            let scene = LabelScene {
                width: 8,
                height: 8,
                width_mm: 1.0,
                height_mm: 1.0,
                dpmm: 8,
                ops: vec![Op::Image {
                    image: Arc::new(RgbaImage::from_pixel(8, 8, pixel)),
                    x: 0.0,
                    y: 0.0,
                }],
                stats: SceneStats::default(),
            };
            let pdf = lopdf::Document::load_mem(&scene.to_pdf().unwrap()).unwrap();
            assert!(pdf.objects.values().any(|object| {
                object.as_stream().is_ok_and(|stream| {
                    stream
                        .dict
                        .get(b"Subtype")
                        .and_then(|v| v.as_name())
                        .is_ok_and(|name| name == b"Image")
                })
            }));
        }
        let noisy = RgbaImage::from_fn(130, 130, |x, y| {
            let v = if (x + y) % 2 == 0 { 0 } else { 255 };
            Rgba([v, v, v, 255])
        });
        assert!(binary_image_paths(&noisy).is_none());
    }
}
