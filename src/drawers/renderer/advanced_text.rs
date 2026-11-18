//! Glyph layout for supplied typefaces, fallback faces and complex scripts.
use super::text_layout::{LayoutFont, PlacedGlyph, TextBuffer, TextLayout};
use super::*;
use crate::elements::font::FontResources;
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

struct Run {
    face: usize,
    start: usize,
    glyphs: Vec<(usize, PlacedGlyph)>,
}
struct Line {
    runs: Vec<Run>,
    width: f32,
}
struct Faces<'a> {
    shapers: Vec<rustybuzz::Face<'a>>,
    fonts: Vec<FontRef<'a>>,
    scales: Vec<PxScale>,
    resources: &'a FontResources,
    spacing: f32,
}

impl Faces<'_> {
    fn choose(&self, cluster: &str) -> usize {
        self.fonts
            .iter()
            .position(|f| {
                cluster
                    .chars()
                    .all(|c| c.is_control() || c == '\u{200d}' || f.glyph_id(c).0 != 0)
            })
            .unwrap_or(0)
    }
    fn line(&self, text: &str) -> Line {
        let mut line = Line {
            runs: Vec::new(),
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
                let mut emit =
                    |start: usize, gid: u16, dx: f32, dy: f32, advance: f32, actual: &str| {
                        let ch = actual.chars().next().unwrap_or(' ');
                        if gid != 0 || properties.show_missing {
                            run.glyphs.push((
                                start,
                                PlacedGlyph {
                                    character: ch,
                                    extracted: Some(actual.to_owned()),
                                    glyph: ab_glyph::GlyphId(gid).with_scale_and_position(
                                        scale,
                                        ab_glyph::point(line.width + dx, scaled.ascent() - dy),
                                    ),
                                    x: 0,
                                    y: 0,
                                    color: Rgba([0, 0, 0, 255]),
                                    dedot: false,
                                    line_start: false,
                                    logical_line: 0,
                                },
                            ));
                        }
                        line.width += advance + self.spacing;
                    };
                if properties.shape || properties.open_type {
                    if let Some(hb) = self.shapers.get(face) {
                        let mut buffer = rustybuzz::UnicodeBuffer::new();
                        buffer.push_str(value);
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
                    let mut chars: Vec<_> = value.char_indices().collect();
                    if rtl {
                        chars.reverse()
                    }
                    for (i, c) in chars {
                        let id = font.glyph_id(c);
                        emit(
                            i,
                            id.0,
                            0.0,
                            0.0,
                            scaled.h_advance(id),
                            &value[i..i + c.len_utf8()],
                        );
                    }
                }
                run.glyphs.sort_by_key(|g| g.0);
                line.runs.push(run);
            }
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
    let scales = fonts
        .iter()
        .map(|font| {
            let factor =
                font.height_unscaled() / font.units_per_em().unwrap_or(font.height_unscaled());
            PxScale {
                x: w * factor,
                y: h * factor,
            }
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
        resources,
        spacing: text.flow.character_spacing as f32,
    };
    let block = text.block.as_ref();
    let lines = if let Some(b) = block {
        faces.wrap(&text.text, b.max_width.max(1) as f32, b.wrap_long_words)
    } else {
        text.text.split('\n').collect()
    };
    let mut lines: Vec<_> = lines
        .iter()
        .take(
            block
                .filter(|b| !b.overwrite_last_line)
                .map_or(usize::MAX, |b| b.max_lines.max(1) as usize),
        )
        .map(|line| faces.line(line))
        .collect();
    let width = block.map_or_else(
        || lines.iter().map(|l| l.width).fold(0.0, f32::max),
        |b| b.max_width.max(1) as f32,
    );
    let pitch = h + block.map_or(0, |b| b.line_spacing) as f32;
    let slots = block
        .map_or(lines.len(), |b| {
            lines.len().min(b.max_lines.max(1) as usize)
        })
        .max(1);
    let height = h + (slots - 1) as f32 * pitch;
    let (x, y) = get_text_top_left_pos(
        text,
        width as f64,
        h as f64,
        faces.fonts[0].as_scaled(faces.scales[0]).ascent() as f64,
        state,
        pitch as f64,
    );
    state.update_automatic_text_position(text, f64::from(width));
    let rotated = text.font.orientation != FieldOrientation::Normal;
    let bw = width.ceil().max(1.0) as u32;
    let bh = height.ceil().max(1.0) as u32;
    if rotated && (bw > 32000 || bh > 32000 || u64::from(bw) * u64::from(bh) > 16 * 1024 * 1024) {
        return Err("Text field is too large".into());
    }
    let buffer = rotated.then_some(TextBuffer {
        width: bw,
        height: bh,
        orientation: text.font.orientation,
        x: x as i32,
        y: y as i32,
    });
    let mut out = Vec::new();
    for (n, line) in lines.iter_mut().enumerate() {
        let left = match block.map(|b| b.alignment) {
            Some(crate::elements::text_alignment::TextAlignment::Right) => width - line.width,
            Some(crate::elements::text_alignment::TextAlignment::Center) => {
                (width - line.width) / 2.0
            }
            _ => 0.0,
        };
        for run in &mut line.runs {
            let mut glyphs = Vec::new();
            for (_, mut g) in run.glyphs.drain(..) {
                g.x = if rotated { 0 } else { x as i32 };
                g.y = if rotated { 0 } else { y as i32 };
                g.glyph.position.x += left;
                g.glyph.position.y += n.min(slots - 1) as f32 * pitch;
                g.line_start = glyphs.is_empty();
                g.logical_line = n;
                glyphs.push(g);
            }
            if !glyphs.is_empty() {
                out.push(TextLayout {
                    face: assets[run.face].clone(),
                    glyphs,
                    buffer: buffer.clone(),
                });
            }
        }
    }
    Ok(out)
}
