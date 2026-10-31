//! DPL frontend: compiles Datamax-O'Neil DPL label formats into the
//! shared IR.
//!
//! A DPL file is a byte stream of `<STX>` (0x02) system-level commands
//! (ignored — printer configuration) and `<STX>L` … `E` label formats.
//! Inside a format, configuration lines are a letter plus digits
//! (`D11`, `Q0002`, `C0050`, …) and print records start with a rotation
//! digit `1`-`4` followed by the 15-byte header `{a}{b[bb]}{c}{d}{eee}
//! {ffff row}{gggg column}` (plus optional scalable size fields), the
//! data, and a terminator (CR by default, reassignable via `T`).
//!
//! The home position is the label's **bottom-left** corner and the row
//! coordinate grows upward — positions land on the IR's
//! `from_bottom` anchor. Coordinates are hundredths of an inch
//! (`m` switches to tenths of a millimetre) and are resolved to
//! millimetres here. Unsupported constructs error out instead of
//! misrendering; device-only commands are no-ops like the TSPL/CPCL
//! frontends.

use crate::elements::barcode_128::{Barcode128, Barcode128WithData, BarcodeMode};
use crate::elements::barcode_2of5::{Barcode2of5, Barcode2of5WithData};
use crate::elements::barcode_39::{Barcode39, Barcode39WithData};
use crate::elements::barcode_aztec::{BarcodeAztec, BarcodeAztecWithData};
use crate::elements::barcode_datamatrix::{
    BarcodeDatamatrix, BarcodeDatamatrixWithData, DatamatrixRatio,
};
use crate::elements::barcode_ean13::{BarcodeEan13, BarcodeEan13WithData};
use crate::elements::barcode_ean8::{BarcodeEan8, BarcodeEan8WithData};
use crate::elements::barcode_pdf417::{BarcodePdf417, BarcodePdf417WithData};
use crate::elements::barcode_qr::{BarcodeQr, BarcodeQrWithData, QrErrorCorrectionLevel};
use crate::elements::barcode_upca::{BarcodeUca, BarcodeUcaWithData};
use crate::elements::barcode_upce::{BarcodeUcpe, BarcodeUcpeWithData};
use crate::elements::field_orientation::FieldOrientation;
use crate::elements::font::FontInfo;
use crate::elements::font_face::FontFace;
use crate::elements::graphic_box::GraphicBox;
use crate::elements::label_element::LabelElement;
use crate::elements::label_info::LabelInfo;
use crate::elements::label_position::LabelPosition;
use crate::elements::line_color::LineColor;
use crate::elements::maxicode::{Maxicode, MaxicodeWithData};
use crate::elements::reverse_print::ReversePrint;
use crate::elements::text_field::TextField;

pub struct DplParser {
    dpmm: i32,
}

impl Default for DplParser {
    fn default() -> Self {
        Self { dpmm: 8 }
    }
}

impl DplParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_dpmm(dpmm: i32) -> Self {
        Self { dpmm: dpmm.max(1) }
    }

    /// Parse DPL data into one [`LabelInfo`] per `<STX>L` format that
    /// produced printable content. `E` terminates and prints, `X`
    /// terminates without printing (the rendered image is the same).
    pub fn parse(&self, dpl_data: &[u8]) -> Result<Vec<LabelInfo>, String> {
        let mut cursor = DplCursor::new(dpl_data);
        let mut labels = Vec::new();
        let mut state = DplState::new(self.dpmm);

        let mut elements: Vec<LabelElement> = Vec::new();
        let mut in_format = false;
        let mut record_no = 0usize;

        while let Some(record) = cursor.next_record()? {
            let line = record.data;
            if line.is_empty() {
                continue;
            }

            // <STX> system-level bytes: 0x02 + a letter (sometimes a second
            // letter). Only <STX>L opens a format; everything else is
            // printer configuration.
            if line.first() == Some(&0x02) {
                if line.len() >= 2 && (line[1] == b'L' || line[1] == b'l') {
                    // A stray open without E flushes (lenient tail flush).
                    if in_format && !elements.is_empty() {
                        labels.push(state.to_label(std::mem::take(&mut elements)));
                    }
                    in_format = true;
                    record_no = 0;
                } else if in_format {
                    // System commands may not appear inside a format; treat
                    // as leaving it (matches "must be issued outside").
                    in_format = false;
                    elements.clear();
                }
                continue;
            }

            if !in_format {
                continue; // records outside a format draw nothing
            }

            match line[0] {
                // Print records start with a rotation digit.
                b'1'..=b'4' => {
                    record_no += 1;
                    parse_record(&mut state, &mut elements, line)
                        .map_err(|e| format!("DPL record {record_no}: {e}"))?;
                }
                b'E' | b'e' => {
                    if !elements.is_empty() {
                        labels.push(state.to_label(std::mem::take(&mut elements)));
                    }
                    in_format = false;
                }
                b'X' | b'x' => {
                    // Terminate without printing — the image is the same.
                    if !elements.is_empty() {
                        labels.push(state.to_label(std::mem::take(&mut elements)));
                    }
                    in_format = false;
                }
                _ => {
                    parse_config(&mut state, line)?;
                }
            }
        }

        if in_format && !elements.is_empty() {
            labels.push(state.to_label(elements));
        }
        Ok(labels)
    }
}

