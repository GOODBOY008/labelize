//! CPCL frontend: compiles Zebra mobile-printer CPCL commands into the
//! shared IR.
//!
//! A CPCL file is a sequence of control sessions: a `! {offset} {hdpi}
//! {vdpi} {height} {qty}` start line, drawing commands, then `PRINT`
//! (print), `END` (execute without printing) or `ABORT` (discard). Lines
//! are keyword + space-separated fields terminated by CRLF; comments start
//! with `;` in column 1. Coordinates are dots at the header's resolution
//! (origin top-left, y downward) unless a UNITS command switches the
//! session to a physical unit — every value is resolved to millimetres
//! here and handed to the IR in renderer dots at the engine's dpmm.
//!
//! Unsupported constructs error out instead of misrendering (see the
//! explicit arms in [`parse_line`]); device-only commands (CONTRAST, TONE,
//! PACE, …) are accepted as no-ops, mirroring the TSPL frontend, and
//! `COUNT` is inert serial state like ZPL `^SN` (its effect only shows on
//! copies after the first).

use crate::elements::barcode_128::{Barcode128, Barcode128WithData, BarcodeMode};
use crate::elements::barcode_2of5::{Barcode2of5, Barcode2of5WithData};
use crate::elements::barcode_39::{Barcode39, Barcode39WithData};
use crate::elements::barcode_aztec::{BarcodeAztec, BarcodeAztecWithData};
use crate::elements::barcode_ean13::{BarcodeEan13, BarcodeEan13WithData};
use crate::elements::barcode_ean8::{BarcodeEan8, BarcodeEan8WithData};
use crate::elements::barcode_pdf417::{BarcodePdf417, BarcodePdf417WithData};
use crate::elements::barcode_qr::{BarcodeQr, BarcodeQrWithData};
use crate::elements::barcode_upca::{BarcodeUca, BarcodeUcaWithData};
use crate::elements::barcode_upce::{BarcodeUcpe, BarcodeUcpeWithData};
use crate::elements::field_alignment::FieldAlignment;
use crate::elements::field_block::FieldBlock;
use crate::elements::field_orientation::FieldOrientation;
use crate::elements::font::FontInfo;
use crate::elements::font_face::FontFace;
use crate::elements::graphic_box::GraphicBox;
use crate::elements::graphic_diagonal_line::GraphicDiagonalLine;
use crate::elements::graphic_field::GraphicField;
use crate::elements::graphic_field::GraphicFieldFormat;
use crate::elements::graphic_field::GraphicFieldMode;
use crate::elements::invert_region::InvertRegion;
use crate::elements::label_element::LabelElement;
use crate::elements::label_info::LabelInfo;
use crate::elements::label_position::LabelPosition;
use crate::elements::line_color::LineColor;
use crate::elements::maxicode::{Maxicode, MaxicodeWithData};
use crate::elements::reverse_print::ReversePrint;
use crate::elements::text_alignment::TextAlignment;
use crate::elements::text_field::TextField;

/// Measurement system of a session's numeric fields.
#[derive(Clone, Copy, PartialEq)]
enum Units {
    Dots,
    Inches,
    Centimeters,
    Millimeters,
}

/// Session-wide justification state (`LEFT`/`CENTER`/`RIGHT` [end]).
#[derive(Clone, Copy, PartialEq)]
enum Justify {
    Left,
    Center,
    Right,
}

/// Dots-per-millimetre class of a header resolution: the CPCL manual's
/// own conversion (8 dots/mm at 200-203 dpi, 12 at 300, 24 at 600).
fn header_dpmm(dpi: f64) -> f64 {
    if dpi <= 250.0 {
        8.0
    } else if dpi <= 350.0 {
        12.0
    } else {
        24.0
    }
}

/// One control session (`!` header … PRINT/END/ABORT).
struct CpclSession {
    elements: Vec<LabelElement>,
    units: Units,
    hdpi: f64,
    vdpi: f64,
    dpmm: f64,
    /// Header offset/height in raw header units (a UNITS command right
    /// after the header retroactively applies to them).
    offset_raw: Option<f64>,
    height_raw: Option<f64>,
    header_units_resolved: bool,
    offset_mm: f64,
    height_mm: Option<f64>,
    page_width_mm: Option<f64>,
    mag_w: f64,
    mag_h: f64,
    justify: Justify,
    /// Justification end point, millimetres from the label origin.
    justify_end_mm: f64,
    hri_text: bool,
    rotate_deg: f64,
}

impl CpclSession {
    /// Convert a horizontal unit value to millimetres. Dot units use the
    /// manual's own arithmetic ("on 200 d.p.i. printers 8 dots equal one
    /// millimeter", 12 on 300 d.p.i.) rather than the exact reciprocal, so
    /// a 203-dpi file renders dot-identical at the engine's 8 dpmm.
    fn x_mm(&self, v: f64) -> f64 {
        match self.units {
            Units::Dots => v / header_dpmm(self.hdpi),
            Units::Inches => v * 25.4,
            Units::Centimeters => v * 10.0,
            Units::Millimeters => v,
        }
    }

    /// Convert a vertical unit value to millimetres.
    fn y_mm(&self, v: f64) -> f64 {
        match self.units {
            Units::Dots => v / header_dpmm(self.vdpi),
            Units::Inches => v * 25.4,
            Units::Centimeters => v * 10.0,
            Units::Millimeters => v,
        }
    }

    /// Horizontal position in renderer dots (header offset included).
    fn x(&self, v: f64) -> i32 {
        ((self.x_mm(v) + self.offset_mm) * self.dpmm + 0.5) as i32
    }

