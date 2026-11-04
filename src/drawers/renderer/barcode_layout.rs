//! Shared symbol placement and human-readable barcode text layout.
use super::text_layout::{PlacedGlyph, TextBuffer, TextLayout};
use super::*;

#[derive(Clone, Debug)]
pub(crate) enum BarcodePart {
    Symbol {
        image: RgbaImage,
        position: LabelPosition,
        orientation: FieldOrientation,
    },
    Text {
        raster: RgbaImage,
        x: i32,
        y: i32,
        #[cfg_attr(not(feature = "vector"), allow(dead_code))]
        layout: TextLayout,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct BarcodeLayout {
    height: u32,
    pub parts: Vec<BarcodePart>,
}

impl BarcodeLayout {
    fn height(&self) -> u32 {
        self.height
    }
    pub(crate) fn draw(&self, canvas: &mut RgbaImage) {
        for part in &self.parts {
            match part {
                BarcodePart::Symbol {
                    image,
                    position,
                    orientation,
                } => super::overlay_with_rotation(canvas, image, position, *orientation),
                BarcodePart::Text { raster, x, y, .. } => super::overlay_at(canvas, raster, *x, *y),
            }
        }
    }
}

pub(crate) fn layout(
    element: &LabelElement,
    height: u32,
    options: &DrawerOptions,
) -> Result<BarcodeLayout, String> {
    let mut canvas = BarcodeLayout {
        height,
        parts: Vec::new(),
    };
    match element {
        LabelElement::Barcode128(b) => draw_barcode_128(&mut canvas, b)?,
        LabelElement::BarcodeEan13(b) => draw_barcode_ean13(&mut canvas, b)?,
        LabelElement::BarcodeEan8(b) => draw_barcode_ean8(&mut canvas, b)?,
        LabelElement::BarcodeUca(b) => draw_barcode_upca(&mut canvas, b)?,
        LabelElement::BarcodeUcpe(b) => draw_barcode_upce(&mut canvas, b)?,
        LabelElement::Barcode2of5(b) => draw_barcode_2of5(&mut canvas, b)?,
        LabelElement::Barcode39(b) => draw_barcode_39(&mut canvas, b)?,
        LabelElement::BarcodePdf417(b) => draw_barcode_pdf417(&mut canvas, b)?,
        LabelElement::BarcodeAztec(b) => draw_barcode_aztec(&mut canvas, b)?,
        LabelElement::BarcodeDatamatrix(b) => draw_barcode_datamatrix(&mut canvas, b)?,
        LabelElement::BarcodeQr(b) => draw_barcode_qr(&mut canvas, b, options)?,
        LabelElement::Maxicode(b) => draw_maxicode(&mut canvas, b)?,
        _ => return Err("expected a barcode element".into()),
    }
    Ok(canvas)
}

trait Overlay {
    fn place(&self, target: &mut BarcodeLayout, x: i32, y: i32);
}
impl Overlay for RgbaImage {
    fn place(&self, target: &mut BarcodeLayout, x: i32, y: i32) {
        target.parts.push(BarcodePart::Symbol {
            image: self.clone(),
            position: LabelPosition {
                x,
                y,
                ..Default::default()
            },
            orientation: FieldOrientation::Normal,
        });
    }
}
fn overlay_at<T: Overlay>(target: &mut BarcodeLayout, image: &T, x: i32, y: i32) {
    image.place(target, x, y);
}
fn overlay_with_rotation(
    target: &mut BarcodeLayout,
    image: &RgbaImage,
    position: &LabelPosition,
    orientation: FieldOrientation,
) {
    target.parts.push(BarcodePart::Symbol {
        image: image.clone(),
        position: position.clone(),
        orientation,
    });
}

#[derive(Clone)]
struct TextImage {
    raster: RgbaImage,
    layout: TextLayout,
}
impl TextImage {
    fn new(raster: RgbaImage, face: FontFace, glyphs: Vec<PlacedGlyph>) -> Self {
        let buffer = TextBuffer {
            width: raster.width(),
            height: raster.height(),
            orientation: FieldOrientation::Normal,
            x: 0,
            y: 0,
        };
        Self {
            raster,
            layout: TextLayout {
                face,
                glyphs,
                buffer: Some(buffer),
            },
        }
    }
    fn width(&self) -> u32 {
        self.raster.width()
    }
    fn height(&self) -> u32 {
        self.raster.height()
    }
}
impl Overlay for TextImage {
    fn place(&self, target: &mut BarcodeLayout, x: i32, y: i32) {
        let mut layout = self.layout.clone();
        if let Some(buffer) = &mut layout.buffer {
            buffer.x = x;
            buffer.y = y;
        }
        target.parts.push(BarcodePart::Text {
            raster: self.raster.clone(),
            x,
            y,
            layout,
        });
    }
}
fn rotated(image: &TextImage, orientation: FieldOrientation) -> TextImage {
    let raster = match orientation {
        FieldOrientation::Rotated90 => super::rotate_90(&image.raster),
        FieldOrientation::Rotated180 => super::rotate_180(&image.raster),
        FieldOrientation::Rotated270 => super::rotate_270(&image.raster),
        _ => image.raster.clone(),
    };
    let mut layout = image.layout.clone();
    if let Some(buffer) = &mut layout.buffer {
        buffer.orientation = orientation;
    }
    TextImage { raster, layout }
}
fn rotate_90(image: &TextImage) -> TextImage {
    rotated(image, FieldOrientation::Rotated90)
}
fn rotate_180(image: &TextImage) -> TextImage {
    rotated(image, FieldOrientation::Rotated180)
}
fn rotate_270(image: &TextImage) -> TextImage {
    rotated(image, FieldOrientation::Rotated270)
}

// The legacy HRI uses imageproc's uncalibrated pen walk at 3x resolution.
// Preserve those glyph positions, expressed back in source dots.
fn plain_glyphs(font: &FontRef, scale: PxScale, text: &str, x: i32, y: i32) -> Vec<PlacedGlyph> {
    let ss = PxScale {
        x: scale.x * 3.0,
        y: scale.y * 3.0,
    };
    let scaled = font.as_scaled(ss);
    let mut pen = 0.0;
    let mut previous = None;
    let mut glyphs = Vec::new();
    for character in text.chars() {
        let id = font.glyph_id(character);
        let large = id.with_scale_and_position(ss, ab_glyph::point(pen, scaled.ascent()));
        glyphs.push(PlacedGlyph {
            character,
            glyph: id
                .with_scale_and_position(scale, ab_glyph::point(pen / 3.0, scaled.ascent() / 3.0)),
            x,
            y,
            color: Rgba([0, 0, 0, 255]),
            dedot: false,
            line_start: false,
        });
        pen += scaled.h_advance(id);
        if font.outline_glyph(large).is_some() {
            if let Some(prev) = previous {
                pen += scaled.kern(id, prev);
            }
            previous = Some(id);
        }
    }
    glyphs
}

fn draw_barcode_128(
    canvas: &mut BarcodeLayout,
    bc: &crate::elements::barcode_128::Barcode128WithData,
) -> Result<(), String> {
    let content = &bc.data;
    let (img, display_text) = match bc.barcode.mode {
        BarcodeMode::No => barcodes::code128::encode_no_mode(content, bc.barcode.height, bc.width)?,
        _ => {
            // Modes U and D (UCC/EAN) automatically insert FNC1 at start per ZPL spec
            let (content_to_encode, display_text) = match bc.barcode.mode {
                BarcodeMode::Ucc => {
                    // Mode U: truncate to 19 digits, append GS1 Mod-10 check digit, prepend FNC1
                    (
                        barcodes::code128::prepare_ucc_mode_data(content),
                        content.clone(),
                    )
                }
                BarcodeMode::Ean => {
                    // Mode D: FNC1 prepended automatically; >8 in data = embedded FNC1 separator
                    // for chaining GS1 application identifiers; parentheses and spaces stripped
                    // from encoding but preserved in display text per ZPL spec. Invocation
                    // codes affect the symbol only and must not be printed as glyphs.
                    barcodes::code128::prepare_ean_mode_data(content)
                }
                _ => (content.clone(), content.clone()),
            };
            let img =
                barcodes::code128::encode_auto(&content_to_encode, bc.barcode.height, bc.width)?;
            (img, display_text)
        }
    };

    let pos =
        adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation, canvas.height());
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
            bc.barcode.mode == BarcodeMode::Ean,
            bc.line_alignment,
        );
    }
    Ok(())
}