struct DplState {
    dpmm: f64,
    metric: bool,
    /// Column offset (`Cnnnn`) and row offset (`Rnnnn`), in units.
    column_offset: f64,
    row_offset: f64,
}

impl DplState {
    fn new(dpmm: i32) -> Self {
        DplState {
            dpmm: dpmm as f64,
            metric: false,
            column_offset: 0.0,
            row_offset: 0.0,
        }
    }

    /// Unit value → millimetres (hundredths of an inch, or tenths of a
    /// millimetre in metric mode).
    fn mm(&self, units: f64) -> f64 {
        if self.metric {
            units * 0.1
        } else {
            units * 25.4 / 100.0
        }
    }

    fn dots(&self, units: f64) -> i32 {
        (self.mm(units) * self.dpmm).round() as i32
    }

    fn to_label(&self, elements: Vec<LabelElement>) -> LabelInfo {
        LabelInfo {
            mirrored: false,
            print_width: 0,
            inverted: false,
            // DPL declares no in-band label size (the printer config does).
            label_width_mm: None,
            label_height_mm: None,
            elements,
        }
    }
}

/// The per-record header fields the type-specific parsers need.
struct RecordCtx<'a> {
    orientation: FieldOrientation,
    c: u8,
    d: u8,
    eee: String,
    position: LabelPosition,
    rest: &'a [u8],
}

/// Parse one print record (`1`-`4` rotation digit prefix).
fn parse_record(
    state: &mut DplState,
    elements: &mut Vec<LabelElement>,
    line: &[u8],
) -> Result<(), String> {
    let header_len = if line.get(1) == Some(&b'W') { 17 } else { 15 };
    if line.len() < header_len {
        return Err(format!(
            "DPL record header is too short (expected {header_len} bytes)"
        ));
    }
    let orientation = match line[0] {
        b'2' => FieldOrientation::Rotated90,
        b'3' => FieldOrientation::Rotated180,
        b'4' => FieldOrientation::Rotated270,
        _ => FieldOrientation::Normal,
    };

    let b = line[1];
    let shift = if b == b'W' { 2 } else { 0 };
    let eee = std::str::from_utf8(&line[4 + shift..7 + shift])
        .map_err(|_| "invalid DPL font or height field")?
        .to_string();
    let row = parse_units(&line[7 + shift..11 + shift])?;
    let col = parse_units(&line[11 + shift..15 + shift])?;
    let position = LabelPosition {
        x: state.dots(col + state.column_offset),
        y: state.dots(row + state.row_offset),
        calculate_from_bottom: false,
        automatic_position: false,
        from_bottom: true,
    };
    let counted = matches!(b, b'Z' | b'U') || line.get(1..4) == Some(b"W1C");
    let ctx = RecordCtx {
        orientation,
        c: line[2 + shift],
        d: line[3 + shift],
        eee: eee.clone(),
        position: position.clone(),
        rest: line
            .get(header_len + if counted { 4 } else { 0 }..)
            .ok_or("DPL record missing byte count")?,
    };
    let rest = ctx.rest;

    // Scalable fonts carry two optional 4-char size fields before data.
    if b == b'9' && starts_with_scalable_id(&eee) {
        let mut font = scalable_size_fields(state, &eee, rest)?;
        font.orientation = orientation;
        font.width *= super::fonts::multiplier(ctx.c).max(1.0);
        font.height *= super::fonts::multiplier(ctx.d).max(1.0);
        let data = record_data(&rest[8..]);
        if data.is_empty() {
            return Ok(());
        }
        elements.push(LabelElement::Text(TextField {
            reverse_print: ReversePrint::default(),
            font,
            position,
            alignment: crate::elements::field_alignment::FieldAlignment::Left,
            text: data,
            block: None,
        }));
        return Ok(());
    }

    // W-expansion records insert a two-character type after W.
    if b == b'W' {
        return match &line[2..4] {
            b"1c" | b"1C" => parse_datamatrix(state, elements, &ctx),
            b"1d" => parse_qr(state, elements, &ctx, false),
            b"1D" => parse_qr(state, elements, &ctx, true),
            b"1f" | b"1F" => parse_aztec(state, elements, &ctx),
            other => Err(format!(
                "unsupported DPL expanded bar code W{}",
                String::from_utf8_lossy(other)
            )),
        };
    }

    // PDF-417: `[byte-count] T/F security aspect rows columns data`.
    if b == b'z' || b == b'Z' {
        return parse_pdf417(state, elements, &ctx);
    }

    // MaxiCode: fixed-field form `zip5 zip4 country class message`.
    if b == b'u' || b == b'U' {
        return parse_maxicode(elements, &ctx);
    }

    match b {
        b'0'..=b'8' => {
            let font = bitmap_text_font(state, b, ctx.c, ctx.d, orientation);
            let data = record_data(rest);
            if data.is_empty() {
                return Ok(());
            }
            elements.push(LabelElement::Text(TextField {
                reverse_print: ReversePrint::default(),
                font,
                position,
                alignment: crate::elements::field_alignment::FieldAlignment::Left,
                text: data,
                block: None,
            }));
            Ok(())
        }
        b'9' => {
            let font = smooth_text_font(state, &eee, ctx.c, ctx.d, orientation);
            let data = record_data(rest);
            if data.is_empty() {
                return Ok(());
            }
            elements.push(LabelElement::Text(TextField {
                reverse_print: ReversePrint::default(),
                font,
                position,
                alignment: crate::elements::field_alignment::FieldAlignment::Left,
                text: data,
                block: None,
            }));
            Ok(())
        }
        b'X' => parse_graphic(state, elements, &ctx),
        b'Y' => Err("DPL stored images (Y records) are not supported".to_string()),
        _ => parse_barcode_record(state, elements, b, &ctx),
    }
}

