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
    BarcodeBlock, BarcodeEl, BarcodeSpec, Block, ChartBlock, ChartEl, ChartSeries, ChartSpec,
    Color, Component, Design, Document, Element, Grid, Inline, Item, LayoutCell, Length,
    MASTER_PRESETS, Master, Meta, Notes, OnPage, Op, OpError, PdfAttachment, PdfConfig, Place,
    Print, Problem, Row, RuleBlock, Run, Style, TextStyle, VERSION, master_preset_size, validate,
};

#[cfg(feature = "typst")]
pub use matenadar_typst::{
    Change, Changes, CheckOptions, Editor, EngineError, Files, Fonts, Issue, PageInfo, Paths,
    PdfAttachmentData, PdfOptions, Session, Severity, SourceExport, SourceNode, UsedFont, check,
    inspect, source_of,
};
