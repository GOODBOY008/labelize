use labelize::DrawerOptions;
use proptest::prelude::*;

/// Strategy for valid DrawerOptions with positive dimensions.
pub fn arb_drawer_options() -> impl Strategy<Value = DrawerOptions> {
    let dpmm_values = prop_oneof![Just(6), Just(8), Just(12), Just(24)];
    (10.0f64..200.0, 10.0f64..300.0, dpmm_values).prop_map(|(w, h, d)| DrawerOptions {
        label_width_mm: w,
        label_height_mm: h,
        dpmm: d,
        enable_inverted_labels: false,
    })
}

/// Strategy for simple ZPL label blocks: ^XA + random commands + ^XZ.
pub fn arb_zpl_label() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("^XA^FO50,50^A0N,30,30^FDHello^FS^XZ".to_string()),
        Just("^XA^GB200,100,3^FS^XZ".to_string()),
        Just("^XA^FO10,10^BCN,100,Y,N,N^FD12345^FS^XZ".to_string()),
        "[A-Za-z0-9 ,]{1,50}".prop_map(|s| format!("^XA^FO50,50^A0N,30,30^FD{}^FS^XZ", s)),
    ]
}

/// Strategy for non-empty ASCII strings (chars 32-127) for Code128.
pub fn arb_code128_input() -> impl Strategy<Value = String> {
    "[\\x20-\\x7E]{1,30}"
}

/// Strategy for even-length digit strings for Interleaved 2-of-5.
pub fn arb_2of5_input() -> impl Strategy<Value = String> {
    "[0-9]{1,15}".prop_map(|s| {
        if s.len() % 2 != 0 {
            format!("{}0", s)
        } else {
            s
        }
    })
}

/// Strategy for 12-digit strings for EAN-13.
pub fn arb_ean13_input() -> impl Strategy<Value = String> {
    "[0-9]{12}"
}

/// Strategy for ASCII strings (1-100 chars) for QR code.
pub fn arb_qr_input() -> impl Strategy<Value = String> {
    "[\\x20-\\x7E]{1,100}"
}

/// Strategy for valid hex strings (0-9, a-f, A-F).
pub fn arb_hex_string() -> impl Strategy<Value = String> {
    "[0-9a-fA-F]{2,40}"
}
