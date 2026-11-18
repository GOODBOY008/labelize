//! Glyph layout for supplied typefaces, fallback faces and complex scripts.
use super::text_layout::ResidentMetrics;
use super::text_layout::{
    align_directed, directed_buffer, expand_buffer_to_ink, update_directed_position, LayoutFont,
    PlacedGlyph, TextBuffer, TextLayout,
};
use super::*;
use crate::elements::font::FontResources;
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

struct Run {
    face: usize,
    start: usize,
    // Logical byte offset and visual cluster index for each glyph.
    glyphs: Vec<(usize, usize, PlacedGlyph)>,
}
struct Cluster {
    start: f32,
    whitespace: bool,
}

struct Line {
    runs: Vec<Run>,
    clusters: Vec<Cluster>,
    width: f32,
}
impl Line {
    /// Distribute unused width across the gaps between words in visual order.
    /// Painted glyphs and pen positions share the same offsets, including
    /// when several shaping or fallback runs contribute to a line.
    fn justify(&mut self, width: f32, spacing: f32) {
        let mut gaps = 0usize;
        let mut seen_word = false;
        let mut in_gap = false;
        let gaps_before: Vec<_> = self
            .clusters
            .iter()
            .map(|c| {
                if c.whitespace {
                    in_gap = seen_word;
                } else {
                    gaps += usize::from(in_gap);
                    seen_word = true;
                    in_gap = false;
                }
                gaps
            })
            .collect();
        if gaps == 0 {
            return;
        }
        // The pen includes the trailing inter-character gap. Align the end
        // of the last word with the block edge.
        let extra = (width + spacing - self.width) / gaps as f32;
        for (c, n) in self.clusters.iter_mut().zip(&gaps_before) {
            c.start += *n as f32 * extra;
        }
        for run in &mut self.runs {
            for (_, cluster, glyph) in &mut run.glyphs {
                glyph.glyph.position.x += gaps_before[*cluster] as f32 * extra;
            }
        }
        self.width = width + spacing;
    }
}
struct Faces<'a> {
    shapers: Vec<rustybuzz::Face<'a>>,
    fonts: Vec<FontRef<'a>>,
    scales: Vec<PxScale>,
    resident: Vec<Option<ResidentMetrics>>,
    resources: &'a FontResources,
    spacing: f32,
}

