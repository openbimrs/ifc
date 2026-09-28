//! Compiled binary artifact for a bundled EXPRESS schema.
//!
//! Mirrors `ifc-template-catalog`'s archive pattern: a versioned, checksummed
//! binary encoding that `include_bytes!` ships inside the crate, decoded once
//! behind a `OnceLock`. The wire format stores the already-parsed
//! declarations (entities, types, attributes) rather than EXPRESS source
//! text, so a consumer pays a bincode decode, not an EXPRESS parse.
//!
//! The wire structs below are private mirrors of this crate's public
//! declaration types. They decouple the byte layout from the public API: a
//! field added to [`EntityDef`] does not change the format until this module
//! chooses to record it, under a new `FORMAT_VERSION`.
//!
//! # Versions
//!
//! | Version | Records |
//! | --- | --- |
//! | 1 | names, supertypes, attributes (with an aggregate flag), derived names |
//! | 2 | adds WHERE-rule labels |
//! | 3 | adds aggregation levels with bounds, INVERSE attributes, UNIQUE rules (#111) |
//!
//! bincode is positional, so each older shape survives as its own type and
//! is lifted into the current one: what it never recorded becomes empty,
//! which is exactly what the older table stated.

use bincode::{Decode, Encode};
use thiserror::Error;

use crate::attribute::{AggregateKind, Aggregation, Attribute, Bound};
use crate::entity::{EntityDef, InverseAttribute, UniqueRule, WhereRule};
use crate::registry::Schema;
use crate::types::{TypeDef, TypeKind};

const MAGIC: [u8; 8] = *b"NEHSCHM\0";
const FORMAT_VERSION: u16 = 3;
const MIN_HEADER_BYTES: usize = MAGIC.len() + 1;
const MAX_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;

/// Wire-format mirror of [`AggregateKind`].
#[derive(Encode, Decode)]
enum WireAggregateKind {
    List,
    Set,
    Bag,
    Array,
}

/// Wire-format mirror of [`Bound`].
#[derive(Encode, Decode)]
enum WireBound {
    Integer(u64),
    Unbounded,
    Expression(String),
}

/// Wire-format mirror of [`Aggregation`].
#[derive(Encode, Decode)]
struct WireAggregation {
    kind: WireAggregateKind,
    lower: WireBound,
    upper: WireBound,
    unique: bool,
    optional_elements: bool,
}

/// Wire-format mirror of [`Attribute`] (v3).
#[derive(Encode, Decode)]
struct WireAttribute {
    name: String,
    type_name: String,
    optional: bool,
    aggregate: bool,
    aggregation: Vec<WireAggregation>,
}

/// Wire-format mirror of [`InverseAttribute`].
#[derive(Encode, Decode)]
struct WireInverse {
    name: String,
    redeclares: Option<String>,
    entity: String,
    for_attribute: String,
    aggregation: Option<WireAggregation>,
}

/// Wire-format mirror of [`UniqueRule`].
#[derive(Encode, Decode)]
struct WireUnique {
    label: Option<String>,
    attributes: Vec<String>,
}

/// Wire-format mirror of [`WhereRule`].
#[derive(Encode, Decode)]
struct WireWhereRule {
    label: String,
    expression: String,
}

/// Wire-format mirror of [`EntityDef`] (v3).
#[derive(Encode, Decode)]
struct WireEntity {
    name: String,
    supertype: Option<String>,
    abstract_: bool,
    attributes: Vec<WireAttribute>,
    derived: Vec<String>,
    where_rules: Vec<WireWhereRule>,
    inverses: Vec<WireInverse>,
    unique_rules: Vec<WireUnique>,
}

/// Wire-format mirror of [`TypeKind`].
#[derive(Encode, Decode)]
enum WireTypeKind {
    Defined(String),
    Enumeration(Vec<String>),
    Select(Vec<String>),
}

/// Wire-format mirror of [`TypeDef`].
#[derive(Encode, Decode)]
struct WireType {
    name: String,
    kind: WireTypeKind,
}

#[derive(Encode, Decode)]
struct WireSchema {
    name: String,
    entities: Vec<WireEntity>,
    types: Vec<WireType>,
}

/// The v1/v2 attribute shape: an aggregate flag, no bounds.
#[derive(Encode, Decode)]
struct WireAttributeV2 {
    name: String,
    type_name: String,
    optional: bool,
    aggregate: bool,
}

