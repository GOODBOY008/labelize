use std::io::Read;

const B_IN_MB: usize = 1024 * 1024;
const MAX_EMBEDDED_IMAGE_SIZE_MB: usize = 3 * B_IN_MB;

pub fn decode_escaped_string(value: &str, escape_char: u8) -> Result<String, String> {
    let esc = escape_char as char;
    let mut result = String::new();
    let chars: Vec<char> = value.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == esc && i + 2 < chars.len() {
            // Collect consecutive hex-escaped bytes for proper multi-byte UTF-8 decoding
            let mut bytes = Vec::new();
            while i < chars.len() && chars[i] == esc && i + 2 < chars.len() {
                let hex_str: String = chars[i + 1..i + 3].iter().collect();
                if let Ok(decoded) = hex::decode(&hex_str) {
                    if let Some(&b) = decoded.first() {
                        bytes.push(b);
                        i += 3;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            if !bytes.is_empty() {
                match String::from_utf8(bytes.clone()) {
                    Ok(s) => result.push_str(&s),
                    Err(_) => {
                        // Fallback: push bytes as Latin-1 characters
                        for b in bytes {
                            result.push(b as char);
                        }
                    }
                }
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    Ok(result)
}

fn compress_counts() -> &'static std::collections::HashMap<u8, usize> {
    use std::sync::OnceLock;
    static COUNTS: OnceLock<std::collections::HashMap<u8, usize>> = OnceLock::new();
    COUNTS.get_or_init(|| {
        let mut m = std::collections::HashMap::new();
        m.insert(b'G', 1); m.insert(b'H', 2); m.insert(b'I', 3); m.insert(b'J', 4);
        m.insert(b'K', 5); m.insert(b'L', 6); m.insert(b'M', 7); m.insert(b'N', 8);
        m.insert(b'O', 9); m.insert(b'P', 10); m.insert(b'Q', 11); m.insert(b'R', 12);
        m.insert(b'S', 13); m.insert(b'T', 14); m.insert(b'U', 15); m.insert(b'V', 16);
        m.insert(b'W', 17); m.insert(b'X', 18); m.insert(b'Y', 19);
        m.insert(b'g', 20); m.insert(b'h', 40); m.insert(b'i', 60); m.insert(b'j', 80);
        m.insert(b'k', 100); m.insert(b'l', 120); m.insert(b'm', 140); m.insert(b'n', 160);
        m.insert(b'o', 180); m.insert(b'p', 200); m.insert(b'q', 220); m.insert(b'r', 240);
        m.insert(b's', 260); m.insert(b't', 280); m.insert(b'u', 300); m.insert(b'v', 320);
        m.insert(b'w', 340); m.insert(b'x', 360); m.insert(b'y', 380); m.insert(b'z', 400);
        m
    })
}

pub fn decode_graphic_field_data(data: &str, row_bytes: i32) -> Result<Vec<u8>, String> {
    if z64_encoded(data) {
        return decode_z64(data);
    }

    let counts = compress_counts();
    let mut result = String::new();
    let mut line = String::new();
    let mut prev_line = String::new();
    let mut compress_count: usize = 0;
    let row_hex = (row_bytes * 2) as usize;

    let bytes = data.as_bytes();
    for &ch in bytes {
        if line.len() >= row_hex {
            validate_embedded_image_size(result.len() + line.len(), 0)?;
            prev_line = line.clone();
            result.push_str(&prev_line);
            line.clear();
        }

        if let Some(&c) = counts.get(&ch) {
            compress_count += c;
            continue;
        }

        match ch {
            b',' => {
                let l = row_hex.saturating_sub(line.len());
                validate_embedded_image_size(result.len() + line.len(), l)?;
                if row_hex > line.len() {
                    line.extend(std::iter::repeat('0').take(l));
                }
            }
            b'!' => {
                let l = row_hex.saturating_sub(line.len());
                validate_embedded_image_size(result.len() + line.len(), l)?;
                if row_hex > line.len() {
                    line.extend(std::iter::repeat('1').take(l));
                }
            }
            b':' => {
                validate_embedded_image_size(result.len() + line.len(), prev_line.len())?;
                line.push_str(&prev_line);
            }
            _ => {
                let l = compress_count.max(1);
                validate_embedded_image_size(result.len() + line.len(), l)?;
                let c = ch as char;
                for _ in 0..l {
                    line.push(c);
                }
                compress_count = 0;
            }
        }
    }

    if !line.is_empty() {
        validate_embedded_image_size(result.len() + line.len(), 0)?;
        result.push_str(&line);
    }

    hex::decode(&result).map_err(|e| format!("hex decode error: {}", e))
}

fn validate_embedded_image_size(current: usize, additional: usize) -> Result<(), String> {
    if current + additional > 2 * MAX_EMBEDDED_IMAGE_SIZE_MB {
        Err(format!(
            "embedded image size cannot be greater than {} MB",
            MAX_EMBEDDED_IMAGE_SIZE_MB / B_IN_MB
        ))
    } else {
        Ok(())
    }
}

const Z64_PREFIX: &str = ":Z64:";

fn z64_encoded(value: &str) -> bool {
    value.starts_with(Z64_PREFIX)
}

fn decode_z64(value: &str) -> Result<Vec<u8>, String> {
    let value = value.strip_prefix(Z64_PREFIX).unwrap_or(value);
    let value = if let Some(idx) = value.rfind(':') {
        &value[..idx]
    } else {
        value
    };

    let dec = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value)
        .map_err(|e| format!("base64 decode error: {}", e))?;

    let mut z = flate2::read::ZlibDecoder::new(&dec[..]);
    let mut result = Vec::new();
    z.read_to_end(&mut result)
        .map_err(|e| format!("zlib decompress error: {}", e))?;

    Ok(result)
}

// Minimal hex module to avoid external dependency
mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        let s = s.trim();
        if s.len() % 2 != 0 {
            // Pad with trailing zero
            let padded = format!("{}0", s);
            return decode_even(&padded);
        }
        decode_even(s)
    }

    fn decode_even(s: &str) -> Result<Vec<u8>, String> {
        let mut result = Vec::with_capacity(s.len() / 2);
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let hi = hex_val(bytes[i]).ok_or_else(|| format!("invalid hex char: {}", bytes[i] as char))?;
            let lo = if i + 1 < bytes.len() {
                hex_val(bytes[i + 1]).ok_or_else(|| format!("invalid hex char: {}", bytes[i + 1] as char))?
            } else {
                0
            };
            result.push((hi << 4) | lo);
            i += 2;
        }
        Ok(result)
    }

    fn hex_val(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }
}
