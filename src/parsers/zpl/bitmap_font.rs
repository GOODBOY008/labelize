//! Convert downloaded bitmap glyphs to rectangular TrueType contours so the
//! same metrics, font linking, raster and PDF text paths can use them.
use std::collections::BTreeMap;
use write_fonts::{types::Tag, FontBuilder};

type Point = (i16, i16);
struct Glyph {
    advance: u16,
    contours: Vec<[Point; 4]>,
}

fn u16s(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_be_bytes()).collect()
}
fn set16(bytes: &mut [u8], at: usize, v: u16) {
    bytes[at..at + 2].copy_from_slice(&v.to_be_bytes())
}
fn set32(bytes: &mut [u8], at: usize, v: u32) {
    bytes[at..at + 4].copy_from_slice(&v.to_be_bytes())
}

pub(super) struct BitmapTypeface {
    pub name: String,
    pub font: Vec<u8>,
    pub cell: (u16, u16),
}

pub(super) fn decode(data: &[u8], delimiter: u8) -> Result<BitmapTypeface, String> {
    let mut fields = data.splitn(9, |b| *b == delimiter);
    let string = |b: &[u8]| String::from_utf8_lossy(b).trim().to_string();
    let num = |b: Option<&[u8]>| {
        b.and_then(|s| string(s).parse::<u16>().ok())
            .ok_or("Invalid bitmap font header")
    };
    let name = string(fields.next().ok_or("Missing font name")?);
    let _orientation = fields.next();
    let height = num(fields.next())?;
    let width = num(fields.next())?;
    let base = num(fields.next())?;
    let space = num(fields.next())?;
    let count = num(fields.next())?;
    let _copyright = fields.next();
    if !(1..=32000).contains(&width)
        || !(1..=32000).contains(&height)
        || !(1..=256).contains(&count)
    {
        return Err("Invalid bitmap font dimensions or count".into());
    }
    let scaled = |v: i32| {
        i16::try_from((i64::from(v) * 1024) / i64::from(height))
            .map_err(|_| "Bitmap font coordinates out of range")
    };
    let mut glyphs = BTreeMap::new();
    let mut pixels = 0usize;
    for entry in fields
        .next()
        .ok_or("Missing bitmap glyphs")?
        .split(|b| *b == b'#')
        .skip(1)
    {
        let p: Vec<_> = entry.splitn(7, |b| *b == b'.').collect();
        if p.len() != 7 {
            return Err("Incomplete bitmap glyph record".into());
        }
        let code =
            u16::from_str_radix(&string(p[0]), 16).map_err(|_| "Invalid bitmap character code")?;
        let h = num(Some(p[1]))? as usize;
        let w = num(Some(p[2]))? as usize;
        let x: i32 = string(p[3])
            .parse()
            .map_err(|_| "Invalid bitmap x offset")?;
        let y: i32 = string(p[4])
            .parse()
            .map_err(|_| "Invalid bitmap y offset")?;
        if x.unsigned_abs() > 32000 || y.unsigned_abs() > 32000 {
            return Err("Invalid bitmap glyph offset".into());
        }
        let advance = scaled(i32::from(num(Some(p[5]))?))? as u16;
        let stride = w.div_ceil(8);
        pixels = pixels
            .checked_add(w * h)
            .ok_or("Bitmap font size overflow")?;
        if pixels > 16 * 1024 * 1024 {
            return Err("Bitmap font exceeds 16 Mi pixels".into());
        }
        let bits = super::objects::ascii_bytes(p[6], stride * h)?;
        if bits.len() != stride * h {
            return Err("Invalid bitmap font payload size".into());
        }
        let mut contours = Vec::new();
        for row in 0..h {
            let mut col = 0;
            while col < w {
                let ink = |i: usize| bits[row * stride + i / 8] & (128 >> (i % 8)) != 0;
                if !ink(col) {
                    col += 1;
                    continue;
                }
                let start = col;
                while col < w && ink(col) {
                    col += 1
                }
                let l = scaled(x + start as i32)?;
                let r = scaled(x + col as i32)?;
                let t = scaled(y - row as i32)?;
                let b = scaled(y - row as i32 - 1)?;
                if contours.len() >= 16383 {
                    return Err("Too many bitmap glyph contours".into());
                }
                contours.push([(l, t), (r, t), (r, b), (l, b)]);
            }
        }
        if code == 0xffff || glyphs.insert(code, Glyph { advance, contours }).is_some() {
            return Err("Invalid or duplicate bitmap character".into());
        }
    }
    if glyphs.len() != count as usize {
        return Err("Bitmap glyph count mismatch".into());
    }
    let space = scaled(i32::from(space))? as u16;
    glyphs.entry(32).or_insert(Glyph {
        advance: space,
        contours: Vec::new(),
    });
    let font = typeface(&name, &glyphs, scaled(i32::from(base))?, space)?;
    Ok(BitmapTypeface {
        name,
        font,
        cell: (width, height),
    })
}

