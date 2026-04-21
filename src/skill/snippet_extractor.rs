/// Extracts problematic ZPL commands into standalone ZPL files for isolated testing.
use std::path::Path;

use super::error::ExtractError;
use super::models::{ZplCommand, ZplCommandSpan, ZplSnippet};

/// Global state command prefixes that affect element rendering.
const GLOBAL_STATE_PREFIXES: &[&str] = &[
    "^LH", "^PW", "^CF", "^BY", "^CI", "^FW", "^PO", "^LR", "^LL",
];

/// Split raw ZPL text into individual commands at ^ and ~ boundaries.
pub fn split_zpl_commands(zpl: &str) -> Vec<ZplCommand> {
    let mut commands = Vec::new();
    let bytes = zpl.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'^' || bytes[i] == b'~' {
            let start = i;
            let delim = bytes[i] as char;
            i += 1;
            // Read command prefix (1-2 alpha chars after ^ or ~)
            let prefix_start = i;
            while i < bytes.len() && bytes[i].is_ascii_alphabetic() && (i - prefix_start) < 2 {
                i += 1;
            }
            let prefix = format!("{}{}", delim, &zpl[prefix_start..i]);

            // Read parameters until next ^ or ~ or end
            let params_start = i;
            while i < bytes.len() && bytes[i] != b'^' && bytes[i] != b'~' {
                i += 1;
            }
            let params = zpl[params_start..i].to_string();

            commands.push(ZplCommand {
                prefix,
                params,
                offset: start,
            });
        } else {
            i += 1;
        }
    }

    commands
}

/// Group commands into spans, where each span represents one drawable element.
/// A span runs from a position command (^FO/^FT) through ^FS.
pub fn group_commands_into_spans(commands: &[ZplCommand]) -> Vec<ZplCommandSpan> {
    let mut spans = Vec::new();
    let mut current_span: Option<Vec<usize>> = None;
    let mut element_index = 0;

    for (i, cmd) in commands.iter().enumerate() {
        match cmd.prefix.as_str() {
            "^FO" | "^FT" => {
                // Start a new span (or restart if we had a dangling one)
                current_span = Some(vec![i]);
            }
            "^FS" => {
                if let Some(mut span_indices) = current_span.take() {
                    span_indices.push(i);
                    let start_offset = commands[span_indices[0]].offset;
                    let end_cmd = &commands[*span_indices.last().unwrap()];
                    let end_offset = end_cmd.offset + end_cmd.prefix.len() + end_cmd.params.len();

                    let span_commands: Vec<ZplCommand> = span_indices
                        .iter()
                        .map(|&idx| commands[idx].clone())
                        .collect();

                    spans.push(ZplCommandSpan {
                        start_offset,
                        end_offset,
                        commands: span_commands,
                        element_index,
                    });
                    element_index += 1;
                }
            }
            "^XA" | "^XZ" => {
                // Label delimiters — not part of any span
                current_span = None;
            }
            _ => {
                // Add to current span if we have one
                if let Some(ref mut span_indices) = current_span {
                    span_indices.push(i);
                }
            }
        }
    }

    spans
}

/// Extract global state commands from the ZPL.
pub fn extract_global_state_commands(commands: &[ZplCommand]) -> Vec<&ZplCommand> {
    commands
        .iter()
        .filter(|cmd| {
            GLOBAL_STATE_PREFIXES
                .iter()
                .any(|prefix| cmd.prefix == *prefix)
        })
        .collect()
}

/// Build a standalone ZPL snippet for a specific element.
pub fn extract_element(
    zpl: &str,
    label_name: &str,
    element_index: usize,
    diff_percent: f64,
) -> Result<ZplSnippet, ExtractError> {
    let commands = split_zpl_commands(zpl);
    let spans = group_commands_into_spans(&commands);

    let span = spans
        .iter()
        .find(|s| s.element_index == element_index)
        .ok_or(ExtractError::ElementOutOfRange {
            index: element_index,
            total: spans.len(),
        })?;

    let global_cmds = extract_global_state_commands(&commands);

    // Only include global state commands that appear BEFORE this element
    // Deduplicate: keep only the LAST occurrence of each prefix (most recent state)
    let relevant_globals: Vec<&&ZplCommand> = global_cmds
        .iter()
        .filter(|g| g.offset < span.start_offset)
        .collect();

    // Deduplicate by prefix — keep last occurrence of each
    let mut seen_prefixes: std::collections::HashMap<&str, &ZplCommand> =
        std::collections::HashMap::new();
    for cmd in &relevant_globals {
        seen_prefixes.insert(&cmd.prefix, cmd);
    }
    let deduped_globals: Vec<&&ZplCommand> = {
        let mut v: Vec<_> = seen_prefixes.values().collect();
        v.sort_by_key(|c| c.offset);
        v
    };

    let mut snippet = String::new();
    snippet.push_str("^XA\n");

    // Add global state commands
    for cmd in &deduped_globals {
        snippet.push_str(&format!("{}{}\n", cmd.prefix, cmd.params));
    }

    // Add element commands
    for cmd in &span.commands {
        snippet.push_str(&format!("{}{}\n", cmd.prefix, cmd.params));
    }

    snippet.push_str("^XZ\n");

    let file_path = format!("testdata/snippets/{}_{}.zpl", label_name, element_index);

    Ok(ZplSnippet {
        label_name: label_name.to_string(),
        element_index,
        zpl_content: snippet,
        file_path,
        original_diff_percent: diff_percent,
    })
}

