use super::{canvas_dots, BitmapRows, DrawerOptions, LabelElement, LabelInfo, Renderer, RgbaImage};
use crate::elements::raster_label::RasterLabel;
use std::{collections::HashMap, rc::Rc};

/// Keep at most one maximum-size RGBA canvas worth of packed source bitmaps
/// per render. A cache miss is charged after its dependencies are prepared.
const MAX_CACHE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Hash, Eq, PartialEq)]
struct BitmapKey {
    // The root label owns all of these immutable sources for the whole pass.
    source: *const LabelInfo,
    width_mm: u64,
    height_mm: u64,
    dpmm: i32,
    allow_inversion: bool,
}

#[derive(Default)]
pub(super) struct SavedImageCache {
    entries: HashMap<BitmapKey, Rc<SavedBitmap>>,
    bytes: usize,
}

pub(super) struct SavedBitmap {
    data: Vec<u8>,
    row_bytes: usize,
}

impl SavedBitmap {
    fn rows(&self) -> BitmapRows<'_> {
        BitmapRows {
            data: &self.data,
            row_bytes: self.row_bytes,
            zero_bits_black: false,
        }
    }

    pub(super) fn draw_at_origin(&self, target: &mut RgbaImage, offset: i64) {
        self.rows()
            .draw(target, &Default::default(), (1, 1), offset, false);
    }

    pub(super) fn draw(
        &self,
        target: &mut RgbaImage,
        placement: &RasterLabel,
        x_offset: i64,
        xor: bool,
    ) {
        self.rows().draw(
            target,
            &placement.position,
            (placement.magnification_x, placement.magnification_y),
            x_offset,
            xor,
        );
    }
}

impl SavedImageCache {
    pub(super) fn bitmap(
        &mut self,
        saved: &RasterLabel,
        options: &DrawerOptions,
    ) -> Result<Rc<SavedBitmap>, String> {
        let mut stages = Vec::new();
        let mut frame = saved;
        let mut parent = options.clone();
        let mut prior = loop {
            let label = frame.label();
            let dpmm = frame.source_dpmm.max(1);
            let ratio = f64::from(parent.dpmm) / f64::from(dpmm);
            let options = DrawerOptions {
                label_width_mm: label
                    .label_width_mm
                    .unwrap_or(parent.label_width_mm * ratio),
                label_height_mm: label
                    .label_height_mm
                    .unwrap_or(parent.label_height_mm * ratio),
                dpmm,
                antialias: false,
                ..parent
            };
            let key = BitmapKey {
                source: label,
                width_mm: options.label_width_mm.to_bits(),
                height_mm: options.label_height_mm.to_bits(),
                dpmm,
                allow_inversion: options.enable_inverted_labels,
            };
            if let Some(bitmap) = self.entries.get(&key) {
                break Some(Rc::clone(bitmap));
            }
            parent = options.clone();
            stages.push((frame, options, key));
            match frame.background() {
                Some(previous) => frame = previous,
                None => break None,
            }
        };

        // Each retained format starts with the previous physical bitmap. Keep only
        // the preceding result while folding the history, so a long job adds neither
        // recursion nor a cache entry per intermediate page.
        for (frame, options, key) in stages.into_iter().rev() {
            let label = frame.label();
            let width = canvas_dots(options.label_width_mm, options.dpmm);
            let height = canvas_dots(options.label_height_mm, options.dpmm);
            if !(1.0..=32000.0).contains(&width)
                || !(1.0..=32000.0).contains(&height)
                || width * height > 16_777_216.0
            {
                return Err("Saved image exceeds the 16 Mi pixel or 32000-dot limit".into());
            }
            // Render children before allocating this source canvas. Alternative
            // placements of the same child reuse its pixels even in nested saves.
            for element in &label.elements {
                if let LabelElement::RasterLabel(child) = element {
                    self.bitmap(child, &options)?;
                }
            }
            let row_bytes = (width as usize).div_ceil(8);
            let byte_count = row_bytes * height as usize;
            if byte_count > MAX_CACHE_BYTES - self.bytes {
                return Err("Saved image bitmaps exceed the 64 MiB render cache limit".into());
            }
            let page = Renderer.draw_label_canvas_with_background(
                label,
                &options,
                self,
                prior.as_deref(),
            )?;
            let mut data = vec![0; byte_count];
            // Use the same threshold as monochrome PNG output. Keep both page
            // margins, including the extra dot on the far side after reflection.
            for (row, output) in page.rows().zip(data.chunks_exact_mut(row_bytes)) {
                for (x, pixel) in row.enumerate() {
                    if pixel[0] <= 128 {
                        output[x / 8] |= 0x80 >> (x % 8);
                    }
                }
            }

            let bitmap = Rc::new(SavedBitmap { data, row_bytes });
            if std::ptr::eq(frame, saved) {
                self.entries.insert(key, Rc::clone(&bitmap));
                self.bytes += byte_count;
            }
            prior = Some(bitmap);
        }
        Ok(prior.expect("a saved image was cached or rendered"))
    }