    /// Vertical position in renderer dots.
    fn y(&self, v: f64) -> i32 {
        (self.y_mm(v) * self.dpmm + 0.5) as i32
    }

    /// Horizontal size in renderer dots.
    fn w(&self, v: f64) -> i32 {
        (self.x_mm(v) * self.dpmm + 0.5) as i32
    }

    /// Vertical size in renderer dots.
    fn h(&self, v: f64) -> i32 {
        (self.y_mm(v) * self.dpmm + 0.5) as i32
    }

    fn position(&self, x: f64, y: f64) -> LabelPosition {
        LabelPosition {
            x: self.x(x),
            y: self.y(y),
            calculate_from_bottom: false,
            automatic_position: false,
            from_bottom: false,
        }
    }

    /// Apply a late UNITS command to the still-unresolved header fields.
    fn resolve_header_units(&mut self) {
        if self.header_units_resolved {
            return;
        }
        self.header_units_resolved = true;
        if let Some(v) = self.offset_raw {
            self.offset_mm = self.x_mm(v);
        }
        if let Some(v) = self.height_raw {
            let mm = self.y_mm(v);
            if mm > 0.0 {
                self.height_mm = Some(mm);
            }
        }
    }
}

pub struct CpclParser {
    dpmm: i32,
}

impl Default for CpclParser {
    fn default() -> Self {
        Self { dpmm: 8 }
    }
}

impl CpclParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Printer-independent engine resolution: CPCL positions are converted
    /// to millimetres via the header's dpi, then to dots here.
    pub fn with_dpmm(dpmm: i32) -> Self {
        Self { dpmm: dpmm.max(1) }
    }

    /// Parse CPCL data into one [`LabelInfo`] per control session that
    /// produced printable content. `PRINT` and `END` both flush the
    /// session (`END` executes the label without printing it — the
    /// rendered image is the same); `ABORT` discards it.
    pub fn parse(&self, cpcl_data: &[u8]) -> Result<Vec<LabelInfo>, String> {
        let mut lines = CpclLines::new(cpcl_data);
        let mut labels = Vec::new();
        // SETMAG persists across sessions (manual); everything else resets.
        let mut mag = (1.0f64, 1.0f64);

        let mut session: Option<CpclSession> = None;
        let mut line_no = 0usize;

        while let Some(raw) = lines.next_line() {
            line_no += 1;
            let line = raw.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }

            // `!` starts a new session; an unterminated previous one is
            // flushed (lenient, same convention as the TSPL tail flush).
            if line.starts_with('!') {
                if let Some(s) = session.take() {
                    flush(s, &mut labels);
                }
                session = Some(start_session(line, mag, self.dpmm as f64));
                continue;
            }

            let Some(s) = session.as_mut() else {
                continue; // commands outside a session draw nothing
            };

            match parse_line(s, &mut mag, line, &mut lines) {
                Ok(Flow::Continue) => {}
                Ok(Flow::Flush) => {
                    let s = session.take().unwrap();
                    flush(s, &mut labels);
                }
                Ok(Flow::Discard) => {
                    session.take();
                }
                Err(e) => return Err(format!("CPCL line {line_no}: {e}")),
            }
        }

        if let Some(s) = session {
            flush(s, &mut labels);
        }
        Ok(labels)
    }
}

enum Flow {
    Continue,
    Flush,
    Discard,
}

fn start_session(line: &str, mag: (f64, f64), dpmm: f64) -> CpclSession {
    let mut s = CpclSession {
        elements: Vec::new(),
        units: Units::Dots,
        hdpi: 203.0,
        vdpi: 203.0,
        dpmm,
        offset_raw: None,
        height_raw: None,
        header_units_resolved: false,
        offset_mm: 0.0,
        height_mm: None,
        page_width_mm: None,
        mag_w: mag.0,
        mag_h: mag.1,
        justify: Justify::Left,
        justify_end_mm: 0.0,
        hri_text: false,
        rotate_deg: 0.0,
    };

    // `! {offset} {hdpi} {vdpi} {height} {qty}`. Non-numeric headers
    // (`! UTILITIES`, `! U1 setvar`, `! DF name`) are utility / SGD /
    // stored-format sessions with nothing to render — the session stays
    // empty and consumes until its terminator.
    let fields: Vec<&str> = line[1..].split_whitespace().collect();
    let mut nums = [None; 4];
    for (slot, field) in nums.iter_mut().zip(fields.iter()) {
        *slot = field.parse::<f64>().ok();
    }
    if let [Some(offset), Some(hdpi), Some(vdpi), Some(height)] = nums {
        s.offset_raw = Some(offset);
        s.hdpi = if hdpi > 0.0 { hdpi } else { 203.0 };
        s.vdpi = if vdpi > 0.0 { vdpi } else { s.hdpi };
        s.height_raw = Some(height);
    }
    s
}

fn flush(s: CpclSession, labels: &mut Vec<LabelInfo>) {
    if s.elements.is_empty() {
        return;
    }
    labels.push(LabelInfo {
        mirrored: false,
        print_width: 0,
        inverted: false,
        label_width_mm: s.page_width_mm,
        label_height_mm: s.height_mm,
        elements: s.elements,
    });
}

