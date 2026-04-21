/// Scans the diff report and identifies labels exceeding their tolerance thresholds.
use std::path::Path;

use super::error::ScanError;
use super::models::{DiffReport, DiffReportEntry, DiffStatus};

/// Parse the diff report text file into a structured DiffReport.
pub fn parse_diff_report(report_text: &str) -> Result<DiffReport, ScanError> {
    let mut entries = Vec::new();
    let mut perfect = 0usize;
    let mut good = 0usize;
    let mut minor = 0usize;
    let mut moderate = 0usize;
    let mut high = 0usize;
    let mut skip = 0usize;
    let mut errored = 0usize;

    for line in report_text.lines() {
        let line = line.trim();
        // Skip header, separator, summary, and border lines
        if !line.starts_with('║') || line.contains("Name") || line.contains("Summary") {
            continue;
        }

        // Parse data rows: ║ name │ ext │ diff% │ actual │ expected │ status ║
        let parts: Vec<&str> = line
            .trim_start_matches('║')
            .trim_end_matches('║')
            .split('│')
            .map(|s| s.trim())
            .collect();

        if parts.len() < 6 {
            continue;
        }

        let name = parts[0].to_string();
        let ext = parts[1].to_string();

        if name.is_empty() {
            continue;
        }

        let diff_percent = parts[2]
            .trim_end_matches('%')
            .trim()
            .parse::<f64>()
            .unwrap_or(-1.0);

        let actual_dims = parse_dims(parts[3]);
        let expected_dims = parse_dims(parts[4]);
        let status = DiffStatus::from_report_str(parts[5]);

        match &status {
            DiffStatus::Perfect => perfect += 1,
            DiffStatus::Good => good += 1,
            DiffStatus::Minor => minor += 1,
            DiffStatus::Moderate => moderate += 1,
            DiffStatus::High => high += 1,
            DiffStatus::Skip => skip += 1,
            DiffStatus::Error => errored += 1,
        }

        entries.push(DiffReportEntry {
            label_name: name,
            extension: ext,
            diff_percent,
            actual_dims,
            expected_dims,
            status,
            tolerance: None,
        });
    }

    if entries.is_empty() {
        return Err(ScanError::ParseError {
            reason: "no entries found in diff report".to_string(),
        });
    }

    let total = entries.len();
    Ok(DiffReport {
        entries,
        total_labels: total,
        perfect_count: perfect,
        good_count: good,
        minor_count: minor,
        moderate_count: moderate,
        high_count: high,
        skip_count: skip,
        error_count: errored,
    })
}

/// Parse dimension string like "813x1626" into (u32, u32).
fn parse_dims(s: &str) -> (u32, u32) {
    let s = s.trim();
    if s == "N/A" {
        return (0, 0);
    }
    let parts: Vec<&str> = s.split('x').collect();
    if parts.len() == 2 {
        let w = parts[0].trim().parse::<u32>().unwrap_or(0);
        let h = parts[1].trim().parse::<u32>().unwrap_or(0);
        (w, h)
    } else {
        (0, 0)
    }
}

/// Load and parse the diff report from the standard path.
pub fn load_diff_report(testdata_dir: &Path) -> Result<DiffReport, ScanError> {
    let report_path = testdata_dir.join("diffs").join("diff_report.txt");
    if !report_path.exists() {
        return Err(ScanError::ReportNotFound {
            path: report_path.display().to_string(),
        });
    }
    let content = std::fs::read_to_string(&report_path).map_err(|e| ScanError::ParseError {
        reason: format!("failed to read report: {}", e),
    })?;
    parse_diff_report(&content)
}

/// Enrich report entries with tolerance values from DIFF_THRESHOLDS.md.
pub fn enrich_with_tolerances(report: &mut DiffReport, thresholds_text: &str) {
    // Parse the markdown table in DIFF_THRESHOLDS.md
    // | Label | Ext | Diff % | Tolerance | Primary diff source |
    for line in thresholds_text.lines() {
        let line = line.trim();
        if !line.starts_with('|') || line.contains("Label") || line.contains("---") {
            continue;
        }
        let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
        if parts.len() < 5 {
            continue;
        }
        let name = parts[1].trim();
        let tolerance = parts[4].trim().parse::<f64>().ok();

        if let Some(tol) = tolerance {
            for entry in &mut report.entries {
                if entry.label_name == name {
                    entry.tolerance = Some(tol);
                }
            }
        }
    }
}

/// Find labels that exceed their tolerance threshold.
pub fn find_high_diff_labels(report: &DiffReport) -> Vec<&DiffReportEntry> {
    report
        .entries
        .iter()
        .filter(|e| {
            if let Some(tol) = e.tolerance {
                e.diff_percent > tol
            } else {
                // No tolerance set — use category: anything MODERATE or HIGH
                matches!(e.status, DiffStatus::Moderate | DiffStatus::High)
            }
        })
        .collect()
}

/// Find labels above a specific diff percentage threshold.
pub fn find_labels_above_threshold(report: &DiffReport, threshold: f64) -> Vec<&DiffReportEntry> {
    report
        .entries
        .iter()
        .filter(|e| e.diff_percent > threshold)
        .collect()
}