    /// Normal fields are clipped to the print width. Only recalled bitmaps
    /// can carry earlier page pixels into the margins. Composite those strips
    /// separately so ordinary fields and vector text keep their usual clip.
    pub(super) fn margins(
        &mut self,
        label: &LabelInfo,
        options: &DrawerOptions,
        width: u32,
        print_width: u32,
        height: u32,
        background: Option<&SavedBitmap>,
    ) -> Result<Vec<(u32, RgbaImage)>, String> {
        if width == print_width
            || background.is_none()
                && !label
                    .elements
                    .iter()
                    .any(|e| matches!(e, LabelElement::RasterLabel(_)))
        {
            return Ok(Vec::new());
        }
        let left = (width - print_width) / 2;
        let right_start = left + print_width;
        let mut strips: Vec<_> = [(0, left), (right_start, width - right_start)]
            .into_iter()
            .filter(|&(_, w)| w > 0)
            .map(|(x, w)| (x, RgbaImage::new(w, height)))
            .collect();
        if let Some(background) = background {
            for (x, strip) in &mut strips {
                background.draw_at_origin(strip, -i64::from(*x));
            }
        }
        for element in &label.elements {
            if let LabelElement::RasterLabel(saved) = element {
                let bitmap = self.bitmap(saved, options)?;
                let origin = if saved.page_relative { 0 } else { left };
                for (x, strip) in &mut strips {
                    bitmap.draw(
                        strip,
                        saved,
                        i64::from(origin) - i64::from(*x),
                        saved.reverse_print.value,
                    );
                }
            }
        }
        Ok(strips)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_placements_share_pixels_for_the_same_source_and_canvas() {
        let stamp = RasterLabel::new(LabelInfo::default(), 8).unwrap();
        let mut second = stamp.clone();
        second.position.x = 2;
        second.magnification_x = 3;
        let pair = RasterLabel::new(
            LabelInfo {
                elements: vec![
                    LabelElement::RasterLabel(stamp.clone()),
                    LabelElement::RasterLabel(second.clone()),
                ],
                ..Default::default()
            },
            8,
        )
        .unwrap();
        let options = DrawerOptions {
            label_width_mm: 4.0,
            label_height_mm: 3.0,
            ..Default::default()
        };
        let mut cache = SavedImageCache::default();
        cache.bitmap(&pair, &options).unwrap();
        let first = cache.bitmap(&stamp, &options).unwrap();
        assert!(Rc::ptr_eq(
            &first,
            &cache.bitmap(&second, &options).unwrap()
        ));
        assert_eq!(cache.bytes, 2 * 4 * 24);
        let wider = DrawerOptions {
            label_width_mm: 6.0,
            ..options
        };
        let other = cache.bitmap(&stamp, &wider).unwrap();
        assert_eq!(other.row_bytes, 6);
        assert!(!Rc::ptr_eq(&first, &other));
    }

    #[test]
    fn a_full_cache_can_still_place_a_previous_snapshot() {
        let source = RasterLabel::new(LabelInfo::default(), 8).unwrap();
        let other = RasterLabel::new(LabelInfo::default(), 8).unwrap();
        let options = DrawerOptions {
            label_width_mm: 1.0,
            label_height_mm: 0.125,
            ..Default::default()
        };
        let mut cache = SavedImageCache {
            bytes: MAX_CACHE_BYTES - 1,
            ..Default::default()
        };
        let first = cache.bitmap(&source, &options).unwrap();
        assert!(cache.bitmap(&other, &options).is_err());
        assert!(Rc::ptr_eq(
            &first,
            &cache.bitmap(&source, &options).unwrap()
        ));
    }
}
