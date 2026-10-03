//! Element-level diff analysis for one label: correlate diff pixels with
//! element bboxes and classify each contributor (Content/Position/Mixed).
//! Usage: cargo run --example analyze_diff -- <label_name> [unit]

use labelize::skill::element_analyzer;
use labelize::ZplParser;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name = args.get(1).expect("usage: analyze_diff <label> [unit]");
    let subdir = if args.get(2).map(|s| s == "unit").unwrap_or(false) {
        "unit"
    } else {
        "labels"
    };
    let base = std::path::Path::new("testdata");
    let zpl_path = base.join(subdir).join(format!("{name}.zpl"));
    let diff_path = base.join("diffs").join(format!("{name}_diff.png"));
    let zpl = std::fs::read_to_string(&zpl_path).unwrap_or_else(|e| panic!("{zpl_path:?}: {e}"));
    let mut parser = ZplParser::new();
    let labels = parser.parse(zpl.as_bytes()).expect("parse failed");
    println!("label={name} pages={}", labels.len());
    match element_analyzer::analyze_label_with_classification(&labels[0], &diff_path, &zpl, true) {
        Ok(c) => println!("{}", element_analyzer::format_analysis_report(&c)),
        Err(e) => eprintln!("analysis error: {e}"),
    }
}
