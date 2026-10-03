//! Entity attributes addressed by name rather than by position.
//!
//! A Part 21 record stores attributes by position, and the position of a
//! name is a fact of the release, not of the file: `IfcTask.Status` is
//! slot 6 in IFC2X3 and slot 7 in IFC4, where `IfcProcess` gained
//! `Identification` and `LongDescription`, and `IfcGridPlacement`'s
//! `PlacementLocation` moves from slot 0 to slot 1 in IFC4X3, where
//! `PlacementRelTo` moved up into `IfcObjectPlacement`. The caller passes the
//! schema the file declares, as for [`crate::ids_of_type_including_subtypes`];
//! a name is never resolved against a substituted release.
//!
//! The slots are the explicit attributes in Part 21 order, inherited
//! first (ISO 10303-21 §12.2.5.2): exactly the slots a STEP record holds.
//! `INVERSE` attributes occupy no slot and are not listed, nor are new
//! `DERIVE` attributes. An inherited attribute a subtype redeclares in its
//! `DERIVE` block keeps its slot and is marked [`AttributeSlot::derived`]:
//! the file writes it `*` (ADR 0009), and [`set_attribute_by_name`] refuses
//! to overwrite it.
//!
//! Names match ASCII case-insensitively, as EXPRESS identifiers do
//! (ISO 10303-11 §7.4); every answer spells them as the schema declares.
//!
//! ```
//! let schema = ifc::schema::ifc4();
//! let slot = ifc::attribute_slot(schema, "IFCWALL", "name").unwrap();
//! assert_eq!((slot.index, slot.name, slot.declared_by), (2, "Name", "IfcRoot"));
//! ```
#![cfg(feature = "schema-api")]

use std::fmt;

use ifc_model::{EntityId, Model, Value};
use ifc_schema::Schema;

/// One explicit attribute slot of an entity type, as a release declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AttributeSlot<'s> {
    /// Zero-based position in the STEP record.
    pub index: usize,
    /// The declared name, in the schema's spelling (`GlobalId`).
    pub name: &'s str,
    /// The declared type, or an aggregate's innermost element type
    /// (`IfcLengthMeasure` for `LIST [1:?] OF IfcLengthMeasure`).
    pub type_name: &'s str,
    /// Whether the attribute is `OPTIONAL`: `$` is a valid value.
    pub optional: bool,
    /// Whether the type is a `LIST`, `SET`, `BAG` or `ARRAY`.
    pub aggregate: bool,
    /// Whether this type, or a supertype below the declaring one,
    /// redeclares the attribute in a `DERIVE` block: the slot is written
    /// `*` and holds no value of its own.
    pub derived: bool,
    /// The entity whose declaration introduces the attribute (`IfcRoot`).
    pub declared_by: &'s str,
}

/// Why a by-name lookup or write was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NamedAttributeError {
    /// No entity has this id.
    MissingEntity(EntityId),
    /// The schema does not declare this entity type: a type of another
    /// release, or one no release declares.
    UnknownEntity {
        /// The schema's name, e.g. `IFC4`.
        schema: String,
        /// The type as asked for.
        type_name: String,
    },
    /// The entity type has no explicit attribute of this name in the
    /// schema (an `INVERSE` or new `DERIVE` attribute included).
    UnknownAttribute {
        /// The schema's name.
        schema: String,
        /// The entity type, in the schema's spelling.
        type_name: String,
        /// The name as asked for.
        name: String,
    },
    /// The slot is derived for this entity type (`*` in the file); it
    /// holds no value to write.
    DerivedAttribute {
        /// The entity type, in the schema's spelling.
        type_name: String,
        /// The attribute, in the schema's spelling.
        name: String,
    },
}

impl fmt::Display for NamedAttributeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEntity(EntityId(id)) => write!(f, "no entity #{id}"),
            Self::UnknownEntity { schema, type_name } => {
                write!(f, "{schema} declares no entity {type_name}")
            }
            Self::UnknownAttribute {
                schema,
                type_name,
                name,
            } => write!(f, "{schema} {type_name} has no explicit attribute {name:?}"),
            Self::DerivedAttribute { type_name, name } => write!(
                f,
                "{type_name}.{name} is derived (written `*`) and cannot be set"
            ),
        }
    }
}

impl std::error::Error for NamedAttributeError {}

