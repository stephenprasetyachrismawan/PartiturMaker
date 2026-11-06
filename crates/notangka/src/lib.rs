//! Inti PartiturMaker: bahasa Nada untuk not angka.
//!
//! Alur: teks `.nada` -> [`parse`] -> [`model::Doc`] -> [`layout`] -> SVG / PDF.
//! Editor visual memanggil [`edit::apply`] lalu [`format::format`] untuk
//! menulis ulang kodenya.

#![allow(
    clippy::manual_is_multiple_of,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

pub mod api;
pub mod edit;
pub mod font;
pub mod format;
pub mod layout;
pub mod model;
pub mod parse;
pub mod pdf;
pub mod svg;

pub use format::{format, format_source};
pub use parse::{parse, Diagnostic, Severity};

/// Contoh lagu bawaan (melodi dan syair dibuat khusus, bebas dipakai).
pub const EXAMPLES: &[(&str, &str)] = &[
    (
        "Muliakan Nama-Nya (4 suara)",
        include_str!("../examples-nada/muliakan.nada"),
    ),
    (
        "Fajar Pagi (melodi)",
        include_str!("../examples-nada/fajar.nada"),
    ),
];