impl Faces<'_> {
    fn choose(&self, cluster: &str) -> usize {
        self.fonts
            .iter()
            .enumerate()
            .find_map(|(index, font)| {
                cluster
                    .chars()
                    .all(|c| {
                        let id = self.resident[index].map_or_else(
                            || font.glyph_id(c),
                            |m| effective_glyph_id(font, m.letter(c), m.subst.f0),
                        );
                        c.is_control() || c == '\u{200d}' || id.0 != 0
                    })
                    .then_some(index)
            })
            .unwrap_or(0)
    }
    fn line(&self, text: &str) -> Line {
        let mut line = Line {
            runs: Vec::new(),
            clusters: Vec::new(),
            width: 0.0,
        };
        if text.is_empty() {
            return line;
        }
        let properties = &self.resources.properties;
        let bidi = unicode_bidi::BidiInfo::new(text, None);
        let ranges = if properties.bidirectional {
            let paragraph = &bidi.paragraphs[0];
            bidi.visual_runs(paragraph, paragraph.range.clone()).1
        } else {
            std::iter::once(0..text.len()).collect()
        };
        for range in ranges {
            let rtl = properties.bidirectional && bidi.levels[range.start].is_rtl();
            let mut groups: Vec<(usize, Script, std::ops::Range<usize>)> = Vec::new();
            for (i, cluster) in text[range.clone()].grapheme_indices(true) {
                let font = self.choose(cluster);
                let script = cluster
                    .chars()
                    .map(|c| c.script())
                    .find(|s| !matches!(s, Script::Common | Script::Inherited))
                    .unwrap_or(Script::Common);
                let from = range.start + i;
                if let Some((_, previous, span)) = groups.last_mut().filter(|(index, prior, _)| {
                    *index == font
                        && (script == Script::Common
                            || *prior == Script::Common
                            || *prior == script)
                }) {
                    if *previous == Script::Common {
                        *previous = script
                    }
                    span.end = from + cluster.len();
                } else {
                    groups.push((font, script, from..from + cluster.len()));
                }
            }
            if rtl {
                groups.reverse()
            }
            for (face, _, span) in groups {
                let value = &text[span.clone()];
                let font = &self.fonts[face];
                let scale = self.scales[face];
                let scaled = font.as_scaled(scale);
                let mut run = Run {
                    face,
                    start: span.start,
                    glyphs: Vec::new(),
                };
                let mut last_cluster = None;
                let mut emit =
                    |start: usize, gid: u16, dx: f32, dy: f32, advance: f32, actual: &str| {
                        if last_cluster != Some(start) {
                            if !line.clusters.is_empty() {
                                line.width += self.spacing;
                            }
                            line.clusters.push(Cluster {
                                start: line.width,
                                whitespace: actual.chars().all(char::is_whitespace),
                            });
                            last_cluster = Some(start);
                        }
                        let ch = actual.chars().next().unwrap_or(' ');
                        let mut glyph = ab_glyph::GlyphId(gid).with_scale_and_position(
                            scale,
                            ab_glyph::point(line.width + dx, scaled.ascent() - dy),
                        );
                        let resident = self.resident[face];
                        let advance = resident
                            .map_or(advance, |m| m.calibrate(font, &mut glyph, ch, advance));
                        if glyph.id.0 != 0 || properties.show_missing {
                            run.glyphs.push((
                                start,
                                line.clusters.len() - 1,
                                PlacedGlyph {
                                    character: ch,
                                    extracted: Some(actual.to_owned()),
                                    glyph,
                                    x: 0,
                                    y: 0,
                                    color: Rgba([0, 0, 0, 255]),
                                    dedot: resident
                                        .is_some_and(|m| m.subst.dedot_zero && ch == '0'),
                                    line_start: false,
                                    logical_line: 0,
                                },
                            ));
                        }
                        line.width += advance;
                    };
                if properties.shape || properties.open_type {
                    if let Some(hb) = self.shapers.get(face) {
                        let mut buffer = rustybuzz::UnicodeBuffer::new();
                        if self.resident[face].is_some_and(|m| m.face.uppercases_input()) {
                            buffer.push_str(&value.to_ascii_uppercase());
                        } else {
                            buffer.push_str(value);
                        }
                        buffer.set_direction(if rtl {
                            rustybuzz::Direction::RightToLeft
                        } else {
                            rustybuzz::Direction::LeftToRight
                        });
                        buffer.guess_segment_properties();
                        let features = if properties.open_type {
                            Vec::new()
                        } else {
                            ["kern=0", "liga=0", "clig=0"]
                                .into_iter()
                                .filter_map(|s| s.parse().ok())
                                .collect()
                        };
                        let shaped = rustybuzz::shape(hb, &features, buffer);
                        let mut boundaries: Vec<_> = shaped
                            .glyph_infos()
                            .iter()
                            .map(|i| i.cluster as usize)
                            .collect();
                        boundaries.push(value.len());
                        boundaries.sort_unstable();
                        boundaries.dedup();
                        let mut previous = None;
                        for (g, p) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                            let i = g.cluster as usize;
                            let j = boundaries.partition_point(|c| *c <= i);
                            let end = boundaries.get(j).copied().unwrap_or(value.len());
                            let actual = if previous == Some(i) {
                                ""
                            } else {
                                &value[i..end]
                            };
                            emit(
                                i,
                                g.glyph_id as u16,
                                p.x_offset as f32 * scaled.h_scale_factor(),
                                p.y_offset as f32 * scaled.v_scale_factor(),
                                p.x_advance as f32 * scaled.h_scale_factor(),
                                actual,
                            );
                            previous = Some(i);
                        }
                    }
                } else {
                    let mut clusters: Vec<_> = value.grapheme_indices(true).collect();
                    if rtl {
                        clusters.reverse();
                    }
                    for (start, cluster) in clusters {
                        for (i, ch) in cluster.char_indices() {
                            let id =
                                font.glyph_id(self.resident[face].map_or(ch, |m| m.letter(ch)));
                            emit(
                                start,
                                id.0,
                                0.0,
                                0.0,
                                scaled.h_advance(id),
                                &cluster[i..i + ch.len_utf8()],
                            );
                        }
                    }
                }
                run.glyphs.sort_by_key(|g| g.0);
                line.runs.push(run);
            }
        }
        if !line.clusters.is_empty() {
            line.width += self.spacing;
        }
        line.runs.sort_by_key(|r| r.start);
        line
    }

    fn wrap<'a>(&self, text: &'a str, limit: f32, break_words: bool) -> Vec<&'a str> {
        let mut lines = Vec::new();
        for para in text.split('\n') {
            let mut start = 0;
            let mut width = 0.0;
            for (at, word) in para.split_word_bound_indices() {
                let measured = self.line(word).width;
                if width + measured > limit && at > start && !word.trim().is_empty() {
                    lines.push(para[start..at].trim_end());
                    start = at;
                    width = 0.0;
                }
                if measured > limit && break_words && !word.trim().is_empty() {
                    for (offset, g) in word.grapheme_indices(true) {
                        let w = self.line(g).width;
                        if width + w > limit && at + offset > start {
                            lines.push(&para[start..at + offset]);
                            start = at + offset;
                            width = 0.0;
                        }
                        width += w;
                    }
                } else {
                    width += measured;
                }
            }
            lines.push(para[start..].trim_end());
        }
        lines
    }
}

