//! Lays Matenadar documents out with Typst: generates the layout program
//! from the model, compiles it, renders PDF, PNG and SVG, maps any point on
//! a page back to the node and character it shows, and checks the result
//! the way a printer would.

pub mod check;
pub mod generate;
pub mod glyphs;
pub mod inspect;
pub mod session;
pub mod world;

pub use check::{CheckOptions, Issue, Severity, check};
pub use session::{Change, EngineError, Paths, Session};
pub use world::Fonts;
