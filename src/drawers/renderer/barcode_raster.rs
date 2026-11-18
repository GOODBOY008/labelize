//! Resolve logical barcode modules into printer dots for the shared barcode layout.
use crate::barcodes::symbols::{self, EncodedBars};
use crate::elements::barcode_symbol::{BarcodeKind, BarcodeSymbol, PostalCode};
use anyd::output::{BitMatrix, Encoding, LinearPattern};
use image::{Rgba, RgbaImage};

fn scaled_runs(
    bits: impl IntoIterator<Item = bool>,
    module: u32,
    ratio: Option<f64>,
) -> Vec<(bool, u32)> {
    let mut runs = Vec::new();
    for bit in bits {
        match runs.last_mut() {
            Some((dark, width)) if *dark == bit => *width += 1u32,
            _ => runs.push((bit, 1u32)),
        }
    }
    for (_, width) in &mut runs {
        *width = match ratio {
            Some(ratio) if *width > 1 => (f64::from(module) * ratio).round() as u32,
            _ => width.saturating_mul(module),
        };
    }
    runs
}

fn run_width(runs: &[(bool, u32)]) -> Result<u32, String> {
    runs.iter().try_fold(0u32, |sum, (_, n)| {
        sum.checked_add(*n)
            .ok_or_else(|| "Barcode width overflow".into())
    })
}

fn checked_image(width: u32, height: u32) -> Result<RgbaImage, String> {
    if width == 0
        || height == 0
        || width > 32000
        || height > 32000
        || u64::from(width) * u64::from(height) > 16 * 1024 * 1024
    {
        return Err(format!(
            "Barcode dimensions {width} x {height} exceed the supported image size"
        ));
    }
    Ok(RgbaImage::new(width, height))
}

fn raster(
    encoding: Encoding,
    module: u32,
    height: u32,
    ratio: Option<f64>,
    postal: Option<PostalCode>,
    row_height: bool,
) -> Result<RgbaImage, String> {
    let ink = Rgba([0, 0, 0, 255]);
    match encoding {
        Encoding::Linear(row) => {
            let runs = scaled_runs(row.modules, module, ratio);
            let width = run_width(&runs)?;
            let mut img = checked_image(width, height)?;
            let mut x = 0;
            for (dark, n) in runs {
                if dark {
                    for y in 0..height {
                        for xx in x..x + n {
                            img.put_pixel(xx, y, ink)
                        }
                    }
                }
                x += n;
            }
            Ok(img)
        }
        Encoding::Matrix(m) => {
            let bar_pitch = if postal.is_some() {
                (module as f64 * 2.5).round() as u32
            } else {
                0
            };
            let w = if postal.is_some() {
                (m.width() as u32 / 2).saturating_mul(bar_pitch) + module
            } else {
                (m.width() as u32).saturating_mul(module)
            };
            let h = if row_height {
                height.saturating_mul(m.height() as u32)
            } else {
                height
            };
            let mut img = checked_image(w, h)?;
            for y in 0..h {
                let my = if matches!(postal, Some(PostalCode::Planet | PostalCode::Postnet)) {
                    if y * 5 < h * 3 {
                        0
                    } else {
                        1
                    }
                } else {
                    ((u64::from(y) * m.height() as u64) / u64::from(h)) as usize
                };
                for x in 0..w {
                    let mx = if postal.is_some() {
                        if x % bar_pitch >= module {
                            continue;
                        }
                        (x / bar_pitch * 2) as usize
                    } else {
                        (x / module) as usize
                    };
                    if m.get(mx, my) {
                        img.put_pixel(x, y, ink)
                    }
                }
            }
            Ok(img)
        }
    }
}

