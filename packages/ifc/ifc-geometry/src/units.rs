//! Unit scaling into kernel units.
//!
//! IFC models declare their own length and angle units: millimetres and metres
//! both occur, as do degrees and radians. Every coordinate must be scaled on the
//! way into the kernel, and the tolerance chosen accordingly -- a fixed
//! millimetre epsilon is wrong in a metre model and vice versa.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.