fn starts_with_scalable_id(eee: &str) -> bool {
    let bytes = eee.as_bytes();
    matches!(
        bytes.first(),
        Some(b'S') | Some(b's') | Some(b'U') | Some(b'u')
    )
}

/// Scalable fonts (`S`/`U` IDs): optional `hhhh`/`iiii` size fields in
/// points (`P004`-`P999`) or dots, then the data.
fn scalable_size_fields(state: &DplState, eee: &str, rest: &[u8]) -> Result<FontInfo, String> {
    if rest.len() < 8 {
        return Err("DPL scalable font record missing size fields".to_string());
    }
    let h_field = std::str::from_utf8(&rest[..4]).unwrap_or("");
    let w_field = std::str::from_utf8(&rest[4..8]).unwrap_or("");
    let to_dots = |field: &str| -> Option<f64> {
        if let Some(pts) = field.strip_prefix('P').or_else(|| field.strip_prefix('p')) {
            let pts: f64 = pts.parse().ok()?;
            // 72.307 points per inch (DPL manual).
            Some(pts / 72.307 * 25.4 * state.dpmm)
        } else {
            field.parse::<f64>().ok().map(|v| v / 8.0 * state.dpmm)
        }
    };
    let height =
        to_dots(h_field).ok_or_else(|| format!("invalid scalable font height {h_field:?}"))?;
    let width =
        to_dots(w_field).ok_or_else(|| format!("invalid scalable font width {w_field:?}"))?;
    Ok(FontInfo {
        name: format!("DPL-SCALABLE-{eee}"),
        face: FontFace::CondensedBold,
        advance_mult: 1.0,
        dedot_zero: false,
        width,
        height,
        orientation: FieldOrientation::Normal,
    })
}

fn bitmap_text_font(
    state: &DplState,
    b: u8,
    c: u8,
    d: u8,
    orientation: FieldOrientation,
) -> FontInfo {
    let font =
        super::fonts::bitmap_font(b).unwrap_or_else(|| super::fonts::bitmap_font(b'0').unwrap());
    let w_mult = super::fonts::multiplier(c).max(1.0);
    let h_mult = super::fonts::multiplier(d).max(1.0);
    FontInfo {
        name: (b as char).to_string(),
        face: font.face,
        advance_mult: 1.0,
        dedot_zero: false,
        width: font.width * w_mult * state.dpmm / 8.0,
        height: font.height * h_mult * state.dpmm / 8.0,
        orientation,
    }
}

fn smooth_text_font(
    state: &DplState,
    eee: &str,
    c: u8,
    d: u8,
    orientation: FieldOrientation,
) -> FontInfo {
    let w_mult = super::fonts::multiplier(c).max(1.0);
    let h_mult = super::fonts::multiplier(d).max(1.0);
    let points = super::fonts::smooth_font_points(eee).unwrap_or(10.0);
    let mm = points / 72.307 * 25.4;
    FontInfo {
        name: format!("9-{eee}"),
        face: FontFace::CondensedBold,
        advance_mult: 1.0,
        dedot_zero: false,
        width: mm * state.dpmm * w_mult,
        height: mm * state.dpmm * h_mult,
        orientation,
    }
}

