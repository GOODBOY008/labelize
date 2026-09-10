//! Read-only fixture loading shared by golden tests and diff reports.
use std::path::{Path, PathBuf};

pub struct GoldenFixture {
    pub content: String,
    pub reference: Vec<u8>,
}

pub fn resolve_input(root: &Path, name: &str, extension: &str) -> Result<(PathBuf, bool), String> {
    let filename = format!("{name}.{extension}");
    let candidates = [
        (root.join("labels").join(&filename), false),
        (root.join("unit").join(&filename), true),
        (root.join(&filename), false),
    ];
    for (path, is_unit) in &candidates {
        if path
            .try_exists()
            .map_err(|e| format!("cannot inspect {}: {e}", path.display()))?
        {
            return Ok((path.clone(), *is_unit));
        }
    }
    Err(format!(
        "missing golden input; searched: {}",
        candidates
            .iter()
            .map(|(p, _)| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

pub fn load_fixture(input: &Path) -> Result<GoldenFixture, String> {
    let content = std::fs::read_to_string(input)
        .map_err(|e| format!("cannot read golden input {}: {e}", input.display()))?;
    let path = input.with_extension("png");
    let reference = std::fs::read(&path)
        .map_err(|e| format!("cannot read golden reference {}: {e}; see docs/GOLDEN_TESTS.md for explicit reference generation", path.display()))?;
    image::load_from_memory_with_format(&reference, image::ImageFormat::Png)
        .map_err(|e| format!("invalid golden reference {}: {e}", path.display()))?;
    Ok(GoldenFixture { content, reference })
}

pub fn discover_inputs(dirs: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for dir in dirs {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("cannot read fixture directory {}: {e}", dir.display()))?;
        for entry in entries {
            let path = entry
                .map_err(|e| format!("cannot read entry in {}: {e}", dir.display()))?
                .path();
            if matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("zpl" | "epl")
            ) {
                paths.push(path);
            }
        }
    }
    if paths.is_empty() {
        return Err(format!("no ZPL/EPL inputs found in {dirs:?}"));
    }
    paths.sort();
    Ok(paths)
}
