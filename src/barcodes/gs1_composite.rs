use super::symbols::EncodedBars;
use bwipp::{Encoded, Options, Symbology};

// GS1 General Specifications, predefined-length element strings (Table 7-6).
// Other element strings need a group separator before the next AI even when
// their data happens to be a fixed number of digits.
fn predefined_length(prefix: &[u8]) -> Option<usize> {
    match prefix {
        b"00" => Some(20),
        b"01" | b"02" | b"03" | b"41" => Some(16),
        b"04" => Some(18),
        [b'1', b'1'..=b'9'] => Some(8),
        b"20" => Some(4),
        [b'3', b'1'..=b'6'] => Some(10),
        _ => None,
    }
}

fn ai_width(prefix: &[u8]) -> usize {
    match prefix {
        b"23" | b"24" | b"25" | b"40" | b"41" | b"42" | b"71" => 3,
        [b'3', b'1'..=b'6'] | b"39" | b"43" | b"70" | b"72" | b"80" | b"81" | b"82" => 4,
        _ => 2,
    }
}

pub(super) fn reduced(data: &[u8]) -> Result<std::borrow::Cow<'_, [u8]>, String> {
    use std::borrow::Cow;
    if !data.starts_with(b"(") {
        return Ok(Cow::Borrowed(data));
    }
    let mut out = Vec::with_capacity(data.len());
    let mut fields = data[1..].split(|&b| b == b'(').peekable();
    while let Some(field) = fields.next() {
        let split = field
            .iter()
            .position(|&b| b == b')')
            .ok_or("Unclosed GS1 identifier")?;
        let id = &field[..split];
        if !(2..=4).contains(&id.len())
            || !id.iter().all(u8::is_ascii_digit)
            || ai_width(&id[..2]) != id.len()
        {
            return Err("Invalid GS1 application identifier".into());
        }
        let value = &field[split + 1..];
        if value.is_empty()
            || predefined_length(&id[..2]).is_some_and(|n| n != id.len() + value.len())
        {
            return Err("Invalid GS1 element length".into());
        }
        out.extend_from_slice(id);
        out.extend_from_slice(value);
        if predefined_length(&id[..2]).is_none() && fields.peek().is_some() {
            out.push(29);
        }
    }
    Ok(Cow::Owned(out))
}

fn element_string(mut data: &str) -> Result<String, String> {
    if data.starts_with('(') {
        return Ok(data.into());
    }
    let mut result = String::new();
    while !data.is_empty() {
        let prefix = data
            .as_bytes()
            .get(..2)
            .filter(|p| p.iter().all(u8::is_ascii_digit))
            .ok_or("GS1 composite data requires an application identifier")?;
        let length = predefined_length(prefix)
            .unwrap_or_else(|| data.find(char::from(29)).unwrap_or(data.len()));
        let field = data.get(..length).ok_or("Truncated GS1 element")?;
        let n = ai_width(prefix);
        let id = field
            .get(..n)
            .filter(|s| s.bytes().all(|b| b.is_ascii_digit()))
            .ok_or("Incomplete GS1 application identifier")?;
        result.push('(');
        result.push_str(id);
        result.push(')');
        result.push_str(&field[n..]);
        data = data[length..].trim_start_matches(char::from(29));
    }
    Ok(result)
}

pub(super) fn encode(
    variant: u8,
    segments: usize,
    linear: &str,
    secondary: &str,
) -> Result<EncodedBars, String> {
    use Symbology::*;
    let sym = match variant {
        1 => CompositeDatabarOmniCcb,
        2 => CompositeDatabarTruncatedCcb,
        3 => CompositeDatabarStackedCcb,
        4 => CompositeDatabarStackedOmniCcb,
        5 => CompositeDatabarLimitedCcb,
        6 if segments < 22 => CompositeDatabarExpandedStackedCcb,
        6 => CompositeDatabarExpandedCcb,
        7 => CompositeUpcaCcb,
        8 => CompositeUpceCcb,
        9 => CompositeEan13Ccb,
        10 => CompositeEan8Ccb,
        11 => CompositeGs1_128Ccb,
        12 => CompositeGs1_128Ccc,
        _ => return Err("Invalid composite symbology".into()),
    };
    let linear = match variant {
        1..=5 => {
            let digits = linear.strip_prefix("(01)").unwrap_or(linear);
            let digits = if digits.len() == 16 {
                digits.strip_prefix("01").unwrap_or(digits)
            } else {
                digits
            };
            format!("(01){digits:0>13}")
        }
        6 | 11 | 12 => element_string(linear)?,
        _ => linear.into(),
    };
    let options = Options::default().with("segments", segments.to_string());
    let bits = match element_string(secondary) {
        Ok(ai) => {
            let encoded = sym.encode(&format!("{linear}|{ai}"), &options);
            match encoded {
                Ok(Encoded::Matrix(matrix)) => matrix,
                Err(_) if !secondary.starts_with('(') => {
                    super::composite_field::encode(sym, &options, variant, &linear, secondary)?
                }
                Err(e) => return Err(e.to_string()),
                _ => return Err("Expected a GS1 composite matrix".into()),
            }
        }
        Err(_) => super::composite_field::encode(sym, &options, variant, &linear, secondary)?,
    };
    let w = bits.width();
    let h = bits.height();
    let linear_rows = (0..h)
        .rev()
        .take_while(|&y| (0..w).all(|x| bits.get(x, y) == bits.get(x, h - 1)))
        .count();
    Ok(EncodedBars::Composite {
        modules: bits,
        linear_start: h - linear_rows,
    })
}
