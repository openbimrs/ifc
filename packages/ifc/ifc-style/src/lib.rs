//! `ifc-style` -- Presentation: colours, styles, textures and layer assignment.
//!
//!
//! 48 entities in IFC4. Deliberately **separate from geometry**: the kernel
//! must never carry a colour (see `docs/adr/0001` invariants), so appearance
//! lives here and is joined to shapes only at the consumer.
//!
//! # Module map
//!
//! | Module | Role |
//! |---|---|
//! | `colour` | `IfcColourRgb`, colour specification and normalisation |
//! | `surface_style` | `IfcSurfaceStyle` shading, rendering, lighting, refraction |
//! | `curve_style` | `IfcCurveStyle` fonts, widths and patterns |
//! | `texture` | `IfcSurfaceTexture` and UV coordinate mapping |
//! | `assignment` | `IfcStyledItem`: binding a style to a representation item |
//! | `layer` | `IfcPresentationLayerAssignment` and visibility |
//! | `error` | Why a style resolution failed |
//!
//! # Status
//!
//! Scaffold -- modules are reserved with intent, not implemented. See
//! `../PLAN.md` for the stage that fills them.

mod assignment;
mod colour;
mod curve_style;
mod error;
mod layer;
mod surface_style;
mod texture;
