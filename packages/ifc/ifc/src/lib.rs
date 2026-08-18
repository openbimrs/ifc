//! `ifc` — the facade. Pick your codecs and domains as cargo features.
//!
//! # The shape of the library
//!
//! ```text
//!   codecs                 model                  domain views
//!   ---------------        ---------------        ------------------
//!   ifc-step      \                        /      ifc-cost
//!   ifc-xml        >----->  ifc-model  <--<       ifc-schedule
//!   (ifc-json)    /         (entities)     \      ifc-properties, ...
//! ```
//!
//! Two separations hold this together, and both are enforced by tests rather
//! than convention:
//!
//! **1. The model knows no domain semantics.** [`Model`] stores
//! `(id, type_name, attributes)` and nothing else. It has never heard of a
//! cost item. Domain crates are *views* that borrow a `&Model` and interpret
//! it, so a build without them still reads and writes their data untouched.
//!
//! **2. The model knows no serialization.** [`Codec`] is a trait *in the model
//! crate*; `ifc-step` and `ifc-xml` implement it. IFC-JSON would be a third
//! implementation, requiring no change to the model.
//!
//! # Choosing features
//!
//! | Feature | Pulls in | For |
//! | --- | --- | --- |
//! | `step` *(default)* | `ifc-step` | Reading `.ifc` files |
//! | `ifcxml` | `ifc-xml` | Reading/writing `.ifcxml` |
//! | `schema` | `ifc-schema` | Subtype queries, conformant XML names |
//! | `cost`, `schedule`, ... | one domain crate each | Interpreting that domain |
//! | `codecs` | both codecs | |
//! | `domains` | every domain view | |
//! | `full` | everything | |
//!
//! A thin viewer takes `default-features = false, features = ["step"]` and
//! compiles no domain code and no geometry stack, while still round-tripping
//! every entity in the file.
//!
//! ```
//! # #[cfg(feature = "step")] {
//! use ifc::{Codec, StepCodec};
//!
//! let source = b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
//!                FILE_NAME('t.ifc','',( ''),(''),'','','');\n\
//!                FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n\
//!                #1= IFCCOSTITEM('guid',$,'Excavation',$,$,$,$);\n\
//!                ENDSEC;\nEND-ISO-10303-21;\n";
//!
//! let model = StepCodec.read_bytes(source).unwrap();
//! assert_eq!(model.len(), 1);
//!
//! // The cost entity is present and re-exportable with no `cost` feature on.
//! let out = StepCodec.write_bytes(&model).unwrap();
//! assert!(String::from_utf8_lossy(&out).contains("IFCCOSTITEM"));
//! # }
//! ```

// The model is always available: it is the common vocabulary.
pub use ifc_model::{codec, Codec, Entity, EntityId, Header, Model, ModelError, Value};

/// The STEP physical file codec (`.ifc`).
#[cfg(feature = "step")]
pub use ifc_step::StepCodec;

/// The ifcXML codec (`.ifcxml`).
#[cfg(feature = "ifcxml")]
pub use ifc_xml::XmlCodec;

/// The IFC schema as queryable data.
#[cfg(feature = "schema")]
pub use ifc_schema::{Schema, SchemaVersion};

/// Cost semantics as a borrowed view.
#[cfg(feature = "cost")]
pub use ifc_cost as cost;

/// Property sets and quantities.
#[cfg(feature = "properties")]
pub use ifc_properties as properties;

/// Tasks, sequencing, calendars.
#[cfg(feature = "schedule")]
pub use ifc_schedule as schedule;

/// Material layer sets, profile sets, constituents.
#[cfg(feature = "material")]
pub use ifc_material as material;

/// Classification, documents, libraries.
#[cfg(feature = "classification")]
pub use ifc_classification as classification;

/// Structural analysis model.
#[cfg(feature = "structural")]
pub use ifc_structural as structural;

/// Labour, equipment, crew resources.
#[cfg(feature = "resource")]
pub use ifc_resource as resource;

