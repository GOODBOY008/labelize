use std::io::Write;

use ab_glyph::{Font as _, FontRef, PxScale, ScaleFont as _};
use image::{Pixel, Rgba, RgbaImage};
use imageproc::drawing;

use crate::barcodes;
use crate::elements::barcode_128::BarcodeMode;
use crate::elements::drawer_options::DrawerOptions;
use crate::elements::field_orientation::FieldOrientation;
use crate::elements::font_face::FontFace;
use crate::elements::graphic_field::{GraphicField, GraphicFieldMode};
use crate::elements::label_element::LabelElement;
use crate::elements::label_info::LabelInfo;
use crate::elements::label_position::LabelPosition;
use crate::elements::line_color::LineColor;
use crate::elements::text_field::TextField;
use crate::elements::text_flow::{TextDirection, TextFlow};
use crate::images;

use super::drawer_state::DrawerState;

pub(crate) mod barcode_layout;
#[cfg(feature = "vector")]
pub mod scene;
pub(crate) mod text_layout;

static FONT_ZERO: &[u8] = crate::assets::FONT_ZERO_SUBSTITUTE;
static FONT_DEJAVU_MONO: &[u8] = crate::assets::FONT_DEJAVU_SANS_MONO;

/// Substitute-face layout knobs for one text field, derived from the ZPL font
/// name and threaded through the pen walks (this supersedes the bare `f0`
/// flag those walks used to take). `f0` selects the font-0 corrections;
/// `adv_mult` rescales the substitute's cell advance for the resident bitmap
/// fonts whose real cells advance wider than the coded width (see
/// `tuning::bitmap_advance_mult`); `dedot_zero` marks faces standing in for
/// Zebra bitmap fonts, whose zeros are clean ovals — the DejaVu substitute's
/// dotted zero gets its interior island removed at raster time.
#[derive(Clone, Copy)]
struct SubstMetrics {
    f0: bool,
    adv_mult: f32,
    dedot_zero: bool,
    character_spacing: f32,
    wrap_spacing: f32,
}

impl SubstMetrics {
    /// Neutral metrics for the separately calibrated barcode interpretation
    /// lines: the plain substitute cell advance, no font-0 corrections, no
    /// bitmap zero de-dotting.
    const PLAIN: Self = SubstMetrics {
        f0: false,
        adv_mult: 1.0,
        dedot_zero: false,
        character_spacing: 0.0,
        wrap_spacing: 0.0,
    };
}

pub struct Renderer;

impl Default for Renderer {
    fn default() -> Self {
        Self
    }
}

impl Renderer {
    pub fn new() -> Self {
        Renderer
    }

    pub fn draw_label_as_png(
        &self,
        label: &LabelInfo,
        output: &mut dyn Write,
        options: DrawerOptions,
    ) -> Result<(), String> {
        let options = options.with_defaults();
        let mut state = DrawerState::new();

        let width_mm = options.label_width_mm;
        let height_mm = options.label_height_mm;
        let dpmm = options.dpmm;

        // A dot count converted through millimetres can land a few ULPs
        // above an integer (e.g. 7 / 24000 * 24000). Preserve that integer
        // canvas while still rounding genuine fractional dots upward.
        let canvas_dots = |mm: f64| {
            let dots = mm * dpmm as f64;
            let nearest = dots.round();
            if (dots - nearest).abs() <= dots.abs() * f64::EPSILON * 4.0 {
                nearest as i32
            } else {
                dots.ceil() as i32
            }
        };
        let label_width = canvas_dots(width_mm);
        let image_width = if label.print_width > 0 {
            label_width.min(label.print_width)
        } else {
            label_width
        };
        let image_height = canvas_dots(height_mm);

        let mut canvas = RgbaImage::from_pixel(
            image_width as u32,
            image_height as u32,
            Rgba([255, 255, 255, 255]),
        );

        let mut reverse_buf: Option<RgbaImage> = None;

        // Print-time region inversions (e.g. TSPL REVERSE): collected here,
        // skipped as drawable, and inverted over the finished label before
        // any whole-label transformation.
        let invert_regions: Vec<_> = label
            .elements
            .iter()
            .filter_map(|e| match e {
                LabelElement::InvertRegion(r) => {
                    Some((r.position.x, r.position.y, r.width, r.height))
                }
                _ => None,
            })
            .collect();

        for element in &label.elements {
            if matches!(element, LabelElement::InvertRegion(_)) {
                continue;
            }
            let reverse_print = element.is_reverse_print();

            if reverse_print {
                let buf = reverse_buf.get_or_insert_with(|| {
                    RgbaImage::from_pixel(
                        image_width as u32,
                        image_height as u32,
                        Rgba([0, 0, 0, 0]),
                    )
                });
                // Clear buffer
                for pixel in buf.pixels_mut() {
                    *pixel = Rgba([0, 0, 0, 0]);
                }
                self.draw_element(buf, element, &options, &mut state)?;
                images::reverse_print::reverse_print(buf, &mut canvas);
            } else {
                self.draw_element(&mut canvas, element, &options, &mut state)?;
            }
        }

        // Apply print-time region inversions in label coordinate space,
        // before print-width centering / whole-label inversion / mirroring.
        for (x, y, w, h) in invert_regions {
            let (x0, y0) = (x.max(0) as u32, y.max(0) as u32);
            let (x1, y1) = ((x + w).max(0) as u32, (y + h).max(0) as u32);
            let (x1, y1) = (x1.min(canvas.width()), y1.min(canvas.height()));
            for py in y0..y1 {
                for px in x0..x1 {
                    let p = canvas.get_pixel(px, py);
                    let inverted = 255u8 - p[0];
                    canvas.put_pixel(px, py, Rgba([inverted, inverted, inverted, p[3]]));
                }
            }
        }

        // Handle print width centering and label inversion
        let invert_label = options.enable_inverted_labels && label.inverted;
        if image_width != label_width || invert_label {
            let mut final_canvas = RgbaImage::from_pixel(
                label_width as u32,
                image_height as u32,
                Rgba([255, 255, 255, 255]),
            );

            let offset_x = ((label_width - image_width) / 2) as i64;

            if invert_label {
                // Rotate the rendered content 180° and composite it over the white
                // canvas with imageops::overlay, matching the centering path below.
                // Element drawing can leave semi-transparent pixels on the canvas
                // (rotated text buffers are stamped in without blending), so this
                // branch must also use src-over compositing rather than raw pixel copy;
                // otherwise the 1-bit encode will treat any covered pixel as solid black.
                // offset (label_width - image_width) - offset_x reproduces the previous
                // dst_x = label_width - 1 - x - offset_x mapping one-to-one.
                let rotated = image::imageops::rotate180(&canvas);
                let inverted_offset_x = (label_width - image_width) as i64 - offset_x;
                image::imageops::overlay(&mut final_canvas, &rotated, inverted_offset_x, 0);
            } else {
                image::imageops::overlay(&mut final_canvas, &canvas, offset_x, 0);
            }
            canvas = final_canvas;
        }

        // ^PM mirrors the complete output after print-width centering and ^PO.
        // Flipping only the narrower ^PW canvas misplaces odd margins by a dot.
        if label.mirrored {
            image::imageops::flip_horizontal_in_place(&mut canvas);
        }

        let mut buf = Vec::new();
        images::monochrome::encode_png_with(&canvas, &mut buf, options.antialias)
            .map_err(|e| format!("failed to encode png: {}", e))?;
        output
            .write_all(&buf)
            .map_err(|e| format!("failed to write png: {}", e))
    }

