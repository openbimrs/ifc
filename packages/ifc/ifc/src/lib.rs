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
//! | `material-templates` | `ifc-material` + template catalog | Material PSD applicability |
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

/// Versioned external PSD/QTO template catalogs and correction profiles.
#[cfg(feature = "property-catalog")]
pub use ifc_template_catalog as property_catalog;

/// Tasks, sequencing, calendars.
#[cfg(feature = "schedule")]
pub use ifc_schedule as schedule;

/// Material layer sets, profile sets, constituents.
#[cfg(feature = "material")]
pub use ifc_material as material;
#[cfg(feature = "material-templates")]
pub mod material_templates;

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

mod feature_report;
mod io;

pub use feature_report::compiled_features;
pub use io::{codecs, read_path};
