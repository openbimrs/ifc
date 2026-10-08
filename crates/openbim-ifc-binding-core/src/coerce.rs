//! Plain host values written by name, coerced against the declared type
//! (#342).
//!
//! A tagged value says exactly what it is: `Text` or `Enum`, `Integer` or
//! `Real`, `IFCLABEL('x')` or `'x'`. A host's plain value does not: Python's
//! `"STANDARD"` could be either of the first pair, `1` either of the second.
//! [`IfcModel::set_attribute_by_name_plain`] settles it from the attribute's
//! declared EXPRESS type in the release the header declares
//! (`ifc::attribute_type`), and refuses what the declaration cannot settle.
//!
//! # The rules
//!
//! | Plain value | Declared type | Written |
//! | --- | --- | --- |
//! | any | through a defined type | as for the type it aliases, bare: `IfcLabel` takes `'x'` |
//! | string | `STRING` | text |
//! | string | an enumeration | the item it names (any case), else `type-mismatch` |
//! | integer | `INTEGER`, `NUMBER` | integer |
//! | integer | `REAL` | real, when exact (within 2^53), else `type-mismatch` |
//! | float | `REAL`, `NUMBER` | real |
//! | bool | `BOOLEAN`, `LOGICAL` | `.T.` / `.F.` |
//! | list | an aggregate | each element coerced against the element type |
//! | entity handle | an entity | a reference, when the target exists (`missing-reference`) and is of the type or a subtype (`type-mismatch`) |
//! | entity handle | a SELECT | a reference, when an entity member accepts the target |
//! | other | a SELECT | the one member that accepts it, as a typed parameter (`IFCLABEL('x')`); several: `ambiguous-value`, naming them |
//! | null | any | `$` |
//!
//! Anything else is `type-mismatch`: a float for an `INTEGER`, a string for
//! a `REAL`, any plain value for `BINARY` (write a `binary` explicitly). A
//! declared type the tables do not resolve is `unsupported`. Typed
//! parameters appear only in a SELECT, as ISO 10303-21 §12.1.8 writes them;
//! everywhere else the value is bare (§12.1.6, §12.1.7).
//!
//! [`Plain::Exact`] carries a tagged value through unchanged, nested in a
//! list too, so a caller keeps exact control where it wants it.

use ifc::{attribute_type, DeclaredType, EntityId, Model, Schema, SimpleType};

use crate::value::{Tagged, MAX_NESTING};
use crate::{BindingError, IfcModel};

/// A host value before coercion: what a host language holds without
/// saying which IFC type it means.
#[derive(Debug, Clone, PartialEq)]
pub enum Plain {
    /// `None` / `null`: written `$`.
    Null,
    /// A boolean.
    Bool(bool),
    /// An integer.
    Integer(i64),
    /// A float.
    Real(f64),
    /// A string.
    Text(String),
    /// An entity handle: entity `#id` of the same model.
    Ref(u64),
    /// A sequence.
    List(Vec<Plain>),
    /// A tagged value, written exactly as given.
    Exact(Tagged),
}

/// Integers a REAL holds exactly.
const EXACT_IN_REAL: i64 = 1 << 53;

impl IfcModel {
    /// The tagged value [`Self::set_attribute_by_name_plain`] would write
    /// for `value` into attribute `name` (any case) of entity `id`, without
    /// writing it.
    ///
    /// Refused as [`Self::attribute_by_name`] is, with `derived-attribute`
    /// for a derived slot, and by the rules of the [module](self).
    pub fn coerce_attribute(
        &self,
        id: u64,
        name: &str,
        value: Plain,
    ) -> Result<Tagged, BindingError> {
        let schema = self.declared_schema()?;
        let type_name = &self.entity(id)?.type_name;
        let (slot, declared) =
            attribute_type(schema, type_name, name).map_err(crate::attribute::refused)?;
        if slot.derived {
            return Err(BindingError::DerivedAttribute(format!(
                "{}.{} is derived (written `*`) and cannot be set",
                schema
                    .entity(type_name)
                    .map_or(&**type_name, |e| e.name.as_str()),
                slot.name
            )));
        }
        let context = Context {
            model: &self.inner,
            schema,
        };
        context.coerce(&declared, value, 0)
    }

