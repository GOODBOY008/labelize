//! Per-language parser frontends.
//!
//! Adding a language (e.g. TSPL) means adding a sibling module that maps its
//! commands onto the shared `crate::elements` IR — it must not touch the
//! other languages' modules, `drawers`, or the existing elements' semantics.
//!
//! The `*_parser` / helper modules below are compatibility aliases for the
//! pre-reorganization layout; new code should use `parsers::zpl` /
//! `parsers::epl` directly.

pub mod cpcl;
pub mod dpl;
pub mod epl;
pub mod tspl;
pub mod zpl;

pub mod command_utils {
    pub use crate::parsers::zpl::command_utils::*;
}
pub mod cpcl_parser {
    pub use crate::parsers::cpcl::parser::*;
}
pub mod dpl_parser {
    pub use crate::parsers::dpl::parser::*;
}
pub mod epl_parser {
    pub use crate::parsers::epl::parser::*;
}
pub mod tspl_parser {
    pub use crate::parsers::tspl::parser::*;
}
pub mod fs {
    pub use crate::parsers::zpl::fs::*;
}
pub mod virtual_printer {
    pub use crate::parsers::zpl::virtual_printer::*;
}
pub mod zpl_parser {
    pub use crate::parsers::zpl::parser::*;
}

pub use cpcl::parser::CpclParser;
pub use dpl::parser::DplParser;
pub use epl::parser::EplParser;
pub use tspl::parser::TsplParser;
pub use zpl::parser::ZplParser;