/// The v2 entity shape: WHERE-rule labels, no INVERSE or UNIQUE.
#[derive(Encode, Decode)]
struct WireEntityV2 {
    name: String,
    supertype: Option<String>,
    abstract_: bool,
    attributes: Vec<WireAttributeV2>,
    derived: Vec<String>,
    where_rules: Vec<WireWhereRule>,
}

/// The v1 entity shape, from before `where_rules` existed.
#[derive(Encode, Decode)]
struct WireEntityV1 {
    name: String,
    supertype: Option<String>,
    abstract_: bool,
    attributes: Vec<WireAttributeV2>,
    derived: Vec<String>,
}

#[derive(Encode, Decode)]
struct WireSchemaV2 {
    name: String,
    entities: Vec<WireEntityV2>,
    types: Vec<WireType>,
}

#[derive(Encode, Decode)]
struct WireSchemaV1 {
    name: String,
    entities: Vec<WireEntityV1>,
    types: Vec<WireType>,
}

impl From<WireEntityV1> for WireEntityV2 {
    fn from(old: WireEntityV1) -> Self {
        Self {
            name: old.name,
            supertype: old.supertype,
            abstract_: old.abstract_,
            attributes: old.attributes,
            derived: old.derived,
            where_rules: Vec::new(),
        }
    }
}

impl From<WireSchemaV1> for WireSchemaV2 {
    fn from(old: WireSchemaV1) -> Self {
        Self {
            name: old.name,
            entities: old.entities.into_iter().map(WireEntityV2::from).collect(),
            types: old.types,
        }
    }
}

impl From<WireAttributeV2> for WireAttribute {
    fn from(old: WireAttributeV2) -> Self {
        Self {
            name: old.name,
            type_name: old.type_name,
            optional: old.optional,
            aggregate: old.aggregate,
            aggregation: Vec::new(),
        }
    }
}

impl From<WireEntityV2> for WireEntity {
    fn from(old: WireEntityV2) -> Self {
        Self {
            name: old.name,
            supertype: old.supertype,
            abstract_: old.abstract_,
            attributes: old
                .attributes
                .into_iter()
                .map(WireAttribute::from)
                .collect(),
            derived: old.derived,
            where_rules: old.where_rules,
            inverses: Vec::new(),
            unique_rules: Vec::new(),
        }
    }
}

impl From<WireSchemaV2> for WireSchema {
    fn from(old: WireSchemaV2) -> Self {
        Self {
            name: old.name,
            entities: old.entities.into_iter().map(WireEntity::from).collect(),
            types: old.types,
        }
    }
}

fn wire_aggregation(aggregation: &Aggregation) -> WireAggregation {
    let bound = |bound: &Bound| match bound {
        Bound::Integer(value) => WireBound::Integer(*value),
        Bound::Unbounded => WireBound::Unbounded,
        Bound::Expression(text) => WireBound::Expression(text.clone()),
    };
    WireAggregation {
        kind: match aggregation.kind {
            AggregateKind::List => WireAggregateKind::List,
            AggregateKind::Set => WireAggregateKind::Set,
            AggregateKind::Bag => WireAggregateKind::Bag,
            AggregateKind::Array => WireAggregateKind::Array,
        },
        lower: bound(&aggregation.lower),
        upper: bound(&aggregation.upper),
        unique: aggregation.unique,
        optional_elements: aggregation.optional_elements,
    }
}

fn owned_aggregation(wire: WireAggregation) -> Aggregation {
    let bound = |bound: WireBound| match bound {
        WireBound::Integer(value) => Bound::Integer(value),
        WireBound::Unbounded => Bound::Unbounded,
        WireBound::Expression(text) => Bound::Expression(text),
    };
    let kind = match wire.kind {
        WireAggregateKind::List => AggregateKind::List,
        WireAggregateKind::Set => AggregateKind::Set,
        WireAggregateKind::Bag => AggregateKind::Bag,
        WireAggregateKind::Array => AggregateKind::Array,
    };
    let mut owned = Aggregation::new(kind, bound(wire.lower), bound(wire.upper));
    owned.unique = wire.unique;
    owned.optional_elements = wire.optional_elements;
    owned
}