/// Distribution systems and ports.
#[cfg(feature = "systems")]
pub use ifc_systems as systems;

/// Presentation styles.
#[cfg(feature = "style")]
pub use ifc_style as style;

/// Schema and integrity validation.
#[cfg(feature = "validate")]
pub use ifc_validate as validate;

/// Representation lowering to geometry.
#[cfg(feature = "geometry")]
pub use ifc_geometry as geometry;

/// Map conversion and coordinate reference systems.
#[cfg(feature = "georef")]
pub use ifc_georef as georef;

/// IFC4x3 alignment and linear placement.
#[cfg(feature = "alignment")]
pub use ifc_alignment as alignment;

/// Every codec compiled into this build.
///
/// Lets an application accept whatever the user hands it without hard-coding a
/// format, and shrinks to nothing when only one codec is enabled.
// Each push is `cfg`-gated, so clippy's `vec![]` suggestion is not applicable:
// the contents depend on which features are enabled at compile time.
#[allow(clippy::vec_init_then_push)]
pub fn codecs() -> Vec<Box<dyn Codec>> {
    #[allow(unused_mut)]
    let mut out: Vec<Box<dyn Codec>> = Vec::new();
    #[cfg(feature = "step")]
    out.push(Box::new(ifc_step::StepCodec));
    #[cfg(feature = "ifcxml")]
    out.push(Box::new(ifc_xml::XmlCodec::default()));
    out
}

/// Read a file, choosing the codec by content sniffing then extension.
///
/// Returns [`ModelError::WrongFormat`] when no compiled-in codec recognizes the
/// input, which is a more useful failure than a syntax error from the wrong
/// parser.
pub fn read_path(path: &std::path::Path) -> Result<Model, ModelError> {
    let bytes = std::fs::read(path).map_err(|e| ModelError::Io(e.to_string()))?;
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let available = codecs();
    for codec in &available {
        if codec.detect(&bytes) {
            return codec.read_bytes(&bytes);
        }
    }
    for codec in &available {
        if codec.extensions().contains(&extension.as_str()) {
            return codec.read_bytes(&bytes);
        }
    }
    Err(ModelError::WrongFormat {
        expected: "IFC",
        detail: format!(
            "no compiled-in codec recognized this input (available: {})",
            available
                .iter()
                .map(|c| c.name())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    })
}

/// The feature set this build was compiled with, for diagnostics.
///
/// A support question about a thin build is much easier to answer when the
/// binary can state what it contains.
// Same rationale as `codecs`: the pushes are feature-gated, not a literal.
#[allow(clippy::vec_init_then_push)]
pub fn compiled_features() -> Vec<&'static str> {
    // `mut` is unused when no optional feature is enabled; the allow keeps the
    // no-feature build warning-clean without special-casing the body.
    #[allow(unused_mut)]
    let mut features = Vec::new();
    #[cfg(feature = "step")]
    features.push("step");
    #[cfg(feature = "ifcxml")]
    features.push("ifcxml");
    #[cfg(feature = "schema")]
    features.push("schema");
    #[cfg(feature = "properties")]
    features.push("properties");
    #[cfg(feature = "cost")]
    features.push("cost");
    #[cfg(feature = "schedule")]
    features.push("schedule");
    #[cfg(feature = "material")]
    features.push("material");
    #[cfg(feature = "classification")]
    features.push("classification");
    #[cfg(feature = "structural")]
    features.push("structural");
    #[cfg(feature = "resource")]
    features.push("resource");
    #[cfg(feature = "systems")]
    features.push("systems");
    #[cfg(feature = "style")]
    features.push("style");
    #[cfg(feature = "validate")]
    features.push("validate");
    #[cfg(feature = "geometry")]
    features.push("geometry");
    #[cfg(feature = "georef")]
    features.push("georef");
    #[cfg(feature = "alignment")]
    features.push("alignment");
    features
}
