use std::io::Write;

use ab_glyph::{FontRef, PxScale};
use image::{Rgba, RgbaImage};
use imageproc::drawing;

use crate::barcodes;
use crate::elements::barcode_128::BarcodeMode;
use crate::elements::drawer_options::DrawerOptions;
use crate::elements::field_orientation::FieldOrientation;
use crate::elements::graphic_field::GraphicField;
use crate::elements::label_element::LabelElement;
use crate::elements::label_info::LabelInfo;
use crate::elements::label_position::LabelPosition;
use crate::elements::line_color::LineColor;
use crate::elements::text_field::TextField;
use crate::images;

use super::drawer_state::DrawerState;

static FONT_HELVETICA: &[u8] = crate::assets::FONT_HELVETICA_BOLD;
static FONT_DEJAVU_MONO: &[u8] = crate::assets::FONT_DEJAVU_SANS_MONO;
static FONT_DEJAVU_BOLD: &[u8] = crate::assets::FONT_DEJAVU_SANS_MONO_BOLD;
static FONT_GS: &[u8] = crate::assets::FONT_ZPL_GS;

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

        let label_width = (width_mm * dpmm as f64).ceil() as i32;
        let image_width = if label.print_width > 0 {
            label_width.min(label.print_width)
        } else {
            label_width
        };
        let image_height = (height_mm * dpmm as f64).ceil() as i32;

        let mut canvas = RgbaImage::from_pixel(
            image_width as u32,
            image_height as u32,
            Rgba([255, 255, 255, 255]),
        );

        let mut reverse_buf: Option<RgbaImage> = None;

        for element in &label.elements {
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
                // Draw inverted (rotated 180): pixel at (x,y) in rendered content
                // maps to (label_width - 1 - x - offset_x, image_height - 1 - y)
                for y in 0..canvas.height() {
                    for x in 0..canvas.width() {
                        let src_pixel = *canvas.get_pixel(x, y);
                        let dst_x = (label_width as u32 - 1 - x) as i64 - offset_x;
                        let dst_y = image_height as u32 - 1 - y;
                        if dst_x >= 0
                            && (dst_x as u32) < final_canvas.width()
                            && dst_y < final_canvas.height()
                        {
                            final_canvas.put_pixel(dst_x as u32, dst_y, src_pixel);
                        }
                    }
                }
            } else {
                image::imageops::overlay(&mut final_canvas, &canvas, offset_x, 0);
            }
            canvas = final_canvas;
        }

        let mut buf = Vec::new();
        images::monochrome::encode_png(&canvas, &mut buf)
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
            LabelElement::DiagonalLine(dl) => {
                self.draw_diagonal_line(canvas, dl);
                Ok(())
            }
            LabelElement::GraphicField(gf) => {
                self.draw_graphic_field(canvas, gf);
                Ok(())
            }
            LabelElement::Barcode128(bc) => self.draw_barcode_128(canvas, bc),
            LabelElement::BarcodeEan13(bc) => self.draw_barcode_ean13(canvas, bc),
            LabelElement::Barcode2of5(bc) => self.draw_barcode_2of5(canvas, bc),
            LabelElement::Barcode39(bc) => self.draw_barcode_39(canvas, bc),
            LabelElement::BarcodePdf417(bc) => self.draw_barcode_pdf417(canvas, bc),
            LabelElement::BarcodeAztec(bc) => self.draw_barcode_aztec(canvas, bc),
            LabelElement::BarcodeDatamatrix(bc) => self.draw_barcode_datamatrix(canvas, bc),
            LabelElement::BarcodeQr(bc) => self.draw_barcode_qr(canvas, bc, options),
            LabelElement::Maxicode(mc) => self.draw_maxicode(canvas, mc),
            _ => Ok(()), // Config/template elements are not drawn
        }
    }

    fn draw_text(
        &self,
        canvas: &mut RgbaImage,
        text: &TextField,
        state: &mut DrawerState,
    ) -> Result<(), String> {
        let font_data = get_ttf_font_data(&text.font.name);
        let font = FontRef::try_from_slice(font_data)
            .map_err(|e| format!("failed to load font: {}", e))?;

        let font_size = text.font.get_size() as f32;
        let scale_x = text.font.get_scale_x() as f32;
        let scale = PxScale {
            x: font_size * scale_x,
            y: font_size,
        };

        // Compute font ascent for ^FT baseline positioning.
        // Use a ZPL-proportional ascent (~78% of cell height) to match Zebra font metrics,
        // since our substitute TTF fonts have different ascent ratios.
        let ascent = font_size * 0.78;

        // Measure text width approximately (scale already includes scale_x)
        let text_width = measure_text_width(&text.text, &font, scale) as f64;

        // For field blocks, use block width for positioning instead of measured text width
        let pos_width = if let Some(ref block) = text.block {
            block.max_width as f64
        } else {
            text_width
        };

        let (x, y) = get_text_top_left_pos(text, pos_width, font_size as f64, ascent as f64, state);
        state.update_automatic_text_position(text, pos_width);

        let color = Rgba([0, 0, 0, 255]);

        // Render text to a buffer, then rotate if needed
        let orientation = text.font.orientation;

        if orientation == FieldOrientation::Normal {
            // Normal: draw directly onto canvas (no rotation needed)
            if let Some(ref block) = text.block {
                draw_text_block(
                    canvas, &font, scale, scale_x, color, x as f32, y as f32, block, &text.text,
                );
            } else {
                drawing::draw_text_mut(canvas, color, x as i32, y as i32, scale, &font, &text.text);
            }
        } else {
            // Non-normal: render to transparent buffer, rotate, then overlay
            let (buf_w, buf_h) = if let Some(ref block) = text.block {
                let lines = word_wrap(&text.text, &font, scale, block.max_width as f32);
                let line_height = font_size * (1.0 + block.line_spacing as f32 / font_size);
                let max_lines = block.max_lines.max(1) as usize;
                let num_lines = lines.len().min(max_lines);
                let h = (num_lines as f32 * line_height).ceil() as u32 + 2;
                (block.max_width as u32 + 2, h)
            } else {
                let w = (text_width as f32).ceil() as u32 + 2;
                let h = font_size.ceil() as u32 + 2;
                (w, h)
            };

            if buf_w == 0 || buf_h == 0 {
                return Ok(());
            }

            let mut buf = RgbaImage::from_pixel(buf_w, buf_h, Rgba([0, 0, 0, 0]));

            if let Some(ref block) = text.block {
                draw_text_block(
                    &mut buf, &font, scale, scale_x, color, 0.0, 0.0, block, &text.text,
                );
            } else {
                drawing::draw_text_mut(&mut buf, color, 0, 0, scale, &font, &text.text);
            }

            let rotated = match orientation {
                FieldOrientation::Rotated90 => rotate_90(&buf),
                FieldOrientation::Rotated180 => rotate_180(&buf),
                FieldOrientation::Rotated270 => rotate_270(&buf),
                _ => buf,
            };

            overlay_at(canvas, &rotated, x as i32, y as i32);
        }

        Ok(())
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
            // Draw thick diagonal as a one-sided band from the diagonal line,
            // extending toward the interior of the bounding box, clipped to bounds.
            let (x0, y0, x1, y1) = if dl.top_to_bottom {
                (x, y, x + w, y + h)
            } else {
                (x, y + h, x + w, y)
            };
            let dx = x1 - x0;
            let dy = y1 - y0;
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.001 {
                return;
            }
            let t = thickness as f32;
            // Normal direction pointing toward the interior of the bounding box.
            // For L (\, top_to_bottom): line goes top-left to bottom-right,
            //   interior is below-left, normal = (dy, -dx)/len = (h, -w)/len
            // For R (/, !top_to_bottom): line goes bottom-left to top-right,
            //   interior is below-right, normal = (-dy, dx)/len = (h, w)/len
            let (nx, ny) = if dl.top_to_bottom {
                // L: offset toward bottom-left → (dy/len, -dx/len)
                (dy / len * t, -dx / len * t)
            } else {
                // R: offset toward bottom-right → (-dy/len, dx/len)
                (-dy / len * t, dx / len * t)
            };

            let para = [(x0, y0), (x0 + nx, y0 + ny), (x1 + nx, y1 + ny), (x1, y1)];

            let clipped = clip_polygon_to_rect(&para, x, y, x + w, y + h);
            if clipped.len() >= 3 {
                let points: Vec<imageproc::point::Point<i32>> = clipped
                    .iter()
                    .map(|(px, py)| imageproc::point::Point::new(*px as i32, *py as i32))
                    .collect();
                drawing::draw_polygon_mut(canvas, &points, color);
            }
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

        let black = Rgba([0, 0, 0, 255]);

        for y in 0..height {
            for x in 0..width {
                let idx = (y * (width / 8) + x / 8) as usize;
                if idx >= gf.data.len() {
                    continue;
                }
                let val = (gf.data[idx] >> (7 - x % 8)) & 1;
                if val != 0 {
                    for my in 0..mag_y {
                        for mx in 0..mag_x {
                            let px = (gf.position.x + x * mag_x + mx) as u32;
                            let py = (gf.position.y + y * mag_y + my) as u32;
                            if px < canvas.width() && py < canvas.height() {
                                canvas.put_pixel(px, py, black);
                            }
                        }
                    }
                }
            }
        }
    }

    fn draw_barcode_128(
        &self,
        canvas: &mut RgbaImage,
        bc: &crate::elements::barcode_128::Barcode128WithData,
    ) -> Result<(), String> {
        let content = &bc.data;
        let (img, display_text) = match bc.barcode.mode {
            BarcodeMode::No => {
                barcodes::code128::encode_no_mode(content, bc.barcode.height, bc.width)?
            }
            _ => {
                // Modes U and D (UCC/EAN) automatically insert FNC1 at start per ZPL spec
                let content_to_encode = match bc.barcode.mode {
                    BarcodeMode::Ucc | BarcodeMode::Ean => {
                        format!("{}{}", barcodes::code128::ESCAPE_FNC_1, content)
                    }
                    _ => content.clone(),
                };
                let img = barcodes::code128::encode_auto(
                    &content_to_encode,
                    bc.barcode.height,
                    bc.width,
                )?;
                (img, content.clone())
            }
        };

        let pos = adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation);
        overlay_with_rotation(canvas, &img, &pos, bc.barcode.orientation);

        if bc.barcode.line {
            draw_barcode_interpretation_line(
                canvas,
                &display_text,
                &pos,
                &img,
                bc.barcode.orientation,
                bc.barcode.line_above,
                bc.width,
            );
        }
        Ok(())
    }

    fn draw_barcode_ean13(
        &self,
        canvas: &mut RgbaImage,
        bc: &crate::elements::barcode_ean13::BarcodeEan13WithData,
    ) -> Result<(), String> {
        let img = barcodes::ean13::encode(&bc.data, bc.barcode.height, bc.width)?;
        let pos = adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation);
        overlay_with_rotation(canvas, &img, &pos, bc.barcode.orientation);

        if bc.barcode.line {
            draw_barcode_interpretation_line(
                canvas,
                &bc.data,
                &pos,
                &img,
                bc.barcode.orientation,
                bc.barcode.line_above,
                bc.width,
            );
        }
        Ok(())
    }

    fn draw_barcode_2of5(
        &self,
        canvas: &mut RgbaImage,
        bc: &crate::elements::barcode_2of5::Barcode2of5WithData,
    ) -> Result<(), String> {
        let content: String = bc.data.chars().filter(|c| c.is_ascii_digit()).collect();
        let img = barcodes::twooffive::encode(
            &content,
            bc.barcode.height,
            bc.width_ratio as i32,
            bc.width,
            bc.barcode.check_digit,
        )?;
        let pos = adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation);
        overlay_with_rotation(canvas, &img, &pos, bc.barcode.orientation);

        if bc.barcode.line {
            draw_barcode_interpretation_line(
                canvas,
                &content,
                &pos,
                &img,
                bc.barcode.orientation,
                bc.barcode.line_above,
                bc.width,
            );
        }
        Ok(())
    }

    fn draw_barcode_39(
        &self,
        canvas: &mut RgbaImage,
        bc: &crate::elements::barcode_39::Barcode39WithData,
    ) -> Result<(), String> {
        let img =
            barcodes::code39::encode(&bc.data, bc.barcode.height, bc.width_ratio as i32, bc.width)?;
        let pos = adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation);
        overlay_with_rotation(canvas, &img, &pos, bc.barcode.orientation);

        if bc.barcode.line {
            let display_text = format!("*{}*", bc.data);
            draw_barcode_interpretation_line(
                canvas,
                &display_text,
                &pos,
                &img,
                bc.barcode.orientation,
                bc.barcode.line_above,
                bc.width,
            );
        }
        Ok(())
    }

    fn draw_barcode_pdf417(
        &self,
        canvas: &mut RgbaImage,
        bc: &crate::elements::barcode_pdf417::BarcodePdf417WithData,
    ) -> Result<(), String> {
        let img = barcodes::pdf417::encode(
            &bc.data,
            bc.barcode.row_height,
            bc.barcode.security,
            bc.barcode.columns,
            bc.barcode.rows,
            bc.barcode.truncate,
            bc.barcode.by_height,
        )?;

        // Scale horizontally by module_width (^BY w parameter)
        let mw = bc.barcode.module_width.max(1) as u32;
        let scaled = if mw > 1 {
            image::imageops::resize(
                &img,
                img.width() * mw,
                img.height(),
                image::imageops::FilterType::Nearest,
            )
        } else {
            img
        };

        let pos = adjust_image_typeset_position(&scaled, &bc.position, bc.barcode.orientation);
        overlay_with_rotation(canvas, &scaled, &pos, bc.barcode.orientation);
        Ok(())
    }

    fn draw_barcode_aztec(
        &self,
        canvas: &mut RgbaImage,
        bc: &crate::elements::barcode_aztec::BarcodeAztecWithData,
    ) -> Result<(), String> {
        let mag = bc.barcode.magnification.max(1);
        let img = barcodes::aztec::encode(&bc.data, mag, bc.barcode.size)?;
        let pos = adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation);
        overlay_with_rotation(canvas, &img, &pos, bc.barcode.orientation);
        Ok(())
    }

    fn draw_barcode_datamatrix(
        &self,
        canvas: &mut RgbaImage,
        bc: &crate::elements::barcode_datamatrix::BarcodeDatamatrixWithData,
    ) -> Result<(), String> {
        let scale = bc.barcode.height.max(1);
        let img_raw =
            barcodes::datamatrix::encode(&bc.data, scale, bc.barcode.rows, bc.barcode.columns)?;
        let pos = adjust_image_typeset_position(&img_raw, &bc.position, bc.barcode.orientation);
        overlay_with_rotation(canvas, &img_raw, &pos, bc.barcode.orientation);
        Ok(())
    }

    fn draw_barcode_qr(
        &self,
        canvas: &mut RgbaImage,
        bc: &crate::elements::barcode_qr::BarcodeQrWithData,
        _options: &DrawerOptions,
    ) -> Result<(), String> {
        let (input_data, ec, _) = bc.get_input_data()?;
        let img = barcodes::qrcode::encode(&input_data, bc.barcode.magnification, ec)?;

        let pos = adjust_image_typeset_position(&img, &bc.position, FieldOrientation::Normal);

        overlay_at(canvas, &img, pos.x, pos.y);
        Ok(())
    }

    fn draw_maxicode(
        &self,
        canvas: &mut RgbaImage,
        mc: &crate::elements::maxicode::MaxicodeWithData,
    ) -> Result<(), String> {
        let _input_data = mc.get_input_data()?;
        let img = barcodes::maxicode::encode(&mc.data)?;
        let pos = adjust_image_typeset_position(&img, &mc.position, FieldOrientation::Normal);
        overlay_at(canvas, &img, pos.x, pos.y);
        Ok(())
    }
}