fn draw_barcode_ean13(
    canvas: &mut BarcodeLayout,
    bc: &crate::elements::barcode_ean13::BarcodeEan13WithData,
) -> Result<(), String> {
    let sym = barcodes::ean13::encode(&bc.data, bc.barcode.height, bc.width)?;
    let img = sym.image.clone();
    let pos =
        adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation, canvas.height());
    // For 90°/180° rotations the guard extension hangs on the far side of the
    // data block, keeping the data-block origin at the field origin (Labelary).
    let mut bar_pos = pos.clone();
    match bc.barcode.orientation {
        FieldOrientation::Rotated90 => bar_pos.x -= sym.guard_height as i32,
        FieldOrientation::Rotated180 => bar_pos.y -= sym.guard_height as i32,
        _ => {}
    }
    overlay_with_rotation(canvas, &img, &bar_pos, bc.barcode.orientation);

    if bc.barcode.line {
        let mw = bc.width.max(1) as f32;
        let chars: Vec<char> = sym.digits.iter().map(|d| char::from(b'0' + d)).collect();
        // Number-system digit left of the bars; digits 2..7 and 8..13 (incl.
        // check) at a uniform ~6.6-module cadence calibrated off Labelary.
        let mut centers = vec![pos.x as f32 - 6.5 * mw];
        centers.extend((0..6).map(|i| pos.x as f32 + mw * (7.0 + 6.6 * i as f32)));
        centers.extend((0..6).map(|j| pos.x as f32 + mw * (53.2 + 6.6 * j as f32)));
        draw_module_centered_interpretation_line(
            canvas,
            &chars,
            &centers,
            &pos,
            &img,
            bc.barcode.orientation,
            bc.barcode.line_above,
            bc.width,
            sym.image.height() as i32 - sym.guard_height as i32,
        );
    }
    Ok(())
}

