//! Studio's field validation rules. Capacity bounds allow the printer to
//! choose compaction while rejecting data that cannot fit the requested symbol.
use crate::barcodes::code128::gs1_mod10_check;

pub(super) fn normalize_linear(kind: &str, value: &str, caption: bool) -> Result<String, String> {
    match kind {
        "ean8" => gtin(value, 7, caption),
        "ean13" => gtin(value, 12, caption),
        "upca" => gtin(value, 11, caption),
        "code39"
            if !(1..=512).contains(&value.len())
                || !value.bytes().all(|b| {
                    b.is_ascii_uppercase() || b.is_ascii_digit() || b" .$%/+-".contains(&b)
                }) =>
        {
            Err("Invalid Code 39 alphabet or length".into())
        }
        "itf"
            if !(2..=512).contains(&value.len())
                || !value.len().is_multiple_of(2)
                || !value.bytes().all(|b| b.is_ascii_digit()) =>
        {
            Err("ITF requires an even number of digits, up to 512".into())
        }
        "code128" if !(1..=512).contains(&value.len()) || !value.is_ascii() => {
            Err("Code 128 requires 1 to 512 ASCII bytes".into())
        }
        "code39" | "itf" | "code128" => Ok(value.to_owned()),
        _ => Err(format!("Unknown linear barcode kind {kind}")),
    }
}

fn gtin(value: &str, digits: usize, caption: bool) -> Result<String, String> {
    if !(digits..=digits + 1).contains(&value.len()) || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!(
            "Expected {digits} digits with an optional check digit"
        ));
    }
    let body = &value[..digits];
    let check = gs1_mod10_check(body);
    if value.len() > digits && !value.ends_with(check) {
        return Err("Incorrect check digit".into());
    }
    Ok(if caption {
        format!("{body}{check}")
    } else {
        body.to_owned()
    })
}

pub(super) fn check_pdf_capacity(value: &str, columns: usize, security: u8) -> Result<(), String> {
    if !value.is_ascii() {
        return Err("PDF417 requires ASCII".into());
    }
    let words = ((columns * 90).min(928)).saturating_sub((2 << security) + 1);
    let digits = value.bytes().filter(u8::is_ascii_digit).count();
    if 2 * digits + 3 * (value.len() - digits) > 6 * words {
        return Err("Exceeds PDF417 capacity for the configured columns and correction".into());
    }
    Ok(())
}

pub(super) fn check_matrix_capacity(value: &str, rectangle: bool) -> Result<(), String> {
    let sixths: usize = value
        .bytes()
        .map(|b| match b {
            b'0'..=b'9' => 3,
            0..=127 => 4,
            _ => 6,
        })
        .sum();
    let words = if rectangle { 49 } else { 1558 };
    if sixths > words * 6 {
        return Err("Exceeds Data Matrix symbol capacity".into());
    }
    Ok(())
}

pub(super) fn check_qr_capacity(value: &str, correction: u8) -> Result<(), String> {
    let words = match correction {
        b'L' => 2956,
        b'M' => 2334,
        b'Q' => 1666,
        b'H' => 1276,
        _ => return Err("Invalid QR correction level".into()),
    };
    let sixths: usize = value
        .bytes()
        .map(|b| match b {
            b'0'..=b'9' => 20,
            b'A'..=b'Z' => 33,
            _ if b" $%*+-./:".contains(&b) => 33,
            _ => 48,
        })
        .sum();
    if sixths > words * 48 {
        return Err("Exceeds QR capacity at the selected correction".into());
    }
    Ok(())
}

pub(super) fn check_aztec_capacity(value: &str) -> Result<(), String> {
    if value.len() > 3832 {
        return Err("Exceeds Aztec symbol capacity".into());
    }
    Ok(())
}
