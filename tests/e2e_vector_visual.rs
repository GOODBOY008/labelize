//! Public input-bytes -> PNG visual regressions, with committed pixel baselines.
//! Export real PDFs for the separate PDF structure and visual checks in CI.
#![cfg(feature = "vector")]

#[path = "common/vector_golden.rs"]
mod vector_golden;

macro_rules! golden {
    ($name:ident, $index:expr) => {
        #[test]
        fn $name() {
            vector_golden::compare($index);
        }
    };
}

golden!(golden_vector_glyph_positions, 0);
golden!(golden_vector_bitmap_qr, 1);
golden!(golden_vector_bitmap_shipping, 2);
golden!(golden_vector_epl_crossing_lines, 3);
golden!(golden_vector_tspl_page_rounding, 4);
golden!(golden_vector_tspl_compositing, 5);
golden!(golden_vector_cpcl_barcodes, 6);
golden!(golden_vector_dpl_bottom_origin, 7);
golden!(golden_vector_one_dot_diagonals, 8);
golden!(golden_vector_unicode_and_rotated_text, 9);
golden!(golden_vector_native_content, 10);
golden!(golden_vector_multi_dot_diagonals, 11);

#[test]
#[ignore = "exports candidates only; review and explicitly update with e2e/vector/verify_pdf.py --update"]
fn export_vector_golden_candidates() {
    for case in vector_golden::cases() {
        vector_golden::export(&case);
    }
}
