//! Compatibility alias: the ^CI charset tables moved to
//! `parsers::zpl::charsets` (they are ZPL-only; no other language consumes
//! them). This module keeps the historical `labelize::encodings` path alive.

pub use crate::parsers::zpl::charsets::*;