fn parse_line(
    s: &mut CpclSession,
    mag: &mut (f64, f64),
    line: &str,
    lines: &mut CpclLines,
) -> Result<Flow, String> {
    let (cmd_word, rest) = next_token(line);
    let rest = rest.trim_start();
    let cmd = cmd_word.to_ascii_uppercase();

    // A UNITS command as the session's first command retroactively applies
    // to the header offset/height fields.
    match cmd.as_str() {
        "IN-INCHES" => s.units = Units::Inches,
        "IN-CENTIMETERS" => s.units = Units::Centimeters,
        "IN-MILLIMETERS" => s.units = Units::Millimeters,
        "IN-DOTS" => s.units = Units::Dots,
        _ => {}
    }
    s.resolve_header_units();
    let args: Vec<&str> = rest.split_whitespace().collect();

    match cmd.as_str() {
        "TEXT" | "T" => parse_text(s, rest, FieldOrientation::Normal).map(|_| Flow::Continue),
        // 90° counter-clockwise = 270° clockwise in the IR's terms.
        "VTEXT" | "VT" | "TEXT90" | "T90" => {
            parse_text(s, rest, FieldOrientation::Rotated270).map(|_| Flow::Continue)
        }
        "TEXT180" | "T180" => {
            parse_text(s, rest, FieldOrientation::Rotated180).map(|_| Flow::Continue)
        }
        "TEXT270" | "T270" => {
            parse_text(s, rest, FieldOrientation::Rotated90).map(|_| Flow::Continue)
        }
        "SCALE-TEXT" | "ST" => {
            parse_scale_text(s, rest, FieldOrientation::Normal).map(|_| Flow::Continue)
        }
        "VSCALE-TEXT" | "VST" => {
            parse_scale_text(s, rest, FieldOrientation::Rotated270).map(|_| Flow::Continue)
        }
        "MULTILINE" | "ML" => parse_multiline(s, rest, lines),
        "BARCODE" | "B" | "VBARCODE" | "VB" => {
            parse_barcode(s, rest, cmd == "VBARCODE" || cmd == "VB", lines)
        }
        "BARCODE-TEXT" | "BT" => {
            s.hri_text = !rest.trim().eq_ignore_ascii_case("off");
            Ok(Flow::Continue)
        }
        "BOX" => parse_box(s, &args).map(|_| Flow::Continue),
        "LINE" | "L" => parse_line_graphic(s, &args, false).map(|_| Flow::Continue),
        "INVERSE-LINE" | "IL" => parse_line_graphic(s, &args, true).map(|_| Flow::Continue),
        "EG" => parse_expanded_graphics(s, rest, lines, true).map(|_| Flow::Continue),
        "CG" => parse_expanded_graphics(s, rest, lines, false).map(|_| Flow::Continue),
        "VEG" | "VCG" => Err("vertical CPCL graphics are not supported".to_string()),
        "PCX" => Err("CPCL PCX graphics are not supported".to_string()),
        "PATTERN" => match args.first().and_then(|v| v.parse::<i32>().ok()) {
            Some(100) | None => Ok(Flow::Continue),
            Some(n) => Err(format!("CPCL fill pattern {n} is not supported")),
        },
        "SETMAG" => {
            let mut w = args
                .first()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(1.0);
            let mut h = args
                .get(1)
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(1.0);
            // 0 0 cancels magnification (manual).
            if w == 0.0 {
                w = 1.0;
            }
            if h == 0.0 {
                h = 1.0;
            }
            *mag = (w, h);
            s.mag_w = w;
            s.mag_h = h;
            Ok(Flow::Continue)
        }
        "ROTATE" | "R" => {
            let deg = args
                .first()
                .and_then(|v| v.parse::<f64>().ok())
                .unwrap_or(0.0);
            if deg.rem_euclid(90.0) != 0.0 {
                return Err(format!(
                    "CPCL ROTATE angles beyond 90° steps are not supported ({deg})"
                ));
            }
            s.rotate_deg = deg;
            Ok(Flow::Continue)
        }
        "CENTER" => {
            s.justify = Justify::Center;
            s.justify_end_mm = justify_end(s, args.first().copied());
            Ok(Flow::Continue)
        }
        "RIGHT" => {
            s.justify = Justify::Right;
            s.justify_end_mm = justify_end(s, args.first().copied());
            Ok(Flow::Continue)
        }
        "LEFT" => {
            s.justify = Justify::Left;
            Ok(Flow::Continue)
        }
        "PAGE-WIDTH" | "PW" => {
            if let Some(v) = args.first().and_then(|v| v.parse::<f64>().ok()) {
                let mm = s.x_mm(v);
                if mm > 0.0 {
                    s.page_width_mm = Some(mm);
                }
            }
            Ok(Flow::Continue)
        }
        "PRINT" | "END" => Ok(Flow::Flush),
        "ABORT" => Ok(Flow::Discard),
        // Explicitly unsupported constructs — error rather than misrender.
        "CONCAT" | "VCONCAT" | "ENDCONCAT" | "FG" | "SCALE-TO-FIT" | "STF" | "VSTF"
        | "DEFINE-FORMAT" | "DF" | "USE-FORMAT" | "UF" => {
            Err(format!("unsupported CPCL command {cmd}"))
        }
        // Device behaviour / session control with no drawable effect
        // (CONTRAST, TONE, JOURNAL, PACE, FORM, PREFEED, POSTFEED, WAIT,
        // BEEP, CUT, PARTIAL-CUT, ENCODING, COUNT, …), plus unknown
        // commands — ignored like the TSPL frontend does.
        _ => Ok(Flow::Continue),
    }
}

/// Justification end point in millimetres: the explicit [end] parameter or
/// the page width (PAGE-WIDTH when declared, else the 576-dot mobile
/// printhead — the common QL/MZ 3-inch class).
fn justify_end(s: &CpclSession, arg: Option<&str>) -> f64 {
    if let Some(v) = arg.and_then(|v| v.parse::<f64>().ok()) {
        return s.x_mm(v);
    }
    s.page_width_mm.unwrap_or_else(|| s.x_mm(576.0))
}