/// Extract multiple elements into a single combined snippet.
pub fn extract_element_group(
    zpl: &str,
    label_name: &str,
    element_indices: &[usize],
    diff_percent: f64,
) -> Result<ZplSnippet, ExtractError> {
    let commands = split_zpl_commands(zpl);
    let spans = group_commands_into_spans(&commands);
    let global_cmds = extract_global_state_commands(&commands);

    let mut snippet = String::new();
    snippet.push_str("^XA\n");

    // Add all global state commands
    for cmd in &global_cmds {
        snippet.push_str(&format!("{}{}\n", cmd.prefix, cmd.params));
    }

    // Add each requested element's commands
    for &idx in element_indices {
        let span = spans.iter().find(|s| s.element_index == idx).ok_or(
            ExtractError::ElementOutOfRange {
                index: idx,
                total: spans.len(),
            },
        )?;

        for cmd in &span.commands {
            snippet.push_str(&format!("{}{}\n", cmd.prefix, cmd.params));
        }
        snippet.push('\n');
    }

    snippet.push_str("^XZ\n");

    let indices_str = element_indices
        .iter()
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join("_");
    let file_path = format!("testdata/snippets/{}_{}.zpl", label_name, indices_str);

    Ok(ZplSnippet {
        label_name: label_name.to_string(),
        element_index: element_indices[0],
        zpl_content: snippet,
        file_path,
        original_diff_percent: diff_percent,
    })
}

/// Write a snippet to disk.
pub fn write_snippet(snippet: &ZplSnippet) -> Result<(), ExtractError> {
    let path = Path::new(&snippet.file_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ExtractError::WriteError {
            reason: format!("failed to create directory: {}", e),
        })?;
    }
    std::fs::write(path, &snippet.zpl_content).map_err(|e| ExtractError::WriteError {
        reason: format!("failed to write snippet: {}", e),
    })?;
    Ok(())
}

/// Extract and write all high-diff elements from a label into individual snippet files.
pub fn extract_all_high_diff_elements(
    zpl: &str,
    label_name: &str,
    element_indices: &[usize],
    diff_percent: f64,
) -> Result<Vec<ZplSnippet>, ExtractError> {
    let mut snippets = Vec::new();
    for &idx in element_indices {
        let snippet = extract_element(zpl, label_name, idx, diff_percent)?;
        write_snippet(&snippet)?;
        snippets.push(snippet);
    }
    Ok(snippets)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ZPL: &str = r#"^XA
^PW800
^CF0,30
^BY2
^FO50,100^A0N,40,30^FDHello World^FS
^FO50,200^GB700,3,3^FS
^FO100,300^BCN,100,Y,N,N^FD12345^FS
^XZ"#;

    #[test]
    fn test_split_commands() {
        let cmds = split_zpl_commands(SAMPLE_ZPL);
        let prefixes: Vec<&str> = cmds.iter().map(|c| c.prefix.as_str()).collect();
        assert!(prefixes.contains(&"^XA"));
        assert!(prefixes.contains(&"^PW"));
        assert!(prefixes.contains(&"^CF"));
        assert!(prefixes.contains(&"^BY"));
        assert!(prefixes.contains(&"^FO"));
        assert!(prefixes.contains(&"^A"));
        assert!(prefixes.contains(&"^FD"));
        assert!(prefixes.contains(&"^FS"));
        assert!(prefixes.contains(&"^GB"));
        assert!(prefixes.contains(&"^BC"));
        assert!(prefixes.contains(&"^XZ"));
    }

    #[test]
    fn test_group_spans() {
        let cmds = split_zpl_commands(SAMPLE_ZPL);
        let spans = group_commands_into_spans(&cmds);
        assert_eq!(spans.len(), 3); // text, graphic box, barcode
        assert_eq!(spans[0].element_index, 0);
        assert_eq!(spans[1].element_index, 1);
        assert_eq!(spans[2].element_index, 2);
    }

    #[test]
    fn test_extract_global_state() {
        let cmds = split_zpl_commands(SAMPLE_ZPL);
        let globals = extract_global_state_commands(&cmds);
        let prefixes: Vec<&str> = globals.iter().map(|c| c.prefix.as_str()).collect();
        assert!(prefixes.contains(&"^PW"));
        assert!(prefixes.contains(&"^CF"));
        assert!(prefixes.contains(&"^BY"));
        assert!(!prefixes.contains(&"^FO"));
        assert!(!prefixes.contains(&"^XA"));
    }

    #[test]
    fn test_extract_element_snippet() {
        let snippet = extract_element(SAMPLE_ZPL, "test", 0, 5.0).unwrap();
        assert!(snippet.zpl_content.contains("^XA"));
        assert!(snippet.zpl_content.contains("^XZ"));
        assert!(snippet.zpl_content.contains("^PW"));
        assert!(snippet.zpl_content.contains("^CF"));
        assert!(snippet.zpl_content.contains("^FDHello World"));
        assert!(snippet.zpl_content.contains("^FS"));
    }

    #[test]
    fn test_extract_barcode_snippet() {
        let snippet = extract_element(SAMPLE_ZPL, "test", 2, 5.0).unwrap();
        assert!(snippet.zpl_content.contains("^BC"));
        assert!(snippet.zpl_content.contains("^FD12345"));
        assert!(snippet.zpl_content.contains("^BY")); // global state preserved
    }

    #[test]
    fn test_extract_element_out_of_range() {
        let result = extract_element(SAMPLE_ZPL, "test", 99, 5.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_group() {
        let snippet = extract_element_group(SAMPLE_ZPL, "test", &[0, 2], 5.0).unwrap();
        assert!(snippet.zpl_content.contains("^FDHello World"));
        assert!(snippet.zpl_content.contains("^FD12345"));
    }
}
