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
//! silently producing wrong names.
//!
//! ```
//! use ifc_model::{Codec, Model, Entity, EntityId, Value};
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

pub mod error;
pub mod reader;
pub mod writer;

pub use error::XmlError;

use ifc_model::{Codec, Model, ModelError};

/// The ifcXML codec.
///
/// Construct with [`XmlCodec::default`] for positional fallback naming, or
/// with [`XmlCodec::with_schema`] to emit conformant attribute names.
#[derive(Default)]
pub struct XmlCodec {
    #[cfg(feature = "schema")]
    schema: Option<std::sync::Arc<ifc_schema::Schema>>,
}

impl XmlCodec {
    /// A codec that emits schema-correct attribute names.
    #[cfg(feature = "schema")]
    pub fn with_schema(schema: std::sync::Arc<ifc_schema::Schema>) -> Self {
        Self {
            schema: Some(schema),
        }
    }

    /// The schema in use, if any.
    #[cfg(feature = "schema")]
    pub fn schema(&self) -> Option<&ifc_schema::Schema> {
        self.schema.as_deref()
    }
}

impl Codec for XmlCodec {
    fn name(&self) -> &'static str {
        "ifcXML"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ifcxml", "xml"]
    }

    fn detect(&self, bytes: &[u8]) -> bool {
        reader::looks_like_xml(bytes)
    }

    fn read_bytes(&self, bytes: &[u8]) -> Result<Model, ModelError> {
        reader::read(self, bytes).map_err(|e| ModelError::Syntax {
            offset: 0,
            detail: e.to_string(),
        })
    }

    fn write(&self, model: &Model, out: &mut dyn std::io::Write) -> Result<(), ModelError> {
        let bytes = writer::write(self, model).map_err(|e| ModelError::Write(e.to_string()))?;
        out.write_all(&bytes)
            .map_err(|e| ModelError::Io(e.to_string()))
    }
}
