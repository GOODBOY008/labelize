//! CPCL frontend (Zebra mobile printers). Maps CPCL command syntax onto
//! the shared IR; resident font cells, header dpi scaling and QR payload
//! conventions are resolved here (never via the Zebra tables).

pub mod fonts;
pub mod format;
pub mod parser;
pub mod qr_payload;

pub use parser::CpclParser;