/// `TEXT {font} {size} {x} {y} {data…}` — data is the rest of the line.
fn parse_text(
    s: &mut CpclSession,
    rest: &str,
    orientation: FieldOrientation,
) -> Result<(), String> {
    let (font, r) = next_token(rest);
    let (size, r) = next_token(r);
    let (x_tok, r) = next_token(r);
    let (y_tok, r) = next_token(r);
    let data = r.trim();
    if data.is_empty() {
        return Ok(());
    }
    let x = parse_num(x_tok)?;
    let y = parse_num(y_tok)?;

    let font_id = super::fonts::resident_font(font, size.parse::<usize>().unwrap_or(0));
    let text = TextField {
        reverse_print: ReversePrint::default(),
        font: FontInfo {
            name: font.to_string(),
            face: font_id.face,
            advance_mult: 1.0,
            dedot_zero: false,
            width: s.w(font_id.width * s.mag_w) as f64,
            height: s.h(font_id.height * s.mag_h) as f64,
            orientation,
        },
        position: s.position(x, y),
        alignment: FieldAlignment::Left,
        text: data.to_string(),
        block: None,
    };
    let text = apply_justify(s, text);
    s.elements.push(LabelElement::Text(text));
    Ok(())
}

/// `SCALE-TEXT {name} {w-pt} {h-pt} {x} {y} {data}` — 72 points per inch.
fn parse_scale_text(
    s: &mut CpclSession,
    rest: &str,
    orientation: FieldOrientation,
) -> Result<(), String> {
    let (_name, r) = next_token(rest);
    let (w_pt, r) = next_token(r);
    let (h_pt, r) = next_token(r);
    let (x_tok, r) = next_token(r);
    let (y_tok, r) = next_token(r);
    let data = r.trim();
    if data.is_empty() {
        return Ok(());
    }
    let w_pt = parse_num(w_pt)?;
    let h_pt = parse_num(h_pt)?;
    let x = parse_num(x_tok)?;
    let y = parse_num(y_tok)?;

    let mm_per_pt = 25.4 / 72.0;
    let text = TextField {
        reverse_print: ReversePrint::default(),
        font: FontInfo {
            name: "CPCL-SCALABLE".to_string(),
            face: FontFace::CondensedBold,
            advance_mult: 1.0,
            dedot_zero: false,
            width: (w_pt * mm_per_pt * s.dpmm + 0.5) as i32 as f64,
            height: (h_pt * mm_per_pt * s.dpmm + 0.5) as i32 as f64,
            orientation: rotated_by(s.rotate_deg, orientation),
        },
        position: s.position(x, y),
        alignment: FieldAlignment::Left,
        text: data.to_string(),
        block: None,
    };
    let text = apply_justify(s, text);
    s.elements.push(LabelElement::Text(text));
    Ok(())
}

fn rotated_by(deg: f64, base: FieldOrientation) -> FieldOrientation {
    let total = base.get_degrees() + deg;
    match total.rem_euclid(360.0) as i32 {
        90 => FieldOrientation::Rotated90,
        180 => FieldOrientation::Rotated180,
        270 => FieldOrientation::Rotated270,
        _ => FieldOrientation::Normal,
    }
}

/// Rewrite a text field per the session justification state. CENTER/RIGHT
/// place the field inside a one-line block spanning [x .. end] — the
/// printer justifies the whole field against the end point regardless of
/// its x, which the block alignment reproduces.
fn apply_justify(s: &CpclSession, mut text: TextField) -> TextField {
    match s.justify {
        Justify::Left => text,
        Justify::Center | Justify::Right => {
            let end = s.justify_end_mm * s.dpmm;
            let start = text.position.x as f64;
            text.block = Some(FieldBlock {
                max_width: (end - start).max(0.0) as i32,
                max_lines: 1,
                line_spacing: 0,
                alignment: if s.justify == Justify::Center {
                    TextAlignment::Center
                } else {
                    TextAlignment::Right
                },
                hanging_indent: 0,
            });
            text
        }
    }
}

/// `ML {height}` / `{TEXT…} {font} {size} {x} {y}` / data lines / ENDML.
fn parse_multiline(s: &mut CpclSession, rest: &str, lines: &mut CpclLines) -> Result<Flow, String> {
    let (pitch_tok, _) = next_token(rest);
    let pitch = parse_num(pitch_tok)?;

    let Some(header) = lines.next_line().map(|l| l.trim().to_string()) else {
        return Err("CPCL MULTILINE missing its text command".to_string());
    };
    let (text_cmd, header_rest) = next_token(&header);
    let orientation = match text_cmd.to_ascii_uppercase().as_str() {
        "VTEXT" | "VT" | "TEXT90" | "T90" => FieldOrientation::Rotated270,
        "TEXT180" | "T180" => FieldOrientation::Rotated180,
        "TEXT270" | "T270" => FieldOrientation::Rotated90,
        _ => FieldOrientation::Normal,
    };

    let mut data_lines: Vec<String> = Vec::new();
    while let Some(raw) = lines.next_line() {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("ENDML") || trimmed.eq_ignore_ascii_case("ENDMULTILINE") {
            break;
        }
        if !trimmed.is_empty() {
            data_lines.push(trimmed.to_string());
        }
    }
    let Some(first) = data_lines.first() else {
        return Ok(Flow::Continue);
    };

    // Render the first row through the normal TEXT path, then re-emit the
    // element at y + k*pitch for the remaining rows.
    let mut row_cmd = String::with_capacity(header_rest.len() + first.len() + 1);
    row_cmd.push_str(header_rest.trim_start());
    row_cmd.push(' ');
    row_cmd.push_str(first);
    parse_text(s, &row_cmd, orientation)?;
    let Some(LabelElement::Text(template)) = s.elements.last().cloned() else {
        return Ok(Flow::Continue);
    };
    let pitch_dots = (s.y_mm(pitch) * s.dpmm).round() as i32;
    for (k, data) in data_lines.iter().enumerate().skip(1) {
        let mut t = template.clone();
        t.text = data.clone();
        t.position.y += pitch_dots * k as i32;
        s.elements.push(LabelElement::Text(t));
    }
    Ok(Flow::Continue)
}

