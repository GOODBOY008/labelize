//! ZPL frontend. Everything that knows "how Zebra renders ZPL" lives under
//! this module: the command parser and its `VirtualPrinter` state machine,
//! the ^CI charset tables, ^MU measurement units, and stored-format
//! resolution. Parsers produce the shared, language-neutral IR in
//! `crate::elements` and never the other way around.

mod barcode_commands;
mod binary_graphics;
mod bitmap_font;
pub mod charsets;
pub mod command_utils;
mod field_clock;
pub mod format;
pub mod fs;
pub mod measurement_unit;
mod objects;
pub mod parser;
pub mod qr_payload;
pub mod stored_formats;
mod stream;
mod variable_fields;
pub mod virtual_printer;
pub mod zebra_fonts;

pub use parser::{ZplParseOptions, ZplParser};