    /// Set attribute `name` (any case) of entity `id` from a plain host
    /// value, coerced against its declared type (see the
    /// [module](self)); returns the previous value. Every check runs
    /// before the write, so a refusal changes nothing.
    pub fn set_attribute_by_name_plain(
        &mut self,
        id: u64,
        name: &str,
        value: Plain,
    ) -> Result<Tagged, BindingError> {
        let value = self.coerce_attribute(id, name, value)?;
        self.set_attribute_by_name(id, name, value)
    }
}

struct Context<'a> {
    model: &'a Model,
    schema: &'a Schema,
}

fn mismatch(value: &Plain, declared: &DeclaredType<'_>) -> BindingError {
    BindingError::TypeMismatch(format!(
        "{} does not fit {}",
        describe(value),
        describe_type(declared)
    ))
}

fn describe(value: &Plain) -> String {
    match value {
        Plain::Null => "null".to_owned(),
        Plain::Bool(b) => format!("the boolean {b}"),
        Plain::Integer(i) => format!("the integer {i}"),
        Plain::Real(r) => format!("the float {r}"),
        Plain::Text(s) => format!("the string {s:?}"),
        Plain::Ref(id) => format!("entity #{id}"),
        Plain::List(_) => "a list".to_owned(),
        Plain::Exact(_) => "an exact value".to_owned(),
    }
}

fn describe_type(declared: &DeclaredType<'_>) -> String {
    match declared {
        DeclaredType::Simple(simple) => simple.keyword().to_owned(),
        DeclaredType::Defined { name, underlying } => {
            format!("{name} ({})", describe_type(underlying))
        }
        DeclaredType::Enumeration { name, items } => {
            format!("{name} (one of {})", items.join(", "))
        }
        DeclaredType::Aggregate { element, .. } => {
            format!("an aggregate of {}", describe_type(element))
        }
        other => other
            .name()
            .map_or_else(|| format!("{other:?}"), str::to_owned),
    }
}