/// The cursor has already removed the terminator or honored the byte count.
/// Whitespace and control bytes inside the payload are data.
fn record_data(rest: &[u8]) -> String {
    String::from_utf8_lossy(rest).into_owned()
}

fn parse_units(field: &[u8]) -> Result<f64, String> {
    let text = String::from_utf8_lossy(field);
    text.trim()
        .parse::<f64>()
        .map_err(|_| format!("invalid position field {text:?}"))
}

// ── graphics (X records) ────────────────────────────────────────────

/// `X` records: the data field describes the object — `Lhhhvvv` /
/// `lhhhhvvvv` lines, `Bhhhvvvbbbsss` / `bhhhhvvvvbbbbssss` boxes
/// (dimensions in units), polygons and circles (unsupported).
fn parse_graphic(
    state: &DplState,
    elements: &mut Vec<LabelElement>,
    ctx: &RecordCtx,
) -> Result<(), String> {
    let data = record_data(ctx.rest);
    if data.is_empty() {
        return Ok(());
    }
    if !data.is_ascii() {
        return Err("DPL graphics require ASCII drawing parameters".to_string());
    }
    let (kind, dims) = data.split_at(1);
    let mut position = ctx.position.clone();
    match kind.as_bytes()[0] {
        b'L' | b'l' => {
            let wide = kind == "l";
            let (h_len, v_len) = if wide { (4, 4) } else { (3, 3) };
            if dims.len() < h_len + v_len {
                return Err(format!("DPL line drawing {data:?} is too short"));
            }
            let h = parse_units(&dims.as_bytes()[..h_len])?;
            let v = parse_units(&dims.as_bytes()[h_len..h_len + v_len])?;
            let width = state.dots(h).max(1);
            let height = state.dots(v).max(1);
            position.calculate_from_bottom = true;
            // A line is a filled box whose bottom-left corner is the row/
            // column position (row grows up).
            elements.push(LabelElement::GraphicBox(GraphicBox {
                reverse_print: ReversePrint::default(),
                position,
                width,
                height,
                border_thickness: width.min(height).max(1),
                corner_rounding: 0,
                line_color: LineColor::Black,
            }));
            Ok(())
        }
        b'B' | b'b' => {
            let wide = kind == "b";
            let (h_len, v_len, b_len, s_len) = if wide { (4, 4, 4, 4) } else { (3, 3, 3, 3) };
            let need = h_len + v_len + b_len + s_len;
            if dims.len() < need {
                return Err(format!("DPL box drawing {data:?} is too short"));
            }
            let h = parse_units(&dims.as_bytes()[..h_len])?;
            let v = parse_units(&dims.as_bytes()[h_len..h_len + v_len])?;
            let top_bot = parse_units(&dims.as_bytes()[h_len + v_len..h_len + v_len + b_len])?;
            let sides = parse_units(&dims.as_bytes()[h_len + v_len + b_len..need])?;
            let width = state.dots(h).max(1);
            let height = state.dots(v).max(1);
            let horizontal = state.dots(top_bot).max(1).min(height);
            let vertical = state.dots(sides).max(1).min(width);
            if horizontal != vertical {
                // Resolve independent edge thicknesses into four filled
                // rectangles. The renderer needs no language-specific box mode.
                for (dx, dy, w, h) in [
                    (0, 0, width, horizontal),
                    (0, height - horizontal, width, horizontal),
                    (0, 0, vertical, height),
                    (width - vertical, 0, vertical, height),
                ] {
                    let mut edge_position = ctx.position.clone();
                    edge_position.x = edge_position.x.saturating_add(dx);
                    edge_position.y = edge_position.y.saturating_add(dy);
                    elements.push(LabelElement::GraphicBox(GraphicBox {
                        reverse_print: ReversePrint::default(),
                        position: edge_position,
                        width: w,
                        height: h,
                        border_thickness: w.min(h),
                        corner_rounding: 0,
                        line_color: LineColor::Black,
                    }));
                }
                return Ok(());
            }
            elements.push(LabelElement::GraphicBox(GraphicBox {
                reverse_print: ReversePrint::default(),
                position: ctx.position.clone(),
                width,
                height,
                border_thickness: horizontal,
                corner_rounding: 0,
                line_color: LineColor::Black,
            }));
            Ok(())
        }
        b'P' => Err("DPL polygons are not supported".to_string()),
        b'C' => Err("DPL circles are not supported".to_string()),
        other => Err(format!("unknown DPL graphic kind {}", other as char)),
    }
}

// ── barcodes ────────────────────────────────────────────────────────