fn get_ttf_font_data(name: &str) -> &'static [u8] {
    match name {
        "0" => FONT_HELVETICA,
        "B" | "D" | "P" | "Q" | "S" => FONT_DEJAVU_BOLD,
        "GS" => FONT_GS,
        _ => FONT_DEJAVU_MONO,
    }
}

fn measure_text_width(text: &str, font: &FontRef, scale: PxScale) -> f32 {
    use ab_glyph::{Font, ScaleFont};
    let scaled = font.as_scaled(scale);
    let mut width = 0.0f32;
    let mut prev = None;
    for ch in text.chars() {
        let glyph_id = font.glyph_id(ch);
        if let Some(prev_id) = prev {
            width += scaled.kern(prev_id, glyph_id);
        }
        width += scaled.h_advance(glyph_id);
        prev = Some(glyph_id);
    }
    width
}

fn word_wrap(text: &str, font: &FontRef, scale: PxScale, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for line in text.split('\n') {
        let words: Vec<&str> = line.split_whitespace().collect();
        if words.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current_line = words[0].to_string();
        for word in &words[1..] {
            let test = format!("{} {}", current_line, word);
            let w = measure_text_width(&test, font, scale);
            if w > max_width {
                lines.push(current_line);
                current_line = word.to_string();
            } else {
                current_line = test;
            }
        }
        lines.push(current_line);
    }
    lines
}

