//! `ifc-style` — presentation, styling, and annotation.
//!
//! # Why this is its own crate
//!
//! IFC4 carries **48 presentation/style entities**. Keeping them out of the
//! geometry kernel is a hard invariant of this project (`docs/adr/0001`): a
//! kernel whose base type knows about colour cannot be refactored without
//! touching a renderer.
//!
//! Separating them also means a headless checker never compiles texture
//! handling.
//!
//! # Scope
//!
//! - Surface/curve/fill styles, shading, rendering, lighting, refraction
//! - Colour (RGB, colour maps) and indexed colouring of tessellations
//! - Textures: image, pixel, blob; texture coordinate mapping
//! - Presentation layer assignment, annotation, draughting predefined styles
//!
//! # Not here
//!
//! Any rendering. This crate reports what a model *says* about appearance; how
//! that becomes pixels belongs to an application.
