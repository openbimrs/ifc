//! Classification systems, hierarchical references, and IFC2X3 notations.
//!
//! Classification codes are identifiers, not numbers: `21.22` and `21.220`
//! are different codes, and a leading zero is significant. They are returned
//! as the authored text, and the reference hierarchy is kept as authored
//! rather than reconstructed by splitting codes.
mod notation;
mod reference;
mod system;
pub use notation::ClassificationNotation;
pub use reference::ClassificationReference;
pub use system::ClassificationSystem;