impl From<&Schema> for WireSchema {
    fn from(schema: &Schema) -> Self {
        Self {
            name: schema.name().to_owned(),
            entities: schema
                .entities()
                .map(|entity| WireEntity {
                    name: entity.name.clone(),
                    supertype: entity.supertype().map(str::to_owned),
                    abstract_: entity.abstract_,
                    attributes: entity
                        .attributes
                        .iter()
                        .map(|attribute| WireAttribute {
                            name: attribute.name.clone(),
                            type_name: attribute.type_name.clone(),
                            optional: attribute.optional,
                            aggregate: attribute.aggregate,
                            aggregation: attribute
                                .aggregation
                                .iter()
                                .map(wire_aggregation)
                                .collect(),
                        })
                        .collect(),
                    derived: entity.derived.clone(),
                    where_rules: entity
                        .where_rules
                        .iter()
                        .map(|rule| WireWhereRule {
                            label: rule.label.clone(),
                            expression: rule.expression.clone(),
                        })
                        .collect(),
                    inverses: entity
                        .inverses
                        .iter()
                        .map(|inverse| WireInverse {
                            name: inverse.name.clone(),
                            redeclares: inverse.redeclares.clone(),
                            entity: inverse.entity.clone(),
                            for_attribute: inverse.for_attribute.clone(),
                            aggregation: inverse.aggregation.as_ref().map(wire_aggregation),
                        })
                        .collect(),
                    unique_rules: entity
                        .unique_rules
                        .iter()
                        .map(|rule| WireUnique {
                            label: rule.label.clone(),
                            attributes: rule.attributes.clone(),
                        })
                        .collect(),
                })
                .collect(),
            types: schema
                .types()
                .map(|type_def| WireType {
                    name: type_def.name.clone(),
                    kind: match &type_def.kind {
                        TypeKind::Defined(alias) => WireTypeKind::Defined(alias.clone()),
                        TypeKind::Enumeration(members) => {
                            WireTypeKind::Enumeration(members.clone())
                        }
                        TypeKind::Select(members) => WireTypeKind::Select(members.clone()),
                    },
                })
                .collect(),
        }
    }
}

impl From<WireSchema> for Schema {
    fn from(wire: WireSchema) -> Self {
        let entities = wire
            .entities
            .into_iter()
            .map(|entity| {
                let mut def = EntityDef::new(entity.name);
                if let Some(supertype) = entity.supertype {
                    def = def.with_supertype(supertype);
                }
                def.abstract_ = entity.abstract_;
                def.attributes = entity
                    .attributes
                    .into_iter()
                    .map(|attribute| {
                        let mut built = Attribute::new(attribute.name, attribute.type_name);
                        built.optional = attribute.optional;
                        built.aggregate = attribute.aggregate;
                        built.aggregation = attribute
                            .aggregation
                            .into_iter()
                            .map(owned_aggregation)
                            .collect();
                        built
                    })
                    .collect();
                def.derived = entity.derived;
                def.where_rules = entity
                    .where_rules
                    .into_iter()
                    .map(|rule| WhereRule::new(rule.label, rule.expression))
                    .collect();
                def.inverses = entity
                    .inverses
                    .into_iter()
                    .map(|inverse| {
                        let mut built = InverseAttribute::new(
                            inverse.name,
                            inverse.entity,
                            inverse.for_attribute,
                        );
                        built.redeclares = inverse.redeclares;
                        built.aggregation = inverse.aggregation.map(owned_aggregation);
                        built
                    })
                    .collect();
                def.unique_rules = entity
                    .unique_rules
                    .into_iter()
                    .map(|rule| {
                        let mut built = UniqueRule::unlabelled(rule.attributes);
                        built.label = rule.label;
                        built
                    })
                    .collect();
                def
            })
            .collect();
        let types = wire
            .types
            .into_iter()
            .map(|type_def| {
                let kind = match type_def.kind {
                    WireTypeKind::Defined(alias) => TypeKind::Defined(alias),
                    WireTypeKind::Enumeration(members) => TypeKind::Enumeration(members),
                    WireTypeKind::Select(members) => TypeKind::Select(members),
                };
                TypeDef::new(type_def.name, kind)
            })
            .collect();
        Schema::new(wire.name, entities, types)
    }
}