/// Barcode defaults (Appendix F-2): `(height in hundredths of an inch,
/// ratio or module)` per symbology letter.
fn barcode_defaults(b: u8) -> (f64, f64, bool) {
    // (height units, default param, module-based?)
    match b {
        b'A' | b'a' => (40.0, 3.0, false), // Code 39, 6:2
        b'B' | b'b' => (80.0, 3.0, true),  // UPC-A
        b'C' | b'c' => (80.0, 3.0, true),  // UPC-E
        b'D' | b'd' => (40.0, 3.0, false), // I2of5, 6:2
        b'E' | b'e' => (40.0, 2.0, true),  // Code 128
        b'F' | b'f' => (80.0, 3.0, true),  // EAN-13
        b'G' | b'g' => (80.0, 3.0, true),  // EAN-8
        b'H' | b'h' => (40.0, 3.0, false), // HIBC (39 + mod43)
        b'J' | b'j' => (40.0, 2.5, false), // I2of5 + mod10
        b'K' | b'k' => (40.0, 3.0, false),
        b'L' | b'l' => (130.0, 2.5, false), // I2of5 mod10 + bearer
        b'M' | b'm' => (90.0, 3.0, true),
        b'N' | b'n' => (80.0, 3.0, true),
        b'O' | b'o' => (40.0, 3.0, false),
        b'p' => (8.0, 0.0, false), // Postnet
        b'Q' | b'q' | b'R' | b'r' | b'S' | b's' => (140.0, 2.0, true),
        b'T' | b't' => (80.0, 1.0, false),
        b'v' => (50.0, 1.0, false),      // FIM
        b'U' | b'u' => (0.0, 0.0, true), // MaxiCode
        b'Z' | b'z' => (0.0, 0.0, true), // PDF417
        _ => (40.0, 2.0, true),
    }
}

fn parse_barcode_record(
    state: &mut DplState,
    elements: &mut Vec<LabelElement>,
    b: u8,
    ctx: &RecordCtx,
) -> Result<(), String> {
    let orientation = ctx.orientation;
    let c = ctx.c;
    let d = ctx.d;
    let eee = ctx.eee.as_str();
    let position = ctx.position.clone();
    let rest = ctx.rest;
    let hri = b.is_ascii_uppercase();
    let (default_height, default_param, module_based) = barcode_defaults(b);

    let height_units: f64 = eee.trim().parse().unwrap_or(0.0);
    let height_units = if height_units == 0.0 {
        default_height
    } else {
        height_units
    };
    let height = state.dots(height_units).max(1);

    let wide_dots = mult_char_to_dots(state, c);
    let narrow_dots = mult_char_to_dots(state, d);
    // 0 in both fields selects the default ratio/module.
    let (narrow, ratio) = if c == b'0' && d == b'0' {
        let module = default_param;
        (module.max(1.0) as i32, default_param)
    } else if module_based {
        let module = if narrow_dots > 0 {
            narrow_dots
        } else {
            default_param as i32
        };
        (module.max(1), 1.0)
    } else {
        let narrow = narrow_dots.max(1);
        let ratio = if wide_dots > 0 && narrow > 0 {
            wide_dots as f64 / narrow as f64
        } else {
            default_param
        };
        (narrow, ratio.max(1.0))
    };

    let data_raw = record_data(rest);
    if data_raw.is_empty() {
        return Ok(());
    }

    let element = match b {
        // Code 3 of 9 (H = Health Industry variant with mod-43 check).
        b'A' | b'a' | b'H' | b'h' => LabelElement::Barcode39(Barcode39WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode39 {
                orientation,
                height,
                line: hri,
                line_above: false,
                check_digit: b == b'H' || b == b'h',
            },
            width: narrow,
            width_ratio: ratio,
            position,
            line_alignment: crate::elements::field_alignment::FieldAlignment::Center,
            data: data_raw.clone(),
        }),
        // Interleaved 2 of 5, optionally with modulo-10 check digit
        // (L adds bearer bars, which are not modeled).
        b'D' | b'd' | b'J' | b'j' | b'L' | b'l' => {
            let check = matches!(b, b'J' | b'j' | b'L' | b'l');
            LabelElement::Barcode2of5(Barcode2of5WithData {
                reverse_print: ReversePrint::default(),
                barcode: Barcode2of5 {
                    orientation,
                    height,
                    line: hri,
                    line_above: false,
                    check_digit: check,
                },
                width: narrow,
                width_ratio: ratio,
                position,
                line_alignment: crate::elements::field_alignment::FieldAlignment::Center,
                data: data_raw.clone(),
            })
        }
        b'E' | b'e' => LabelElement::Barcode128(Barcode128WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode128 {
                orientation,
                height,
                line: hri,
                line_above: false,
                check_digit: false,
                mode: BarcodeMode::Automatic,
            },
            width: narrow,
            position,
            line_alignment: crate::elements::field_alignment::FieldAlignment::Center,
            data: data_raw.clone(),
        }),
        // UCC/EAN Code 128 (Q/R/S variants).
        b'Q' | b'q' | b'R' | b'r' | b'S' | b's' => LabelElement::Barcode128(Barcode128WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode128 {
                orientation,
                height,
                line: hri,
                line_above: false,
                check_digit: false,
                mode: BarcodeMode::Ucc,
            },
            width: narrow,
            position,
            line_alignment: crate::elements::field_alignment::FieldAlignment::Center,
            data: data_raw.clone(),
        }),
        b'B' | b'b' => LabelElement::BarcodeUca(BarcodeUcaWithData {
            reverse_print: ReversePrint::default(),
            barcode: BarcodeUca {
                orientation,
                height,
                line: hri,
                line_above: false,
            },
            width: narrow,
            position,
            data: data_raw.clone(),
        }),
        b'C' | b'c' => LabelElement::BarcodeUcpe(BarcodeUcpeWithData {
            reverse_print: ReversePrint::default(),
            barcode: BarcodeUcpe {
                orientation,
                height,
                line: hri,
                line_above: false,
                check_digit: false,
            },
            width: narrow,
            position,
            data: data_raw.clone(),
        }),
        b'F' | b'f' => LabelElement::BarcodeEan13(BarcodeEan13WithData {
            reverse_print: ReversePrint::default(),
            barcode: BarcodeEan13 {
                orientation,
                height,
                line: hri,
                line_above: false,
            },
            width: narrow,
            position,
            data: data_raw.clone(),
        }),
        b'G' | b'g' => LabelElement::BarcodeEan8(BarcodeEan8WithData {
            reverse_print: ReversePrint::default(),
            barcode: BarcodeEan8 {
                orientation,
                height,
                line: hri,
                line_above: false,
            },
            width: narrow,
            position,
            data: data_raw.clone(),
        }),
        other => return Err(format!("unsupported DPL bar code type {}", other as char)),
    };
    elements.push(element);
    Ok(())
}

