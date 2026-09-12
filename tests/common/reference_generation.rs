//! Used only by explicitly invoked Labelary generation commands and their tests.
use std::path::Path;

/// Fetch and validate before touching the reference. False means an existing
/// reference was retained; failures never substitute a local renderer image.
pub fn write_labelary_reference(
    input: &Path,
    overwrite: bool,
    fetch: impl FnOnce(&str) -> Option<Vec<u8>>,
) -> Result<bool, String> {
    let output = input.with_extension("png");
    if !overwrite
        && output
            .try_exists()
            .map_err(|e| format!("cannot inspect {}: {e}", output.display()))?
    {
        return Ok(false);
    }
    let content = std::fs::read_to_string(input)
        .map_err(|e| format!("cannot read {}: {e}", input.display()))?;
    let png = fetch(&content)
        .ok_or_else(|| format!("Labelary reference unavailable for {}", input.display()))?;
    image::load_from_memory_with_format(&png, image::ImageFormat::Png)
        .map_err(|e| format!("invalid Labelary PNG for {}: {e}", input.display()))?;
    std::fs::write(&output, png).map_err(|e| format!("cannot write {}: {e}", output.display()))?;
    Ok(true)
}