fn draw_barcode_ean8(
    canvas: &mut BarcodeLayout,
    bc: &crate::elements::barcode_ean8::BarcodeEan8WithData,
) -> Result<(), String> {
    let sym = barcodes::ean8::encode(&bc.data, bc.barcode.height, bc.width)?;
    let img = sym.image.clone();
    let pos =
        adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation, canvas.height());
    // 90°/180°: guard hangs on the far side; data origin stays at the field
    // origin (Labelary behavior).
    let mut bar_pos = pos.clone();
    match bc.barcode.orientation {
        FieldOrientation::Rotated90 => bar_pos.x -= sym.guard_height as i32,
        FieldOrientation::Rotated180 => bar_pos.y -= sym.guard_height as i32,
        _ => {}
    }
    overlay_with_rotation(canvas, &img, &bar_pos, bc.barcode.orientation);

    if bc.barcode.line {
        let mw = bc.width.max(1) as f32;
        let chars: Vec<char> = sym.digits.iter().map(|d| char::from(b'0' + d)).collect();
        // Left digits under modules 3..30, right digits under 36..63.
        let centers: Vec<f32> = (0..4)
            .map(|i| pos.x as f32 + mw * (6.5 + 7.0 * i as f32))
            .chain((0..4).map(|j| pos.x as f32 + mw * (39.5 + 7.0 * j as f32)))
            .collect();
        draw_module_centered_interpretation_line(
            canvas,
            &chars,
            &centers,
            &pos,
            &img,
            bc.barcode.orientation,
            bc.barcode.line_above,
            bc.width,
            sym.image.height() as i32 - sym.guard_height as i32,
        );
    }
    Ok(())
}

fn draw_barcode_upca(
    canvas: &mut BarcodeLayout,
    bc: &crate::elements::barcode_upca::BarcodeUcaWithData,
) -> Result<(), String> {
    let sym = barcodes::upca::encode(&bc.data, bc.barcode.height, bc.width)?;
    let img = sym.image.clone();
    let pos =
        adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation, canvas.height());
    // 90°/180°: guard hangs on the far side; data origin stays at the field
    // origin (Labelary behavior).
    let mut bar_pos = pos.clone();
    match bc.barcode.orientation {
        FieldOrientation::Rotated90 => bar_pos.x -= sym.guard_height as i32,
        FieldOrientation::Rotated180 => bar_pos.y -= sym.guard_height as i32,
        _ => {}
    }
    overlay_with_rotation(canvas, &img, &bar_pos, bc.barcode.orientation);

    if bc.barcode.line {
        let mw = bc.width.max(1) as f32;
        let chars: Vec<char> = sym
            .digits
            .iter()
            .map(|d| char::from(b'0' + d))
            .chain(std::iter::once(char::from(b'0' + sym.check_digit)))
            .collect();
        // Number-system digit left of the bars; M1..M5 under left groups
        // 2..6, P1..P5 under right groups 1..5, check digit right of the
        // end guard (~module 99) — all calibrated against Labelary.
        let mut centers = vec![pos.x as f32 - 9.0 * mw];
        centers.extend((0..5).map(|i| pos.x as f32 + mw * (13.5 + 7.0 * i as f32)));
        centers.extend((0..5).map(|j| pos.x as f32 + mw * (53.5 + 7.0 * j as f32)));
        centers.push(pos.x as f32 + 99.0 * mw);
        draw_module_centered_interpretation_line(
            canvas,
            &chars,
            &centers,
            &pos,
            &img,
            bc.barcode.orientation,
            bc.barcode.line_above,
            bc.width,
            sym.image.height() as i32 - sym.guard_height as i32,
        );
    }
    Ok(())
}