/// BARCODE dispatch: the two-dimensional types (`PDF-417`, `QR`,
/// `MAXICODE`, `AZTEC`) take following data lines until an END tag; the
/// rest are single-line linear symbologies.
fn parse_barcode(
    s: &mut CpclSession,
    rest: &str,
    vertical: bool,
    lines: &mut CpclLines,
) -> Result<Flow, String> {
    let (code, _) = next_token(rest);
    match code.to_ascii_uppercase().as_str() {
        "PDF-417" | "PDF417" => parse_2d_pdf417(s, rest, vertical, lines),
        "QR" => parse_2d_qr(s, rest, vertical, lines),
        "MAXICODE" => parse_2d_maxicode(s, rest, lines),
        "AZTEC" => parse_2d_aztec(s, rest, vertical, lines),
        _ => parse_linear_barcode(s, rest, vertical).map(|_| Flow::Continue),
    }
}

/// `KEY value` option pairs from the fields after `{x} {y}`.
fn option_pairs<'a>(fields: impl Iterator<Item = &'a str>) -> Vec<(String, f64)> {
    let mut pairs = Vec::new();
    let mut pending: Option<String> = None;
    for token in fields {
        if let Some(key) = pending.take() {
            if let Ok(v) = token.parse::<f64>() {
                pairs.push((key, v));
            }
        } else {
            pending = Some(token.to_ascii_uppercase());
        }
    }
    pairs
}

fn parse_2d_pdf417(
    s: &mut CpclSession,
    rest: &str,
    vertical: bool,
    lines: &mut CpclLines,
) -> Result<Flow, String> {
    let mut it = rest.split_whitespace();
    let _code = it.next();
    let x = it.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let y = it.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let pairs = option_pairs(it);

    let mut barcode = BarcodePdf417 {
        orientation: orientation_for(vertical),
        row_height: 6,
        security: 1,
        columns: 3,
        rows: 0,
        truncate: false,
        module_width: 2,
        by_height: 0,
    };
    for (key, v) in pairs {
        match key.as_str() {
            "XD" => barcode.module_width = v.max(1.0) as i32,
            "YD" => barcode.row_height = v.max(1.0) as i32,
            "C" => barcode.columns = v.max(0.0) as i32,
            "S" => barcode.security = v.clamp(0.0, 8.0) as i32,
            _ => {}
        }
    }

    let data = collect_until(lines, &["ENDPDF"])?;
    if data.is_empty() {
        return Ok(Flow::Continue);
    }
    s.elements
        .push(LabelElement::BarcodePdf417(BarcodePdf417WithData {
            reverse_print: ReversePrint::default(),
            barcode,
            position: s.position(x, y),
            data,
        }));
    Ok(Flow::Continue)
}

fn parse_2d_qr(
    s: &mut CpclSession,
    rest: &str,
    vertical: bool,
    lines: &mut CpclLines,
) -> Result<Flow, String> {
    let mut it = rest.split_whitespace();
    let _code = it.next();
    let x = it.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let y = it.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let pairs = option_pairs(it);

    let mut magnification = 6i32;
    for (key, v) in pairs {
        if key == "U" {
            magnification = v.max(1.0) as i32;
        }
        // `M` selects QR model 1/2; the encoder emits model-2 symbols
        // either way (model 1 is legacy and physically near-identical).
    }

    let data = collect_until(lines, &["ENDQR"])?;
    if data.is_empty() {
        return Ok(Flow::Continue);
    }
    let (payload, ec) = super::qr_payload::resolve_cpcl_qr_payload(&data)?;
    s.elements.push(LabelElement::BarcodeQr(BarcodeQrWithData {
        reverse_print: ReversePrint::default(),
        barcode: BarcodeQr {
            magnification,
            orientation: orientation_for(vertical),
        },
        height: 0,
        position: s.position(x, y),
        data: payload,
        error_correction: ec,
    }));
    Ok(Flow::Continue)
}

fn parse_2d_maxicode(
    s: &mut CpclSession,
    rest: &str,
    lines: &mut CpclLines,
) -> Result<Flow, String> {
    let mut it = rest.split_whitespace();
    let _code = it.next();
    let x = it.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let y = it.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);

    let mut postal = String::new();
    let mut country = "840".to_string();
    let mut class = "001".to_string();
    let mut secondary = String::new();
    while let Some(raw) = lines.next_line() {
        let line = raw.trim();
        if line.eq_ignore_ascii_case("ENDMAXICODE") {
            break;
        }
        let (tag, value) = next_token(line);
        let value = value.trim();
        match tag.to_ascii_uppercase().as_str() {
            "POST" => postal = value.to_string(),
            "CC" => country = value.to_string(),
            "SC" => class = value.to_string(),
            "MSG" => secondary = value.to_string(),
            "UPS5" if value.trim() == "1" => {
                return Err(
                    "CPCL MAXICODE UPS5 structured carrier messages are not supported".to_string(),
                )
            }
            _ => {}
        }
    }

    // The IR's MaxiCode payload: class(3) + country(3) + postal followed
    // by the [)>RS01GS structured-message header; the postal code pads
    // right with zeros to 9 digits (mode-2 primary).
    let mut padded_postal = postal;
    while padded_postal.len() < 9 {
        padded_postal.push('0');
    }
    let payload = if secondary.is_empty() {
        format!("{class:0>3}{country:0>3}{padded_postal}[)>\x1e\x1d")
    } else {
        format!("{class:0>3}{country:0>3}{padded_postal}[)>\x1e\x1d{secondary}")
    };
    s.elements.push(LabelElement::Maxicode(MaxicodeWithData {
        reverse_print: ReversePrint::default(),
        code: Maxicode { mode: 2 },
        position: s.position(x, y),
        data: payload,
    }));
    Ok(Flow::Continue)
}

