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
//! Both round-trip losslessly, and the fallback is clearly marked in the
//! output rather than silently producing wrong names. Namespace conformance
//! is separately explicit: [`XmlCodec::strict`] selects one exact
//! [`XmlProfile`], while the default keeps the historical compatibility
//! dialect.
//!
//! Neither is valid against the release XSD. The strict profile writes this
//! crate's own layout under the XSD's target namespace and the release's
//! schema token; it does not write the XSD configuration (upper-case STEP
//! type names, `i<n>` ids, `kind` elements and a `schema` root attribute
//! are its own). For output the XSD accepts, use [`XmlCodec::xsd`] (below).
//! An opt-in test, `tests/xsd_output.rs`, validates both with `xmllint`
//! against the fetched XSDs: strict output departs only as it documents,
//! XSD-layout output has no validity error at all.
//!
//! # Reading with a schema is strict
//!
//! XML attribute values are untyped strings. With a schema, the reader types
//! each value from its attribute's declaration ([`SchemaReading::Strict`],
//! the default): `Name="1"` is the label `'1'`, never the integer `1`, and a
//! name the entity does not declare is an [`XmlError::UnknownAttribute`]
//! naming entity, element and attribute, never a value in the next free
//! slot. [`SchemaReading::Lenient`] restores inference, which round-trips
//! models the schema does not describe at the price of misreading such
//! values.
//!
//! # The buildingSMART XSD configuration
//!
//! [`XmlCodec::xsd`] reads and writes the ifcXML configuration the release
//! XSD declares (IFC4 ADD2 TC1, IFC4X3 ADD2): entities nested and defined in
//! place, `ref`/`href` references, inverse attributes, `-wrapper` typed
//! values, space-separated list attributes. It reads into the same
//! [`ifc_model::Model`] as the document's STEP form, and writes that model
//! back: every entity at the top level, references as `ref` with `xsi:nil`,
//! the attributes the configuration leaves off a relationship through the
//! inverse of the entity they name. Written documents validate against
//! `IFC4.xsd` and read back to the same model. Both directions refuse with
//! a typed error what they cannot carry exactly; a model the configuration
//! cannot represent is [`XmlError::Unrepresentable`], never different
//! output. Reader and writer share one derivation of each attribute's form
//! from the schema; the rules are in the `xsd` module documentation, and a
//! schema-backed test checks them, element by element, against both XSDs.
//!
//! ```
//! # #[cfg(feature = "schema")] {
//! use ifc_xml::{XmlCodec, XmlProfile};
//! use std::sync::Arc;
//!
//! # let schema = ifc_schema::Schema::from_express(
//! #     "SCHEMA IFC4; ENTITY IfcPerson; FamilyName : OPTIONAL STRING; END_ENTITY; END_SCHEMA;",
//! # );
//! // `schema` is the IFC4 schema, e.g. `ifc_schema::ifc4()`.
//! let codec = XmlCodec::xsd(Arc::new(schema), XmlProfile::Ifc4Add2Tc1);
//! let xml = br#"<ifcXML xmlns="https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/XML">
//!   <IfcPerson FamilyName="1"/>
//! </ifcXML>"#;
//! let model = ifc_xml::reader::read(&codec, xml).unwrap();
//! let person = model.get(ifc_model::EntityId(1)).unwrap();
//! assert_eq!(&*person.type_name, "IFCPERSON");
//! assert_eq!(person.text(0), Some("1"));
//! # }
//! ```
//!
//! ```
//! # #[cfg(feature = "schema")] {
//! use ifc_model::{Codec, Entity, EntityId, Model, Value};
//! use ifc_xml::{XmlCodec, XmlProfile};
//! use std::sync::Arc;
//!
//! # let schema = ifc_schema::Schema::from_express(
//! #     "SCHEMA IFC4; ENTITY IfcPerson; FamilyName : OPTIONAL STRING; END_ENTITY; END_SCHEMA;",
//! # );
//! let codec = XmlCodec::xsd(Arc::new(schema), XmlProfile::Ifc4Add2Tc1);
//! let mut model = Model::new();
//! model.header_mut().schema = vec!["IFC4".into()];
//! model.insert(EntityId(1), Entity::new("IFCPERSON", vec![Value::Text("1".into())]));
//! let xml = String::from_utf8(codec.write_bytes(&model).unwrap()).unwrap();
//! assert!(xml.contains(r#"<IfcPerson id="i1" FamilyName="1"/>"#));
//! assert_eq!(codec.read_bytes(xml.as_bytes()).unwrap().get(EntityId(1)), model.get(EntityId(1)));
//! # }
//! ```
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
mod scalar;
mod slots;
#[cfg(feature = "schema")]
mod typing;
pub mod writer;
#[cfg(feature = "schema")]
mod xsd;

pub use codec::{SchemaReading, XmlCodec, XmlLayout};
pub use error::{XmlError, XmlPath};
pub use profile::XmlProfile;