fn draw_barcode_upce(
    canvas: &mut BarcodeLayout,
    bc: &crate::elements::barcode_upce::BarcodeUcpeWithData,
) -> Result<(), String> {
    let sym = barcodes::upce::encode(&bc.data, bc.barcode.height, bc.width)?;
    let img = sym.image.clone();
    let pos =
        adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation, canvas.height());
    // 90°/180°: guard hangs on the far side; data origin stays at the field
    // origin (Labelary behavior).
    let guard = (img.height() - sym.data_height) as i32;
    let mut bar_pos = pos.clone();
    match bc.barcode.orientation {
        FieldOrientation::Rotated90 => bar_pos.x -= guard,
        FieldOrientation::Rotated180 => bar_pos.y -= guard,
        _ => {}
    }
    overlay_with_rotation(canvas, &img, &bar_pos, bc.barcode.orientation);

    if bc.barcode.line {
        let mw = bc.width.max(1) as f32;
        let mut chars = vec![char::from(b'0' + sym.number_system)];
        chars.extend(sym.digits.iter().map(|d| char::from(b'0' + d)));
        let mut centers = vec![pos.x as f32 - 9.0 * mw];
        centers.extend((0..6).map(|i| pos.x as f32 + mw * (6.5 + 7.0 * i as f32)));
        if bc.barcode.check_digit {
            chars.push(char::from(b'0' + sym.check_digit));
            centers.push(pos.x as f32 + 55.0 * mw);
        }
        draw_module_centered_interpretation_line(
            canvas,
            &chars,
            &centers,
            &pos,
            &img,
            bc.barcode.orientation,
            bc.barcode.line_above,
            bc.width,
            sym.data_height as i32,
        );
    }
    Ok(())
}

fn draw_barcode_2of5(
    canvas: &mut BarcodeLayout,
    bc: &crate::elements::barcode_2of5::Barcode2of5WithData,
) -> Result<(), String> {
    let content: String = bc.data.chars().filter(|c| c.is_ascii_digit()).collect();
    let img = barcodes::twooffive::encode(
        &content,
        bc.barcode.height,
        bc.width_ratio,
        bc.width,
        bc.barcode.check_digit,
    )?;
    let pos =
        adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation, canvas.height());
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
            false,
            bc.line_alignment,
        );
    }
    Ok(())
}

fn draw_barcode_39(
    canvas: &mut BarcodeLayout,
    bc: &crate::elements::barcode_39::Barcode39WithData,
) -> Result<(), String> {
    let img = barcodes::code39::encode(&bc.data, bc.barcode.height, bc.width_ratio, bc.width)?;
    let pos =
        adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation, canvas.height());
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
            false,
            bc.line_alignment,
        );
    }
    Ok(())
}

fn draw_barcode_pdf417(
    canvas: &mut BarcodeLayout,
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

    let pos = adjust_image_typeset_position(
        &scaled,
        &bc.position,
        bc.barcode.orientation,
        canvas.height(),
    );
    overlay_with_rotation(canvas, &scaled, &pos, bc.barcode.orientation);
    Ok(())
}

fn draw_barcode_aztec(
    canvas: &mut BarcodeLayout,
    bc: &crate::elements::barcode_aztec::BarcodeAztecWithData,
) -> Result<(), String> {
    let mag = bc.barcode.magnification.max(1);
    let img = barcodes::aztec::encode(&bc.data, mag, bc.barcode.size)?;
    let pos =
        adjust_image_typeset_position(&img, &bc.position, bc.barcode.orientation, canvas.height());
    overlay_with_rotation(canvas, &img, &pos, bc.barcode.orientation);
    Ok(())
}

