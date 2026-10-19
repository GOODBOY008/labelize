//! TSPL/TSC frontend. Maps TSC command syntax onto the shared IR; TSPL font
//! cells and SIZE dimensions are resolved here (never via the Zebra tables).

pub mod fonts;
pub mod format;
pub mod parser;

pub use parser::TsplParser;