/// Decodes a compiled schema artifact produced by `encode_schema`
/// (the `generation` feature).
///
/// # Errors
///
/// Returns `BundledSchemaError` if the artifact is malformed, oversized, or
/// carries an unsupported format version.
pub fn decode_schema(bytes: &[u8]) -> Result<Schema, BundledSchemaError> {
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err(BundledSchemaError::TooLarge {
            actual: bytes.len(),
            limit: MAX_ARTIFACT_BYTES,
        });
    }
    if !bytes.starts_with(&MAGIC) {
        return Err(BundledSchemaError::BadMagic);
    }
    if bytes.len() < MIN_HEADER_BYTES {
        return Err(BundledSchemaError::TruncatedHeader {
            actual: bytes.len(),
            required: MIN_HEADER_BYTES,
        });
    }
    let header_config = bincode::config::standard().with_limit::<16>();
    let (format_version, version_bytes): (u16, usize) =
        bincode::decode_from_slice(&bytes[MAGIC.len()..], header_config)
            .map_err(|error| BundledSchemaError::Decode(error.to_string()))?;
    let payload_bytes = &bytes[MAGIC.len() + version_bytes..];
    let config = bincode::config::standard().with_limit::<MAX_ARTIFACT_BYTES>();
    // Older payloads decode through their own shapes and are lifted (see the
    // module docs): what they never recorded becomes empty.
    let (wire, consumed): (WireSchema, usize) = match format_version {
        1 => {
            let (old, consumed): (WireSchemaV1, usize) =
                bincode::decode_from_slice(payload_bytes, config)
                    .map_err(|error| BundledSchemaError::Decode(error.to_string()))?;
            (WireSchema::from(WireSchemaV2::from(old)), consumed)
        }
        2 => {
            let (old, consumed): (WireSchemaV2, usize) =
                bincode::decode_from_slice(payload_bytes, config)
                    .map_err(|error| BundledSchemaError::Decode(error.to_string()))?;
            (WireSchema::from(old), consumed)
        }
        FORMAT_VERSION => bincode::decode_from_slice(payload_bytes, config)
            .map_err(|error| BundledSchemaError::Decode(error.to_string()))?,
        other => return Err(BundledSchemaError::UnsupportedVersion(other)),
    };
    if consumed != payload_bytes.len() {
        return Err(BundledSchemaError::TrailingBytes(
            payload_bytes.len() - consumed,
        ));
    }
    Ok(wire.into())
}

/// Encodes `schema` into the versioned compiled artifact format.
///
/// # Errors
///
/// Returns a bincode encode error if `schema` cannot be serialized.
#[cfg(feature = "generation")]
pub fn encode_schema(schema: &Schema) -> Result<Vec<u8>, bincode::error::EncodeError> {
    let wire = WireSchema::from(schema);
    let payload = bincode::encode_to_vec(wire, bincode::config::standard())?;
    let version = bincode::encode_to_vec(FORMAT_VERSION, bincode::config::standard())?;
    let mut bytes = Vec::with_capacity(MAGIC.len() + version.len() + payload.len());
    bytes.extend_from_slice(&MAGIC);
    bytes.extend_from_slice(&version);
    bytes.extend_from_slice(&payload);
    Ok(bytes)
}

/// Failure decoding a compiled schema artifact.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum BundledSchemaError {
    /// The payload is not a valid encoding of the declared format version.
    #[error("cannot decode schema artifact: {0}")]
    Decode(String),
    /// The input exceeds the decoder's resource budget.
    #[error("schema artifact is {actual} bytes; limit is {limit} bytes")]
    TooLarge {
        /// Input length in bytes.
        actual: usize,
        /// Largest accepted input in bytes.
        limit: usize,
    },
    /// The input ends inside the header.
    #[error("schema artifact header is {actual} bytes; at least {required} bytes are required")]
    TruncatedHeader {
        /// Input length in bytes.
        actual: usize,
        /// Shortest possible header in bytes.
        required: usize,
    },
    /// The input does not start with the artifact magic.
    #[error("schema artifact magic is invalid")]
    BadMagic,
    /// The header names a format version this build cannot read.
    #[error("unsupported schema artifact format version {0}")]
    UnsupportedVersion(u16),
    /// Bytes follow the decoded payload.
    #[error("schema artifact has {0} trailing bytes")]
    TrailingBytes(usize),
}

#[cfg(all(test, feature = "generation"))]
mod tests {
    use super::*;

