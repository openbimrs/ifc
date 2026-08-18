//! Constructive solid geometry and clipping.
//!
//! `IfcBooleanResult`, `IfcBooleanClippingResult`, `IfcHalfSpaceSolid` and the
//! polygonal bounded half space. This module builds the operation tree and hands
//! the boolean itself to the injected kernel -- it never implements a boolean.
//!
//! # Pitfall
//!
//! `IfcHalfSpaceSolid` is unbounded. Naively meshing it produces the
//! 'halfspace flyaway' artefact (see the fixture of that name): it must be
//! clipped against the subject's bounds before use.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.
