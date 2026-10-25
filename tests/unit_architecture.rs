//! Architecture guards: the shared rendering pipeline stays language-blind.
//!
//! The engine's contract is that `parsers/{zpl,epl,…}` are the only modules
//! that know a printer language, `elements` is a language-neutral IR, and
//! `drawers`/`images` draw whatever the IR describes without caring which
//! language produced it. These tests enforce both directions in CI so a new
//! language frontend (e.g. TSPL) cannot leak semantics into the shared
//! pipeline by accident.

use std::fs;
use std::path::{Path, PathBuf};

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rust_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries =
            fs::read_dir(&dir).unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Shared pipeline modules that must never reach into a language frontend.
const DOWNSTREAM_MODULES: &[&str] = &["drawers", "elements", "barcodes", "images"];

#[test]
fn shared_pipeline_never_depends_on_parsers() {
    let mut violations = Vec::new();
    for module in DOWNSTREAM_MODULES {
        for path in rust_files_under(&src_dir().join(module)) {
            let content = fs::read_to_string(&path).unwrap();
            // Comments are stripped: prose may legitimately cross-reference
            // where a struct's resolution logic lives (e.g. stored-format
            // docs pointing at parsers::zpl). A real dependency is a `use`
            // or path expression, which survives stripping.
            let code = strip_comments(&content);
            for (line_no, line) in code.lines().enumerate() {
                if line.contains("parsers::") || line.contains("crate::parsers") {
                    violations.push(format!("{}:{}: {line}", path.display(), line_no + 1));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "shared pipeline modules must not depend on parsers (dependency direction is \
         parsers → elements → drawers); violations:\n{}",
        violations.join("\n")
    );
}

/// Remove `//` line comments and `/* */` block comments so that prose in
/// doc-comments (which legitimately cites the languages being modeled, e.g.
/// calibrating against Labelary's ZPL font metrics) does not trip the token
/// scan. Strings are left intact on purpose: a language name inside a string
/// literal in drawers would be exactly the kind of leak this guard exists for.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut closed = false;
                while let Some(c) = chars.next() {
                    if c == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        closed = true;
                        break;
                    }
                    if c == '\n' {
                        out.push('\n');
                    }
                }
                assert!(closed, "unterminated block comment");
            }
            _ => out.push(c),
        }
    }
    out
}

#[test]
fn drawers_are_language_blind() {
    let mut violations = Vec::new();
    for path in rust_files_under(&src_dir().join("drawers")) {
        let content = fs::read_to_string(&path).unwrap();
        let code = strip_comments(&content);
        for (line_no, line) in code.lines().enumerate() {
            let lower = line.to_lowercase();
            for token in ["zpl", "epl", "tspl", "cpcl", "dpl"] {
                // Word-boundary match: `display_zpl` would be a violation,
                // `sizezpling` (hypothetical identifier substring) would not.
                if lower
                    .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .any(|word| word == token)
                {
                    violations.push(format!(
                        "{}:{}: language token '{token}' in code: {line}",
                        path.display(),
                        line_no + 1
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "drawers must be language-blind: per-language calibration is resolved by parsers \
         into the neutral IR (FontFace, raw QR payload, …); violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn comment_stripping_preserves_code_and_drops_prose() {
    let src = "let x = 1; // mentions ZPL\n/* block EPL\n   TSPL */ let y = 2; // tspl\nlet z = 3;";
    let stripped = strip_comments(src);
    assert!(stripped.contains("let x = 1;"));
    assert!(stripped.contains("let y = 2;"));
    assert!(stripped.contains("let z = 3;"));
    assert!(!stripped.contains("ZPL"));
    assert!(!stripped.contains("EPL"));
    assert!(!stripped.contains("TSPL"));
    assert!(!stripped.contains("tspl"));
}
