//! labelize compiled for Android, exposed through JNI.
//!
//! Mirrors the wasm surface (`wasm/src/lib.rs`): one raw `render` entry point
//! plus a version string, with everything else (String/ByteArray conveniences)
//! living on the Kotlin side in `lib/src/main/kotlin`. Error stage codes match
//! the wasm contract: `1:` = parse failure, `2:` = render failure.

use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};

use jni::objects::{JByteArray, JClass, JThrowable, JValue};
use jni::sys::{jboolean, jbyteArray, jdouble, jint, jstring};
use jni::JNIEnv;

/// `LabelizeException.STAGE_PARSE` — bad ZPL/EPL input (HTTP 400 analog).
const STAGE_PARSE: jint = 1;
/// `LabelizeException.STAGE_RENDER` — internal rendering failure (HTTP 500 analog).
const STAGE_RENDER: jint = 2;

fn render_payload(
    bytes: &[u8],
    width_mm: f64,
    height_mm: f64,
    dpmm: i32,
    antialias: bool,
    want_pdf: bool,
    is_epl: bool,
) -> Result<Vec<u8>, (jint, String)> {
    let labels = if is_epl {
        labelize::EplParser::new().parse(bytes)
    } else {
        labelize::ZplParser::with_dpmm(dpmm).parse(bytes)
    }
    .map_err(|e| (STAGE_PARSE, e.to_string()))?;

    let label = labels
        .into_iter()
        .next()
        .ok_or_else(|| (STAGE_PARSE, "No labels found".to_string()))?;

    let options = labelize::DrawerOptions {
        label_width_mm: width_mm,
        label_height_mm: height_mm,
        dpmm,
        antialias,
        ..Default::default()
    };

    let renderer = labelize::Renderer::new();
    let mut png_buf = Cursor::new(Vec::new());
    renderer
        .draw_label_as_png(&label, &mut png_buf, options.clone())
        .map_err(|e| (STAGE_RENDER, e.to_string()))?;

    if want_pdf {
        let img = image::load_from_memory(png_buf.get_ref())
            .map_err(|e| (STAGE_RENDER, format!("image decode: {e}")))?
            .to_rgba8();
        let mut pdf_buf = Cursor::new(Vec::new());
        labelize::encode_pdf(&img, &options, &mut pdf_buf)
            .map_err(|e| (STAGE_RENDER, e.to_string()))?;
        Ok(pdf_buf.into_inner())
    } else {
        Ok(png_buf.into_inner())
    }
}

/// Throw `com.goodboy008.labelize.LabelizeException(stage, message)` on the
/// JVM. If raising it fails too (class missing, JVM shutting down), leave
/// whatever exception is pending — the caller must see *some* failure.
fn throw_labelize(env: &mut JNIEnv, stage: jint, message: &str) {
    let raise = |env: &mut JNIEnv| -> jni::errors::Result<()> {
        let class = env.find_class("com/goodboy008/labelize/LabelizeException")?;
        let msg = env.new_string(message)?;
        let exc = env.new_object(
            class,
            "(ILjava/lang/String;)V",
            &[JValue::Int(stage), JValue::Object(&msg)],
        )?;
        env.throw(JThrowable::from(exc))
    };
    let _ = raise(env);
}

/// Parse ZPL/EPL and render to PNG (or PDF when `pdf` is true).
///
/// Mapping for `Labelize.render(...)` in
/// `lib/src/main/kotlin/com/goodboy008/labelize/Labelize.kt`.
#[no_mangle]
pub extern "system" fn Java_com_goodboy008_labelize_Labelize_render<'local>(
    mut env: JNIEnv<'local>,
    _this: JClass<'local>,
    src: JByteArray<'local>,
    width_mm: jdouble,
    height_mm: jdouble,
    dpmm: jint,
    antialias: jboolean,
    pdf: jboolean,
    epl: jboolean,
) -> jbyteArray {
    // A panic must never unwind across the JNI boundary; surface it as a
    // render-stage exception instead of aborting the app.
    let payload = catch_unwind(AssertUnwindSafe(|| {
        let bytes = match env.convert_byte_array(src) {
            Ok(b) => b,
            Err(e) => return Err((STAGE_RENDER, format!("jni: {e}"))),
        };
        render_payload(
            &bytes,
            width_mm,
            height_mm,
            dpmm,
            antialias != 0,
            pdf != 0,
            epl != 0,
        )
    }));

    let out = match payload {
        Ok(Ok(bytes)) => bytes,
        Ok(Err((stage, msg))) => {
            throw_labelize(&mut env, stage, &msg);
            return std::ptr::null_mut();
        }
        Err(_) => {
            throw_labelize(&mut env, STAGE_RENDER, "internal panic while rendering");
            return std::ptr::null_mut();
        }
    };

    let write = |env: &mut JNIEnv| -> jni::errors::Result<jbyteArray> {
        let arr = env.new_byte_array(out.len().try_into().expect("png larger than jsize::MAX"))?;
        // JNI's byte arrays are signed; reinterpret without copying (u8 and
        // i8 have identical layout, so this is sound).
        let signed = unsafe { std::slice::from_raw_parts(out.as_ptr().cast::<i8>(), out.len()) };
        env.set_byte_array_region(&arr, 0, signed)?;
        Ok(arr.into_raw())
    };
    match write(&mut env) {
        Ok(arr) => arr,
        Err(e) => {
            throw_labelize(&mut env, STAGE_RENDER, &format!("jni: {e}"));
            std::ptr::null_mut()
        }
    }
}

/// Engine version, for support diagnostics. Maps to `Labelize.version()`.
#[no_mangle]
pub extern "system" fn Java_com_goodboy008_labelize_Labelize_version<'local>(
    env: JNIEnv<'local>,
    _this: JClass<'local>,
) -> jstring {
    match env.new_string(format!("labelize-android {}", env!("CARGO_PKG_VERSION"))) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ZPL: &str = "^XA^FO50,50^A0N,40,40^FDHELLO WORLD^FS^XZ";
    const SAMPLE_EPL: &str = include_str!("../../testdata/labels/dpduk.epl");
    const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    fn png(bytes: &[u8]) -> bool {
        bytes.len() >= 8 && bytes[..8] == PNG_MAGIC
    }

    #[test]
    fn renders_zpl_to_png() {
        let out = render_payload(SAMPLE_ZPL.as_bytes(), 102.0, 152.0, 8, false, false, false)
            .expect("render ok");
        assert!(png(&out), "expected PNG magic");
    }

    #[test]
    fn renders_epl_to_png() {
        let out = render_payload(SAMPLE_EPL.as_bytes(), 102.0, 152.0, 8, false, false, true)
            .expect("render ok");
        assert!(png(&out), "expected PNG magic");
    }

    #[test]
    fn renders_to_pdf() {
        let out = render_payload(SAMPLE_ZPL.as_bytes(), 102.0, 152.0, 8, false, true, false)
            .expect("render ok");
        assert!(out.starts_with(b"%PDF"), "expected PDF header");
    }

    #[test]
    fn parse_error_is_stage_1() {
        let err = render_payload(b"this is not a label", 102.0, 152.0, 8, false, false, false)
            .expect_err("garbage input must fail");
        assert_eq!(err.0, STAGE_PARSE);
    }

    #[test]
    fn empty_input_yields_no_labels_error() {
        let err = render_payload(b"", 102.0, 152.0, 8, false, false, false)
            .expect_err("empty input must fail");
        assert_eq!(err.1, "No labels found".to_string());
    }
}