fn parse_2d_aztec(
    s: &mut CpclSession,
    rest: &str,
    vertical: bool,
    lines: &mut CpclLines,
) -> Result<Flow, String> {
    let mut it = rest.split_whitespace();
    let _code = it.next();
    let x = it.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let y = it.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let pairs = option_pairs(it);

    let mut barcode = BarcodeAztec {
        orientation: orientation_for(vertical),
        magnification: 6,
        size: 0,
    };
    for (key, v) in pairs {
        match key.as_str() {
            "XD" => barcode.magnification = v.max(1.0) as i32,
            "EC" => barcode.size = v.clamp(0.0, 99.0) as i32,
            _ => {}
        }
    }

    let data = collect_until(lines, &["ENDAZTEC"])?;
    if data.is_empty() {
        return Ok(Flow::Continue);
    }
    s.elements
        .push(LabelElement::BarcodeAztec(BarcodeAztecWithData {
            reverse_print: ReversePrint::default(),
            barcode,
            position: s.position(x, y),
            data,
        }));
    Ok(Flow::Continue)
}

fn orientation_for(vertical: bool) -> FieldOrientation {
    if vertical {
        FieldOrientation::Rotated270
    } else {
        FieldOrientation::Normal
    }
}

/// `BARCODE {type} {width} {ratio} {height} {x} {y} {data}`.
fn parse_linear_barcode(s: &mut CpclSession, rest: &str, vertical: bool) -> Result<(), String> {
    let (code, r) = next_token(rest);
    let (w_tok, r) = next_token(r);
    let (ratio_tok, r) = next_token(r);
    let (h_tok, r) = next_token(r);
    let (x_tok, r) = next_token(r);
    let (y_tok, r) = next_token(r);
    let data = r.trim();
    if data.is_empty() {
        return Ok(());
    }
    let code = code.to_ascii_uppercase();
    let narrow = parse_num(w_tok)?;
    let ratio_code = ratio_tok.trim().parse::<i32>().unwrap_or(0);
    let height = parse_num(h_tok)?;
    let x = parse_num(x_tok)?;
    let y = parse_num(y_tok)?;

    let orientation = orientation_for(vertical);
    let line = s.hri_text;
    let height_dots = s.h(height).max(1);
    let width_dots = s.w(narrow).max(1);
    let width_ratio = cpcl_ratio(ratio_code);
    let position = s.position(x, y);
    let centered = FieldAlignment::Center;

    let element = match code.as_str() {
        "128" => Barcode128WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode128 {
                orientation,
                height: height_dots,
                line,
                line_above: false,
                check_digit: false,
                mode: BarcodeMode::Automatic,
            },
            width: width_dots,
            position,
            line_alignment: centered,
            data: data.to_string(),
        }
        .into(),
        "UCCEAN128" => Barcode128WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode128 {
                orientation,
                height: height_dots,
                line,
                line_above: false,
                check_digit: false,
                mode: BarcodeMode::Ucc,
            },
            width: width_dots,
            position,
            line_alignment: centered,
            data: data.to_string(),
        }
        .into(),
        "39" | "F39" => Barcode39WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode39 {
                orientation,
                height: height_dots,
                line,
                line_above: false,
                check_digit: false,
            },
            width: width_dots,
            width_ratio,
            position,
            line_alignment: centered,
            data: data.to_string(),
        }
        .into(),
        "39C" | "F39C" => Barcode39WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode39 {
                orientation,
                height: height_dots,
                line,
                line_above: false,
                check_digit: true,
            },
            width: width_dots,
            width_ratio,
            position,
            line_alignment: centered,
            data: data.to_string(),
        }
        .into(),
        "UPCA" => BarcodeUcaWithData {
            reverse_print: ReversePrint::default(),
            barcode: BarcodeUca {
                orientation,
                height: height_dots,
                line,
                line_above: false,
            },
            width: width_dots,
            position,
            data: data.to_string(),
        }
        .into(),
        "UPCE" => BarcodeUcpeWithData {
            reverse_print: ReversePrint::default(),
            barcode: BarcodeUcpe {
                orientation,
                height: height_dots,
                line,
                line_above: false,
                check_digit: false,
            },
            width: width_dots,
            position,
            data: data.to_string(),
        }
        .into(),
        "EAN13" => BarcodeEan13WithData {
            reverse_print: ReversePrint::default(),
            barcode: BarcodeEan13 {
                orientation,
                height: height_dots,
                line,
                line_above: false,
            },
            width: width_dots,
            position,
            data: data.to_string(),
        }
        .into(),
        "EAN8" => BarcodeEan8WithData {
            reverse_print: ReversePrint::default(),
            barcode: BarcodeEan8 {
                orientation,
                height: height_dots,
                line,
                line_above: false,
            },
            width: width_dots,
            position,
            data: data.to_string(),
        }
        .into(),
        // I2OF5G (German Post) prints plain interleaved 2 of 5.
        "I2OF5" | "I2OF5G" => Barcode2of5WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode2of5 {
                orientation,
                height: height_dots,
                line,
                line_above: false,
                check_digit: false,
            },
            width: width_dots,
            width_ratio,
            position,
            line_alignment: centered,
            data: data.to_string(),
        }
        .into(),
        "I2OF5C" => Barcode2of5WithData {
            reverse_print: ReversePrint::default(),
            barcode: Barcode2of5 {
                orientation,
                height: height_dots,
                line,
                line_above: false,
                check_digit: true,
            },
            width: width_dots,
            width_ratio,
            position,
            line_alignment: centered,
            data: data.to_string(),
        }
        .into(),
        other => return Err(format!("unsupported CPCL barcode type {other}")),
    };
    s.elements.push(element);
    Ok(())
}

