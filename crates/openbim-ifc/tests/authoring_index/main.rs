//! The indexes behind a checked authoring batch change its cost, never its
//! answer (#587).
//!
//! A batch is staged against a model whose indexes the model keeps current,
//! and against a clone whose indexes were dropped and rebuild on demand.
//! Over a history of batches, undos and redos, and over the cascade
//! fixtures of three releases, both must give the same edits, ids,
//! reports and copies, or the same refusal; after every write the model's
//! reverse index must equal a fresh build; a refused batch leaves the
//! entities and the revision as they were. Where the model is ambiguous
//! (an object in two relationships), the refusal names the one the type
//! index lists first.

#![cfg(all(
    feature = "step",
    feature = "authoring",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

mod histories;
mod lab;
mod typing;
