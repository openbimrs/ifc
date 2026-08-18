//! `ifc` — the facade. Pick your codecs and domains as cargo features.
//!
//! # The shape of the library
//!
//! ```text
//!   codecs                 model                  domain views
//!   ------                 -----                  ------------
//!   ifc-step  ──┐                              ┌── ifc-cost
//!   (ifcXML)  ──┼──> Codec ──> ifc-model <─────┼── ifc-schedule
//!   (IFC-JSON)──┘                              └── ifc-properties, ...
//! ```
//!
//! Two separations hold this together, and both are enforced by tests rather
//! than convention (see `docs/adr/0006`):
//!
//! **The model knows no domain semantics.** [`Model`] stores entities
//! structurally. It cannot tell you what a cost item means — a domain crate
//! does that by borrowing it. So data this build has no crate for is still
//! parsed, stored, and re-exported intact.
//!
//! **The model knows no serialization.** STEP is one [`Codec`]; ifcXML and
//! IFC-JSON are others. Converting between them is "read with one, write with
//! another".
//!
//! # Choosing features
//!
//! | Build | Features | Gets you |
//! | --- | --- | --- |
//! | thin file mover | `default` (= `step`) | parse, edit ids, re-export |
//! | quantity surveying | `quantities` | cost, schedule, properties |
//! | full toolkit | `full` | everything |
//!
//! A file containing cost data round-trips losslessly even in the thin build.
//! That is the property that makes feature-gating safe, and it is verified in
//! `ifc-step/tests/roundtrip.rs` and `tests/thin_build.rs`.
//!
//! # Example
//!
//! ```no_run
//! # #[cfg(feature = "step")] {
//! use ifc::{Codec, StepCodec};
//!
//! let codec = StepCodec;
//! let model = codec.read_path(std::path::Path::new("model.ifc")).unwrap();
//! println!("{} entities", model.len());
//! # }
//! ```

// The model is always available: it is the common vocabulary.
pub use ifc_model::{Codec, Entity, EntityId, Header, Model, ModelError, ModelResult, Value};

/// The STEP physical file codec.
#[cfg(feature = "step")]
pub use ifc_step::StepCodec;

/// Re-exported domain views. Each is present only when its feature is on.
pub mod domain {
    #[cfg(feature = "alignment")]
    pub use ifc_alignment;
    #[cfg(feature = "classification")]
    pub use ifc_classification;
    #[cfg(feature = "cost")]
    pub use ifc_cost;
    #[cfg(feature = "geometry")]
    pub use ifc_geometry;
    #[cfg(feature = "georef")]
    pub use ifc_georef;
    #[cfg(feature = "material")]
    pub use ifc_material;
    #[cfg(feature = "properties")]
    pub use ifc_properties;
    #[cfg(feature = "resource")]
    pub use ifc_resource;
    #[cfg(feature = "schedule")]
    pub use ifc_schedule;
    #[cfg(feature = "structural")]
    pub use ifc_structural;
    #[cfg(feature = "style")]
    pub use ifc_style;
    #[cfg(feature = "systems")]
    pub use ifc_systems;
    #[cfg(feature = "validate")]
    pub use ifc_validate;
}

/// Schema services, when the `schema` feature is on.
#[cfg(feature = "schema")]
pub use ifc_schema;

/// Which optional features this build was compiled with.
///
/// Exposed so an application can report its own capabilities, and so the
/// feature wiring is observable rather than a matter of trust.
pub fn compiled_features() -> Vec<&'static str> {
    let mut v = Vec::new();
    if cfg!(feature = "step") {
        v.push("step");
    }
    if cfg!(feature = "schema") {
        v.push("schema");
    }
    if cfg!(feature = "cost") {
        v.push("cost");
    }
    if cfg!(feature = "schedule") {
        v.push("schedule");
    }
    if cfg!(feature = "properties") {
        v.push("properties");
    }
    if cfg!(feature = "material") {
        v.push("material");
    }
    if cfg!(feature = "classification") {
        v.push("classification");
    }
    if cfg!(feature = "structural") {
        v.push("structural");
    }
    if cfg!(feature = "resource") {
        v.push("resource");
    }
    if cfg!(feature = "systems") {
        v.push("systems");
    }
    if cfg!(feature = "style") {
        v.push("style");
    }
    if cfg!(feature = "validate") {
        v.push("validate");
    }
    if cfg!(feature = "geometry") {
        v.push("geometry");
    }
    if cfg!(feature = "georef") {
        v.push("georef");
    }
    if cfg!(feature = "alignment") {
        v.push("alignment");
    }
    v
}