    fn sample() -> Schema {
        Schema::new(
            "IFC4",
            vec![
                EntityDef::new("IfcRoot")
                    .abstract_entity()
                    .with_attribute(Attribute::new("GlobalId", "IfcGloballyUniqueId"))
                    .with_where_rule(WhereRule::new("WR1", "")),
                EntityDef::new("IfcWall")
                    .with_supertype("IfcRoot")
                    .with_attribute(Attribute::new("Name", "IfcLabel").optional())
                    .with_attribute(Attribute::new("Tags", "IfcLabel").aggregate())
                    .with_attribute(
                        Attribute::new("Coords", "IfcLengthMeasure")
                            .with_aggregation(
                                Aggregation::new(
                                    AggregateKind::List,
                                    Bound::Integer(1),
                                    Bound::Unbounded,
                                )
                                .unique(),
                            )
                            .with_aggregation(
                                Aggregation::new(
                                    AggregateKind::Array,
                                    Bound::Integer(1),
                                    Bound::Expression("SELF\\IfcWall.Dim".into()),
                                )
                                .optional_elements(),
                            ),
                    )
                    .with_inverse(
                        InverseAttribute::new("Holes", "Voiding", "RelatingElement")
                            .with_aggregation(Aggregation::new(
                                AggregateKind::Set,
                                Bound::Integer(0),
                                Bound::Unbounded,
                            )),
                    )
                    .with_inverse(
                        InverseAttribute::new("Owner", "Owning", "Owned").redeclaring("IfcRoot"),
                    )
                    .with_unique_rule(UniqueRule::new("UR1", vec!["Name".into()]))
                    .with_unique_rule(UniqueRule::unlabelled(
                        vec!["SELF\\IfcRoot.GlobalId".into()],
                    ))
                    .with_derived("Dim"),
            ],
            vec![
                TypeDef::new("IfcLabel", TypeKind::Defined("STRING".into())),
                TypeDef::new("IfcSide", TypeKind::Enumeration(vec!["LEFT".into()])),
                TypeDef::new("IfcValue", TypeKind::Select(vec!["IfcLabel".into()])),
            ],
        )
    }

    #[test]
    fn round_trips_through_the_wire_format() {
        let schema = sample();
        let bytes = encode_schema(&schema).expect("encode");
        let decoded = decode_schema(&bytes).expect("decode");
        assert_eq!(decoded, schema);
    }

    #[test]
    fn rejects_trailing_bytes() {
        let schema = sample();
        let mut bytes = encode_schema(&schema).expect("encode");
        bytes.push(0);
        assert!(matches!(
            decode_schema(&bytes),
            Err(BundledSchemaError::TrailingBytes(1))
        ));
    }
}

#[cfg(test)]
mod header_tests {
    use super::*;

    fn framed(version: u16, payload: &[u8]) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend(bincode::encode_to_vec(version, bincode::config::standard()).unwrap());
        bytes.extend_from_slice(payload);
        bytes
    }

    fn v2_payload() -> Vec<u8> {
        let old = WireSchemaV2 {
            name: "IFC4".into(),
            entities: vec![WireEntityV2 {
                name: "IfcPolyline".into(),
                supertype: None,
                abstract_: false,
                attributes: vec![WireAttributeV2 {
                    name: "Points".into(),
                    type_name: "IfcCartesianPoint".into(),
                    optional: false,
                    aggregate: true,
                }],
                derived: Vec::new(),
                where_rules: vec![WireWhereRule {
                    label: "SameDim".into(),
                    expression: String::new(),
                }],
            }],
            types: Vec::new(),
        };
        bincode::encode_to_vec(old, bincode::config::standard()).unwrap()
    }

    /// A format-2 artifact (no bounds, INVERSE or UNIQUE) still decodes:
    /// the aggregate flag survives and the facts it never recorded are empty.
    #[test]
    fn a_format_2_artifact_still_decodes() {
        let schema = decode_schema(&framed(2, &v2_payload())).expect("v2 decodes");
        let entity = schema.entity("IfcPolyline").unwrap();
        assert!(entity.attributes[0].aggregate);
        assert!(entity.attributes[0].aggregation.is_empty());
        assert!(entity.inverses.is_empty() && entity.unique_rules.is_empty());
        assert_eq!(entity.where_rules[0].label, "SameDim");
        // The same bytes under the current version are not a v3 payload.
        assert!(decode_schema(&framed(FORMAT_VERSION, &v2_payload())).is_err());
    }

    #[test]
    fn rejects_bad_magic() {
        let bytes = vec![0u8; MIN_HEADER_BYTES + 1];
        assert!(matches!(
            decode_schema(&bytes),
            Err(BundledSchemaError::BadMagic)
        ));
    }

    #[test]
    fn rejects_truncated_header() {
        assert!(matches!(
            decode_schema(&MAGIC),
            Err(BundledSchemaError::TruncatedHeader { .. })
        ));
    }

    #[test]
    fn decode_rejects_input_above_resource_budget() {
        let bytes = vec![0; MAX_ARTIFACT_BYTES + 1];
        assert!(matches!(
            decode_schema(&bytes),
            Err(BundledSchemaError::TooLarge { .. })
        ));
    }
}
