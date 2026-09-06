//! Compiled binary artifact for a bundled EXPRESS schema.
//!
//! Mirrors `ifc-template-catalog`'s archive pattern: a versioned, checksummed
//! binary encoding that `include_bytes!` ships inside the crate, decoded once
//! behind a `OnceLock`. The wire format stores the already-parsed
//! `ParsedSchema` (entities, types, attributes) rather than EXPRESS source
//! text, so a consumer pays a bincode decode, not an EXPRESS parse.

use bincode::{Decode, Encode};
use thiserror::Error;

use openbim_step::express::{Attribute, EntityDef, ParsedSchema, TypeDef, TypeKind, WhereRule};

const MAGIC: [u8; 8] = *b"NEHSCHM\0";
const FORMAT_VERSION: u16 = 2;
const MIN_HEADER_BYTES: usize = MAGIC.len() + 1;
const MAX_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;

/// Wire-format mirror of [`openbim_step::express::Attribute`].
#[derive(Encode, Decode)]
struct WireAttribute {
    name: String,
    type_name: String,
    optional: bool,
    aggregate: bool,
}

/// Wire-format mirror of [`openbim_step::express::EntityDef`].
#[derive(Encode, Decode)]
struct WireEntity {
    name: String,
    supertype: Option<String>,
    abstract_: bool,
    attributes: Vec<WireAttribute>,
    derived: Vec<String>,
    where_rules: Vec<WireWhereRule>,
}

/// The v1 entity shape, kept so artifacts generated before `where_rules`
/// existed still decode. bincode is positional: a v1 payload cannot be read
/// into the v2 struct, so the old shape has to survive as its own type.
#[derive(Encode, Decode)]
struct WireEntityV1 {
    name: String,
    supertype: Option<String>,
    abstract_: bool,
    attributes: Vec<WireAttribute>,
    derived: Vec<String>,
}

impl From<WireEntityV1> for WireEntity {
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

/// Wire-format mirror of [`openbim_step::express::WhereRule`].
#[derive(Encode, Decode)]
struct WireWhereRule {
    label: String,
    expression: String,
}

/// Wire-format mirror of [`openbim_step::express::TypeKind`].
#[derive(Encode, Decode)]
enum WireTypeKind {
    Defined(String),
    Enumeration(Vec<String>),
    Select(Vec<String>),
}

/// Wire-format mirror of [`openbim_step::express::TypeDef`].
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

/// The v1 schema shape. See [`WireEntityV1`].
#[derive(Encode, Decode)]
struct WireSchemaV1 {
    name: String,
    entities: Vec<WireEntityV1>,
    types: Vec<WireType>,
}

impl From<WireSchemaV1> for WireSchema {
    fn from(old: WireSchemaV1) -> Self {
        Self {
            name: old.name,
            entities: old.entities.into_iter().map(WireEntity::from).collect(),
            types: old.types,
        }
    }
}

impl From<&ParsedSchema> for WireSchema {
    fn from(schema: &ParsedSchema) -> Self {
        Self {
            name: schema.name.clone(),
            entities: schema
                .entities
                .iter()
                .map(|entity| WireEntity {
                    name: entity.name.clone(),
                    supertype: entity.supertype.clone(),
                    abstract_: entity.abstract_,
                    attributes: entity
                        .attributes
                        .iter()
                        .map(|attribute| WireAttribute {
                            name: attribute.name.clone(),
                            type_name: attribute.type_name.clone(),
                            optional: attribute.optional,
                            aggregate: attribute.aggregate,
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
                })
                .collect(),
            types: schema
                .types
                .iter()
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

impl From<WireSchema> for ParsedSchema {
    fn from(wire: WireSchema) -> Self {
        Self {
            name: wire.name,
            entities: wire
                .entities
                .into_iter()
                .map(|entity| {
                    let mut def = EntityDef::new(entity.name);
                    if let Some(supertype) = entity.supertype {
                        def = def.with_supertype(supertype);
                    }
                    def.abstract_ = entity.abstract_;
                    for attribute in entity.attributes {
                        let mut built = Attribute::new(attribute.name, attribute.type_name);
                        if attribute.optional {
                            built = built.optional();
                        }
                        if attribute.aggregate {
                            built = built.aggregate();
                        }
                        def = def.with_attribute(built);
                    }
                    for derived in entity.derived {
                        def = def.with_derived(derived);
                    }
                    def.where_rules = entity
                        .where_rules
                        .into_iter()
                        .map(|rule| WhereRule {
                            label: rule.label,
                            expression: rule.expression,
                        })
                        .collect();
                    def
                })
                .collect(),
            types: wire
                .types
                .into_iter()
                .map(|type_def| TypeDef {
                    name: type_def.name,
                    kind: match type_def.kind {
                        WireTypeKind::Defined(alias) => TypeKind::Defined(alias),
                        WireTypeKind::Enumeration(members) => TypeKind::Enumeration(members),
                        WireTypeKind::Select(members) => TypeKind::Select(members),
                    },
                })
                .collect(),
        }
    }
}

/// Decodes a compiled schema artifact produced by `encode_schema`
/// (the `generation` feature).
///
/// # Errors
///
/// Returns `BundledSchemaError` if the artifact is malformed, oversized, or
/// carries an unsupported format version.
pub fn decode_schema(bytes: &[u8]) -> Result<ParsedSchema, BundledSchemaError> {
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
    // v1 predates `where_rules`. bincode is positional, so an old payload
    // cannot be read into the current struct; it decodes through its own shape
    // and gains an empty rule list. Reading it is still correct -- the rules
    // were never recorded, which is exactly what an empty list states.
    let (wire, consumed): (WireSchema, usize) = match format_version {
        1 => {
            let (old, consumed): (WireSchemaV1, usize) =
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
pub fn encode_schema(schema: &ParsedSchema) -> Result<Vec<u8>, bincode::error::EncodeError> {
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
    #[error("cannot decode schema artifact: {0}")]
    Decode(String),
    #[error("schema artifact is {actual} bytes; limit is {limit} bytes")]
    TooLarge { actual: usize, limit: usize },
    #[error("schema artifact header is {actual} bytes; at least {required} bytes are required")]
    TruncatedHeader { actual: usize, required: usize },
    #[error("schema artifact magic is invalid")]
    BadMagic,
    #[error("unsupported schema artifact format version {0}")]
    UnsupportedVersion(u16),
    #[error("schema artifact has {0} trailing bytes")]
    TrailingBytes(usize),
}

#[cfg(all(test, feature = "generation"))]
mod tests {
    use super::*;

    fn sample() -> ParsedSchema {
        openbim_step::express::parse(
            "SCHEMA IFC4;\n\
             ENTITY IfcRoot; GlobalId : IfcGloballyUniqueId; END_ENTITY;\n\
             ENTITY IfcWall SUBTYPE OF (IfcRoot); Name : IfcLabel; END_ENTITY;\n\
             END_SCHEMA;",
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

#[cfg(all(test, feature = "ifc4"))]
mod header_tests {
    use super::*;

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