/// Look up a single label by name.
pub fn scan_label<'a>(
    report: &'a DiffReport,
    name: &str,
) -> Result<&'a DiffReportEntry, ScanError> {
    report
        .entries
        .iter()
        .find(|e| e.label_name == name)
        .ok_or_else(|| {
            let available: Vec<String> = report
                .entries
                .iter()
                .map(|e| e.label_name.clone())
                .collect();
            let suggestion = suggest_closest_label(name, &available);
            ScanError::LabelNotFound {
                name: name.to_string(),
                available,
                suggestion,
            }
        })
}

/// Suggest the closest matching label name using edit distance.
fn suggest_closest_label(name: &str, available: &[String]) -> String {
    available
        .iter()
        .min_by_key(|s| edit_distance(name, s))
        .cloned()
        .unwrap_or_default()
}

/// Simple Levenshtein edit distance.
fn edit_distance(a: &str, b: &str) -> usize {
    let a = a.as_bytes();
    let b = b.as_bytes();
    let mut dp = vec![vec![0usize; b.len() + 1]; a.len() + 1];

    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in dp[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dims() {
        assert_eq!(parse_dims("813x1626"), (813, 1626));
        assert_eq!(parse_dims("N/A"), (0, 0));
        assert_eq!(parse_dims(""), (0, 0));
    }

    #[test]
    fn test_classify_status() {
        assert_eq!(DiffStatus::from_percent(0.0), DiffStatus::Perfect);
        assert_eq!(DiffStatus::from_percent(0.5), DiffStatus::Good);
        assert_eq!(DiffStatus::from_percent(1.0), DiffStatus::Minor);
        assert_eq!(DiffStatus::from_percent(3.0), DiffStatus::Minor);
        assert_eq!(DiffStatus::from_percent(5.0), DiffStatus::Moderate);
        assert_eq!(DiffStatus::from_percent(14.9), DiffStatus::Moderate);
        assert_eq!(DiffStatus::from_percent(15.0), DiffStatus::High);
        assert_eq!(DiffStatus::from_percent(-1.0), DiffStatus::Skip);
    }

    #[test]
    fn test_edit_distance() {
        assert_eq!(edit_distance("amazon", "amazon"), 0);
        assert_eq!(edit_distance("amazon", "amazn"), 1);
        assert_eq!(edit_distance("fedex", "fedx"), 1);
        assert_eq!(edit_distance("ups", "usps"), 1);
    }

    #[test]
    fn test_suggest_closest() {
        let available = vec!["amazon".to_string(), "fedex".to_string(), "ups".to_string()];
        assert_eq!(suggest_closest_label("amazn", &available), "amazon");
        assert_eq!(suggest_closest_label("fedx", &available), "fedex");
    }

    #[test]
    fn test_parse_report_sample() {
        let sample = r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                    ZPL/EPL Rendering Diff Report                            ║
╠══════════════════════════════════════════════════════════════════════════════╣
║ Name                           │ Ext  │    Diff% │   Actual(WxH) │ Expected(WxH) │ Status       ║
╠══════════════════════════════════════════════════════════════════════════════╣
║ bstc                           │ zpl  │    0.00% │      813x1626 │      813x1626 │ PERFECT      ║
║ amazon                         │ zpl  │    2.26% │      813x1626 │      813x1626 │ MINOR(<5%)   ║
║ ups                            │ zpl  │    6.91% │      813x1626 │      813x1626 │ MODERATE(<15%) ║
╠══════════════════════════════════════════════════════════════════════════════╣
║ Summary: 3 total │ 1 perfect │ 0 good │ 1 minor │ 1 moderate │ 0 high │ 0 skip │ 0 err
╚══════════════════════════════════════════════════════════════════════════════╝
"#;
        let report = parse_diff_report(sample).unwrap();
        assert_eq!(report.entries.len(), 3);
        assert_eq!(report.perfect_count, 1);
        assert_eq!(report.minor_count, 1);
        assert_eq!(report.moderate_count, 1);

        assert_eq!(report.entries[0].label_name, "bstc");
        assert_eq!(report.entries[0].diff_percent, 0.0);
        assert_eq!(report.entries[0].status, DiffStatus::Perfect);

        assert_eq!(report.entries[1].label_name, "amazon");
        assert!((report.entries[1].diff_percent - 2.26).abs() < 0.01);

        assert_eq!(report.entries[2].label_name, "ups");
        assert_eq!(report.entries[2].status, DiffStatus::Moderate);
    }

    #[test]
    fn test_scan_label_not_found() {
        let sample = r#"
╔══════════════════════════════════════════════════════════════════════════════╗
║                    ZPL/EPL Rendering Diff Report                            ║
╠══════════════════════════════════════════════════════════════════════════════╣
║ Name                           │ Ext  │    Diff% │   Actual(WxH) │ Expected(WxH) │ Status       ║
╠══════════════════════════════════════════════════════════════════════════════╣
║ bstc                           │ zpl  │    0.00% │      813x1626 │      813x1626 │ PERFECT      ║
╠══════════════════════════════════════════════════════════════════════════════╣
║ Summary: 1 total │ 1 perfect │ 0 good │ 0 minor │ 0 moderate │ 0 high │ 0 skip │ 0 err
╚══════════════════════════════════════════════════════════════════════════════╝
"#;
        let report = parse_diff_report(sample).unwrap();
        let result = scan_label(&report, "nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            ScanError::LabelNotFound { suggestion, .. } => {
                assert_eq!(suggestion, "bstc");
            }
            _ => panic!("expected LabelNotFound"),
        }
    }
}
