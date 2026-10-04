//! The operations a batch carries, as every host spells them, and the one
//! reader that turns the shared tape form into the facade's operation.
//!
//! An operation is named by `op` and carries named fields of six kinds
//! ([`FieldKind`]). [`OPS`] lists them all: each host converts its own
//! idiom (a JavaScript object, a Python dict, a C# record) field by field
//! into the tape form below, guided by the table, so no host interprets an
//! operation differently.
//!
//! The tape form, the C ABI's batch element, is one `LIST`: an `ENUM`
//! naming the operation (`PRODUCT`, `ASSIGN_TYPE`, ...), then alternating
//! field names (`TEXT`, snake case) and values. A `NULL` value is an absent
//! field. Values by kind:
//!
//! | Kind | Tape value |
//! | --- | --- |
//! | text | `TEXT` |
//! | id | `REF`, or a non-negative `INTEGER` |
//! | ids | `LIST` of ids |
//! | reals | `LIST` of three `REAL`s (or `INTEGER`s) |
//! | integer | `INTEGER` |
//! | attributes | `LIST` of `LIST(TEXT name, value)` |

use crate::value::Tagged;
use crate::BindingError;

/// How a field's value is carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FieldKind {
    /// A string.
    Text,
    /// An entity id or a batch handle.
    Id,
    /// A list of ids.
    Ids,
    /// Three numbers: a point or a direction.
    Reals,
    /// A 64-bit integer.
    Integer,
    /// Attribute values by name.
    Attributes,
}

/// One field of an operation.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct FieldSpec {
    /// The snake-case name: the tape's and Python's spelling.
    pub name: &'static str,
    /// The camel-case name: JavaScript's and C#'s spelling.
    pub camel: &'static str,
    /// How its value is carried.
    pub kind: FieldKind,
    /// Whether the operation needs it.
    pub required: bool,
}

/// One operation and its fields.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct OpSpec {
    /// The snake-case name (`assign_type`).
    pub name: &'static str,
    /// The camel-case name (`assignType`).
    pub camel: &'static str,
    /// Its fields.
    pub fields: &'static [FieldSpec],
}

impl OpSpec {
    /// The field named `key` in either spelling, ASCII case-insensitively.
    pub fn field(&self, key: &str) -> Option<&'static FieldSpec> {
        self.fields
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(key) || f.camel.eq_ignore_ascii_case(key))
    }
}

const fn field(name: &'static str, camel: &'static str, kind: FieldKind) -> FieldSpec {
    FieldSpec {
        name,
        camel,
        kind,
        required: false,
    }
}

const fn required(name: &'static str, camel: &'static str, kind: FieldKind) -> FieldSpec {
    FieldSpec {
        name,
        camel,
        kind,
        required: true,
    }
}

use FieldKind as K;

const TYPE: FieldSpec = required("type", "type", K::Text);
const ATTRIBUTES: FieldSpec = field("attributes", "attributes", K::Attributes);
const OWNER: FieldSpec = field("owner_history", "ownerHistory", K::Id);

/// Every operation, with its fields.
pub const OPS: &[OpSpec] = &[
    OpSpec {
        name: "create",
        camel: "create",
        fields: &[TYPE, ATTRIBUTES],
    },
    OpSpec {
        name: "edit",
        camel: "edit",
        fields: &[
            required("entity", "entity", K::Id),
            required("attributes", "attributes", K::Attributes),
        ],
    },
    OpSpec {
        name: "remove",
        camel: "remove",
        fields: &[required("entity", "entity", K::Id)],
    },
    OpSpec {
        name: "project",
        camel: "project",
        fields: &[ATTRIBUTES, OWNER],
    },
    OpSpec {
        name: "spatial",
        camel: "spatial",
        fields: &[
            TYPE,
            required("parent", "parent", K::Id),
            ATTRIBUTES,
            field("placement", "placement", K::Id),
            OWNER,
        ],
    },
    OpSpec {
        name: "product",
        camel: "product",
        fields: &[
            TYPE,
            field("container", "container", K::Id),
            ATTRIBUTES,
            field("placement", "placement", K::Id),
            field("type_object", "typeObject", K::Id),
            OWNER,
        ],
    },
    OpSpec {
        name: "type_object",
        camel: "typeObject",
        fields: &[TYPE, ATTRIBUTES, OWNER],
    },
    OpSpec {
        name: "assign_type",
        camel: "assignType",
        fields: &[
            required("type_object", "typeObject", K::Id),
            required("objects", "objects", K::Ids),
            OWNER,
        ],
    },
    OpSpec {
        name: "contain",
        camel: "contain",
        fields: &[
            required("structure", "structure", K::Id),
            required("elements", "elements", K::Ids),
            OWNER,
        ],
    },
    OpSpec {
        name: "aggregate",
        camel: "aggregate",
        fields: &[
            required("parent", "parent", K::Id),
            required("parts", "parts", K::Ids),
            OWNER,
        ],
    },
    OpSpec {
        name: "placement",
        camel: "placement",
        fields: &[
            field("relative_to", "relativeTo", K::Id),
            field("location", "location", K::Reals),
            field("axis", "axis", K::Reals),
            field("ref_direction", "refDirection", K::Reals),
        ],
    },
    OpSpec {
        name: "owner_history",
        camel: "ownerHistory",
        fields: &[
            field("person_identification", "personIdentification", K::Text),
            field("family_name", "familyName", K::Text),
            field("given_name", "givenName", K::Text),
            required("organization", "organization", K::Text),
            required("application_name", "applicationName", K::Text),
            required("application_version", "applicationVersion", K::Text),
            required("application_identifier", "applicationIdentifier", K::Text),
            field("change_action", "changeAction", K::Text),
            required("creation_date", "creationDate", K::Integer),
            field("last_modified_date", "lastModifiedDate", K::Integer),
        ],
    },
];