fn draw_barcode_datamatrix(
    canvas: &mut BarcodeLayout,
    bc: &crate::elements::barcode_datamatrix::BarcodeDatamatrixWithData,
) -> Result<(), String> {
    // In ZPL, omitted quality is
    // ECC 000, not permission to substitute a different symbology.
    match bc.barcode.quality {
        0 | 50 | 80 | 100 | 140 | 200 => {}
        quality => {
            return Err(crate::error::LabelizeError::Render(format!(
                "Invalid DataMatrix quality {quality}: expected 0, 50, 80, 100, 140, or 200."
            ))
            .to_string());
        }
    }
    // Skip empty fields, preserving upstream's label-level behavior.
    // Legacy bytes are authoritative even when the display text differs.
    let is_empty = if bc.barcode.quality == 200 && bc.barcode.escape != 0 {
        bc.data.is_empty()
    } else {
        bc.data_bytes
            .as_deref()
            .unwrap_or(bc.data.as_bytes())
            .is_empty()
    };
    if is_empty {
        return Ok(());
    }
    let scale = bc.barcode.height.max(1);
    let img_raw = if bc.barcode.quality != 200 {
        // Legacy g (ECC 200 escapes) has no effect. Consume preserved field
        // bytes instead of attempting to reverse the display text decoding.
        let input = match bc.data_bytes.as_deref() {
            Some(bytes) => bytes,
            None if bc.data.is_ascii() => bc.data.as_bytes(),
            None => {
                return Err("Legacy DataMatrix: non-ASCII data requires explicit data_bytes".into())
            }
        };
        // ZD421 CI13/CI27 probes preserve backslashes and pipes after FH.
        // Do not apply the PDF417 substitutions suggested by the BX prose.
        if bc.barcode.ratio
            == Some(crate::elements::barcode_datamatrix::DatamatrixRatio::Rectangular)
        {
            return Err("Legacy DataMatrix: rectangular symbols are not supported".into());
        }
        let format = u8::try_from(bc.barcode.format)
            .map_err(|_| "Legacy DataMatrix: format must be 1 through 6")?;
        let size =
            barcodes::datamatrix_legacy::legacy_symbol_size(bc.barcode.rows, bc.barcode.columns)?;
        barcodes::datamatrix_legacy::encode_with_ecc(
            input,
            format,
            bc.barcode.quality as u16,
            size,
        )?
        .to_image(scale as usize, scale as usize)
    } else {
        let input = if bc.barcode.escape == 0 {
            bc.data_bytes.as_deref().unwrap_or(bc.data.as_bytes())
        } else {
            bc.data.as_bytes()
        };
        let image = barcodes::datamatrix::render_field(
            input,
            scale,
            bc.barcode.rows,
            bc.barcode.columns,
            bc.barcode.escape,
            bc.barcode.ratio,
        )?;
        let Some(image) = image else {
            return Ok(());
        };
        image
    };

    let pos = adjust_image_typeset_position(
        &img_raw,
        &bc.position,
        bc.barcode.orientation,
        canvas.height(),
    );
    overlay_with_rotation(canvas, &img_raw, &pos, bc.barcode.orientation);
    Ok(())
}

fn draw_barcode_qr(
    canvas: &mut BarcodeLayout,
    bc: &crate::elements::barcode_qr::BarcodeQrWithData,
    _options: &DrawerOptions,
) -> Result<(), String> {
    // Labelary draws nothing for a QR field whose payload is empty
    // (`^FD^FS`, or a prefix-only ^FD like `^FDQA,` that parses to no
    // data) — skip the field instead of failing the whole label.
    if bc.data.is_empty() {
        return Ok(());
    }
    let img = barcodes::qrcode::encode(&bc.data, bc.barcode.magnification, bc.error_correction)?;

    let quiet_zone_px = 4 * bc.barcode.magnification;

    let (draw_x, draw_y) = if bc.position.calculate_from_bottom {
        // ^FT baseline positioning: the QR image (including quiet zones) is positioned
        // so that the modules start at y = ^FT_y - img_height + magnification.
        // We shift y by +1 module then subtract quiet_zone_px so modules align correctly.
        let adjusted = LabelPosition {
            y: bc.position.y + bc.barcode.magnification,
            ..bc.position.clone()
        };
        let pos = adjust_image_typeset_position(
            &img,
            &adjusted,
            FieldOrientation::Normal,
            canvas.height(),
        );
        (pos.x - quiet_zone_px, pos.y - quiet_zone_px)
    } else {
        // ^FO origin positioning: modules start at (FO_x, FO_y + by_height).
        // The image (with quiet zone on all sides) is drawn offset so that the
        // quiet zone falls outside the field origin.
        let pos = adjust_image_typeset_position(
            &img,
            &bc.position,
            FieldOrientation::Normal,
            canvas.height(),
        );
        (pos.x - quiet_zone_px, pos.y + bc.height - quiet_zone_px)
    };

    overlay_at(canvas, &img, draw_x, draw_y);
    Ok(())
}

