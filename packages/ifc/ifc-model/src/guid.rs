//! IFC GlobalId encoding and decoding.
//!
//! IFC uses a base-64 variant of a 128-bit GUID, 22 characters long, with its
//! own alphabet. Round-tripping it correctly matters for `bcf` and `diff`,
//! which identify elements across files by GUID.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.
