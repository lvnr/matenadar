//! Lays Matenadar documents out with Typst: generates the layout program
//! from the model, compiles it, renders PDF, PNG and SVG, maps any point on
//! a page back to the node and character it shows, and checks the result
//! the way a printer would.

pub mod check;
pub mod editor;
pub mod effects;
pub mod generate;
pub mod glyphs;
pub mod inspect;
pub mod session;
pub mod world;

pub use check::{CheckOptions, Issue, Severity, check};
pub use editor::Editor;
pub use session::{Change, Changes, EngineError, PageInfo, Paths, Session, UsedFont, source_of};
pub use world::{Files, Fonts};