impl From<Barcode128WithData> for LabelElement {
    fn from(v: Barcode128WithData) -> Self {
        LabelElement::Barcode128(v)
    }
}
impl From<Barcode39WithData> for LabelElement {
    fn from(v: Barcode39WithData) -> Self {
        LabelElement::Barcode39(v)
    }
}
impl From<BarcodeUcaWithData> for LabelElement {
    fn from(v: BarcodeUcaWithData) -> Self {
        LabelElement::BarcodeUca(v)
    }
}
impl From<BarcodeUcpeWithData> for LabelElement {
    fn from(v: BarcodeUcpeWithData) -> Self {
        LabelElement::BarcodeUcpe(v)
    }
}
impl From<BarcodeEan13WithData> for LabelElement {
    fn from(v: BarcodeEan13WithData) -> Self {
        LabelElement::BarcodeEan13(v)
    }
}
impl From<BarcodeEan8WithData> for LabelElement {
    fn from(v: BarcodeEan8WithData) -> Self {
        LabelElement::BarcodeEan8(v)
    }
}
impl From<Barcode2of5WithData> for LabelElement {
    fn from(v: Barcode2of5WithData) -> Self {
        LabelElement::Barcode2of5(v)
    }
}

/// CPCL ratio codes: 0-4 → 1.5..3.5 : 1, 20-30 → 2.0..3.0 : 1.
fn cpcl_ratio(code: i32) -> f64 {
    match code {
        0 => 1.5,
        1 => 2.0,
        2 => 2.5,
        3 => 3.0,
        4 => 3.5,
        n if (20..=30).contains(&n) => n as f64 / 10.0,
        _ => 2.0,
    }
}

/// `BOX {x0} {y0} {x1} {y1} {thickness}`.
fn parse_box(s: &mut CpclSession, args: &[&str]) -> Result<(), String> {
    if args.len() < 5 {
        return Err(format!(
            "CPCL BOX command requires 5 parameters, got {}",
            args.len()
        ));
    }
    let x0 = parse_num(args[0])?;
    let y0 = parse_num(args[1])?;
    let x1 = parse_num(args[2])?;
    let y1 = parse_num(args[3])?;
    let thickness = parse_num(args[4])?;
    s.elements.push(LabelElement::GraphicBox(GraphicBox {
        reverse_print: ReversePrint::default(),
        position: s.position(x0.min(x1), y0.min(y1)),
        width: s.w((x1 - x0).abs()).max(1),
        height: s.h((y1 - y0).abs()).max(1),
        border_thickness: s.w(thickness).max(1),
        corner_rounding: 0,
        line_color: LineColor::Black,
    }));
    Ok(())
}

/// `LINE`/`INVERSE-LINE {x0} {y0} {x1} {y1} {thickness}`. Axis-aligned
/// lines become filled boxes; diagonals use the diagonal element. The
/// inverse variant inverts what earlier objects drew under the line's
/// rectangle (the IR applies InvertRegion in stream order).
fn parse_line_graphic(s: &mut CpclSession, args: &[&str], inverse: bool) -> Result<(), String> {
    let name = if inverse { "INVERSE-LINE" } else { "LINE" };
    if args.len() < 5 {
        return Err(format!(
            "CPCL {name} command requires 5 parameters, got {}",
            args.len()
        ));
    }
    let x0 = parse_num(args[0])?;
    let y0 = parse_num(args[1])?;
    let x1 = parse_num(args[2])?;
    let y1 = parse_num(args[3])?;
    let thickness = parse_num(args[4])?;

    let left = s.x(x0.min(x1));
    let top = s.y(y0.min(y1));
    let width = s.w((x1 - x0).abs()).max(1);
    let height = s.h((y1 - y0).abs()).max(1);
    let thickness_dots = s.w(thickness).max(1);

    if inverse {
        let (rw, rh) = if y0 == y1 {
            (width, thickness_dots)
        } else if x0 == x1 {
            (thickness_dots, height)
        } else {
            (width.max(thickness_dots), height.max(thickness_dots))
        };
        s.elements.push(LabelElement::InvertRegion(InvertRegion {
            position: LabelPosition {
                x: left,
                y: top,
                calculate_from_bottom: false,
                automatic_position: false,
                from_bottom: false,
            },
            width: rw,
            height: rh,
        }));
        return Ok(());
    }

    let element = if y0 == y1 {
        LabelElement::GraphicBox(filled_box(left, top, width, thickness_dots))
    } else if x0 == x1 {
        LabelElement::GraphicBox(filled_box(left, top, thickness_dots, height))
    } else {
        LabelElement::DiagonalLine(GraphicDiagonalLine {
            reverse_print: ReversePrint::default(),
            position: LabelPosition {
                x: left,
                y: top,
                calculate_from_bottom: false,
                automatic_position: false,
                from_bottom: false,
            },
            width,
            height,
            border_thickness: thickness_dots,
            line_color: LineColor::Black,
            top_to_bottom: (x1 > x0) == (y1 > y0),
        })
    };
    s.elements.push(element);
    Ok(())
}