fn draw_maxicode(
    canvas: &mut BarcodeLayout,
    mc: &crate::elements::maxicode::MaxicodeWithData,
) -> Result<(), String> {
    // Labelary draws nothing for an empty MaxiCode field (^FD^FS) — skip
    // instead of failing the whole label. The encoder keeps rejecting
    // empty input.
    if mc.data.is_empty() {
        return Ok(());
    }
    let img = barcodes::maxicode::encode(&mc.data, mc.code.mode)?;
    let pos = adjust_image_typeset_position(
        &img,
        &mc.position,
        FieldOrientation::Normal,
        canvas.height(),
    );
    overlay_at(canvas, &img, pos.x, pos.y);
    Ok(())
}
/// Draw the human-readable interpretation line below (or above) a barcode.
#[allow(clippy::too_many_arguments)]
fn draw_barcode_interpretation_line(
    canvas: &mut BarcodeLayout,
    text: &str,
    pos: &LabelPosition,
    barcode_img: &RgbaImage,
    orientation: FieldOrientation,
    line_above: bool,
    module_width: i32,
    ucc_ean_font: bool,
    line_alignment: crate::elements::field_alignment::FieldAlignment,
) {
    // Code 128 mode D (UCC/EAN) uses a larger condensed bold interpretation font,
    // matching Labelary. Reference probe (code128_mode_d_fnc1 at module 2):
    // paren-inclusive ink band ~29px tall, digit pitch ~15.7px. The Roboto
    // substitute renders the paren band at 0.7055 em (hhea-normalized), giving
    // the 20.55×module size; its digit pitch lands within ~13% of Labelary's,
    // absorbed by the centered layout. Other modes use the standard monospace
    // font that scales with module width.
    let (font_data, font_size) = if ucc_ean_font {
        (
            FONT_ZERO,
            (module_width.max(1) as f32 * 20.55).clamp(21.0, 140.0),
        )
    } else {
        (
            FONT_DEJAVU_MONO,
            (module_width.max(1) as f32 * 11.0).clamp(12.0, 72.0),
        )
    };
    let font = match ab_glyph::FontRef::try_from_slice(font_data) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Zebra's interpretation line font scales with the barcode module width.
    // At module_width=2 (default), the standard font is ~23px to match reference width.
    let scale = PxScale {
        x: font_size,
        y: font_size,
    };
    // Where the ink top sits inside the draw buffer, relative to the pen origin
    // ab_glyph uses: the old Helvetica substitute's cap equaled its ascent, so ink
    // started at the
    // buffer top (plus the 4px Labelary line gap); Roboto's cap sits 0.243 em
    // below its ascent, so the offset compensates to keep the same 4px gap.
    let ink_top_off: f32 = if ucc_ean_font {
        4.0 - 0.2429 * font_size
    } else {
        0.0
    };
    let text_y_off: i32 = ink_top_off.round() as i32;

    // Strip control characters (like FNC1 escape) from display text
    let display: String = text
        .chars()
        .filter(|c| !c.is_control() && *c != '\u{00F1}')
        .collect();

    let text_width = measure_text_width(&display, &font, scale, SubstMetrics::PLAIN);
    let bw = barcode_img.width() as i32;
    let bh = barcode_img.height() as i32;

    // Supersampled crisp text: render at 3× then box-filter downsample and threshold.
    // This gives sub-pixel shape accuracy before the binary threshold, producing
    // crisper strokes than direct thresholding of a single-resolution render.
    let render_text_crisp = |w: u32, h: u32| -> TextImage {
        const SS: u32 = 3;
        let ss_scale = PxScale {
            x: scale.x * SS as f32,
            y: scale.y * SS as f32,
        };
        let ss_w = (w * SS).max(1);
        let ss_h = (h * SS).max(1);
        let mut big = RgbaImage::from_pixel(ss_w, ss_h, Rgba([0, 0, 0, 0]));
        drawing::draw_text_mut(
            &mut big,
            Rgba([0, 0, 0, 255]),
            0,
            text_y_off * SS as i32,
            ss_scale,
            &font,
            &display,
        );
        // Box-filter downsample: average the SS×SS block's alpha, then threshold at 50%
        let mut out = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0u32;
                for dy in 0..SS {
                    for dx in 0..SS {
                        let sx = x * SS + dx;
                        let sy = y * SS + dy;
                        if sx < ss_w && sy < ss_h {
                            sum += big.get_pixel(sx, sy)[3] as u32;
                        }
                    }
                }
                let avg = sum / (SS * SS);
                if avg > 127 {
                    out.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                }
            }
        }
        TextImage::new(
            out,
            if ucc_ean_font {
                FontFace::CondensedBold
            } else {
                FontFace::Mono
            },
            plain_glyphs(&font, scale, &display, 0, text_y_off),
        )
    };

    match orientation {
        FieldOrientation::Normal => {
            // Interpretation lines center on the barcode by default (the
            // ZPL/Labelary calibration); languages with an explicit
            // alignment parameter (TSPL BARCODE) select left/right.
            use crate::elements::field_alignment::FieldAlignment as FA;
            let cx = match line_alignment {
                FA::Left => pos.x,
                FA::Right => pos.x + bw - text_width as i32,
                _ => pos.x + (bw - text_width as i32) / 2,
            };
            let ty = if line_above {
                // ink bottom (4 + 0.706 em of parens/digits) must sit ~2px above
                // the barcode top, mirroring the 4px gap of the below case
                pos.y - (4.0 + 0.706 * font_size) as i32 - 4
            } else {
                pos.y + bh + 2
            };
            let buf_w = (text_width.ceil() as u32).max(1) + 2;
            let buf_h = if ucc_ean_font {
                // ink spans 4px gap + 0.706 em of paren extent
                (4.0 + 0.72 * font_size).ceil() as u32 + 2
            } else {
                font_size.ceil() as u32 + 2
            };
            let buf = render_text_crisp(buf_w, buf_h);
            overlay_at(canvas, &buf, cx, ty);
        }
        _ => {
            // Render text to buffer, rotate to match barcode orientation, then overlay
            let buf_w = (text_width.ceil() as u32).max(1) + 2;
            let buf_h = if ucc_ean_font {
                (4.0 + 0.72 * font_size).ceil() as u32 + 2
            } else {
                font_size.ceil() as u32 + 2
            };
            let buf = render_text_crisp(buf_w, buf_h);

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

/// Interpretation line for the EAN/UPC family (^B8/^B9/^BU): each digit glyph is
/// centered on its own 7-module group; the UPC-E/UPC-A number-system digit sits
/// left of the start guard and the UPC-E check digit right of the end guard --
/// all calibrated against Labelary renders. The check digit selects digit parity, so it
/// is always encoded even when not printed.
#[allow(clippy::too_many_arguments)]
fn draw_module_centered_interpretation_line(
    canvas: &mut BarcodeLayout,
    chars: &[char],
    centers: &[f32],
    pos: &LabelPosition,
    barcode_img: &RgbaImage,
    orientation: FieldOrientation,
    line_above: bool,
    module_width: i32,
    data_height: i32,
) {
    let mw = module_width.max(1) as f32;
    // Labelary's UPC-E interpretation font scales 9.5px per module width, capped at
    // ~28.5px (ink height measured 14px at mw=2, 21px at mw>=3).
    let font_size = (9.5 * mw).min(28.5);
    let font = match FontRef::try_from_slice(FONT_DEJAVU_MONO) {
        Ok(f) => f,
        Err(_) => return,
    };
    // Labelary's interpretation font has a larger x-height and narrower advance
    // than DejaVu Sans Mono; stretch non-uniformly to match (calibrated).
    let scale = PxScale {
        x: font_size * 0.67,
        y: font_size * 1.17,
    };

    // Render each glyph crisply (3x supersample + threshold) into its own buffer.
    let render_glyph = |ch: char, w: u32, h: u32| -> RgbaImage {
        const SS: u32 = 3;
        let ss_scale = PxScale {
            x: scale.x * SS as f32,
            y: scale.y * SS as f32,
        };
        let mut big = RgbaImage::from_pixel((w * SS).max(1), (h * SS).max(1), Rgba([0, 0, 0, 0]));
        drawing::draw_text_mut(
            &mut big,
            Rgba([0, 0, 0, 255]),
            0,
            0,
            ss_scale,
            &font,
            &ch.to_string(),
        );
        let mut out = RgbaImage::from_pixel(w.max(1), h.max(1), Rgba([0, 0, 0, 0]));
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0u32;
                for dy in 0..SS {
                    for dx in 0..SS {
                        let sx = x * SS + dx;
                        let sy = y * SS + dy;
                        if sx < big.width() && sy < big.height() {
                            sum += big.get_pixel(sx, sy)[3] as u32;
                        }
                    }
                }
                if sum / (SS * SS) > 127 {
                    out.put_pixel(x, y, Rgba([0, 0, 0, 255]));
                }
            }
        }
        out
    };

    let _buf_w = (font_size.ceil() as u32).max(1) + 2;
    let buf_h = font_size.ceil() as u32 + 2;
    let mut glyphs: Vec<(RgbaImage, f32)> = Vec::with_capacity(chars.len());
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    for (ch, cx) in chars.iter().zip(centers.iter()) {
        // Measure the glyph's ink width to center it on the module group.
        let tw = measure_text_width(&ch.to_string(), &font, scale, SubstMetrics::PLAIN);
        let w = (tw.ceil() as i32).max(2) as u32;
        let gbuf = render_glyph(*ch, w, buf_h);
        let left = cx - tw / 2.0;
        min_x = min_x.min(left);
        max_x = max_x.max(left + tw);
        glyphs.push((gbuf, left));
    }

    // Strip dimensions and the demo text baseline spacing used by Labelary: glyph
    // top sits 4px below the data bars (6px for the capped 28.5px font).
    let strip_w = (max_x - min_x).ceil() as i32 + 2;
    let mut strip = RgbaImage::from_pixel(strip_w.max(1) as u32, buf_h, Rgba([0, 0, 0, 0]));
    for (gbuf, left) in &glyphs {
        let ox = (left - min_x).round() as i32;
        for y in 0..gbuf.height() {
            for x in 0..gbuf.width() {
                if gbuf.get_pixel(x, y)[3] > 0 {
                    let px = ox + x as i32;
                    if px >= 0 && px < strip.width() as i32 {
                        strip.put_pixel(px as u32, y, Rgba([0, 0, 0, 255]));
                    }
                }
            }
        }
    }

    let mut positioned = Vec::new();
    for (ch, (_, left)) in chars.iter().zip(&glyphs) {
        let ox = (left - min_x).round() as i32;
        positioned.extend(plain_glyphs(&font, scale, &ch.to_string(), ox, 0));
    }
    let strip = TextImage::new(strip, FontFace::Mono, positioned);

    let bh = barcode_img.height() as i32;
    let top_off: i32 = if font_size > 19.0 { 2 } else { 0 };
    let text_h = buf_h as i32;

    match orientation {
        FieldOrientation::Normal => {
            // Text top sits top_off below the DATA bars, not the guard extension.
            let ty = if line_above {
                pos.y - text_h - 2
            } else {
                pos.y + data_height + top_off
            };
            overlay_at(canvas, &strip, min_x.round() as i32 - 1, ty);
        }
        _ => {
            let rotated = match orientation {
                FieldOrientation::Rotated90 => rotate_90(&strip),
                FieldOrientation::Rotated180 => rotate_180(&strip),
                FieldOrientation::Rotated270 => rotate_270(&strip),
                _ => strip,
            };
            let (tx, ty) = match orientation {
                FieldOrientation::Rotated90 => {
                    let cy = pos.y + (barcode_img.width() as i32 - strip_w) / 2;
                    if line_above {
                        (pos.x + bh + 2, cy)
                    } else {
                        (pos.x - rotated.width() as i32 - 2, cy)
                    }
                }
                FieldOrientation::Rotated180 => {
                    let cx = pos.x + (barcode_img.width() as i32 - strip_w) / 2;
                    if line_above {
                        (cx, pos.y + bh + 2)
                    } else {
                        (cx, pos.y - rotated.height() as i32 - 2)
                    }
                }
                FieldOrientation::Rotated270 => {
                    let cy = pos.y + (barcode_img.width() as i32 - strip_w) / 2;
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
