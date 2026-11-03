//! Font outlines shared by scene consumers, including the resident zero repair.
use super::*;
use ab_glyph::OutlineCurve;

pub(super) fn path(font: &FontRef, glyph: &text_layout::PlacedGlyph) -> Option<Path> {
    let outline = font.outline(glyph.glyph.id)?;
    let mut contours = Vec::<Vec<OutlineCurve>>::new();
    let mut previous = None;
    for curve in outline.curves {
        let (start, end) = match curve {
            OutlineCurve::Line(a, b) => (a, b),
            OutlineCurve::Quad(a, _, c) => (a, c),
            OutlineCurve::Cubic(a, _, _, d) => (a, d),
        };
        if previous != Some(start) {
            contours.push(Vec::new());
        }
        contours.last_mut()?.push(curve);
        previous = Some(end);
    }
    // The substitute monospace zero has an outer contour, counter, and dot.
    // Keep the existing size-dependent decision: at tiny sizes the dot can
    // merge into the ring and the legacy consumer leaves it intact. When the
    // old coverage test removes an island, remove its smallest contour here.
    let remove_dot = glyph.dedot
        && contours.len() == 3
        && font
            .outline_glyph(glyph.glyph.clone())
            .is_some_and(|g| dedotted_pixels(&g).iter().any(|v| *v));
    let dot = remove_dot
        .then(|| {
            contours
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| contour_area(a).total_cmp(&contour_area(b)))
                .map(|(i, _)| i)
        })
        .flatten();
    let mut builder = PathBuilder::new();
    for (index, contour) in contours.iter().enumerate() {
        if dot == Some(index) {
            continue;
        }
        for (n, curve) in contour.iter().enumerate() {
            let start = match curve {
                OutlineCurve::Line(a, _)
                | OutlineCurve::Quad(a, _, _)
                | OutlineCurve::Cubic(a, _, _, _) => a,
            };
            if n == 0 {
                builder.move_to(start.x, start.y);
            }
            match curve {
                OutlineCurve::Line(_, b) => builder.line_to(b.x, b.y),
                OutlineCurve::Quad(_, b, c) => builder.quad_to(b.x, b.y, c.x, c.y),
                OutlineCurve::Cubic(_, b, c, d) => builder.cubic_to(b.x, b.y, c.x, c.y, d.x, d.y),
            }
        }
        builder.close();
    }
    builder.finish()
}

fn contour_area(curves: &[OutlineCurve]) -> f32 {
    let (mut x0, mut y0, mut x1, mut y1) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    for curve in curves {
        let points: &[ab_glyph::Point] = match curve {
            OutlineCurve::Line(a, b) => &[*a, *b],
            OutlineCurve::Quad(a, b, c) => &[*a, *b, *c],
            OutlineCurve::Cubic(a, b, c, d) => &[*a, *b, *c, *d],
        };
        for p in points {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
    }
    (x1 - x0) * (y1 - y0)
}
