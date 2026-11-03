use super::*;
use tiny_skia::{Mask, Paint, Pixmap, PixmapPaint};

pub(super) fn encode(
    scene: &LabelScene,
    options: ScenePngOptions,
) -> Result<Vec<u8>, LabelizeError> {
    let scale = options.scale;
    let (w, h) = (
        (scene.width as f64 * scale).ceil(),
        (scene.height as f64 * scale).ceil(),
    );
    if !scale.is_finite()
        || scale <= 0.0
        || w < 1.0
        || h < 1.0
        || w > MAX_DIMENSION
        || h > MAX_DIMENSION
        || w * h > MAX_OUTPUT_PIXELS
    {
        return Err(LabelizeError::Render(
            "invalid output scale or output pixel limit exceeded".into(),
        ));
    }
    let (w, h) = (w as u32, h as u32);
    let mut pixmap = Pixmap::new(w, h)
        .ok_or_else(|| LabelizeError::Render("could not allocate output canvas".into()))?;
    pixmap.fill(tiny_skia::Color::WHITE);
    draw_ops(
        &scene.ops,
        &mut pixmap,
        Transform::from_scale(scale as f32, scale as f32),
        None,
    )?;
    let samples: Vec<u8> = pixmap
        .pixels()
        .iter()
        .map(|p| {
            let value = p.red();
            if options.antialias {
                value
            } else if value > 128 {
                255
            } else {
                0
            }
        })
        .collect();
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, w, h);
        encoder.set_color(png::ColorType::Grayscale);
        encoder.set_depth(png::BitDepth::Eight);
        let ppm = (scene.dpmm as f64 * scale * 1000.0).round();
        if ppm < 1.0 || ppm > u32::MAX as f64 {
            return Err(LabelizeError::Render(
                "PNG physical resolution out of range".into(),
            ));
        }
        encoder.set_pixel_dims(Some(png::PixelDimensions {
            xppu: ppm as u32,
            yppu: ppm as u32,
            unit: png::Unit::Meter,
        }));
        let mut writer = encoder
            .write_header()
            .map_err(|e| LabelizeError::Encode(e.to_string()))?;
        writer
            .write_image_data(&samples)
            .map_err(|e| LabelizeError::Encode(e.to_string()))?;
    }
    Ok(out)
}

fn draw_ops(
    ops: &[Op],
    pixmap: &mut Pixmap,
    transform: Transform,
    clip: Option<&Mask>,
) -> Result<(), LabelizeError> {
    for op in ops {
        match op {
            Op::Path { path, color, rule } => {
                let mut paint = Paint::default();
                paint.set_color_rgba8(*color, *color, *color, 255);
                pixmap.fill_path(path, &paint, *rule, transform, clip);
            }
            Op::Group {
                transform: local,
                clip: bounds,
                ops,
            } => {
                let transform = transform.pre_concat(*local);
                let mut mask = clip.cloned();
                if let Some(bounds) = bounds {
                    let path = PathBuilder::from_rect(*bounds);
                    if let Some(mask) = &mut mask {
                        mask.intersect_path(&path, FillRule::Winding, true, transform);
                    } else {
                        let mut created =
                            Mask::new(pixmap.width(), pixmap.height()).ok_or_else(|| {
                                LabelizeError::Render("could not allocate clip mask".into())
                            })?;
                        created.fill_path(&path, FillRule::Winding, true, transform);
                        mask = Some(created);
                    }
                }
                draw_ops(ops, pixmap, transform, mask.as_ref())?;
            }
            Op::Image { image, x, y } => {
                let mut source = Pixmap::new(image.width(), image.height())
                    .ok_or_else(|| LabelizeError::Render("invalid image".into()))?;
                for (output, input) in source.pixels_mut().iter_mut().zip(image.pixels()) {
                    *output = tiny_skia::ColorU8::from_rgba(input[0], input[1], input[2], input[3])
                        .premultiply();
                }
                pixmap.draw_pixmap(
                    0,
                    0,
                    source.as_ref(),
                    &PixmapPaint::default(),
                    transform.pre_translate(*x, *y),
                    clip,
                );
            }
            Op::Text { face, glyphs } => {
                let font = FontRef::try_from_slice(face.ttf_data())
                    .map_err(|e| LabelizeError::Render(e.to_string()))?;
                for g in glyphs {
                    if let Some(path) = super::glyphs::path(&font, g) {
                        let mut paint = Paint::default();
                        paint.set_color_rgba8(g.color[0], g.color[1], g.color[2], g.color[3]);
                        let local = Transform::from_row(
                            g.glyph.scale.x / font.height_unscaled(),
                            0.0,
                            0.0,
                            -g.glyph.scale.y / font.height_unscaled(),
                            g.glyph.position.x + g.x as f32,
                            g.glyph.position.y + g.y as f32,
                        );
                        pixmap.fill_path(
                            &path,
                            &paint,
                            FillRule::Winding,
                            transform.pre_concat(local),
                            clip,
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
