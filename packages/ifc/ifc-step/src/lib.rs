//! `ifc-step` — STEP physical file (IFC-SPF) reader.
//!
//! # Design (carried over from a validated sibling implementation)
//!
//! The parse plan is: mmap the file, split it into partitions **aligned to
//! record starts**, scan partitions in parallel with rayon, then resolve
//! `#id` references into dense indices.
//!
//! # Module map
//!
//! Each stage of that pipeline is its own module, so no single file grows into
//! the whole reader:
//!
//! | Module | Role |
//! |---|---|
//! | [`error`] | `ParseError` — every way a parse can fail |
//! | [`header`] | `ISO-10303-21` magic, `FILE_SCHEMA`/`FILE_DESCRIPTION` |
//! | [`lexer`] | byte-level tokenizer over a record body |
//! | [`value`] | the STEP value model (refs, lists, typed values, enums) |
//! | [`partition`] | record-aligned splitting for the parallel scan |
//! | [`scan`] | parallel body scan producing raw records |
//! | [`resolve`] | `#id` → dense index resolution |
//! | [`escape`] | `\X\`, `\X2\`, `\X4\` string decoding |
//! | [`reader`] | the public façade tying the stages together |
//!
//! # Status
//!
//! Scaffold. Header detection is implemented and tested against real fixtures;
//! the remaining modules are Stage 1 in `docs/ROADMAP.md`.

pub mod error;
pub mod escape;
pub mod header;
pub mod lexer;
pub mod partition;
pub mod reader;
pub mod resolve;
pub mod scan;
pub mod value;

pub use error::ParseError;
pub use header::is_step_file;
