//! Shared glyph layout and its legacy raster consumer.
//!
//! Layout emits positioned glyphs and an optional offscreen-buffer transform.
//! It never rasterizes. The raster and vector backends consume the same result.
use super::*;

#[derive(Clone, Debug)]
pub(crate) struct PlacedGlyph {
    #[cfg_attr(not(feature = "vector"), allow(dead_code))]
    pub character: char,
    pub glyph: ab_glyph::Glyph,
    pub x: i32,
    pub y: i32,
    pub color: Rgba<u8>,
    pub dedot: bool,
    /// Begins another logical line within this text operation.
    pub line_start: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct TextBuffer {
    pub width: u32,
    pub height: u32,
    pub orientation: FieldOrientation,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct TextLayout {
    pub face: FontFace,
    pub glyphs: Vec<PlacedGlyph>,
    pub buffer: Option<TextBuffer>,
}

impl TextLayout {
    pub(crate) fn draw(&self, canvas: &mut RgbaImage) -> Result<(), String> {
        let font = FontRef::try_from_slice(self.face.ttf_data()).map_err(|e| e.to_string())?;
        if let Some(buffer) = &self.buffer {
            let mut image = RgbaImage::from_pixel(buffer.width, buffer.height, Rgba([0, 0, 0, 0]));
            self.draw_glyphs(&mut image, &font);
            let image = match buffer.orientation {
                FieldOrientation::Rotated90 => rotate_90(&image),
                FieldOrientation::Rotated180 => rotate_180(&image),
                FieldOrientation::Rotated270 => rotate_270(&image),
                _ => image,
            };
            overlay_at(canvas, &image, buffer.x, buffer.y);
        } else {
            self.draw_glyphs(canvas, &font);
        }
        Ok(())
    }

    fn draw_glyphs(&self, canvas: &mut RgbaImage, font: &FontRef) {
        for placed in &self.glyphs {
            if let Some(outline) = font.outline_glyph(placed.glyph.clone()) {
                blit_glyph(
                    canvas,
                    placed.color,
                    &outline,
                    placed.x,
                    placed.y,
                    placed.dedot,
                );
            }
        }
    }
}

pub(crate) fn layout(text: &TextField, state: &mut DrawerState) -> Result<TextLayout, String> {
    let mut glyphs = Vec::new();
    // Some bitmap faces carry no lowercase glyphs; the printer (and the
    // reference renderer) draw lowercase field data as uppercase. Match that
    // here so the TTF substitute doesn't produce mixed-case output real
    // hardware can't.
    let face = text.font.face;
    let drawn_text: String = if face.uppercases_input() {
        text.text.to_uppercase()
    } else {
        text.text.clone()
    };
    let font_data = face.ttf_data();
    let font =
        FontRef::try_from_slice(font_data).map_err(|e| format!("failed to load font: {}", e))?;

    // The advance table and the vertical pen offset are calibrated for the scalable
    // font 0 only; the bitmap fonts are left on their existing metrics.
    let f0 = face == FontFace::CondensedBold;
    let font_size = text.font.get_size() as f32;
    let scale_x = text.font.get_scale_x() as f32;
    let mut scale = PxScale {
        x: font_size * scale_x,
        y: font_size,
    };

    // Compute font ascent for ^FT baseline positioning.
    // Use a ZPL-proportional ascent (~76% of cell height) to match Zebra font metrics,
    // since our substitute TTF fonts have different ascent ratios.
    let ascent = font_size * 0.76;

    // Bitmap fonts (A–H) correction:
    // Zebra's built-in bitmap Font A uses 7 of its 9 dot rows for capital letters (cap-height
    // = 7/9 of cell height).  DejaVu Mono, our TTF substitute, renders cap letters at ~6/9 of
    // the em-square.  To match Zebra's proportions we:
    //   1. Scale scale.y by 7/6 so the rendered cap-height becomes 7/9 × cell.
    //   2. Shift the draw-Y upward so the cap top aligns with the field origin (^FO y-coord)
    //      instead of being pushed down by the TTF ascender gap.
    // The shift is computed from the actual 'H' glyph bounds at the ORIGINAL scale so that
    // it remains correct for any font size or magnification.
    let subst = SubstMetrics {
        f0,
        adv_mult: text.font.advance_mult as f32,
        dedot_zero: text.font.dedot_zero,
    };
    // Font B cap height measured on Labelary: ~11.5 dots per magnification for an 11-dot
    // cell (caps overshoot the nominal cell), where DejaVu Mono Bold caps land at ~0.657 of
    // the ab_glyph scale -- hence 11.5/11/0.657 = 1.59 vs the generic 7/6 font-A correction.
    // Fonts P-V (resident bitmaps): Labelary caps measure ~0.60-0.625x the base
    // cell (P: 12/20, Q: 17/28, S: 25/40), where DejaVu Mono Bold caps land at
    // ~0.72em -- hence ~0.85 (sweep 0.80-0.90 converged on 0.85).
    // Font 1 (mono substitute): DejaVu Sans Mono regular caps land at 1493/2384
    // ≈ 0.626 of the ab_glyph scale and must read as 0.73 of the height param.
    let line_height_factor: f32 = face.line_height_factor() as f32;
    // Font 0's substitute needs a cap-height correction like the bitmap
    // fonts (see tuning::FONT0_CAP_SCALE): inflate the em so caps land at
    // 0.75 of the cell, matching Labelary. The font-0 constants expressed
    // in scaled-em units (advance deltas, missing-glyph advance, the
    // size-proportional pen offset) already account for this factor.
    if f0 {
        scale.y = font_size * crate::tuning::FONT0_CAP_SCALE as f32;
    }
    let cap_scale: f32 = face.cap_scale() as f32;
    // Bitmap fonts (A-H, P-V) enlarge the em by cap_scale only to match glyph
    // size; the ^FB line pitch on the printer is still the font cell height
    // (+ line spacing). Without this, an 18-dot font C block advances 21 dots
    // per line instead of 18 and long blocks overflow into following fields.
    let line_height_factor: f32 = if face.divides_line_height_by_cap() {
        line_height_factor / cap_scale
    } else {
        line_height_factor
    };
    let bitmap_y_shift: f64 = if face.uses_cell_shift() {
        scale.y = font_size * cap_scale;

        let orig_scale = PxScale {
            x: scale.x,
            y: font_size,
        };
        let orig_ascent = font.as_scaled(orig_scale).ascent();
        let orig_gap = font
            .outline_glyph(
                font.glyph_id('H')
                    .with_scale_and_position(orig_scale, ab_glyph::point(0.0, orig_ascent)),
            )
            .map(|g| g.px_bounds().min.y as f64)
            .unwrap_or(font_size as f64 * 0.148);

        // With the cap-scaled em, the ascender gap also scales by the same factor.
        // Shift the draw origin UP by that new gap so cap_top = field y.
        // Labelary anchors P-V caps ~0.15x cell BELOW the field origin (measured:
        // P top is 3px below origin at 20-dot cell, Q 4px at 28-dot, S 6px at
        // 40-dot), so push the origin back down by that amount.
        let p_low_cap = font_size as f64 * face.cap_anchor_drop();
        -(orig_gap * cap_scale as f64) + p_low_cap
    } else {
        0.0
    };

    // Measure text width approximately (scale already includes scale_x).
    // Use superscript-aware measurement so that ® is counted at its rendered size.
    let text_width = measure_text_width_with_superscript(&drawn_text, &font, scale, subst) as f64;

    // For field blocks, use block width for positioning instead of measured text width
    let pos_width = if let Some(ref block) = text.block {
        block.max_width as f64
    } else {
        text_width
    };

    // Effective between-line pitch actually drawn by draw_text_block. For
    // font 1 (fractional factor + cap-scaled em) this differs from the raw
    // height param, and multiline ^FT anchoring must span the drawn extent.
    let line_pitch = scale.y * line_height_factor
        + text
            .block
            .as_ref()
            .map(|b| b.line_spacing as f32)
            .unwrap_or(0.0);
    let (x, y) = get_text_top_left_pos(
        text,
        pos_width,
        font_size as f64,
        ascent as f64,
        state,
        line_pitch as f64,
    );
    // Apply bitmap y-correction (zero for non-bitmap fonts).
    let y = y + bitmap_y_shift;
    // Font 1's mono substitute lsb rounds one pixel wider than Labelary's
    // face. The nudge lives in LOCAL text space (pen axis) so it rotates
    // with the field instead of shifting page-space X for R/I/B.
    let pen_x_offset: f32 = face.pen_x_offset() as f32;
    state.update_automatic_text_position(text, pos_width);

    let color = Rgba([0, 0, 0, 255]);

    // Render text to a buffer, then rotate if needed
    let orientation = text.font.orientation;
    // Overprinting blends glyph edges repeatedly. Use the same transparent
    // buffer for every orientation in that case: blending directly onto
    // white rounds coverage differently from accumulating alpha, which can
    // change the final monochrome mask after several overlapping lines.
    let buffer_overflow = orientation == FieldOrientation::Normal
        && text.block.as_ref().is_some_and(|block| {
            block.overwrite_last_line
                && word_wrap(&drawn_text, &font, scale, block, subst).len()
                    > block.max_lines.max(1) as usize
        });

    if orientation == FieldOrientation::Normal && !buffer_overflow {
        // Normal: draw directly onto canvas (no rotation needed)
        if let Some(ref block) = text.block {
            draw_text_block(
                &mut glyphs,
                &font,
                scale,
                0.0,
                color,
                x as f32 + pen_x_offset,
                y as f32,
                block,
                &drawn_text,
                subst,
                line_height_factor,
                0,
            );
        } else {
            draw_text_with_superscript(
                &mut glyphs,
                &font,
                scale,
                color,
                x as f32 + pen_x_offset,
                y as f32,
                &drawn_text,
                subst,
            );
        }
    } else {
        // Render to a transparent buffer, rotate as needed, then overlay.
        // Headroom above the ascent keeps marks that rise above it (Ä, É) from
        // being clipped at the buffer's top edge; a matching `bottom_pad`
        // below the descent keeps deep-extending glyphs (ˎ, j) from being
        // clipped at the bottom.
        //
        // The buffer's width must cover the laid-out INK, not just the
        // corrected advance, and must leave a left margin for ink that
        // reaches left of the pen: font-0 advance deltas can be negative
        // enough (Ā −0.41 em) that the corrected advance undercuts the
        // glyph's own ink, and negative left bearings (Ï, the Greek tonos
        // capitals, j) overhang the pen origin — both were silently
        // clipped at the buffer edges before. Growing the trailing side
        // (`delta_r`) and shifting the pen right by `left_pad` keep every
        // previously rendered pixel in place once the overlay anchors are
        // rebased per orientation (see below).
        let headroom = scale.y.ceil() as u32;
        // Baseline-to-buffer-bottom depth the draw needs: glyph point.y is
        // `ascent (+ yoff for f0)` below the pen top, `ink_max_y` below
        // that. The pen top is the first line's top: headroom alone for
        // blocks, headroom + f0's rotated pen bias (see below) for plain
        // fields — block drawing does not apply that bias.
        let f0_rot_pen_bias: f32 = if f0 { -1.0 - 0.0253 * font_size } else { 0.0 };
        let ascent = font.as_scaled(scale).ascent()
            + if f0 {
                crate::tuning::TEXT_Y_OFFSET as f32
                    + (crate::tuning::TEXT_Y_OFFSET_EM * scale.y as f64) as f32
            } else {
                0.0
            };
        let bottom_pad_for = |pen_top: f32, content_h: u32, ink_max_y: f32| -> u32 {
            if ink_max_y == f32::MIN {
                return 0;
            }
            // The deepest glyph hangs from the DEEPEST line's baseline
            // (the last one with positive pitch, line 1 with negative), so
            // the caller passes that line's top as `pen_top`; +1 for the
            // exclusive pixel edge, the ceil absorbs sub-pixel pitch and
            // the blit's independent rounding.
            let needed = pen_top + ascent + ink_max_y + 1.0;
            (needed.ceil() as i64 - (headroom + content_h) as i64).max(0) as u32
        };
        let (buf_w, buf_h, left_pad, delta_r, bottom_pad, top_pad) = if let Some(ref block) =
            text.block
        {
            let lines = word_wrap(&drawn_text, &font, scale, block, subst);
            // Same pitch draw_text_block uses (cap-scaled em × factor).
            // ^FB line spacing may be negative: line tops are
            // (k−1)·pitch, so later lines then sit ABOVE the pen top and
            // the deepest line need not be the last — size both vertical
            // margins from the actual line-top extents.
            let pitch = scale.y * line_height_factor + block.line_spacing as f32;
            let max_lines = block.max_lines.max(1) as usize;
            let num_lines = lines.len().min(max_lines);
            let spread = (num_lines - 1) as f32 * pitch;
            let (min_top, max_top) = (spread.min(0.0), spread.max(0.0));
            let top_pad = (-min_top).ceil().max(0.0) as u32;
            let h = (max_top - min_top + scale.y).ceil() as u32 + 2;
            // Per-line ink extents at the block origin cover unbreakable
            // words wider than max_width and per-character overhangs
            // past the advance edge. Centered and right-aligned lines
            // get draw_text_block's alignment offset —
            // negative for lines wider than max_width, which is exactly
            // the unbreakable-word case that must reach left of the
            // origin. Justified lines stay within [0, max_width].
            let mut ink_min = f32::MAX;
            let mut ink_max = f32::MIN;
            let mut ink_max_y = f32::MIN;
            let drawn_lines = if block.overwrite_last_line {
                lines.len()
            } else {
                num_lines
            };
            for line in lines.iter().take(drawn_lines) {
                let (a, b, y) = measure_text_ink_bounds_with_superscript(line, &font, scale, subst);
                if a <= b {
                    let lw = measure_text_width(line, &font, scale, subst);
                    let lx_offset = match block.alignment {
                        crate::elements::text_alignment::TextAlignment::Center => {
                            (block.max_width as f32 - lw) / 2.0
                        }
                        crate::elements::text_alignment::TextAlignment::Right => {
                            block.max_width as f32 - lw
                        }
                        _ => 0.0,
                    };
                    ink_min = ink_min.min(lx_offset + a);
                    ink_max = ink_max.max(lx_offset + b);
                    ink_max_y = ink_max_y.max(y);
                }
            }
            let (over_l, over_r) = block_ink_overhangs(&drawn_text, &font, scale, subst);
            let left_pad = if ink_min <= ink_max {
                (-(pen_x_offset + ink_min.min(over_l))).round().max(0.0) as u32
            } else {
                (-(pen_x_offset + over_l)).round().max(0.0) as u32
            };
            let old_w = block.max_width as u32 + 2;
            let content_w = old_w.max(
                (ink_max.max(block.max_width as f32 + pen_x_offset + over_r) + 2.0)
                    .ceil()
                    .max(0.0) as u32,
            );
            // The deepest line's top: lines extend (k−1)·pitch from the
            // pen, so with negative spacing it is line 1, not the last
            // (draw_text_block accumulates `pitch` per line, snapping
            // later line tops — sub-pixel vs this unrounded value,
            // absorbed by the pad's ceil and +1).
            let deepest_top = headroom as f32 + top_pad as f32 + max_top;
            (
                content_w + left_pad,
                h,
                left_pad,
                content_w - old_w,
                bottom_pad_for(deepest_top, h, ink_max_y),
                top_pad,
            )
        } else {
            let old_w = (text_width as f32).ceil() as u32 + 2;
            let (ink_min, ink_max, ink_max_y) =
                measure_text_ink_bounds_with_superscript(&drawn_text, &font, scale, subst);
            let (left_pad, content_w) = if ink_min <= ink_max {
                let left_pad = (-(pen_x_offset + ink_min)).round().max(0.0) as u32;
                // +1 for the exclusive pixel edge, +1 to absorb the blit's
                // independent rounding of the glyph bounds.
                let content_w = old_w.max((pen_x_offset + ink_max + 2.0).ceil().max(0.0) as u32);
                (left_pad, content_w)
            } else {
                // Nothing outlines (blank/missing glyphs only).
                (0, old_w)
            };
            // Use scale.y (may be larger than font_size for bitmap fonts) for buffer height.
            // Rotated font 0 keeps the cell-height buffer the rotated anchors were
            // calibrated with — the cap-scaled em would shift the ink 0.37 em after
            // the flip (see the pen bias below).
            let h = if f0 { font_size } else { scale.y }.ceil() as u32 + 2;
            (
                content_w + left_pad,
                h,
                left_pad,
                content_w - old_w,
                bottom_pad_for(headroom as f32 + f0_rot_pen_bias, h, ink_max_y),
                0,
            )
        };

        if buf_w == 0 || buf_h == 0 {
            return Ok(TextLayout {
                face,
                glyphs,
                buffer: None,
            });
        }

        let buffer_height = buf_h + headroom + bottom_pad + top_pad;

        if let Some(ref block) = text.block {
            draw_text_block(
                &mut glyphs,
                &font,
                scale,
                if orientation == FieldOrientation::Normal {
                    (y - y.trunc()) as f32
                } else {
                    0.0
                },
                color,
                pen_x_offset,
                headroom as f32 + top_pad as f32,
                block,
                &drawn_text,
                subst,
                line_height_factor,
                left_pad as i32,
            );
        } else {
            // Rotated font 0: place the ink where the previous substitute's
            // calibration put it (ink top at −0.8 − 0.02·cell), undoing the
            // cap-scale gap so the rotation anchors reproduce the old geometry.
            // The headroom offset keeps that above-origin ink — and any
            // diacritics rising above it — inside the buffer instead of
            // clipping it at the top edge.
            let pen_y = if f0 {
                headroom as f32 - 1.0 - 0.0253 * font_size
            } else {
                headroom as f32
            };
            draw_text_with_superscript(
                &mut glyphs,
                &font,
                scale,
                color,
                pen_x_offset + left_pad as f32,
                pen_y,
                &drawn_text,
                subst,
            );
        }

        // Font 0 only: the calibrated advance-axis correction for I/B rotations
        // (see `tuning::ROTATED_ADVANCE_OFFSET`). Blocks whose text is centred
        // inside the block box are excluded — the box padding cancels out for
        // centred lines, and their rotated golden cases (amazonshipping ^FWB ^FB
        // centered fields) already align with the raw overlay position. Left-
        // justified rotated blocks (dhlparceluk) anchor at the pen like plain
        // text and need the same correction. Bitmap fonts stay uncorrected too.
        let anchored_at_pen = text
            .block
            .as_ref()
            .map(|b| {
                matches!(
                    b.alignment,
                    crate::elements::text_alignment::TextAlignment::Left
                        | crate::elements::text_alignment::TextAlignment::Justified
                        | crate::elements::text_alignment::TextAlignment::Right
                )
            })
            .unwrap_or(true);
        let (ox, oy) = if f0 && anchored_at_pen {
            match orientation {
                FieldOrientation::Rotated180 => (x - crate::tuning::ROTATED_ADVANCE_OFFSET, y),
                FieldOrientation::Rotated270 => (x, y - crate::tuning::ROTATED_ADVANCE_OFFSET),
                _ => (x, y),
            }
        } else {
            (x, y)
        };
        // Only 270° turns the headroom rows into leading columns.
        let ox = if orientation == FieldOrientation::Rotated270 {
            ox - headroom as f64
        } else {
            ox
        };
        // #64's unclipping exposed (and probe-measured against Labelary at
        // 12–90 pt confirmed) a glyph-top anchor debt on rotated font-0
        // plain text: the glyph sits 1–4 px toward its cap side depending
        // on orientation — R +3..4 px along +x, I 3 px along +y, B 1..2 px
        // along −x. Shift each orientation back along its top axis.
        let (ox, oy) = if f0 && text.block.is_none() {
            match orientation {
                FieldOrientation::Rotated90 => (ox - 3.0, oy),
                FieldOrientation::Rotated180 => (ox, oy - 3.0),
                FieldOrientation::Rotated270 => (ox + 2.0, oy),
                _ => (ox, oy),
            }
        } else {
            (ox, oy)
        };
        // Rebase the anchors for the ink-extent buffer sizing above, so
        // previously rendered pixels stay exactly where they were and only
        // previously clipped ink appears. Which buffer edge each rotation
        // anchors on decides the correction:
        //   R:  buffer columns map to canvas rows — the left margin would
        //       push ink down one-for-one (trailing growth only extends it
        //       further down, away from the anchor); buffer rows map to
        //       canvas columns flipped, so the bottom pad pushes ink right.
        //   I:  buffer columns map to canvas columns, flipped — trailing
        //       growth pushes ink toward the anchor, while the left margin
        //       cancels itself against the flip; the bottom pad pushes ink
        //       down toward the anchor the same way.
        //   B:  buffer columns map to canvas rows, flipped — same as I on
        //       the other axis; rows map to columns directly, so both the
        //       left margin and the bottom pad extend away from the anchor
        //       (the top headroom is already rebased above).
        let (mut ox, oy) = match orientation {
            FieldOrientation::Normal => {
                // Snap before subtracting padding so crossing zero cannot
                // shift a near-edge field by a pixel.
                (
                    ox.trunc() - left_pad as f64,
                    oy.trunc() - (headroom + top_pad) as f64,
                )
            }
            FieldOrientation::Rotated90 => (ox - bottom_pad as f64, oy - left_pad as f64),
            FieldOrientation::Rotated180 => (ox - delta_r as f64, oy - bottom_pad as f64),
            FieldOrientation::Rotated270 => (ox - top_pad as f64, oy - delta_r as f64),
        };
        // Labelary reserves the full ^FB box (max_lines × pitch, signed —
        // negative line spacing flips the stacking direction) for rotated
        // blocks: line k renders at field_x + (max_lines − k + 1)·pitch
        // along the box's stacking axis. Probed with FB,1/FB,2/FB,3/FB,4 ×
        // 1- and 2-line content at positive and negative pitch — the
        // offset tracks max_lines regardless of how many lines wrap. Our
        // buffer stacks lines from the pen, so re-anchor line 1 onto
        // field_x + max_lines·pitch; the buffer's own (k−1)·pitch stacking
        // (flipped by the rotation) then places every line k.
        if f0 && orientation == FieldOrientation::Rotated90 {
            if let Some(ref block) = text.block {
                let pitch = scale.y * line_height_factor + block.line_spacing as f32;
                let reserved = block.max_lines.max(1) as f32 * pitch;
                ox += reserved as f64 - buf_h as f64 - f0_rot_pen_bias as f64;
            }
        }

        return Ok(TextLayout {
            face,
            glyphs,
            buffer: Some(TextBuffer {
                width: buf_w,
                height: buffer_height,
                orientation,
                x: ox as i32,
                y: oy as i32,
            }),
        });
    }

    Ok(TextLayout {
        face,
        glyphs,
        buffer: None,
    })
}

#[allow(clippy::too_many_arguments)]
/// Draw `text` replicating imageproc's `draw_text_mut` layout, with the calibrated
/// vertical offset applied to the pen before glyphs snap to the pixel grid, the
/// font-0 per-character advance corrections folded into the pen advance, and the
/// bitmap-font advance multiplier applied to the substitute's cell advance.
///
/// The font-0 corrections are calibrated for the scalable font 0 and applied only
/// when `m.f0` is set; the bitmap advance multiplier only rescales the pen (glyph
/// rasterization keeps the substitute's own shape width).
///
/// The layout quirks here are deliberate copies of imageproc's `layout_glyphs`
/// (advance added before kerning, kern arguments in `(current, prev)` order, kerning
/// only applied for glyphs that outline), so for a plain `SubstMetrics` this renders
/// exactly what the upstream helper renders.
fn draw_text_snapped(
    glyphs: &mut Vec<PlacedGlyph>,
    color: Rgba<u8>,
    x: i32,
    y: i32,
    scale: PxScale,
    font: &FontRef,
    text: &str,
    m: SubstMetrics,
) {
    use ab_glyph::{Font, GlyphId, ScaleFont};

    let yoff = if m.f0 {
        crate::tuning::TEXT_Y_OFFSET as f32
            + (crate::tuning::TEXT_Y_OFFSET_EM * scale.y as f64) as f32
    } else {
        0.0
    };
    let scaled = font.as_scaled(scale);
    // Same width-ratio scaling as measure_text_width: the additive font-0
    // deltas are calibrated at width == height and must track the hmtx
    // component's ∝(w/h) scaling on cells where the two parameters differ.
    let width_ratio = if m.f0 {
        (scale.x / scale.y) * (crate::tuning::FONT0_CAP_SCALE / crate::tuning::FONT0_RATIO) as f32
    } else {
        1.0
    };

    let mut w = 0.0f32;
    let mut prev: Option<GlyphId> = None;

    for c in text.chars() {
        let glyph_id = effective_glyph_id(font, c, m.f0);
        if glyph_id == ab_glyph::GlyphId(0) {
            // Blank like Labelary: no glyph is drawn and no kerning applies on
            // either side (imageproc only kerns outlined glyphs), but the pen
            // still advances — see `missing_glyph_advance`.
            w += missing_glyph_advance(font, m, scale, width_ratio);
            prev = None;
            continue;
        }
        let (draw_scale, draw_dx, draw_dy, draw_id) =
            outline_placement(font, glyph_id, c, m, scale, width_ratio);
        let glyph = draw_id.with_scale_and_position(
            draw_scale,
            ab_glyph::point(w + draw_dx, scaled.ascent() + yoff + draw_dy),
        );
        w += scaled.h_advance(glyph_id) * m.adv_mult;
        if m.f0 {
            w += crate::tuning::font0_advance_delta(c) as f32 * scale.y * width_ratio;
        }
        glyphs.push(PlacedGlyph {
            character: c,
            glyph: glyph.clone(),
            x,
            y,
            color,
            dedot: m.dedot_zero && c == '0',
            line_start: false,
        });
        if font.outline_glyph(glyph).is_some() {
            if let Some(prev_id) = prev {
                w += scaled.kern(glyph_id, prev_id);
            }
            prev = Some(glyph_id);
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// Draw text onto `canvas`, rendering ® (and, for font 0, ©) as a top-aligned
/// superscript at REGISTERED_MARK_SCALE.
fn draw_text_with_superscript(
    canvas: &mut Vec<PlacedGlyph>,
    font: &FontRef,
    scale: PxScale,
    color: Rgba<u8>,
    x: f32,
    y: f32,
    text: &str,
    m: SubstMetrics,
) {
    if !text.chars().any(|c| is_superscript_mark(c, m.f0)) {
        draw_text_snapped(canvas, color, x as i32, y as i32, scale, font, text, m);
        return;
    }
    let super_scale = PxScale {
        x: scale.x * REGISTERED_MARK_SCALE,
        y: scale.y * REGISTERED_MARK_SCALE,
    };
    let mut cx = x;
    for (seg, mark) in superscript_segments(text, m.f0) {
        if !seg.is_empty() {
            draw_text_snapped(canvas, color, cx as i32, y as i32, scale, font, seg, m);
            cx += measure_text_width(seg, font, scale, m);
        }
        if let Some(mark) = mark {
            let mark_str = mark.to_string();
            draw_text_snapped(
                canvas,
                color,
                cx as i32,
                y as i32,
                super_scale,
                font,
                &mark_str,
                m,
            );
            cx += measure_text_width(&mark_str, font, super_scale, m);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_text_block(
    canvas: &mut Vec<PlacedGlyph>,
    font: &FontRef,
    scale: PxScale,
    line_y_phase: f32,
    color: Rgba<u8>,
    x: f32,
    y: f32,
    block: &crate::elements::field_block::FieldBlock,
    text: &str,
    m: SubstMetrics,
    line_height_factor: f32,
    pen_x_pad: i32,
) {
    let max_width = block.max_width as f32;
    let lines = word_wrap(text, font, scale, block, m);
    let font_size = scale.y;
    let raw_line_height = font_size * line_height_factor + block.line_spacing as f32;
    // Bitmap fonts divide the cap-scaled em back down to the cell height, which
    // lands a float epsilon off an integer pitch; treat that as integral.
    let integral_pitch = (raw_line_height - raw_line_height.round()).abs() < 1e-3;
    let line_height = if integral_pitch {
        raw_line_height.round()
    } else {
        raw_line_height
    };

    let mut cy = y;
    let max_lines = block.max_lines.max(1) as usize;
    // A fractional line pitch (font 1's calibrated 0.7625 factor) accumulates
    // sub-pixel drift on wrapped lines; round those line tops. The first line
    // keeps the raw (truncating) pen — its anchor is already probe-calibrated
    // via bitmap_y_shift, and rounding it shifts single-line fields by 1px.
    let snap_y = |v: f32, first: bool| {
        if first || line_height_factor == 1.0 || integral_pitch {
            v
        } else {
            // Buffered normal fields subtract their integer page origin.
            // Retain its fractional phase when snapping later line tops.
            (v + line_y_phase).round()
        }
    };
    for (i, line) in lines.iter().enumerate() {
        if i >= max_lines && !block.overwrite_last_line {
            break;
        }
        let slot = i.min(max_lines - 1);
        let first_glyph = canvas.len();
        let first = slot == 0;
        let lx = match block.alignment {
            crate::elements::text_alignment::TextAlignment::Center => {
                let lw = measure_text_width(line, font, scale, m);
                x + (block.max_width as f32 - lw) / 2.0
            }
            crate::elements::text_alignment::TextAlignment::Right => {
                let lw = measure_text_width(line, font, scale, m);
                x + block.max_width as f32 - lw
            }
            // Justified (J): every line except the block's last is stretched to
            // the full block width by distributing the slack across the word
            // gaps; the last line stays left-aligned (ZPL ^FB d=J semantics).
            crate::elements::text_alignment::TextAlignment::Justified
                if i + 1 < lines.len().min(max_lines) =>
            {
                draw_justified_line(
                    canvas,
                    font,
                    scale,
                    color,
                    x,
                    snap_y(cy, first),
                    line,
                    m,
                    max_width,
                    pen_x_pad,
                );
                if let Some(glyph) = canvas.get_mut(first_glyph) {
                    glyph.line_start = i > 0;
                }
                if i + 1 < max_lines {
                    cy += line_height;
                }
                continue;
            }
            _ => x,
        };
        // The rotated-buffer caller passes its left margin as `pen_x_pad`
        // rather than in `x`: the per-line pen must be SNAPPED first and the
        // integer pad added afterwards, or `as i32` truncation of a
        // negative pen that crosses zero inside the padded value shifts one
        // line relative to the others instead of translating them all.
        let pen_x = if pen_x_pad == 0 {
            lx
        } else {
            lx.trunc() + pen_x_pad as f32
        };
        draw_text_with_superscript(
            canvas,
            font,
            scale,
            color,
            pen_x,
            snap_y(cy, first),
            line,
            m,
        );
        if let Some(glyph) = canvas.get_mut(first_glyph) {
            glyph.line_start = i > 0;
        }
        if i + 1 < max_lines {
            cy += line_height;
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// Draw one ^FB justified line: words at their natural advances with the
/// block's leftover width distributed evenly across the inter-word gaps.
fn draw_justified_line(
    canvas: &mut Vec<PlacedGlyph>,
    font: &FontRef,
    scale: PxScale,
    color: Rgba<u8>,
    x: f32,
    y: f32,
    line: &str,
    m: SubstMetrics,
    max_width: f32,
    pen_x_pad: i32,
) {
    let words: Vec<&str> = line.split_whitespace().collect();
    if words.len() < 2 {
        draw_text_with_superscript(
            canvas,
            font,
            scale,
            color,
            x.round() + pen_x_pad as f32,
            y,
            line,
            m,
        );
        return;
    }
    // Superscript-aware draw and measurement, matching every other text path:
    // a ® inside a justified word must render at REGISTERED_MARK_SCALE and its
    // width must not distort the distributed slack.
    let lw = measure_text_width_with_superscript(line, font, scale, m);
    let space_w = measure_text_width_with_superscript(" ", font, scale, m);
    let extra = (max_width - lw) / (words.len() - 1) as f32;
    let mut cx = x;
    for (k, word) in words.iter().enumerate() {
        // Keep the logical word separator even though its advance is set by
        // justification. A space has no ink and leaves legacy pixels unchanged.
        let word_with_space = if k + 1 < words.len() {
            format!("{word} ")
        } else {
            (*word).to_owned()
        };
        // round() commutes with integer shifts, so adding the rotated
        // buffer's pad here stays a pure translation (and matches the
        // normal-path word snapping).
        draw_text_with_superscript(
            canvas,
            font,
            scale,
            color,
            cx.round() + pen_x_pad as f32,
            y,
            &word_with_space,
            m,
        );
        cx += measure_text_width_with_superscript(word, font, scale, m);
        if k + 1 < words.len() {
            cx += space_w + extra;
        }
    }
}

fn blit_glyph(
    canvas: &mut RgbaImage,
    color: Rgba<u8>,
    g: &ab_glyph::OutlinedGlyph,
    x: i32,
    y: i32,
    dedot: bool,
) {
    use image::Pixel;
    let (cw, ch) = (canvas.width() as i32, canvas.height() as i32);
    let bb = g.px_bounds();
    let x_shift = x + bb.min.x.round() as i32;
    let y_shift = y + bb.min.y.round() as i32;
    // Which pixels to drop: when de-dotting a bitmap-font zero, ink not
    // connected to the raster border is skipped entirely (see
    // `dedotted_pixels`). Computed once as a coverage grid.
    let dropped = if dedot {
        Some(dedotted_pixels(g))
    } else {
        None
    };
    let raster_w = dropped_w(&bb);
    g.draw(|gx, gy, gv| {
        if let Some(dropped) = &dropped {
            if dropped[(gy as usize) * raster_w + gx as usize] {
                return;
            }
        }
        let px = gx as i32 + x_shift;
        let py = gy as i32 + y_shift;
        if (0..cw).contains(&px) && (0..ch).contains(&py) {
            let mut pixel = *canvas.get_pixel(px as u32, py as u32);
            let gv = gv.clamp(0.0, 1.0);
            // imageproc's Clamp<f32> for u8 truncates rather than rounds;
            // match it exactly so a zero offset is bit-identical to upstream.
            let a = color[3] as f32 * gv;
            let a = if a < 255.0 {
                if a > 0.0 {
                    a as u8
                } else {
                    0
                }
            } else {
                255
            };
            let src = Rgba([color[0], color[1], color[2], a]);
            pixel.blend(&src);
            canvas.put_pixel(px as u32, py as u32, pixel);
        }
    });
}