/// `z`/`Z` PDF-417: `c`/`d` are module multipliers (0 = the 2x6 default),
/// then `[byte-count]` (Z only), `T|F` truncation, security digit, aspect
/// ratio, rows and columns, then the data.
fn parse_pdf417(
    state: &mut DplState,
    elements: &mut Vec<LabelElement>,
    ctx: &RecordCtx,
) -> Result<(), String> {
    let orientation = ctx.orientation;
    let position = ctx.position.clone();
    let rest = ctx.rest;
    let mut module_w = super::fonts::multiplier(ctx.c) as i32;
    let mut module_h = super::fonts::multiplier(ctx.d) as i32;
    if module_w <= 0 {
        module_w = 2;
    }
    if module_h <= 0 {
        module_h = 6;
    }

    let tail = rest;
    if tail.len() < 8 {
        return Err("DPL PDF417 record missing option fields".to_string());
    }
    // i(T/F) j(security) kk(aspect) ll(rows) mm(columns) = 8 chars.
    let truncate = tail[0] == b'T' || tail[0] == b't';
    let security = (tail[1] as char).to_digit(10).unwrap_or(1) as i32;
    let rows = std::str::from_utf8(&tail[4..6])
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let columns = std::str::from_utf8(&tail[6..8])
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let data = std::str::from_utf8(&tail[8..])
        .map_err(|_| "DPL PDF417 non-UTF-8 binary payloads are not supported")?
        .to_string();
    if data.is_empty() {
        return Ok(());
    }
    elements.push(LabelElement::BarcodePdf417(BarcodePdf417WithData {
        reverse_print: ReversePrint::default(),
        barcode: BarcodePdf417 {
            orientation,
            row_height: (module_h as f64 * state.dpmm / 8.0).round().max(1.0) as i32,
            security,
            columns,
            rows,
            truncate,
            module_width: (module_w as f64 * state.dpmm / 8.0).round().max(1.0) as i32,
            by_height: 0,
        },
        position,
        data,
    }));
    Ok(())
}

/// `u`/`U` MaxiCode, fixed-field form: `zip5 zip4 country class message`
/// builds the mode-2 structured payload the shared encoder consumes. The
/// `[)>RS01GS` UPS-structured form is rejected explicitly.
fn parse_maxicode(elements: &mut Vec<LabelElement>, ctx: &RecordCtx) -> Result<(), String> {
    let position = ctx.position.clone();
    let data = record_data(ctx.rest);
    if data.is_empty() {
        return Ok(());
    }
    if data.starts_with("[)>") {
        return Err(
            "DPL MaxiCode structured ([)>RS…) messages are not supported — use the fixed-field form"
                .to_string(),
        );
    }
    if data.len() < 15 {
        return Err("DPL MaxiCode fixed-field data is too short".to_string());
    }
    if !data.as_bytes()[..15].iter().all(u8::is_ascii_digit) {
        return Err("DPL MaxiCode postal/country/class fields must be digits".to_string());
    }
    let zip = &data[..9];
    let country = &data[9..12];
    let class = &data[12..15];
    let message = &data[15..];
    let payload = if message.is_empty() {
        format!("{class}{country}{zip}[)>\x1e\x1d")
    } else {
        format!("{class}{country}{zip}[)>\x1e\x1d{message}")
    };
    elements.push(LabelElement::Maxicode(MaxicodeWithData {
        reverse_print: ReversePrint::default(),
        code: Maxicode { mode: 2 },
        position,
        data: payload,
    }));
    Ok(())
}