pub(super) fn layout(
    text: &TextField,
    state: &mut DrawerState,
    resources: &FontResources,
) -> Result<Vec<TextLayout>, String> {
    let mut assets = vec![resources
        .primary
        .clone()
        .map(LayoutFont::Downloaded)
        .unwrap_or(LayoutFont::Resident(text.font.face))];
    assets.extend(
        resources
            .fallbacks
            .iter()
            .cloned()
            .map(LayoutFont::Downloaded),
    );
    if !resources.fallbacks.is_empty() {
        assets.push(LayoutFont::Resident(FontFace::CondensedBold));
    }
    let fonts = assets
        .iter()
        .map(|a| FontRef::try_from_slice(a.ttf_data()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let h = text.font.height.max(1.0) as f32;
    let mut w = text.font.width.max(1.0) as f32;
    if let Some((cw, ch)) = resources.native_cell {
        w *= f32::from(ch) / f32::from(cw)
    }
    let resident: Vec<_> = assets
        .iter()
        .zip(&fonts)
        .map(|(asset, font)| match asset {
            LayoutFont::Resident(face) => Some(ResidentMetrics::new(text, *face, font)),
            LayoutFont::Downloaded(_) => None,
        })
        .collect();
    let scales = fonts
        .iter()
        .zip(&resident)
        .map(|(font, calibration)| {
            calibration.map_or_else(
                || {
                    let factor = font.height_unscaled()
                        / font.units_per_em().unwrap_or(font.height_unscaled());
                    PxScale {
                        x: w * factor,
                        y: h * factor,
                    }
                },
                |m| m.scale,
            )
        })
        .collect();
    let faces = Faces {
        shapers: if resources.properties.shape || resources.properties.open_type {
            assets
                .iter()
                .map(|a| {
                    rustybuzz::Face::from_slice(a.ttf_data(), 0)
                        .ok_or_else(|| "Could not load font shaping tables".to_string())
                })
                .collect::<Result<_, _>>()?
        } else {
            Vec::new()
        },
        fonts,
        scales,
        resident,
        resources,
        spacing: text.flow.character_spacing as f32,
    };
    let block = text.block.as_ref();
    let direction = text.flow.direction;
    let directed_field = block.is_none() && direction != TextDirection::Horizontal;
    // A reverse block wraps from the end of the field. Shape the original
    // graphemes within each selected line so marks stay with their base.
    let reversed = (block.is_some() && direction == TextDirection::Reverse)
        .then(|| text.text.graphemes(true).rev().collect::<String>());
    let source = reversed.as_deref().unwrap_or(&text.text);
    let lines = if let Some(b) = block {
        faces.wrap(source, b.max_width.max(1) as f32, b.wrap_long_words)
    } else {
        source.split('\n').collect()
    };
    let mut lines: Vec<_> = lines
        .into_iter()
        .take(
            block
                .filter(|b| !b.overwrite_last_line)
                .map_or(usize::MAX, |b| b.max_lines.max(1) as usize),
        )
        .map(|line| {
            if reversed.is_some() {
                faces.line(&line.graphemes(true).rev().collect::<String>())
            } else {
                faces.line(line)
            }
        })
        .collect();
    let width = block.map_or_else(
        || lines.iter().map(|l| l.width).fold(0.0, f32::max),
        |b| b.max_width.max(1) as f32,
    );
    let pitch = faces.resident[0].map_or(h, |m| m.scale.y * m.line_height_factor)
        + block.map_or(0, |b| b.line_spacing) as f32;
    let slots = block
        .map_or(lines.len(), |b| {
            lines.len().min(b.max_lines.max(1) as usize)
        })
        .max(1);
    let height = h + (slots - 1) as f32 * pitch;
    let ascent = f64::from(faces.resident[0].map_or_else(
        || faces.fonts[0].as_scaled(faces.scales[0]).ascent(),
        |m| m.ascent,
    ));
    let rotated = text.font.orientation != FieldOrientation::Normal;
    let cells = lines
        .iter()
        .map(|line| line.clusters.len())
        .max()
        .unwrap_or(0);
    let (x, y) = if directed_field {
        let (x, y) = state.get_text_position(text);
        (
            x,
            if text.position.calculate_from_bottom {
                y - ascent
            } else {
                y
            },
        )
    } else {
        get_text_top_left_pos(text, width.into(), h.into(), ascent, state, pitch.into())
    };
    let buffer = if directed_field {
        directed_buffer(text, state, cells, ascent)
    } else {
        rotated.then(|| TextBuffer {
            width: width.ceil().max(1.0) as u32,
            height: height.ceil().max(1.0) as u32,
            orientation: text.font.orientation,
            x: x as i32,
            y: y as i32,
        })
    };
    if directed_field {
        update_directed_position(text, state, width.into(), cells);
    } else {
        state.update_automatic_text_position(text, f64::from(width));
    }
    let mut out = Vec::new();
    for (n, line) in lines.iter_mut().enumerate() {
        if n + 1 < slots
            && block.is_some_and(|b| {
                b.alignment == crate::elements::text_alignment::TextAlignment::Justified
            })
        {
            line.justify(width, faces.spacing);
        }
        let left = match block.map(|b| b.alignment) {
            Some(crate::elements::text_alignment::TextAlignment::Right) => width - line.width,
            Some(crate::elements::text_alignment::TextAlignment::Center) => {
                (width - line.width) / 2.0
            }
            _ => 0.0,
        };
        for run in &mut line.runs {
            let mut glyphs = Vec::new();
            for (_, cluster, mut g) in run.glyphs.drain(..) {
                let start = line.clusters[cluster].start;
                match direction {
                    TextDirection::Horizontal => {}
                    TextDirection::Reverse => {
                        let end = line
                            .clusters
                            .get(cluster + 1)
                            .map_or(line.width, |cluster| cluster.start);
                        let origin = if directed_field {
                            text.flow.origin_width as f32
                        } else {
                            0.0
                        };
                        g.glyph.position.x += origin - end - start;
                    }
                    TextDirection::Vertical => {
                        g.glyph.position.x -= start;
                        if directed_field {
                            g.glyph.position.y += cluster as f32 * h;
                        }
                    }
                }
                g.x = if rotated { 0 } else { x as i32 };
                let shift = faces.resident[run.face].map_or(0.0, |m| m.bitmap_y_shift);
                g.y = ((if rotated { 0.0 } else { y }) + shift) as i32;
                g.glyph.position.x += left;
                g.glyph.position.y += n.min(slots - 1) as f32 * pitch;
                g.line_start = glyphs.is_empty();
                g.logical_line = n;
                glyphs.push(g);
            }
            if !glyphs.is_empty() {
                let mut layout = TextLayout {
                    face: assets[run.face].clone(),
                    glyphs,
                    buffer: buffer.clone(),
                };
                expand_buffer_to_ink(&mut layout, &faces.fonts[run.face])?;
                if directed_field {
                    align_directed(text, &mut layout);
                }
                if layout.buffer.as_ref().is_some_and(|b| {
                    b.width > 32000
                        || b.height > 32000
                        || u64::from(b.width) * u64::from(b.height) > 16 * 1024 * 1024
                }) {
                    return Err("Text field is too large".into());
                }
                out.push(layout);
            }
        }
    }
    Ok(out)
}
