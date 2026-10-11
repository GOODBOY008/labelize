pub const STORED_FORMAT_DEFAULT_PATH: &str = "R:UNKNOWN.ZPL";
pub const STORED_GRAPHICS_DEFAULT_PATH: &str = "R:UNKNOWN.GRF";

const VALID_DEVICES: &[&str] = &["R", "E", "B", "A", "Z"];

pub fn validate_device(path: &str) -> Result<(), String> {
    if let Some(idx) = path.find(':') {
        let device = &path[..idx];
        if VALID_DEVICES.contains(&device) {
            return Ok(());
        }
        return Err(format!(
            "invalid device name {}, must be one of {:?}",
            device, VALID_DEVICES
        ));
    }
    Err("path does not contain device name".to_string())
}

pub fn ensure_extension(path: &str, ext: &str) -> String {
    if let Some(idx) = path.find('.') {
        format!("{}.{}", &path[..idx], ext)
    } else {
        format!("{}.{}", path, ext)
    }
}
