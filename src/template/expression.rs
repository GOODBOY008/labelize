use super::TemplateValues;
use std::collections::BTreeMap;

#[derive(Debug)]
enum Part {
    Literal(String),
    Variable(usize),
}

#[derive(Debug)]
pub(super) struct Expression {
    parts: Vec<Part>,
    pub(super) names: Vec<String>,
}

impl Expression {
    pub(super) fn parse(mut source: &[u8]) -> Result<Self, String> {
        let mut parts = Vec::new();
        let mut names = Vec::new();
        let mut indices = BTreeMap::new();
        while !source.is_empty() {
            if source.starts_with(b"_") {
                let mut literal = Vec::new();
                while let Some(rest) = source.strip_prefix(b"_") {
                    let [high, low, rest @ ..] = rest else {
                        return Err("Truncated hex literal".into());
                    };
                    literal.push(hex(*high)? * 16 + hex(*low)?);
                    source = rest;
                }
                parts.push(Part::Literal(
                    String::from_utf8(literal).map_err(|_| "Invalid UTF-8 template literal")?,
                ));
            } else {
                let rest = source
                    .strip_prefix(b"{{")
                    .ok_or("Expected a {{name}} or _HH literal")?;
                let end = rest
                    .windows(2)
                    .position(|p| p == b"}}")
                    .ok_or("Unclosed variable")?;
                let name = &rest[..end];
                if !matches!(name.first(), Some(b'A'..=b'Z' | b'a'..=b'z' | b'_'))
                    || !name.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_')
                {
                    return Err("Invalid template variable name".into());
                }
                let name = std::str::from_utf8(name).expect("ASCII variable name");
                let index = *indices.entry(name).or_insert_with(|| {
                    names.push(name.to_owned());
                    names.len() - 1
                });
                parts.push(Part::Variable(index));
                source = &rest[end + 2..];
            }
        }
        if names.is_empty() {
            return Err("Field expression must contain a variable".into());
        }
        Ok(Self { parts, names })
    }

    pub(super) fn expand(&self, values: &TemplateValues) -> Result<String, String> {
        let mut text = String::new();
        // Studio's expansion limit counts UTF-16 code units. Encoders
        // apply their own byte limits after escaping and character encoding.
        let mut units = 0;
        for part in &self.parts {
            let s = match part {
                Part::Literal(s) => s,
                Part::Variable(index) => &values[&self.names[*index]],
            };
            units += s.encode_utf16().take(4097 - units).count();
            if units > 4096 {
                return Err("Filled expression exceeds the 4096 character limit".into());
            }
            text.push_str(s);
        }
        Ok(text)
    }
}

fn hex(digit: u8) -> Result<u8, String> {
    match digit {
        b'0'..=b'9' => Ok(digit - b'0'),
        b'A'..=b'F' => Ok(digit - b'A' + 10),
        _ => Err("Expected uppercase hexadecimal digits in _HH literal".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_names_only_once_and_decodes_split_utf8_bytes() {
        let expr = Expression::parse(b"{{id}}_E6_9D_AF{{product}}{{id}}").unwrap();
        assert_eq!(expr.names, ["id", "product"]);
        let values = TemplateValues::from([
            ("id".into(), "0001".into()),
            ("product".into(), "{{id}}".into()),
        ]);
        assert_eq!(expr.expand(&values).unwrap(), "0001杯{{id}}0001");
    }

    #[test]
    fn rejects_invalid_names_literals_and_incomplete_expressions() {
        for bad in [
            b"_41".as_slice(),
            b"_41_{{a}}",
            b"_ff{{name}}",
            b"_C3{{bad}}",
            b"{{}}",
            b"{{0id}}",
            b"{{a-b}}",
            b"{{id}",
            b"{{id}}fixed",
        ] {
            assert!(Expression::parse(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn bounds_expansion_across_repeated_values_in_utf16_units() {
        let expr = Expression::parse(b"{{v}}{{v}}").unwrap();
        let mut input = TemplateValues::from([("v".into(), "📦".repeat(1024))]);
        assert!(expr.expand(&input).is_ok());
        input.get_mut("v").unwrap().push('x');
        assert!(expr.expand(&input).is_err());
    }
}
