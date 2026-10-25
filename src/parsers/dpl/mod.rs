//! DPL frontend (Datamax-O'Neil / Honeywell). Maps DPL label-format
//! records onto the shared IR; font 0-8 cells, the smooth font 9 point
//! table and the bottom-left home position are resolved here (never via
//! the Zebra tables).

pub mod fonts;
pub mod format;
pub mod parser;
pub mod qr_payload;

pub use parser::DplParser;