fn filled_box(x: i32, y: i32, width: i32, height: i32) -> GraphicBox {
    GraphicBox {
        reverse_print: ReversePrint::default(),
        position: LabelPosition {
            x,
            y,
            calculate_from_bottom: false,
            automatic_position: false,
            from_bottom: false,
        },
        width: width.max(1),
        height: height.max(1),
        border_thickness: width.min(height).max(1),
        corner_rounding: 0,
        line_color: LineColor::Black,
    }
}

/// `EG {byte-width} {dot-height} {x} {y} {hex…}` (expanded) or `CG` with
/// the same shape in binary. The payload starts on the command line right
/// after the fourth parameter's separator space and is counted, not
/// line-delimited: hex digits / raw bytes run across line breaks until
/// the declared size is met. Set bits print black (the manual's F0…/0F…
/// checkerboard example), matching the ^GF polarity.
fn parse_expanded_graphics(
    s: &mut CpclSession,
    rest: &str,
    lines: &mut CpclLines,
    hex: bool,
) -> Result<(), String> {
    let name = if hex { "EG" } else { "CG" };
    let mut remainder = rest.trim_start();
    let mut params = [0.0f64; 4];
    for slot in params.iter_mut() {
        let (tok, r) = next_token(remainder);
        *slot = parse_num(tok)
            .map_err(|_| format!("CPCL {name} command requires 4 numeric parameters"))?;
        remainder = r;
    }
    let row_bytes = params[0].max(1.0) as usize;
    let height = params[1].max(1.0) as usize;
    let x = params[2];
    let y = params[3];
    let total_bytes = row_bytes * height;

    // The payload begins right after the single separator space; anything
    // beyond it (including further spaces in CG binary data) is payload.
    let mut data: Vec<u8> = Vec::with_capacity(total_bytes);
    if let Some(after_sep) = remainder.strip_prefix(' ') {
        data.extend_from_slice(after_sep.as_bytes());
    }

    let data = if hex {
        let need = total_bytes * 2;
        let mut hex_digits: Vec<u8> = data
            .iter()
            .copied()
            .filter(|b| b.is_ascii_hexdigit())
            .collect();
        hex_digits.truncate(need);
        while hex_digits.len() < need {
            let Some(raw) = lines.next_line() else {
                break;
            };
            for b in raw.bytes() {
                if b.is_ascii_hexdigit() {
                    hex_digits.push(b);
                    if hex_digits.len() == need {
                        break;
                    }
                }
            }
        }
        if hex_digits.len() < need {
            return Err("CPCL EG graphics data shorter than declared".to_string());
        }
        hex_digits
            .chunks(2)
            .map(|pair| {
                let hi = (pair[0] as char).to_digit(16).unwrap_or(0) as u8;
                let lo = pair.get(1).copied().unwrap_or(b'0') as char;
                (hi << 4) | lo.to_digit(16).unwrap_or(0) as u8
            })
            .collect()
    } else {
        while data.len() < total_bytes {
            let Some(raw) = lines.next_line() else {
                break;
            };
            data.extend_from_slice(raw.as_bytes());
        }
        data.truncate(total_bytes);
        if data.len() < total_bytes {
            return Err("CPCL CG graphics data shorter than declared".to_string());
        }
        data
    };

    s.elements.push(LabelElement::GraphicField(GraphicField {
        reverse_print: ReversePrint::default(),
        position: s.position(x, y),
        format: if hex {
            GraphicFieldFormat::Hex
        } else {
            GraphicFieldFormat::Raw
        },
        data_bytes: total_bytes as i32,
        total_bytes: total_bytes as i32,
        row_bytes: row_bytes as i32,
        data,
        magnification_x: 1,
        magnification_y: 1,
        mode: GraphicFieldMode::Overwrite,
        zero_bits_black: false,
    }));
    Ok(())
}

/// Collect 2-D data lines until one of the END tags. Lines join with `\n`.
fn collect_until(lines: &mut CpclLines, end_tags: &[&str]) -> Result<String, String> {
    let mut out = Vec::new();
    while let Some(raw) = lines.next_line() {
        let trimmed = raw.trim();
        if end_tags.iter().any(|tag| trimmed.eq_ignore_ascii_case(tag)) {
            break;
        }
        out.push(raw.trim_end().to_string());
    }
    Ok(out.join("\n"))
}

/// Split off the first whitespace-delimited token and the remainder.
fn next_token(rest: &str) -> (&str, &str) {
    let rest = rest.trim_start();
    match rest.find(char::is_whitespace) {
        Some(end) => (&rest[..end], &rest[end..]),
        None => (rest, ""),
    }
}

fn parse_num(token: &str) -> Result<f64, String> {
    token
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("invalid numeric parameter {token:?}"))
}

// ── line cursor ─────────────────────────────────────────────────────

pub(super) struct CpclLines<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> CpclLines<'a> {
    pub(super) fn new(data: &'a [u8]) -> Self {
        CpclLines { data, pos: 0 }
    }

    /// Next physical line without its terminator (CR, LF, or CRLF).
    pub(super) fn next_line(&mut self) -> Option<String> {
        if self.pos >= self.data.len() {
            return None;
        }
        let start = self.pos;
        let mut end = self.pos;
        while end < self.data.len() && self.data[end] != b'\n' && self.data[end] != b'\r' {
            end += 1;
        }
        let line = String::from_utf8_lossy(&self.data[start..end]).to_string();
        self.pos = end;
        if self.pos < self.data.len() && self.data[self.pos] == b'\r' {
            self.pos += 1;
        }
        if self.pos < self.data.len() && self.data[self.pos] == b'\n' {
            self.pos += 1;
        }
        Some(line)
    }
}