/// Every explicit attribute slot of `type_name` in `schema`, in Part 21
/// order, inherited first.
///
/// # Errors
///
/// [`NamedAttributeError::UnknownEntity`] when `schema` does not declare
/// `type_name`.
pub fn attribute_slots<'s>(
    schema: &'s Schema,
    type_name: &str,
) -> Result<Vec<AttributeSlot<'s>>, NamedAttributeError> {
    let entity = schema
        .entity(type_name)
        .ok_or_else(|| NamedAttributeError::UnknownEntity {
            schema: schema.name().to_owned(),
            type_name: type_name.to_owned(),
        })?;
    // The entity itself, then its ancestors nearest first: the chain that
    // can redeclare an inherited attribute as derived.
    let chain: Vec<_> = std::iter::once(entity)
        .chain(
            schema
                .supertypes(&entity.name)
                .into_iter()
                .filter_map(|name| schema.entity(name)),
        )
        .collect();
    // Part 21 order is each ancestor's own attributes, root first.
    let owners = chain.iter().rev().flat_map(|owner| {
        owner
            .attributes
            .iter()
            .map(move |attribute| (owner, attribute))
    });
    Ok(owners
        .enumerate()
        .map(|(index, (owner, attribute))| AttributeSlot {
            index,
            name: &attribute.name,
            type_name: &attribute.type_name,
            optional: attribute.optional,
            aggregate: attribute.aggregate,
            derived: chain
                .iter()
                .any(|definition| definition.is_derived(&attribute.name)),
            declared_by: &owner.name,
        })
        .collect())
}

/// The slot named `name` (any case) of `type_name` in `schema`.
///
/// # Errors
///
/// [`NamedAttributeError::UnknownEntity`] for an undeclared type,
/// [`NamedAttributeError::UnknownAttribute`] for a name it has no explicit
/// slot for.
pub fn attribute_slot<'s>(
    schema: &'s Schema,
    type_name: &str,
    name: &str,
) -> Result<AttributeSlot<'s>, NamedAttributeError> {
    attribute_slots(schema, type_name)?
        .into_iter()
        .find(|slot| slot.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| NamedAttributeError::UnknownAttribute {
            schema: schema.name().to_owned(),
            type_name: schema
                .entity(type_name)
                .map_or(type_name, |entity| entity.name.as_str())
                .to_owned(),
            name: name.to_owned(),
        })
}

/// The value of attribute `name` of entity `id`, resolved against
/// `schema`; `None` when the record is shorter than the slot, which a
/// reader treats as `$`. A derived slot reads as stored, normally `*`.
///
/// # Errors
///
/// [`NamedAttributeError::MissingEntity`], then as for [`attribute_slot`].
pub fn attribute_by_name<'m>(
    model: &'m Model,
    schema: &Schema,
    id: EntityId,
    name: &str,
) -> Result<Option<&'m Value>, NamedAttributeError> {
    let entity = model
        .get(id)
        .ok_or(NamedAttributeError::MissingEntity(id))?;
    let slot = attribute_slot(schema, &entity.type_name, name)?;
    Ok(entity.attribute(slot.index))
}

/// Set attribute `name` of entity `id`, resolved against `schema`,
/// returning the previous value (`$` past the record's end, which is
/// padded with `$` as [`Model::set_attribute`] does).
///
/// Every check runs before the one write, so a refusal leaves the model
/// unchanged. The value itself is not checked against the declared type,
/// as for a positional write; schema validation reports a mismatch.
///
/// # Errors
///
/// As for [`attribute_by_name`], and
/// [`NamedAttributeError::DerivedAttribute`] for a derived slot, whatever
/// the value, `*` included.
pub fn set_attribute_by_name(
    model: &mut Model,
    schema: &Schema,
    id: EntityId,
    name: &str,
    value: Value,
) -> Result<Value, NamedAttributeError> {
    let entity = model
        .get(id)
        .ok_or(NamedAttributeError::MissingEntity(id))?;
    let slot = attribute_slot(schema, &entity.type_name, name)?;
    if slot.derived {
        return Err(NamedAttributeError::DerivedAttribute {
            type_name: schema
                .entity(&entity.type_name)
                .map_or(&*entity.type_name, |def| def.name.as_str())
                .to_owned(),
            name: slot.name.to_owned(),
        });
    }
    model
        .set_attribute(id, slot.index, value)
        .ok_or(NamedAttributeError::MissingEntity(id))
}

#[cfg(all(test, feature = "ifc4"))]
mod tests {
    use super::*;
    use ifc_model::Entity;

    #[test]
    fn a_subtype_redeclaration_marks_the_inherited_slot_derived() {
        let slots = attribute_slots(ifc_schema::ifc4(), "IfcSIUnit").unwrap();
        let dimensions = slots.iter().find(|slot| slot.name == "Dimensions").unwrap();
        assert!(dimensions.derived);
        assert_eq!(dimensions.declared_by, "IfcNamedUnit");
        let named = attribute_slots(ifc_schema::ifc4(), "IfcConversionBasedUnit").unwrap();
        assert!(!named[0].derived, "only the redeclaring subtype derives it");
    }

    #[test]
    fn a_refused_write_leaves_the_model_unchanged() {
        let mut model = Model::new();
        let id = model.push(Entity::new(
            "IFCSIUNIT",
            vec![Value::Derived, Value::Null, Value::Null, Value::Null],
        ));
        let before = model.get(id).cloned();
        let refused = set_attribute_by_name(
            &mut model,
            ifc_schema::ifc4(),
            id,
            "dimensions",
            Value::Null,
        );
        assert!(matches!(
            refused,
            Err(NamedAttributeError::DerivedAttribute { ref name, .. }) if name == "Dimensions"
        ));
        assert_eq!(model.get(id).cloned(), before);
    }
}
