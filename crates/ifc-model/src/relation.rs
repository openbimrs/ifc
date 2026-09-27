//! Objectified relationships (`IfcRel*`).
//!
//! IFC models relationships as entities, not pointers -- voids, aggregation,
//! type assignment, property assignment and containment are all `IfcRel*`
//! instances. Traversal helpers belong here.
//!
//! Not implemented here: the relationship readers live in the domain crates
//! and `ifc-spatial`. Whether this module still reserves anything is
//! <https://github.com/openbimrs/ifc/issues/127>.