fn typeface(
    name: &str,
    glyphs: &BTreeMap<u16, Glyph>,
    ascent: i16,
    space: u16,
) -> Result<Vec<u8>, String> {
    let mut font = FontBuilder::new();
    let mut add = |tag: &[u8; 4], data: Vec<u8>| {
        font.add_raw(Tag::new(tag), data);
    };
    let mut glyf = Vec::new();
    let mut loca = vec![0; 4];
    let mut hmtx = u16s(&[space, 0]);
    let mut bounds = [0i16; 4];
    for g in glyphs.values() {
        loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
        let mut bb = [i16::MAX, i16::MAX, i16::MIN, i16::MIN];
        for &(x, y) in g.contours.iter().flatten() {
            bb = [bb[0].min(x), bb[1].min(y), bb[2].max(x), bb[3].max(y)];
        }
        if g.contours.is_empty() {
            bb = [0; 4]
        } else {
            glyf.extend_from_slice(&u16s(&[
                g.contours.len() as u16,
                bb[0] as u16,
                bb[1] as u16,
                bb[2] as u16,
                bb[3] as u16,
            ]));
            for i in 1..=g.contours.len() {
                glyf.extend_from_slice(&((4 * i - 1) as u16).to_be_bytes());
            }
            glyf.extend_from_slice(&[0, 0]);
            glyf.extend(std::iter::repeat_n(1, g.contours.len() * 4));
            for axis in 0..2 {
                let mut prev = 0i16;
                for &(x, y) in g.contours.iter().flatten() {
                    let v = if axis == 0 { x } else { y };
                    glyf.extend_from_slice(
                        &v.checked_sub(prev)
                            .ok_or("Bitmap font glyph delta out of range")?
                            .to_be_bytes(),
                    );
                    prev = v;
                }
            }
            if glyf.len() % 2 != 0 {
                glyf.push(0);
            }
        }
        bounds = [
            bounds[0].min(bb[0]),
            bounds[1].min(bb[1]),
            bounds[2].max(bb[2]),
            bounds[3].max(bb[3]),
        ];
        hmtx.extend_from_slice(&u16s(&[g.advance, bb[0] as u16]));
    }
    loca.extend_from_slice(&(glyf.len() as u32).to_be_bytes());
    let n = glyphs.len() as u16 + 1;
    let mut maxp = vec![0; 32];
    set32(&mut maxp, 0, 65536);
    set16(&mut maxp, 4, n);
    let contours = glyphs.values().map(|g| g.contours.len()).max().unwrap_or(0) as u16;
    set16(&mut maxp, 6, contours * 4);
    set16(&mut maxp, 8, contours);
    set16(&mut maxp, 14, 1);
    let mut head = vec![0; 54];
    set32(&mut head, 0, 65536);
    set32(&mut head, 4, 65536);
    set32(&mut head, 12, 0x5f0f3cf5);
    set16(&mut head, 18, 1024);
    for (i, v) in bounds.iter().enumerate() {
        set16(&mut head, 36 + i * 2, *v as u16);
    }
    set16(&mut head, 46, 8);
    set16(&mut head, 48, 2);
    set16(&mut head, 50, 1);
    let mut hhea = vec![0; 36];
    set32(&mut hhea, 0, 65536);
    set16(&mut hhea, 4, ascent as u16);
    set16(&mut hhea, 6, (ascent - 1024) as u16);
    set16(
        &mut hhea,
        10,
        glyphs.values().map(|g| g.advance).max().unwrap_or(space),
    );
    set16(&mut hhea, 18, 1);
    set16(&mut hhea, 34, n);
    let mut cmap = vec![0, 0, 0, 1, 0, 3, 0, 1, 0, 0, 0, 12];
    let codes: Vec<_> = glyphs.keys().copied().chain([65535]).collect();
    let segs = codes.len() as u16;
    let power = segs.ilog2() as u16;
    cmap.extend(u16s(&[
        4,
        16 + 8 * segs,
        0,
        2 * segs,
        2 << power,
        power,
        2 * segs - (2 << power),
    ]));
    cmap.extend(u16s(&codes));
    cmap.extend([0, 0]);
    cmap.extend(u16s(&codes));
    for (i, &code) in codes.iter().enumerate() {
        cmap.extend_from_slice(
            &(if code == 65535 {
                1
            } else {
                (i as u16 + 1).wrapping_sub(code)
            })
            .to_be_bytes(),
        );
    }
    cmap.resize(cmap.len() + 2 * codes.len(), 0);
    let mut post = vec![0; 32];
    set32(&mut post, 0, 0x0003_0000);
    let mut os = vec![0; 78];
    set16(&mut os, 2, space);
    set16(&mut os, 4, 400);
    set16(&mut os, 6, 5);
    os[58..62].copy_from_slice(b"Lblz");
    set16(&mut os, 62, 64);
    set16(&mut os, 64, codes[0]);
    set16(&mut os, 66, codes[codes.len() - 2]);
    set16(&mut os, 68, ascent as u16);
    set16(&mut os, 70, (ascent - 1024) as u16);
    set16(&mut os, 74, ascent.max(0) as u16);
    set16(&mut os, 76, (1024 - ascent).max(0) as u16);
    let clean: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(60)
        .collect();
    let family = if clean.is_empty() { "Bitmap" } else { &clean };
    let mut names = u16s(&[0, 4, 54]);
    let mut strings = Vec::new();
    for (id, txt) in [(1, family), (2, "Regular"), (4, family), (6, family)] {
        let b: Vec<_> = txt.encode_utf16().flat_map(u16::to_be_bytes).collect();
        names.extend(u16s(&[
            3,
            1,
            0x409,
            id,
            b.len() as u16,
            strings.len() as u16,
        ]));
        strings.extend(b);
    }
    names.extend(strings);
    for (tag, table) in [
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", hmtx),
        (b"maxp", maxp),
        (b"cmap", cmap),
        (b"glyf", glyf),
        (b"loca", loca),
        (b"name", names),
        (b"post", post),
        (b"OS/2", os),
    ] {
        add(tag, table)
    }
    Ok(font.build())
}