    fn draw_element(
        &self,
        canvas: &mut RgbaImage,
        element: &LabelElement,
        options: &DrawerOptions,
        state: &mut DrawerState,
    ) -> Result<(), String> {
        // Bottom-of-label-anchored elements that are not drawn from an
        // encoded image flip here, where the canvas height is known; the
        // image-drawn elements (barcodes, 2-D codes) flip inside
        // `adjust_image_typeset_position`, which sees the real bitmap
        // extents.
        let flipped;
        let element = match flip_from_bottom(element, canvas.height()) {
            Some(adjusted) => {
                flipped = adjusted;
                &flipped
            }
            None => element,
        };
        match element {
            LabelElement::Text(text) => self.draw_text(canvas, text, state),
            LabelElement::GraphicBox(gb) => {
                self.draw_graphic_box(canvas, gb);
                Ok(())
            }
            LabelElement::GraphicCircle(gc) => {
                self.draw_graphic_circle(canvas, gc);
                Ok(())
            }
            LabelElement::GraphicEllipse(ge) => {
                self.draw_graphic_ellipse(canvas, ge);
                Ok(())
            }
            LabelElement::DiagonalLine(dl) => {
                self.draw_diagonal_line(canvas, dl);
                Ok(())
            }
            LabelElement::GraphicField(gf) => {
                self.draw_graphic_field(canvas, gf);
                Ok(())
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
            | LabelElement::Maxicode(_)
            | LabelElement::BarcodeUcpe(_) => {
                barcode_layout::layout(element, canvas.height(), options)?.draw(canvas);
                Ok(())
            }
            _ => Ok(()), // Config/template elements are not drawn
        }
    }

    fn draw_text(
        &self,
        canvas: &mut RgbaImage,
        text: &TextField,
        state: &mut DrawerState,
    ) -> Result<(), String> {
        text_layout::layout(text, state)?.draw(canvas)
    }

    fn draw_graphic_box(
        &self,
        canvas: &mut RgbaImage,
        gb: &crate::elements::graphic_box::GraphicBox,
    ) {
        let color = line_color_to_rgba(gb.line_color);
        let x = gb.position.x;
        let y = gb.position.y;
        let w = gb.width.max(gb.border_thickness);
        let h = gb.height.max(gb.border_thickness);
        let border = gb.border_thickness;

        if gb.corner_rounding > 0 {
            // ZPL corner_rounding 1-8: radius = (shorter_side / 2) * (rounding / 8)
            let shorter = w.min(h);
            let radius =
                ((shorter as f64 / 2.0) * (gb.corner_rounding as f64 / 8.0)).round() as i32;
            draw_rounded_rect(canvas, x, y, w, h, border, radius, color);
        } else {
            // Draw box with border
            if border >= w || border >= h {
                // Filled box
                draw_filled_rect(canvas, x, y, w, h, color);
            } else {
                // Top
                draw_filled_rect(canvas, x, y, w, border, color);
                // Bottom
                draw_filled_rect(canvas, x, y + h - border, w, border, color);
                // Left
                draw_filled_rect(canvas, x, y, border, h, color);
                // Right
                draw_filled_rect(canvas, x + w - border, y, border, h, color);
            }
        }
    }

    fn draw_graphic_circle(
        &self,
        canvas: &mut RgbaImage,
        gc: &crate::elements::graphic_circle::GraphicCircle,
    ) {
        let color = line_color_to_rgba(gc.line_color);
        let cx = gc.position.x as f32 + gc.circle_diameter as f32 / 2.0;
        let cy = gc.position.y as f32 + gc.circle_diameter as f32 / 2.0;
        let outer_r = gc.circle_diameter as f32 / 2.0;
        let thickness = gc.border_thickness.max(1) as f32;

        if thickness >= outer_r {
            // Filled circle
            drawing::draw_filled_circle_mut(canvas, (cx as i32, cy as i32), outer_r as i32, color);
        } else {
            // Ring: draw filled outer, then erase inner with opposite pass
            // Use per-pixel distance check for accurate ring rendering
            let inner_r = outer_r - thickness;
            let outer_r_sq = outer_r * outer_r;
            let inner_r_sq = inner_r * inner_r;
            let (w, h) = canvas.dimensions();
            let min_x = ((cx - outer_r - 1.0).max(0.0)) as u32;
            let max_x = ((cx + outer_r + 1.0).min(w as f32 - 1.0)) as u32;
            let min_y = ((cy - outer_r - 1.0).max(0.0)) as u32;
            let max_y = ((cy + outer_r + 1.0).min(h as f32 - 1.0)) as u32;
            for py in min_y..=max_y {
                for px in min_x..=max_x {
                    let dx = px as f32 - cx;
                    let dy = py as f32 - cy;
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq <= outer_r_sq && dist_sq >= inner_r_sq {
                        canvas.put_pixel(px, py, color);
                    }
                }
            }
        }
    }

    fn draw_graphic_ellipse(
        &self,
        canvas: &mut RgbaImage,
        ge: &crate::elements::graphic_ellipse::GraphicEllipse,
    ) {
        let color = line_color_to_rgba(ge.line_color);
        let rx = (ge.width.max(1) as f32) / 2.0;
        let ry = (ge.height.max(1) as f32) / 2.0;
        let cx = ge.position.x as f32 + rx;
        let cy = ge.position.y as f32 + ry;
        let thickness = ge.border_thickness.max(1) as f32;

        // Fill when the border reaches the minor axis (like the circle rule).
        let min_r = rx.min(ry);
        let (w, h) = canvas.dimensions();
        let min_x = ((cx - rx - 1.0).max(0.0)) as u32;
        let max_x = ((cx + rx + 1.0).min(w as f32 - 1.0)) as u32;
        let min_y = ((cy - ry - 1.0).max(0.0)) as u32;
        let max_y = ((cy + ry + 1.0).min(h as f32 - 1.0)) as u32;
        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let dx = px as f32 - cx;
                let dy = py as f32 - cy;
                let d = (dx / rx) * (dx / rx) + (dy / ry) * (dy / ry);
                if d <= 1.0
                    && (thickness >= min_r || {
                        let inner_rx = rx - thickness;
                        let inner_ry = ry - thickness;
                        let d2 =
                            (dx / inner_rx) * (dx / inner_rx) + (dy / inner_ry) * (dy / inner_ry);
                        d2 >= 1.0
                    })
                {
                    canvas.put_pixel(px, py, color);
                }
            }
        }
    }

    fn draw_diagonal_line(
        &self,
        canvas: &mut RgbaImage,
        dl: &crate::elements::graphic_diagonal_line::GraphicDiagonalLine,
    ) {
        let color = line_color_to_rgba(dl.line_color);
        let x = dl.position.x as f32;
        let y = dl.position.y as f32;
        let w = dl.width as f32;
        let h = dl.height as f32;
        let thickness = dl.border_thickness.max(1);

        if thickness <= 1 {
            if dl.top_to_bottom {
                drawing::draw_line_segment_mut(canvas, (x, y), (x + w, y + h), color);
            } else {
                drawing::draw_line_segment_mut(canvas, (x, y + h), (x + w, y), color);
            }
        } else {
            // Thick diagonal: fill a horizontal band of width `t` starting from the diagonal,
            // extending to the right (positive x direction), unclipped vertically but bounded
            // by the vertical span [y, y+h].
            //
            // The fill parallelogram for R (/): diagonal goes from (x+w,y) to (x,y+h).
            //   At each row, fill starts at diag_x and extends t pixels right.
            //   Parallelogram: (x+w, y), (x+w+t, y), (x+t, y+h), (x, y+h)
            //
            // The fill parallelogram for L (\): diagonal goes from (x,y) to (x+w,y+h).
            //   At each row, fill starts at diag_x and extends t pixels right.
            //   Parallelogram: (x, y), (x+t, y), (x+w+t, y+h), (x+w, y+h)
            let t = (thickness - 1) as f32; // inclusive fill: t pixels wide
            let para = if dl.top_to_bottom {
                // L (\)
                [(x, y), (x + t, y), (x + w + t, y + h), (x + w, y + h)]
            } else {
                // R (/)
                [(x + w, y), (x + w + t, y), (x + t, y + h), (x, y + h)]
            };

            let points: Vec<imageproc::point::Point<i32>> = para
                .iter()
                .map(|(px, py)| imageproc::point::Point::new(*px as i32, *py as i32))
                .collect();
            drawing::draw_polygon_mut(canvas, &points, color);
        }
    }

    fn draw_graphic_field(&self, canvas: &mut RgbaImage, gf: &GraphicField) {
        let data_len = if gf.total_bytes > 0 {
            (gf.total_bytes as usize).min(gf.data.len())
        } else {
            gf.data.len()
        };

        if gf.row_bytes <= 0 || data_len == 0 {
            return;
        }

        let width = gf.row_bytes * 8;
        let height = data_len as i32 / gf.row_bytes;

        let mag_x = gf.magnification_x.max(1);
        let mag_y = gf.magnification_y.max(1);

        // When positioned by ^FT, y is the bottom edge of the field; adjust to top-left.
        let base_x = gf.position.x;
        let base_y = if gf.position.calculate_from_bottom {
            gf.position.y - height * mag_y
        } else {
            gf.position.y
        };

        let black = Rgba([0, 0, 0, 255]);
        let white = Rgba([255, 255, 255, 255]);

        for y in 0..height {
            for x in 0..width {
                let idx = (y * (width / 8) + x / 8) as usize;
                if idx >= gf.data.len() {
                    continue;
                }
                let val = (gf.data[idx] >> (7 - x % 8)) & 1;
                let ink = if gf.zero_bits_black {
                    val == 0
                } else {
                    val != 0
                };
                if ink {
                    for my in 0..mag_y {
                        for mx in 0..mag_x {
                            let px = base_x + x * mag_x + mx;
                            let py = base_y + y * mag_y + my;
                            if px >= 0
                                && py >= 0
                                && (px as u32) < canvas.width()
                                && (py as u32) < canvas.height()
                            {
                                // XOR inverts whatever is already there (black
                                // ink over black canvas erases it); Overwrite
                                // and Or just paint black — on a 1-bit canvas
                                // Or is indistinguishable from Overwrite.
                                let current = canvas.get_pixel(px as u32, py as u32);
                                if gf.mode == GraphicFieldMode::Xor && *current == black {
                                    canvas.put_pixel(px as u32, py as u32, white);
                                } else {
                                    canvas.put_pixel(px as u32, py as u32, black);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Effective glyph id for laying out `ch`: Labelary's font 0 blanks the
/// characters in `FONT0_LABELARY_BLANKED` even though the substitute subset
/// carries their outlines, so for font 0 those must take the missing-glyph
/// path (no ink, calibrated advance) exactly like glyphs the subset lacks.
fn effective_glyph_id(font: &FontRef, ch: char, f0: bool) -> ab_glyph::GlyphId {
    let id = font.glyph_id(ch);
    if f0 && crate::tuning::font0_is_labelary_blanked(ch) {
        return ab_glyph::GlyphId(0);
    }
    id
}

/// Font-0 ink calibration for `ch` (issue #78): the substitute's glyph is
/// stretched horizontally and shifted so its pen-relative ink box matches
/// Labelary's measured extents (see `tuning::FONT0_INK_TARGETS`). Returns the
/// stretched `PxScale` for drawing the outline and the pen-space `(dx, dy)`
/// shifts to add to the glyph's position. `None` when the character is not
/// calibrated (or outlines nothing), leaving the glyph untouched.
fn font0_ink_adjust(
    font: &FontRef,
    glyph_id: ab_glyph::GlyphId,
    ch: char,
    scale: PxScale,
    width_ratio: f32,
) -> Option<(PxScale, f32, f32)> {
    use ab_glyph::Font as _;
    let (left_em, width_em, dy_em) = crate::tuning::font0_ink_target(ch)?;
    let probe = glyph_id.with_scale_and_position(scale, ab_glyph::point(0.0, 0.0));
    let probe = font.outline_glyph(probe)?;
    let bb = probe.px_bounds();
    let w0 = bb.width();
    if w0 <= 0.0 || w0.is_nan() {
        return None;
    }
    // Same em scaling as the advance deltas: the targets are calibrated at
    // width == height and must track the hmtx component's ∝(w/h) scaling.
    // The vertical shift follows scale.y only — vertical ink does not scale
    // with the w/h cell ratio.
    let target_w = width_em as f32 * scale.y * width_ratio;
    let target_l = left_em as f32 * scale.y * width_ratio;
    let dy = dy_em as f32 * scale.y;
    let k = (target_w / w0).max(0.1);
    let stretched = PxScale {
        x: scale.x * k,
        y: scale.y,
    };
    let dx = target_l - bb.min.x * k;
    Some((stretched, dx, dy))
}

/// Scale, pen shifts and glyph id for drawing `ch`'s outline in a layout walk:
/// font 0 applies the ink calibration plus the curly-quote shape substitution
/// (Labelary's font-0 substitute draws `'`/`` ` ``/`"` as `’`/`‘`/`“`);
/// every other face draws the glyph untouched. Advances and kerning stay
/// keyed on the original character's metrics either way.
#[inline]
fn outline_placement(
    font: &FontRef,
    glyph_id: ab_glyph::GlyphId,
    ch: char,
    m: SubstMetrics,
    scale: PxScale,
    width_ratio: f32,
) -> (PxScale, f32, f32, ab_glyph::GlyphId) {
    if !m.f0 {
        return (scale, 0.0, 0.0, glyph_id);
    }
    // Substitute the shape FIRST and calibrate against the shape glyph's own
    // outline — stretching by the original char's width would mis-scale the
    // substituted quotes badly (the straight-quote bar is far narrower than
    // the curly glyph that gets drawn).
    let shape_id = font.glyph_id(crate::tuning::font0_shape_glyph(ch));
    let (scale, dx, dy) =
        font0_ink_adjust(font, shape_id, ch, scale, width_ratio).unwrap_or((scale, 0.0, 0.0));
    (scale, dx, dy, shape_id)
}

/// Pen advance for a character the substitute face has no glyph for
/// (`font.glyph_id(c) == GlyphId(0)`). Labelary renders such characters as
/// blank space — no .notdef box — for every substitute font (probed against
/// font 0, font 1 and bitmap font A with CJK field data), while the pen still
/// advances: by the calibrated missing-glyph advance for font 0, and by
/// .notdef's own cell advance for the DejaVu faces, which matches Labelary's
/// one-cell blank to ±1 px.
fn missing_glyph_advance(font: &FontRef, m: SubstMetrics, scale: PxScale, width_ratio: f32) -> f32 {
    use ab_glyph::{Font as _, ScaleFont};
    if m.f0 {
        crate::tuning::FONT0_MISSING_GLYPH_ADVANCE_EM as f32 * scale.y * width_ratio
    } else {
        font.as_scaled(scale).h_advance(ab_glyph::GlyphId(0)) * m.adv_mult
    }
}

fn measure_text_width(text: &str, font: &FontRef, scale: PxScale, m: SubstMetrics) -> f32 {
    use ab_glyph::{Font, ScaleFont};
    let scaled = font.as_scaled(scale);
    // The advance deltas (and the missing-glyph advance) are calibrated in
    // y-scaled em at cells where the width parameter equals the height. The
    // glyph's own hmtx advance scales with scale.x (∝ w/h); the additive
    // deltas must scale with the same width ratio or narrow/wide cells
    // (^A0,60,35 ...) drift per character against Labelary.
    let width_ratio = if m.f0 {
        (scale.x / scale.y) * (crate::tuning::FONT0_CAP_SCALE / crate::tuning::FONT0_RATIO) as f32
    } else {
        1.0
    };
    let mut width = 0.0f32;
    let mut prev = None;
    for ch in text.chars() {
        let glyph_id = effective_glyph_id(font, ch, m.f0);
        if glyph_id == ab_glyph::GlyphId(0) {
            width += missing_glyph_advance(font, m, scale, width_ratio);
            // Nothing is drawn, so no kerning applies on either side —
            // matching imageproc's layout, which only kerns outlined glyphs.
            prev = None;
            continue;
        }
        if let Some(prev_id) = prev {
            width += scaled.kern(prev_id, glyph_id);
        }
        width += scaled.h_advance(glyph_id) * m.adv_mult;
        if m.f0 {
            width += crate::tuning::font0_advance_delta(ch) as f32 * scale.y * width_ratio;
        }
        prev = Some(glyph_id);
    }
    width + text.chars().count().saturating_sub(1) as f32 * m.character_spacing
}

/// The registered trademark symbol ® (U+00AE) is rendered as a superscript in Zebra's
/// built-in fonts: it appears smaller and top-aligned relative to surrounding text.
/// This scale factor approximates Zebra's built-in glyph size (≈55% of the em height).
const REGISTERED_MARK: char = '\u{00AE}';
const COPYRIGHT_MARK: char = '\u{00A9}';
const REGISTERED_MARK_SCALE: f32 = 0.55;

/// Characters rendered through the superscript path: ® for every face
/// (pre-existing behaviour), and for font 0 also `©` — Labelary's font-0
/// substitute draws both marks the same small size (~0.34 em ink width,
/// probe-verified identical extents), while the Roboto Condensed subset
/// carries © at full glyph size.
fn is_superscript_mark(ch: char, f0: bool) -> bool {
    ch == REGISTERED_MARK || (f0 && ch == COPYRIGHT_MARK)
}

/// Split `text` into `(plain segment, following superscript mark)` pairs; the
/// final segment has no mark. Empty segments occur when the text starts with
/// (or has adjacent) marks — callers skip them.
fn superscript_segments<'a>(
    text: &'a str,
    f0: bool,
) -> impl Iterator<Item = (&'a str, Option<char>)> + 'a {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        match rest.find(|c: char| is_superscript_mark(c, f0)) {
            Some(i) => {
                let (seg, tail) = rest.split_at(i);
                let mark = tail.chars().next().unwrap();
                rest = &tail[mark.len_utf8()..];
                Some((seg, Some(mark)))
            }
            None => {
                let seg = rest;
                rest = "";
                Some((seg, None))
            }
        }
    })
}

/// Measure text width treating ® (and, for font 0, ©) as superscripts
/// (at REGISTERED_MARK_SCALE of main scale).
fn measure_text_width_with_superscript(
    text: &str,
    font: &FontRef,
    scale: PxScale,
    m: SubstMetrics,
) -> f32 {
    if !text.chars().any(|c| is_superscript_mark(c, m.f0)) {
        return measure_text_width(text, font, scale, m);
    }
    let super_scale = PxScale {
        x: scale.x * REGISTERED_MARK_SCALE,
        y: scale.y * REGISTERED_MARK_SCALE,
    };
    let mut width = 0.0f32;
    let mut segments = 0usize;
    for (seg, mark) in superscript_segments(text, m.f0) {
        if !seg.is_empty() {
            if segments > 0 {
                width += m.character_spacing;
            }
            width += measure_text_width(seg, font, scale, m);
            segments += 1;
        }
        if let Some(mark) = mark {
            if segments > 0 {
                width += m.character_spacing;
            }
            let mark_str = mark.to_string();
            width += measure_text_width(&mark_str, font, super_scale, m);
            segments += 1;
        }
    }
    width
}

/// Alignment uses the full pen advance, including the final extra gap. Keep
/// the zero-gap font calibration unchanged while applying the extra separator
/// contribution of centred, word-wrapped text relative to that baseline.
fn block_alignment_width(
    text: &str,
    font: &FontRef,
    scale: PxScale,
    m: SubstMetrics,
    block: &crate::elements::field_block::FieldBlock,
) -> f32 {
    let mut width = measure_text_width(text, font, scale, m);
    if text.is_empty() {
        return width;
    }
    width += m.character_spacing;
    if block.alignment == crate::elements::text_alignment::TextAlignment::Center
        && m.character_spacing != 0.0
    {
        let space = measure_text_width(" ", font, scale, m);
        if width + space + m.character_spacing <= block.max_width as f32 {
            width += m.character_spacing;
        } else {
            width -= space;
        }
    }
    width
}

/// Ink extents of `text` as `draw_text_snapped` lays it out, pen-relative:
/// `(min_x, max_x, max_y)`. Mirrors the draw walk exactly — each glyph is
/// positioned at the pen before its own advance (and kern) are added — so
/// buffer sizes derived from these bounds contain every pixel the draw emits.
/// `px_bounds()` already includes the glyph position, so the horizontal values
/// are absolute against the pen origin; `max_y` is the deepest extent below
/// the glyph's baseline anchor (ab_glyph y grows downward, baseline at 0).
/// Missing glyphs contribute no ink (they render blank) but advance the pen.
/// Returns `(MAX, MIN, MIN)` when nothing outlines (caller checks order).
fn measure_text_ink_bounds(
    text: &str,
    font: &FontRef,
    scale: PxScale,
    m: SubstMetrics,
) -> (f32, f32, f32) {
    use ab_glyph::{Font, ScaleFont};
    let scaled = font.as_scaled(scale);
    let width_ratio = if m.f0 {
        (scale.x / scale.y) * (crate::tuning::FONT0_CAP_SCALE / crate::tuning::FONT0_RATIO) as f32
    } else {
        1.0
    };
    let mut w = 0.0f32;
    let mut prev: Option<ab_glyph::GlyphId> = None;
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for (index, c) in text.chars().enumerate() {
        if index > 0 {
            w += m.character_spacing;
        }
        let glyph_id = effective_glyph_id(font, c, m.f0);
        if glyph_id == ab_glyph::GlyphId(0) {
            w += missing_glyph_advance(font, m, scale, width_ratio);
            prev = None;
            continue;
        }
        // The glyph is positioned at the pen BEFORE its own advance and kern
        // are added — mirror `draw_text_snapped` exactly, including the
        // font-0 ink calibration's stretched scale and pen shifts.
        let (draw_scale, draw_dx, draw_dy, draw_id) =
            outline_placement(font, glyph_id, c, m, scale, width_ratio);
        let glyph =
            draw_id.with_scale_and_position(draw_scale, ab_glyph::point(w + draw_dx, draw_dy));
        w += scaled.h_advance(glyph_id) * m.adv_mult;
        if m.f0 {
            w += crate::tuning::font0_advance_delta(c) as f32 * scale.y * width_ratio;
        }
        if let Some(g) = font.outline_glyph(glyph) {
            if let Some(prev_id) = prev {
                w += scaled.kern(glyph_id, prev_id);
            }
            prev = Some(glyph_id);
            let bb = g.px_bounds();
            min_x = min_x.min(bb.min.x);
            max_x = max_x.max(bb.max.x);
            max_y = max_y.max(bb.max.y);
        }
    }
    (min_x, max_x, max_y)
}

/// Superscript-aware ink bounds, mirroring `draw_text_with_superscript`'s
/// split layout: ® (and, for font 0, ©) renders at REGISTERED_MARK_SCALE and
/// the segments advance the pen by their measured widths.
fn measure_text_ink_bounds_with_superscript(
    text: &str,
    font: &FontRef,
    scale: PxScale,
    m: SubstMetrics,
) -> (f32, f32, f32) {
    if !text.chars().any(|c| is_superscript_mark(c, m.f0)) {
        return measure_text_ink_bounds(text, font, scale, m);
    }
    let super_scale = PxScale {
        x: scale.x * REGISTERED_MARK_SCALE,
        y: scale.y * REGISTERED_MARK_SCALE,
    };
    let mut cx = 0.0f32;
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut segments = 0usize;
    for (seg, mark) in superscript_segments(text, m.f0) {
        if !seg.is_empty() {
            if segments > 0 {
                cx += m.character_spacing;
            }
            let (a, b, y) = measure_text_ink_bounds(seg, font, scale, m);
            if a <= b {
                min_x = min_x.min(cx + a);
                max_x = max_x.max(cx + b);
                max_y = max_y.max(y);
            }
            cx += measure_text_width(seg, font, scale, m);
            segments += 1;
        }
        if let Some(mark) = mark {
            if segments > 0 {
                cx += m.character_spacing;
            }
            let mark_str = mark.to_string();
            let (a, b, y) = measure_text_ink_bounds(&mark_str, font, super_scale, m);
            if a <= b {
                min_x = min_x.min(cx + a);
                max_x = max_x.max(cx + b);
                max_y = max_y.max(y);
            }
            cx += measure_text_width(&mark_str, font, super_scale, m);
            segments += 1;
        }
    }
    (min_x, max_x, max_y)
}

/// Conservative per-character ink overhangs for ^FB blocks at `scale`:
/// `(min bb.min.x, max (bb.max.x − corrected advance))` over the characters
/// present in `text`. Line alignment only moves pens right of the block
/// origin, so these bound how far block ink may reach past the origin (left)
/// and past `max_width` (right, via a line-final glyph's overhang).
fn block_ink_overhangs(text: &str, font: &FontRef, scale: PxScale, m: SubstMetrics) -> (f32, f32) {
    use ab_glyph::{Font, ScaleFont};
    let scaled = font.as_scaled(scale);
    let width_ratio = if m.f0 {
        (scale.x / scale.y) * (crate::tuning::FONT0_CAP_SCALE / crate::tuning::FONT0_RATIO) as f32
    } else {
        1.0
    };
    let mut min_lsb = 0.0f32;
    let mut max_over = 0.0f32;
    let mut seen = std::collections::HashSet::new();
    for c in text.chars() {
        if !seen.insert(c) {
            continue;
        }
        let glyph_id = effective_glyph_id(font, c, m.f0);
        if glyph_id == ab_glyph::GlyphId(0) {
            continue;
        }
        let (draw_scale, draw_dx, draw_dy, draw_id) =
            outline_placement(font, glyph_id, c, m, scale, width_ratio);
        let glyph = draw_id.with_scale_and_position(draw_scale, ab_glyph::point(draw_dx, draw_dy));
        let Some(g) = font.outline_glyph(glyph) else {
            continue;
        };
        let bb = g.px_bounds();
        min_lsb = min_lsb.min(bb.min.x);
        let mut adv = scaled.h_advance(glyph_id) * m.adv_mult;
        if m.f0 {
            adv += crate::tuning::font0_advance_delta(c) as f32 * scale.y * width_ratio;
        }
        max_over = max_over.max(bb.max.x - adv);
    }
    (min_lsb, max_over)
}

/// Raster width of an outlined glyph's pixel grid (matches `OutlinedGlyph::draw`
/// coordinates, which run `0..ceil(width)`).
fn dropped_w(bb: &ab_glyph::Rect) -> usize {
    (bb.width().ceil() as usize).max(1)
}

/// Ink mask of the glyph raster with interior islands cleared: a flood fill
/// from the raster border marks every ink pixel (coverage above the 1-bit
/// threshold) reachable via 8-connectivity; unreachable ink — the DejaVu
/// substitute's dotted zero's center dot, an island surrounded by the ring's
/// counter — is dropped. Ring strokes always touch the raster border, so a
/// clean zero is returned untouched and small sizes whose dot merges with the
/// ring lose nothing.
fn dedotted_pixels(g: &ab_glyph::OutlinedGlyph) -> Vec<bool> {
    let bb = g.px_bounds();
    let w = dropped_w(&bb);
    let h = (bb.height().ceil() as usize).max(1);
    let mut ink = vec![false; w * h];
    g.draw(|gx, gy, gv| {
        if gv.clamp(0.0, 1.0) > 0.5 {
            ink[gy as usize * w + gx as usize] = true;
        }
    });
    let mut keep = vec![false; w * h];
    let mut queue = std::collections::VecDeque::new();
    let border = (0..w).chain((w * (h - 1))..(w * h));
    for i in border {
        if ink[i] && !keep[i] {
            keep[i] = true;
            queue.push_back(i);
        }
    }
    for r in 0..h {
        for c in [0, w - 1] {
            let i = r * w + c;
            if ink[i] && !keep[i] {
                keep[i] = true;
                queue.push_back(i);
            }
        }
    }
    while let Some(i) = queue.pop_front() {
        let (r, c) = (i / w, i % w);
        for dr in [-1i64, 0, 1] {
            for dc in [-1i64, 0, 1] {
                if (dr == 0 && dc == 0) || !(0..h as i64).contains(&(r as i64 + dr)) {
                    continue;
                }
                let nc = c as i64 + dc;
                if !(0..w as i64).contains(&nc) {
                    continue;
                }
                let n = (r as i64 + dr) as usize * w + nc as usize;
                if ink[n] && !keep[n] {
                    keep[n] = true;
                    queue.push_back(n);
                }
            }
        }
    }
    // The caller skips what this returns: inked but never reached by the
    // border flood — the zero's interior island.
    ink.iter().zip(&keep).map(|(&i, &k)| i && !k).collect()
}

fn word_wrap(
    text: &str,
    font: &FontRef,
    scale: PxScale,
    block: &crate::elements::field_block::FieldBlock,
    m: SubstMetrics,
) -> Vec<String> {
    word_wrap_details(text, font, scale, block, m)
        .into_iter()
        .map(|line| line.text)
        .collect()
}

struct WrappedLine {
    text: String,
    trailing_word_space: bool,
}

fn word_wrap_details(
    text: &str,
    font: &FontRef,
    scale: PxScale,
    block: &crate::elements::field_block::FieldBlock,
    m: SubstMetrics,
) -> Vec<WrappedLine> {
    let m = SubstMetrics {
        character_spacing: m.wrap_spacing,
        ..m
    };
    let max_width = block.max_width as f32;
    let mut lines = Vec::new();
    for line in text.split('\n') {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.is_empty() {
            lines.push(WrappedLine {
                text: String::new(),
                trailing_word_space: false,
            });
            continue;
        }
        let mut current_line = String::new();
        for word in words {
            if !current_line.is_empty() {
                let test = format!("{} {}", current_line, word);
                if measure_text_width(&test, font, scale, m) <= max_width {
                    current_line = test;
                    continue;
                }
                lines.push(WrappedLine {
                    text: std::mem::take(&mut current_line),
                    trailing_word_space: true,
                });
            }
            if block.wrap_long_words && measure_text_width(word, font, scale, m) > max_width {
                for ch in word.chars() {
                    let previous_len = current_line.len();
                    current_line.push(ch);
                    if previous_len > 0
                        && measure_text_width(&current_line, font, scale, m) > max_width
                    {
                        current_line.truncate(previous_len);
                        lines.push(WrappedLine {
                            text: std::mem::take(&mut current_line),
                            trailing_word_space: false,
                        });
                        current_line.push(ch);
                    }
                }
            } else {
                current_line.push_str(word);
            }
        }
        lines.push(WrappedLine {
            text: current_line,
            trailing_word_space: false,
        });
    }
    lines
}

#[allow(clippy::too_many_arguments)]
fn get_text_top_left_pos(
    text: &TextField,
    w: f64,
    h: f64,
    ascent: f64,
    state: &DrawerState,
    line_pitch: f64,
) -> (f64, f64) {
    let (x, y) = state.get_text_position(text);

    if !text.position.calculate_from_bottom {
        // ^FO: position is top-left of the field area. Handle justification parameter.
        let x = match text.alignment {
            crate::elements::field_alignment::FieldAlignment::Right => x - w,
            crate::elements::field_alignment::FieldAlignment::Center => x - w / 2.0,
            _ => x,
        };
        return (x, y);
    }

    // ^FT: position is baseline (bottom-left for Normal).
    // Convert to top-left of the rendering area.
    // Use ascent (not full height) for the baseline-to-top distance of the last line.
    // `line_pitch` is the per-line advance actually drawn (cap-scaled em ×
    // factor + spacing), which differs from the raw height h for font 1.
    let lines = if let Some(ref block) = text.block {
        block.max_lines.max(1) as f64
    } else {
        1.0
    };
    let total_h = ascent + (lines - 1.0) * line_pitch;

    // ZPL spec: ^FT coordinate is "always for the left end of the baseline regardless of rotation".
    // For rotated text, the "baseline left end" rotates with the text.
    // Different rotation directions may need different offset ratios due to font metrics differences
    // between our substitute fonts and Zebra's built-in fonts.
    // Rotated90: text reads bottom-to-top, baseline on right side
    // Rotated270: text reads top-to-bottom, baseline on left side
    let rotated90_offset = h * 0.25; // Smaller offset for Rotated90 (baseline right side)

    // When text is rotated, the baseline concept rotates with it:
    // - Normal: baseline is at the bottom of text, (x,y) is left end of baseline.
    //   We need to shift y up by total_h to get the top-left corner.
    // - Rotated90 (CW 90°): text reads bottom-to-top, baseline is now on the right side.
    //   The (x,y) point is at the top of the rotated text's baseline.
    //   We need to shift x left by ascent (original y-direction becomes x-direction).
    // - Rotated180: baseline is at the top, (x,y) is right end of baseline.
    //   We need to shift x left by text width.
    //   After rotate_180() the text buffer baseline (originally at y≈actual_ascent from top)
    //   ends up at y=(buf_h - 1 - actual_ascent) from the top of the rotated buffer.
    //   buf_h = h.ceil() + 2, so correction = h.ceil() + 1 - ascent.
    //   Subtracting this from y aligns the baseline with the ^FT y coordinate.
    // - Rotated270 (CW 270°): text reads top-to-bottom, baseline is now on the left side.
    //   The (x,y) x-coordinate is the baseline of the LAST line (symmetric with Normal's y).
    //   For multi-line blocks: offset = ascent + (lines-1)*(h+spacing) = total_h.
    //   This mirrors the Normal case where y is shifted by total_h for baseline-to-top distance.
    match text.font.orientation {
        FieldOrientation::Rotated90 => (x - rotated90_offset, y),
        FieldOrientation::Rotated180 => (x - w, y - (h.ceil() + 1.0 - ascent)),
        FieldOrientation::Rotated270 => (x - total_h, y - w),
        _ => (x, y - total_h),
    }
}

fn line_color_to_rgba(color: LineColor) -> Rgba<u8> {
    match color {
        LineColor::Black => Rgba([0, 0, 0, 255]),
        LineColor::White => Rgba([255, 255, 255, 255]),
    }
}

/// Sutherland-Hodgman polygon clipping against an axis-aligned rectangle.
#[allow(dead_code)]
fn clip_polygon_to_rect(
    polygon: &[(f32, f32)],
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
) -> Vec<(f32, f32)> {
    let mut output = polygon.to_vec();
    // Each edge: (nx, ny, d) where inside = nx*x + ny*y + d >= 0
    let edges: [(f32, f32, f32); 4] = [
        (1.0, 0.0, -min_x),
        (-1.0, 0.0, max_x),
        (0.0, 1.0, -min_y),
        (0.0, -1.0, max_y),
    ];
    for &(nx, ny, d) in &edges {
        if output.is_empty() {
            break;
        }
        let input = std::mem::take(&mut output);
        let n = input.len();
        for i in 0..n {
            let cur = input[i];
            let nxt = input[(i + 1) % n];
            let cur_d = nx * cur.0 + ny * cur.1 + d;
            let nxt_d = nx * nxt.0 + ny * nxt.1 + d;
            if cur_d >= 0.0 {
                output.push(cur);
                if nxt_d < 0.0 {
                    let t = cur_d / (cur_d - nxt_d);
                    output.push((cur.0 + t * (nxt.0 - cur.0), cur.1 + t * (nxt.1 - cur.1)));
                }
            } else if nxt_d >= 0.0 {
                let t = cur_d / (cur_d - nxt_d);
                output.push((cur.0 + t * (nxt.0 - cur.0), cur.1 + t * (nxt.1 - cur.1)));
            }
        }
    }
    output
}

fn draw_filled_rect(canvas: &mut RgbaImage, x: i32, y: i32, w: i32, h: i32, color: Rgba<u8>) {
    for py in y.max(0)..(y + h).min(canvas.height() as i32) {
        for px in x.max(0)..(x + w).min(canvas.width() as i32) {
            canvas.put_pixel(px as u32, py as u32, color);
        }
    }
}

fn element_from_bottom(element: &LabelElement) -> bool {
    match element {
        LabelElement::Text(e) => e.position.from_bottom,
        LabelElement::GraphicBox(e) => e.position.from_bottom,
        LabelElement::GraphicCircle(e) => e.position.from_bottom,
        LabelElement::GraphicEllipse(e) => e.position.from_bottom,
        LabelElement::DiagonalLine(e) => e.position.from_bottom,
        LabelElement::InvertRegion(e) => e.position.from_bottom,
        LabelElement::GraphicField(e) => e.position.from_bottom,
        _ => false,
    }
}

/// Re-anchor a bottom-of-label-origin element in top-down coordinates.
/// Returns `None` when the element is not bottom-anchored (or is drawn
/// from an encoded image, whose true extents only the image typesetter
/// knows).
fn flip_from_bottom(element: &LabelElement, canvas_h: u32) -> Option<LabelElement> {
    fn shift(pos: &mut LabelPosition, height: i32, canvas_h: u32) {
        pos.y = (canvas_h as i32 - pos.y - height).max(0);
        pos.from_bottom = false;
    }
    if !element_from_bottom(element) {
        return None;
    }
    let out = element.clone();
    match out {
        LabelElement::Text(t) => {
            let mut t = t;
            shift(&mut t.position, t.font.height as i32, canvas_h);
            Some(LabelElement::Text(t))
        }
        LabelElement::GraphicBox(g) => {
            let mut g = g;
            shift(&mut g.position, g.height, canvas_h);
            Some(LabelElement::GraphicBox(g))
        }
        LabelElement::GraphicCircle(g) => {
            let mut g = g;
            shift(&mut g.position, g.circle_diameter, canvas_h);
            Some(LabelElement::GraphicCircle(g))
        }
        LabelElement::GraphicEllipse(g) => {
            let mut g = g;
            shift(&mut g.position, g.height, canvas_h);
            Some(LabelElement::GraphicEllipse(g))
        }
        LabelElement::DiagonalLine(g) => {
            let mut g = g;
            shift(&mut g.position, g.height, canvas_h);
            Some(LabelElement::DiagonalLine(g))
        }
        LabelElement::InvertRegion(g) => {
            let mut g = g;
            shift(&mut g.position, g.height, canvas_h);
            Some(LabelElement::InvertRegion(g))
        }
        LabelElement::GraphicField(g) => {
            let mut g = g;
            let rows = if g.row_bytes > 0 {
                (g.total_bytes / g.row_bytes).max(1)
            } else {
                1
            };
            shift(&mut g.position, rows * g.magnification_y.max(1), canvas_h);
            Some(LabelElement::GraphicField(g))
        }
        _ => None,
    }
}

fn adjust_image_typeset_position(
    img: &RgbaImage,
    pos: &LabelPosition,
    ori: FieldOrientation,
    canvas_h: u32,
) -> LabelPosition {
    if pos.from_bottom {
        // Bottom-of-label origin (Datamax-style home): anchor the image's
        // bottom edge at (x, canvas_h - y), then apply the rotation
        // offsets below in top-down coordinates.
        let flipped = LabelPosition {
            x: pos.x,
            y: (canvas_h as i32 - pos.y - img.height() as i32).max(0),
            calculate_from_bottom: pos.calculate_from_bottom,
            automatic_position: pos.automatic_position,
            from_bottom: false,
        };
        return adjust_image_typeset_position(img, &flipped, ori, canvas_h);
    }
    if !pos.calculate_from_bottom {
        return pos.clone();
    }

    let width = img.width() as i32;
    let height = img.height() as i32;
    let mut x = pos.x;
    let mut y = pos.y;

    match ori {
        FieldOrientation::Normal => y = (y - height).max(0),
        FieldOrientation::Rotated180 => x -= width,
        FieldOrientation::Rotated270 => {
            x = (x - height).max(0);
            y -= width;
        }
        _ => {}
    }

    LabelPosition {
        x,
        y,
        calculate_from_bottom: false,
        automatic_position: false,
        from_bottom: false,
    }
}

fn overlay_at(canvas: &mut RgbaImage, img: &RgbaImage, x: i32, y: i32) {
    for iy in 0..img.height() {
        for ix in 0..img.width() {
            let px = x + ix as i32;
            let py = y + iy as i32;
            if px >= 0 && py >= 0 && (px as u32) < canvas.width() && (py as u32) < canvas.height() {
                let pixel = *img.get_pixel(ix, iy);
                if pixel[3] > 0 {
                    let mut dst = *canvas.get_pixel(px as u32, py as u32);
                    // Source-over, not raw copy: rotated text buffers carry real
                    // anti-aliasing coverage in alpha. Stamping (0,0,0,α) here would
                    // leave a near-transparent black pixel that the 1-bit encode
                    // reads as solid black, fattening every rotated glyph stroke.
                    dst.blend(&pixel);
                    canvas.put_pixel(px as u32, py as u32, dst);
                }
            }
        }
    }
}

fn overlay_with_rotation(
    canvas: &mut RgbaImage,
    img: &RgbaImage,
    pos: &LabelPosition,
    orientation: FieldOrientation,
) {
    match orientation {
        FieldOrientation::Normal => {
            overlay_at(canvas, img, pos.x, pos.y);
        }
        FieldOrientation::Rotated90 => {
            let rotated = rotate_90(img);
            overlay_at(canvas, &rotated, pos.x, pos.y);
        }
        FieldOrientation::Rotated180 => {
            let rotated = rotate_180(img);
            overlay_at(canvas, &rotated, pos.x, pos.y);
        }
        FieldOrientation::Rotated270 => {
            let rotated = rotate_270(img);
            overlay_at(canvas, &rotated, pos.x, pos.y);
        }
    }
}

fn rotate_90(img: &RgbaImage) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut out = RgbaImage::new(h, w);
    for y in 0..h {
        for x in 0..w {
            out.put_pixel(h - 1 - y, x, *img.get_pixel(x, y));
        }
    }
    out
}

fn rotate_180(img: &RgbaImage) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut out = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            out.put_pixel(w - 1 - x, h - 1 - y, *img.get_pixel(x, y));
        }
    }
    out
}

fn rotate_270(img: &RgbaImage) -> RgbaImage {
    let (w, h) = (img.width(), img.height());
    let mut out = RgbaImage::new(h, w);
    for y in 0..h {
        for x in 0..w {
            out.put_pixel(y, w - 1 - x, *img.get_pixel(x, y));
        }
    }
    out
}

/// Draw a rounded rectangle with border. ZPL corner rounding uses radius
/// computed as (shorter_side/2) * (rounding/8).
#[allow(clippy::too_many_arguments)]
fn draw_rounded_rect(
    canvas: &mut RgbaImage,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    border: i32,
    radius: i32,
    color: Rgba<u8>,
) {
    let r = radius.min(w / 2).min(h / 2).max(0);

    if border >= w || border >= h {
        // Filled rounded rect
        draw_filled_rounded_rect_region(canvas, x, y, w, h, r, color);
    } else {
        // Draw outer rounded rect, then carve out inner
        draw_filled_rounded_rect_region(canvas, x, y, w, h, r, color);
        let inner_r = (r - border).max(0);
        let bg = Rgba([255, 255, 255, 255]);
        draw_filled_rounded_rect_region(
            canvas,
            x + border,
            y + border,
            w - 2 * border,
            h - 2 * border,
            inner_r,
            bg,
        );
    }
}

/// Fill a rounded rectangle region pixel-by-pixel.
fn draw_filled_rounded_rect_region(
    canvas: &mut RgbaImage,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    r: i32,
    color: Rgba<u8>,
) {
    let r_sq = (r as i64) * (r as i64);
    for py in y.max(0)..(y + h).min(canvas.height() as i32) {
        for px in x.max(0)..(x + w).min(canvas.width() as i32) {
            let lx = px - x;
            let ly = py - y;
            // Check if pixel is in a corner region that should be rounded
            let in_corner = if lx < r && ly < r {
                // Top-left corner
                let dx = (r - 1 - lx) as i64;
                let dy = (r - 1 - ly) as i64;
                dx * dx + dy * dy > r_sq
            } else if lx >= w - r && ly < r {
                // Top-right corner
                let dx = (lx - (w - r)) as i64;
                let dy = (r - 1 - ly) as i64;
                dx * dx + dy * dy > r_sq
            } else if lx < r && ly >= h - r {
                // Bottom-left corner
                let dx = (r - 1 - lx) as i64;
                let dy = (ly - (h - r)) as i64;
                dx * dx + dy * dy > r_sq
            } else if lx >= w - r && ly >= h - r {
                // Bottom-right corner
                let dx = (lx - (w - r)) as i64;
                let dy = (ly - (h - r)) as i64;
                dx * dx + dy * dy > r_sq
            } else {
                false
            };
            if !in_corner {
                canvas.put_pixel(px as u32, py as u32, color);
            }
        }
    }
}