/// `W1c` DataMatrix: `c d eee ffff gggg` already consumed by the generic
/// header and byte count; `after` holds `iii j kkk lll data`.
fn parse_datamatrix(
    state: &mut DplState,
    elements: &mut Vec<LabelElement>,
    ctx: &RecordCtx,
) -> Result<(), String> {
    let orientation = ctx.orientation;
    let position = ctx.position.clone();
    let after = ctx.rest;
    let module = super::fonts::multiplier(ctx.c).max(1.0) as i32;
    let v_module = super::fonts::multiplier(ctx.d).max(1.0) as i32;
    if module != v_module {
        return Err("DPL DataMatrix unequal module dimensions are not supported".to_string());
    }
    let tail = after;
    // iii (ECC level, 3 digits), j (format), kkk rows, lll columns.
    if tail.len() < 10 {
        return Err("DPL DataMatrix record missing option fields".to_string());
    }
    if &tail[..3] != b"200" {
        return Err("DPL DataMatrix supports ECC 200 only".to_string());
    }
    if !tail[..10].iter().all(u8::is_ascii_digit) {
        return Err("invalid DPL DataMatrix option fields".to_string());
    }
    let rows = std::str::from_utf8(&tail[4..7])
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let columns = std::str::from_utf8(&tail[7..10])
        .ok()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let data_bytes = tail[10..].to_vec();
    let data = String::from_utf8_lossy(&data_bytes).into_owned();
    if data.is_empty() {
        return Ok(());
    }
    elements.push(LabelElement::BarcodeDatamatrix(BarcodeDatamatrixWithData {
        reverse_print: ReversePrint::default(),
        barcode: BarcodeDatamatrix {
            orientation,
            height: (module as f64 * state.dpmm / 8.0).round().max(1.0) as i32,
            quality: 200,
            columns,
            rows,
            format: 6,
            escape: 0,
            ratio: Some(DatamatrixRatio::Square),
        },
        position,
        data,
        data_bytes: Some(data_bytes),
    }));
    Ok(())
}

/// `W1d` (auto: data only) / `W1D` (manual: QR payload conventions).
fn parse_qr(
    state: &mut DplState,
    elements: &mut Vec<LabelElement>,
    ctx: &RecordCtx,
    manual: bool,
) -> Result<(), String> {
    let orientation = ctx.orientation;
    let position = ctx.position.clone();
    let after = ctx.rest;
    // Each unit of c (and d) is .01 inch / .1 mm of module size; 0 keeps
    // the manual's 4-dot default cell.
    let module_units = super::fonts::multiplier(ctx.c);
    let module_mm = if module_units == 0.0 {
        4.0 / 8.0
    } else {
        state.mm(module_units)
    };
    let magnification = (module_mm * state.dpmm).round().max(1.0) as i32;
    let tail: &[u8] = after;
    let data = record_data(tail);
    if data.is_empty() {
        return Ok(());
    }
    let (payload, ec) = if manual {
        super::qr_payload::resolve_dpl_qr_payload(&data)?
    } else {
        (data, QrErrorCorrectionLevel::M)
    };
    elements.push(LabelElement::BarcodeQr(BarcodeQrWithData {
        reverse_print: ReversePrint::default(),
        barcode: BarcodeQr {
            magnification,
            orientation,
        },
        height: 0,
        position,
        data: payload,
        error_correction: ec,
    }));
    Ok(())
}

fn parse_aztec(
    state: &mut DplState,
    elements: &mut Vec<LabelElement>,
    ctx: &RecordCtx,
) -> Result<(), String> {
    let orientation = ctx.orientation;
    let position = ctx.position.clone();
    let after = ctx.rest;
    let module_mm = state.mm(super::fonts::multiplier(ctx.c));
    let magnification = (module_mm * state.dpmm).round().max(1.0) as i32;
    let data = record_data(after);
    if data.is_empty() {
        return Ok(());
    }
    elements.push(LabelElement::BarcodeAztec(BarcodeAztecWithData {
        reverse_print: ReversePrint::default(),
        barcode: BarcodeAztec {
            orientation,
            magnification,
            size: 0,
        },
        position,
        data,
    }));
    Ok(())
}