#[allow(clippy::too_many_arguments)]
fn draw_text_block(
    canvas: &mut RgbaImage,
    font: &FontRef,
    scale: PxScale,
    _scale_x: f32,
    color: Rgba<u8>,
    x: f32,
    y: f32,
    block: &crate::elements::field_block::FieldBlock,
    text: &str,
) {
    let max_width = block.max_width as f32;
    let lines = word_wrap(text, font, scale, max_width);
    let font_size = scale.y;
    let line_height = font_size * (1.0 + block.line_spacing as f32 / font_size);

    let mut cy = y;
    let max_lines = block.max_lines.max(1) as usize;
    for (i, line) in lines.iter().enumerate() {
        if i >= max_lines {
            break;
        }
        let lx = match block.alignment {
            crate::elements::text_alignment::TextAlignment::Center => {
                let lw = measure_text_width(line, font, scale);
                x + (block.max_width as f32 - lw) / 2.0
            }
            crate::elements::text_alignment::TextAlignment::Right => {
                let lw = measure_text_width(line, font, scale);
                x + block.max_width as f32 - lw
            }
            _ => x,
        };
        drawing::draw_text_mut(canvas, color, lx as i32, cy as i32, scale, font, line);
        cy += line_height;
    }
}

fn get_text_top_left_pos(
    text: &TextField,
    w: f64,
    h: f64,
    ascent: f64,
    state: &DrawerState,
) -> (f64, f64) {
    let (x, y) = state.get_text_position(text);

    if !text.position.calculate_from_bottom {
        // ^FO: position is top-left of the field area. Handle justification parameter.
        let x = match text.alignment {
            crate::elements::field_alignment::FieldAlignment::Right => x - w,
            _ => x,
        };
        return (x, y);
    }

    // ^FT: position is baseline (bottom-left for Normal).
    // Convert to top-left of the rendering area.
    // Use ascent (not full height) for the baseline-to-top distance of the last line.
    // Use full font height h for line spacing between lines.
    let lines = if let Some(ref block) = text.block {
        block.max_lines.max(1) as f64
    } else {
        1.0
    };
    let spacing = if let Some(ref block) = text.block {
        block.line_spacing as f64
    } else {
        0.0
    };
    let total_h = ascent + (lines - 1.0) * (h + spacing);

    // ZPL spec: ^FT coordinate is "always for the left end of the baseline regardless of rotation".
    // For rotated text, the "baseline left end" rotates with the text.
    // Different rotation directions may need different offset ratios due to font metrics differences
    // between our substitute fonts and Zebra's built-in fonts.
    // Rotated90: text reads bottom-to-top, baseline on right side
    // Rotated270: text reads top-to-bottom, baseline on left side
    let rotated90_offset = h * 0.25; // Smaller offset for Rotated90 (baseline right side)
    let rotated270_offset = ascent; // Standard ascent for Rotated270 (baseline left side)

    // When text is rotated, the baseline concept rotates with it:
    // - Normal: baseline is at the bottom of text, (x,y) is left end of baseline.
    //   We need to shift y up by ascent to get the top-left corner.
    // - Rotated90 (CW 90°): text reads bottom-to-top, baseline is now on the right side.
    //   The (x,y) point is at the top of the rotated text's baseline.
    //   We need to shift x left by ascent (original y-direction becomes x-direction).
    // - Rotated180: baseline is at the top, (x,y) is right end of baseline.
    //   We need to shift x left by text width.
    // - Rotated270 (CW 270°): text reads top-to-bottom, baseline is now on the left side.
    //   We need to shift both x left by ascent and y up by text width.
    match text.font.orientation {
        FieldOrientation::Rotated90 => (x - rotated90_offset, y),
        FieldOrientation::Rotated180 => (x - w, y),
        FieldOrientation::Rotated270 => (x - rotated270_offset, y - w),
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

fn adjust_image_typeset_position(
    img: &RgbaImage,
    pos: &LabelPosition,
    ori: FieldOrientation,
) -> LabelPosition {
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
                    canvas.put_pixel(px as u32, py as u32, pixel);
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

/// Draw the human-readable interpretation line below (or above) a barcode.
fn draw_barcode_interpretation_line(
    canvas: &mut RgbaImage,
    text: &str,
    pos: &LabelPosition,
    barcode_img: &RgbaImage,
    orientation: FieldOrientation,
    line_above: bool,
    module_width: i32,
) {
    let font_data = FONT_DEJAVU_MONO;
    let font = match ab_glyph::FontRef::try_from_slice(font_data) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Zebra's interpretation line font scales with the barcode module width.
    // At module_width=2 (default), the standard font is ~18px.
    let font_size = (module_width.max(1) as f32 * 9.0).clamp(12.0, 72.0);
    let scale = PxScale {
        x: font_size,
        y: font_size,
    };

    // Strip control characters (like FNC1 escape) from display text
    let display: String = text
        .chars()
        .filter(|c| !c.is_control() && *c != '\u{00F1}')
        .collect();

    let text_width = measure_text_width(&display, &font, scale);
    let bw = barcode_img.width() as i32;
    let bh = barcode_img.height() as i32;

    match orientation {
        FieldOrientation::Normal => {
            let cx = pos.x + (bw - text_width as i32) / 2;
            let ty = if line_above {
                pos.y - font_size as i32 - 2
            } else {
                pos.y + bh + 2
            };
            let color = Rgba([0, 0, 0, 255]);
            drawing::draw_text_mut(canvas, color, cx, ty, scale, &font, &display);
        }
        _ => {
            // Render text to buffer, rotate to match barcode orientation, then overlay
            let buf_w = (text_width.ceil() as u32).max(1) + 2;
            let buf_h = font_size.ceil() as u32 + 2;
            let mut buf = RgbaImage::from_pixel(buf_w, buf_h, Rgba([0, 0, 0, 0]));
            let color = Rgba([0, 0, 0, 255]);
            drawing::draw_text_mut(&mut buf, color, 0, 0, scale, &font, &display);

            let rotated = match orientation {
                FieldOrientation::Rotated90 => rotate_90(&buf),
                FieldOrientation::Rotated180 => rotate_180(&buf),
                FieldOrientation::Rotated270 => rotate_270(&buf),
                _ => buf,
            };

            // Position: center text along the barcode edge
            let (tx, ty) = match orientation {
                FieldOrientation::Rotated90 => {
                    let cy = pos.y + (bw - text_width as i32) / 2;
                    if line_above {
                        (pos.x + bh + 2, cy)
                    } else {
                        (pos.x - rotated.width() as i32 - 2, cy)
                    }
                }
                FieldOrientation::Rotated180 => {
                    let cx = pos.x + (bw - text_width as i32) / 2;
                    if line_above {
                        (cx, pos.y + bh + 2)
                    } else {
                        (cx, pos.y - rotated.height() as i32 - 2)
                    }
                }
                FieldOrientation::Rotated270 => {
                    let cy = pos.y + (bw - text_width as i32) / 2;
                    if line_above {
                        (pos.x - rotated.width() as i32 - 2, cy)
                    } else {
                        (pos.x + bh + 2, cy)
                    }
                }
                _ => (0, 0),
            };
            overlay_at(canvas, &rotated, tx, ty);
        }
    }
}
