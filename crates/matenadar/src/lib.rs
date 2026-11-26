//! Matenadar: a typesetting engine for anything that prints, for people and agents.
//!
//! This crate is a thin facade over the Matenadar workspace crates—and also
//! provides the `matenadar` CLI binary (`cargo install matenadar`):
//!
//! - [`model`] (`matenadar-model`): JSON document model, validation,
//!   and reversible edit operations (no Typst dependency).
//! - [`typst`] (`matenadar-typst`, feature `typst`): Typst layout engine,
//!   PDF/PNG/SVG rendering, hit-testing, layout inspection, and preflight
//!   checks.
//! - [`epub`] (`matenadar-epub`, feature `epub`): reflowable and fixed-layout
//!   EPUB 3 export.
//!
//! If you only need part of the stack, you can disable default features on
//! `matenadar` or depend on `matenadar-model`, `matenadar-typst`, or
//! `matenadar-epub` directly.

pub use matenadar_model as model;
#[cfg(feature = "typst")]
pub use matenadar_typst as typst;
#[cfg(feature = "epub")]
pub use matenadar_epub as epub;

pub use matenadar_model::{
    Block, Color, Component, Design, Document, Element, Inline, Item, Length, Master, Meta,
    OnPage, Op, OpError, Place, Print, Problem, Run, Style, TextStyle, VERSION, validate,
};

#[cfg(feature = "typst")]
pub use matenadar_typst::{
    Change, Changes, CheckOptions, Editor, EngineError, Files, Fonts, Issue, PageInfo, Paths,
    Session, Severity, SourceExport, SourceNode, UsedFont, check, inspect, source_of,
};