fn stacked_bars(spec: &BarcodeSymbol, grid: &BitMatrix) -> Result<RgbaImage, String> {
    let module = spec.module;
    let (ratio, left, right, start, stop) = match spec.kind {
        BarcodeKind::Code49 { .. } => (None, 10 * module, module, 0, 0),
        BarcodeKind::Codablock { mode: b'A', .. } => {
            let wide = (f64::from(module) * spec.ratio).round() as u32;
            let start = 7 * module + 3 * wide;
            (Some(spec.ratio), 0, 0, start, start)
        }
        _ => (None, 0, 0, 11 * module, 13 * module),
    };
    let row_runs = |y| scaled_runs((0..grid.width()).map(|x| grid.get(x, y)), module, ratio);
    let width = run_width(&row_runs(0))?;
    let rows = grid.height() as u32;
    let separators = (rows + 1) * module;
    let area = if matches!(
        spec.kind,
        BarcodeKind::Codablock {
            height_is_total: true,
            ..
        }
    ) {
        spec.height.saturating_sub(separators).max(rows)
    } else {
        spec.height.saturating_mul(rows)
    };
    let h = area.saturating_add(separators);
    let mut image = checked_image(left + width + right, h)?;
    let mut fill = |xs: std::ops::Range<u32>, ys: std::ops::Range<u32>| {
        for y in ys {
            for x in xs.clone() {
                image.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
    };
    // Code 49 includes its 10X leading and 1X trailing quiet zones in the
    // outer bars. Codablock carries the start/stop pattern through inner bars.
    fill(0..left + width + right, 0..module);
    fill(0..left + width + right, h - module..h);
    let boundary = |row| (u64::from(row) * u64::from(area) / u64::from(rows)) as u32 + row * module;
    for row in 0..rows {
        let y = boundary(row);
        let next = boundary(row + 1);
        if row > 0 {
            fill(left + start..left + width - stop, y..y + module);
        }
        let mut x = left;
        for (ink, n) in row_runs(row as usize) {
            if ink {
                fill(x..x + n, y..next);
            }
            x += n;
        }
    }
    Ok(image)
}

fn linked39(
    spec: &BarcodeSymbol,
    linear: LinearPattern,
    secondary: BitMatrix,
    flag: [u8; 9],
    micro_width: u32,
    micro_height: u32,
) -> Result<RgbaImage, String> {
    let primary = raster(
        Encoding::Linear(linear),
        spec.module,
        spec.height,
        Some(spec.ratio),
        None,
        false,
    )?;
    let secondary = raster(
        Encoding::Matrix(secondary),
        micro_width,
        micro_height,
        None,
        None,
        true,
    )?;
    let extension = (f64::from(spec.height) * (0.214 - (spec.ratio - 2.0) * 0.0165)) as u32;
    let top = extension.max(secondary.height() + spec.module);
    let gap = 10 * spec.module;
    let wide = (f64::from(spec.module) * spec.ratio).round() as u32;
    let flag_width = 6 * spec.module + 3 * wide;
    let width = (primary.width() + gap + flag_width).max(spec.module + secondary.width());
    let mut image = checked_image(width, top + spec.height + extension)?;
    image::imageops::overlay(&mut image, &primary, 0, i64::from(top));
    image::imageops::overlay(
        &mut image,
        &secondary,
        i64::from(spec.module),
        i64::from(top - secondary.height() - spec.module),
    );
    let mut x = primary.width() + gap;
    for (i, bar) in flag.into_iter().enumerate() {
        let w = if bar == 0 { spec.module } else { wide };
        if i % 2 == 0 {
            for y in top - extension..image.height() {
                for xx in x..x + w {
                    image.put_pixel(xx, y, Rgba([0, 0, 0, 255]));
                }
            }
        }
        x += w;
    }
    Ok(image)
}

fn composite(
    spec: &BarcodeSymbol,
    bits: &bwipp::BitMatrix,
    boundary: usize,
    variant: u8,
    separator: u32,
) -> Result<RgbaImage, String> {
    let magnify = spec.module;
    let h = bits.height();
    let target_linear = if variant >= 11 {
        spec.height
    } else {
        (h - boundary) as u32 * magnify
    };
    let top = boundary as u32 * magnify;
    let sep = (separator - 1) * magnify;
    let mut image = checked_image(bits.width() as u32 * magnify, top + sep + target_linear)?;
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        let row = if y < top {
            (y / magnify) as usize
        } else if y < top + sep {
            boundary.saturating_sub(1)
        } else {
            h - 1
        };
        if bits.get((x / magnify) as usize, row) {
            *pixel = Rgba([0, 0, 0, 255]);
        }
    }
    Ok(image)
}

pub(super) fn render(spec: &BarcodeSymbol, data: &[u8]) -> Result<(RgbaImage, String), String> {
    use BarcodeKind::*;
    let (encoded, caption) = symbols::encode(&spec.kind, data, spec.caption_checks)?;
    let image = match (encoded, &spec.kind) {
        (
            EncodedBars::Composite {
                modules,
                linear_start,
            },
            DataBar {
                variant, separator, ..
            },
        ) => composite(spec, &modules, linear_start, *variant, *separator)?,
        (
            EncodedBars::Linked39 {
                linear,
                secondary,
                flag,
            },
            Tlc39 {
                micro_width,
                micro_height,
            },
        ) => linked39(spec, linear, secondary, flag, *micro_width, *micro_height)?,
        (EncodedBars::Single(Encoding::Matrix(grid)), Code49 { .. } | Codablock { .. }) => {
            stacked_bars(spec, &grid)?
        }
        (EncodedBars::Single(bars), kind) => {
            let ratio = match kind {
                Code11 { .. }
                | Industrial2of5
                | Standard2of5
                | Codabar { .. }
                | Logmars
                | Tlc39 { .. } => Some(spec.ratio),
                _ => None,
            };
            let postal = if let Postal(code) = kind {
                Some(*code)
            } else {
                None
            };
            let (height, per_row) = match kind {
                MicroPdf417 { .. } => (spec.height, true),
                DataBar { variant: 1..=6, .. } if matches!(bars, Encoding::Matrix(_)) => {
                    (spec.module, true)
                }
                DataBar {
                    variant: v @ 1..=6, ..
                } => (
                    spec.module
                        * match v {
                            1 => 33,
                            2 => 13,
                            5 => 10,
                            _ => 34,
                        },
                    false,
                ),
                _ => (spec.height, false),
            };
            raster(bars, spec.module, height, ratio, postal, per_row)?
        }
        _ => return Err("Barcode component does not match its layout".into()),
    };
    Ok((image, caption))
}