impl Context<'_> {
    fn coerce(
        &self,
        declared: &DeclaredType<'_>,
        value: Plain,
        depth: usize,
    ) -> Result<Tagged, BindingError> {
        if depth > MAX_NESTING {
            return Err(BindingError::InvalidValue(format!(
                "nesting deeper than {MAX_NESTING}"
            )));
        }
        let value = match value {
            Plain::Exact(tagged) => return Ok(tagged),
            Plain::Null => return Ok(Tagged::Null),
            // STEP has no NaN or infinity; refused before any SELECT member
            // could be said to take it.
            Plain::Real(r) if !r.is_finite() => {
                return Err(BindingError::InvalidValue(format!(
                    "real {r} is not finite"
                )))
            }
            value => value,
        };
        match declared {
            DeclaredType::Simple(simple) => simple_value(*simple, value, declared),
            DeclaredType::Defined { underlying, .. } => self.coerce(underlying, value, depth + 1),
            DeclaredType::Enumeration { items, .. } => match &value {
                Plain::Text(text) => items
                    .iter()
                    .find(|item| item.eq_ignore_ascii_case(text))
                    .map(|item| Tagged::Enum(item.to_ascii_uppercase()))
                    .ok_or_else(|| mismatch(&value, declared)),
                _ => Err(mismatch(&value, declared)),
            },
            DeclaredType::Entity(name) => match value {
                Plain::Ref(id) => self.reference(id, &[name]),
                value => Err(mismatch(&value, declared)),
            },
            DeclaredType::Aggregate { element, .. } => match value {
                Plain::List(items) => Ok(Tagged::List(
                    items
                        .into_iter()
                        .map(|item| self.coerce(element, item, depth + 1))
                        .collect::<Result<_, _>>()?,
                )),
                value => Err(mismatch(&value, declared)),
            },
            DeclaredType::Select { name, .. } => self.select(name, declared, value, depth),
            DeclaredType::Unresolved(token) => Err(BindingError::Unsupported(format!(
                "the declared type {token} is not in the tables; write an exact value"
            ))),
            // A declaration form added to the facade later.
            other => Err(BindingError::Unsupported(format!(
                "the declared type {other:?} has no plain coercion; write an exact value"
            ))),
        }
    }

    /// `#id`, when it exists and is one of `accepted` or a subtype.
    fn reference(&self, id: u64, accepted: &[&str]) -> Result<Tagged, BindingError> {
        let target = self.model.get(EntityId(id)).ok_or_else(|| {
            BindingError::MissingReference(format!("entity #{id} is not in the model"))
        })?;
        if accepted
            .iter()
            .any(|name| self.schema.is_a(&target.type_name, name))
        {
            Ok(Tagged::Ref(id))
        } else {
            Err(BindingError::TypeMismatch(format!(
                "#{id} is an {}, not {}",
                target.type_name,
                accepted.join(" or ")
            )))
        }
    }

    /// A SELECT: a reference when an entity member accepts it; otherwise
    /// the one member whose type takes the value, as a typed parameter.
    fn select(
        &self,
        name: &str,
        declared: &DeclaredType<'_>,
        value: Plain,
        depth: usize,
    ) -> Result<Tagged, BindingError> {
        let mut leaves = Vec::new();
        leaves_of(declared, &mut leaves);
        if let Plain::Ref(id) = value {
            let entities: Vec<&str> = leaves
                .iter()
                .filter_map(|leaf| match leaf {
                    DeclaredType::Entity(entity) => Some(*entity),
                    _ => None,
                })
                .collect();
            if entities.is_empty() {
                return Err(BindingError::TypeMismatch(format!(
                    "{name} admits no entity reference"
                )));
            }
            return self.reference(id, &entities);
        }
        let mut fits: Vec<(&str, Tagged)> = Vec::new();
        for leaf in &leaves {
            let (Some(member), false) = (leaf.name(), matches!(leaf, DeclaredType::Entity(_)))
            else {
                continue;
            };
            if fits
                .iter()
                .any(|(seen, _)| seen.eq_ignore_ascii_case(member))
            {
                continue;
            }
            if let Ok(inner) = self.coerce(leaf, value.clone(), depth + 1) {
                fits.push((member, inner));
            }
        }
        match fits.len() {
            0 => Err(BindingError::TypeMismatch(format!(
                "{} fits no member of {name}",
                describe(&value)
            ))),
            1 => {
                let (member, inner) = fits.pop().expect("one fit");
                Ok(Tagged::Typed {
                    type_name: member.to_ascii_uppercase(),
                    value: Box::new(inner),
                })
            }
            _ => Err(BindingError::AmbiguousValue(format!(
                "{} fits {} members of {name}: {}",
                describe(&value),
                fits.len(),
                fits.iter()
                    .map(|(member, _)| *member)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
        }
    }
}

/// The members of a SELECT that are not SELECTs themselves, nested ones
/// flattened, in declaration order.
fn leaves_of<'d, 's>(declared: &'d DeclaredType<'s>, out: &mut Vec<&'d DeclaredType<'s>>) {
    match declared {
        DeclaredType::Select { members, .. } => {
            for member in members {
                leaves_of(member, out);
            }
        }
        // A defined type aliasing a SELECT is encoded as that SELECT
        // (ISO 10303-21 §12.1.8).
        DeclaredType::Defined { underlying, .. }
            if matches!(**underlying, DeclaredType::Select { .. }) =>
        {
            leaves_of(underlying, out);
        }
        leaf => out.push(leaf),
    }
}

fn simple_value(
    simple: SimpleType,
    value: Plain,
    declared: &DeclaredType<'_>,
) -> Result<Tagged, BindingError> {
    Ok(match (simple, value) {
        (SimpleType::Integer | SimpleType::Number, Plain::Integer(i)) => Tagged::Integer(i),
        (SimpleType::Real, Plain::Integer(i)) if (-EXACT_IN_REAL..=EXACT_IN_REAL).contains(&i) => {
            #[allow(clippy::cast_precision_loss)] // Bounded above: exact.
            Tagged::Real(i as f64)
        }
        (SimpleType::Real | SimpleType::Number, Plain::Real(r)) => Tagged::Real(r),
        (SimpleType::String, Plain::Text(s)) => Tagged::Text(s),
        (SimpleType::Boolean | SimpleType::Logical, Plain::Bool(b)) => Tagged::Bool(b),
        (_, value) => return Err(mismatch(&value, declared)),
    })
}