/// The operation named `name` in either spelling, ASCII case-insensitively.
pub fn op_spec(name: &str) -> Option<&'static OpSpec> {
    OPS.iter()
        .find(|op| op.name.eq_ignore_ascii_case(name) || op.camel.eq_ignore_ascii_case(name))
}

/// A field value, read by its kind. Read only with the `author` feature;
/// without it the tape is still checked, then refused.
#[derive(Debug, Clone)]
#[cfg_attr(not(feature = "author"), allow(dead_code))]
pub(crate) enum Arg {
    Text(String),
    Id(u64),
    Ids(Vec<u64>),
    Reals([f64; 3]),
    Integer(i64),
    Attributes(Vec<(String, Tagged)>),
}

/// An operation read from its tape: its spec and its present fields.
#[cfg_attr(not(feature = "author"), allow(dead_code))]
pub(crate) struct Fields {
    pub(crate) spec: &'static OpSpec,
    pub(crate) args: Vec<(&'static str, Arg)>,
}

#[cfg_attr(not(feature = "author"), allow(dead_code))]
impl Fields {
    pub(crate) fn take(&mut self, name: &str) -> Option<Arg> {
        let index = self.args.iter().position(|(key, _)| *key == name)?;
        Some(self.args.remove(index).1)
    }
}

pub(crate) fn invalid(detail: impl Into<String>) -> BindingError {
    BindingError::InvalidValue(detail.into())
}

/// Read the tape form (see the module docs).
pub(crate) fn read(value: &Tagged) -> Result<Fields, BindingError> {
    let Tagged::List(items) = value else {
        return Err(invalid("an operation is a LIST"));
    };
    let Some(Tagged::Enum(name)) = items.first() else {
        return Err(invalid("an operation starts with an ENUM naming it"));
    };
    let spec = op_spec(name).ok_or_else(|| {
        let known: Vec<&str> = OPS.iter().map(|op| op.name).collect();
        invalid(format!(
            "no operation `{name}`; known: {}",
            known.join(", ")
        ))
    })?;
    let rest = &items[1..];
    if rest.len() % 2 != 0 {
        return Err(invalid(format!(
            "`{}` takes alternating field names and values",
            spec.name
        )));
    }
    let mut args: Vec<(&'static str, Arg)> = Vec::new();
    for pair in rest.chunks(2) {
        let Tagged::Text(key) = &pair[0] else {
            return Err(invalid(format!("`{}` field names are TEXT", spec.name)));
        };
        let field = spec.field(key).ok_or_else(|| {
            let known: Vec<&str> = spec.fields.iter().map(|f| f.name).collect();
            invalid(format!(
                "`{}` has no field `{key}`; it takes {}",
                spec.name,
                known.join(", ")
            ))
        })?;
        if args.iter().any(|(name, _)| *name == field.name) {
            return Err(invalid(format!("`{}.{}` is given twice", spec.name, field.name)));
        }
        if matches!(pair[1], Tagged::Null) {
            continue;
        }
        let arg = arg(field.kind, &pair[1])
            .map_err(|detail| invalid(format!("`{}.{}` {detail}", spec.name, field.name)))?;
        args.push((field.name, arg));
    }
    for field in spec.fields.iter().filter(|f| f.required) {
        if !args.iter().any(|(name, _)| *name == field.name) {
            return Err(invalid(format!("`{}` needs `{}`", spec.name, field.name)));
        }
    }
    Ok(Fields { spec, args })
}

fn arg(kind: FieldKind, value: &Tagged) -> Result<Arg, String> {
    Ok(match kind {
        K::Text => match value {
            Tagged::Text(text) => Arg::Text(text.clone()),
            _ => return Err("must be TEXT".into()),
        },
        K::Id => Arg::Id(id(value)?),
        K::Ids => match value {
            Tagged::List(items) => Arg::Ids(items.iter().map(id).collect::<Result<_, _>>()?),
            _ => return Err("must be a LIST of ids".into()),
        },
        K::Reals => {
            let Tagged::List(items) = value else {
                return Err("must be a LIST of three numbers".into());
            };
            let numbers: Vec<f64> = items
                .iter()
                .map(|item| match item {
                    #[allow(clippy::cast_precision_loss)]
                    Tagged::Integer(i) => Ok(*i as f64),
                    Tagged::Real(r) if r.is_finite() => Ok(*r),
                    _ => Err("must hold finite numbers".to_owned()),
                })
                .collect::<Result<_, _>>()?;
            Arg::Reals(
                numbers
                    .try_into()
                    .map_err(|_| "must hold exactly three numbers".to_owned())?,
            )
        }
        K::Integer => match value {
            Tagged::Integer(i) => Arg::Integer(*i),
            _ => return Err("must be an INTEGER".into()),
        },
        K::Attributes => {
            let Tagged::List(items) = value else {
                return Err("must be a LIST of (name, value) pairs".into());
            };
            Arg::Attributes(
                items
                    .iter()
                    .map(|item| match item {
                        Tagged::List(pair) => match pair.as_slice() {
                            [Tagged::Text(name), value] => Ok((name.clone(), value.clone())),
                            _ => Err("holds a pair that is not (TEXT name, value)".to_owned()),
                        },
                        _ => Err("holds an item that is not a (name, value) LIST".to_owned()),
                    })
                    .collect::<Result<_, _>>()?,
            )
        }
    })
}

fn id(value: &Tagged) -> Result<u64, String> {
    match value {
        Tagged::Ref(id) => Ok(*id),
        Tagged::Integer(id) => u64::try_from(*id).map_err(|_| format!("id {id} is negative")),
        _ => Err("must be an id (REF or INTEGER)".into()),
    }
}