fn mult_char_to_dots(state: &DplState, ch: u8) -> i32 {
    // For barcodes the multipliers are bar widths in dots directly
    // (1-9 / A-O), not unit-scaled.
    let _ = state;
    super::fonts::multiplier(ch) as i32
}

// ── configuration commands inside a format ──────────────────────────

fn parse_config(state: &mut DplState, line: &[u8]) -> Result<(), String> {
    let arg = String::from_utf8_lossy(&line[1..]).trim().to_string();
    match line[0] {
        b'm' => state.metric = true,
        b'n' => state.metric = false,
        b'C' => {
            if let Ok(v) = arg.parse::<f64>() {
                state.column_offset = v;
            }
        }
        b'R' => {
            if let Ok(v) = arg.parse::<f64>() {
                state.row_offset = v;
            }
        }
        // Tnn is handled by the shared byte cursor.
        // Device behaviour / printer state with no drawable effect:
        // D (dot size), Q (quantity), speeds P/S/f/p, heat H, cut :/c,
        // mirror M (not modeled per-field), format attribute A (XOR/
        // transparent compositing), font attributes F, symbol set y,
        // zero-slash z, counters + - ^, registers G, replacements U.
        _ => {}
    }
    Ok(())
}

// ── byte cursor ─────────────────────────────────────────────────────

/// One record, with its actual terminator retained for lossless formatting.
pub(super) struct DplRecord<'a> {
    pub(super) data: &'a [u8],
    pub(super) terminator: Option<u8>,
}

pub(super) struct DplCursor<'a> {
    data: &'a [u8],
    pos: usize,
    in_format: bool,
    field_terminator: u8,
}

impl<'a> DplCursor<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            in_format: false,
            field_terminator: b'\r',
        }
    }

    /// Configuration commands remain CR/LF-delimited. Tnn changes only the
    /// next format record; uppercase counted barcodes take precedence over
    /// every terminator, including embedded CR, LF, NUL and STX bytes.
    pub(super) fn next_record(&mut self) -> Result<Option<DplRecord<'a>>, String> {
        while self
            .data
            .get(self.pos)
            .is_some_and(|b| matches!(b, b'\r' | b'\n'))
        {
            self.pos += 1;
        }
        if self.pos == self.data.len() {
            return Ok(None);
        }
        let start = self.pos;
        let remaining = &self.data[start..];
        let field = self.in_format && matches!(remaining[0], b'1'..=b'4');
        let terminator = if field { self.field_terminator } else { b'\r' };
        let counted_header = if field && remaining.get(1..4) == Some(b"W1C") {
            Some(17)
        } else if field && matches!(remaining.get(1), Some(b'Z' | b'U')) {
            Some(15)
        } else {
            None
        };
        let end = if let Some(header) = counted_header {
            let count = remaining
                .get(header..header + 4)
                .filter(|s| s.iter().all(u8::is_ascii_digit))
                .ok_or("DPL counted record requires a four-digit byte count")?;
            let count = count
                .iter()
                .fold(0usize, |n, b| n * 10 + (b - b'0') as usize);
            let length = header + 4 + count;
            if length > remaining.len() {
                return Err("DPL counted record data shorter than declared".to_string());
            }
            start + length
        } else {
            let length = remaining
                .iter()
                .position(|&b| b == terminator || (terminator == b'\r' && b == b'\n'));
            if field && terminator != b'\r' && length.is_none() {
                return Err("DPL record missing its alternate terminator".to_string());
            }
            start + length.unwrap_or(remaining.len())
        };
        let line = &self.data[start..end];
        self.pos = end;
        let actual_terminator = self
            .data
            .get(end)
            .copied()
            .filter(|&b| b == terminator || (terminator == b'\r' && b == b'\n'));
        if actual_terminator.is_some() {
            self.pos += 1;
            if actual_terminator == Some(b'\r') && self.data.get(self.pos) == Some(&b'\n') {
                self.pos += 1;
            }
        }
        if field {
            self.field_terminator = b'\r';
        } else if line.starts_with(b"\x02") {
            self.in_format = matches!(line.get(1), Some(b'L' | b'l'));
            self.field_terminator = b'\r';
        } else if self.in_format && line.first() == Some(&b'T') {
            if line.len() != 3 || !line[1..].iter().all(u8::is_ascii_hexdigit) {
                return Err("DPL T requires two hexadecimal digits".to_string());
            }
            self.field_terminator =
                u8::from_str_radix(std::str::from_utf8(&line[1..]).unwrap(), 16).unwrap();
        } else if self.in_format && matches!(line.first(), Some(b'E' | b'X' | b'e' | b'x')) {
            self.in_format = false;
            self.field_terminator = b'\r';
        }
        Ok(Some(DplRecord {
            data: line,
            terminator: actual_terminator,
        }))
    }
}
