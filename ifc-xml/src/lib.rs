//! `ifc-xml` — the ifcXML (ISO 10303-28) codec.
//!
//! # Why this crate exists
//!
//! It is the proof that serialization is genuinely pluggable. It implements
//! the same [`ifc_model::Codec`] trait as `ifc-step`, over the same
//! [`ifc_model::Model`], and the model needed **no change** to accommodate it.
//! A third encoding (IFC-JSON) would be another crate beside these two.
//!
//! # The interesting difference from STEP
//!
//! STEP records are **positional**: `#5=IFCWALL('guid',#1,$)`. ifcXML is
//! **named**: `<IfcWall id="i5" GlobalId="guid" .../>`. Crossing between them
//! needs the schema to map slot 0 to `GlobalId`.
//!
//! That would make the schema a hard dependency of the codec, which would
//! break round-tripping for files whose schema we do not have. So the schema
//! is **optional**:
//!
//! - **with** a schema: conformant named attributes.
//! - **without**: positional fallback names (`a0`, `a1`, ...).
//!
//! Both round-trip losslessly. Only the first is interoperable with other
//! tools, and the fallback is clearly marked in the output rather than
//! silently producing wrong names. Namespace conformance is separately explicit:
//! [`XmlCodec::strict`] selects one exact [`XmlProfile`], while the default keeps
//! the historical compatibility dialect without claiming XSD conformance.
//!
//! ```
//! use ifc_model::{Codec, Entity, EntityId, Model, Value};
//! use ifc_xml::XmlCodec;
//!
//! let mut model = Model::new();
//! model.insert(
//!     EntityId(1),
//!     Entity::new("IFCCOSTITEM", vec![Value::Text("Excavation".into())]),
//! );
//!
//! let bytes = XmlCodec::default().write_bytes(&model).unwrap();
//! let reparsed = XmlCodec::default().read_bytes(&bytes).unwrap();
//! assert_eq!(&*reparsed.get(EntityId(1)).unwrap().type_name, "IFCCOSTITEM");
//! ```

mod codec;
pub mod error;
mod profile;
pub mod reader;
pub mod writer;

pub use codec::XmlCodec;
pub use error::{XmlError, XmlPath};
pub use profile::XmlProfile;
