//! The IFC release an occurrence is written in (#202).
//!
//! The catalogue in [`crate::table`] is generated from IFC4X3 ADD2, but a
//! model may declare IFC2X3 or IFC4, whose layouts, enumerations and class
//! lists differ. `IfcRoot.OwnerHistory` is the difference that matters most:
//!
//! ```text
//! IFC2X3_TC1   OwnerHistory : IfcOwnerHistory;
//! IFC4         OwnerHistory : OPTIONAL IfcOwnerHistory;
//! IFC4X3_ADD2  OwnerHistory : OPTIONAL IfcOwnerHistory;
//! ```
//!
//! So [`crate::create`] takes each slot, `PredefinedType`'s enumeration and
//! the attributes a class requires from the bound release's own table, by
//! attribute name, and uses the catalogue row only to name the class. The
//! type class an occurrence may be typed by is the bound release's own too
//! (#214), see `Layout::type_class`. Binding, from `FILE_SCHEMA`, as
//! `ifc-material` (#77), `ifc-properties` (#191) and `ifc-classification`
//! (#194) bind their authoring:
//! - one recognised declaration binds that release's table;
//! - one unrecognised declaration fails with
//!   [`OccurrenceError::UnsupportedSchema`];
//! - several fail with [`OccurrenceError::MultipleSchemas`];
//! - none at all (an in-memory [`Model::new`]) binds IFC4.

use ifc_model::{Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, Schema, SchemaVersion, TypeKind};

use crate::error::{OccurrenceError, OccurrenceResult};
use crate::table::Occurrence;

/// The release an occurrence is written in.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Layout {
    version: SchemaVersion,
    schema: &'static Schema,
}

/// Bind `model`'s declared release.
pub(crate) fn bind(model: &Model) -> OccurrenceResult<Layout> {
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token).ok_or_else(|| {
            OccurrenceError::UnsupportedSchema {
                schema: token.clone(),
            }
        })?,
        tokens => {
            return Err(OccurrenceError::MultipleSchemas {
                schemas: tokens.len(),
            })
        }
    };
    let schema = for_version(version).ok_or_else(|| OccurrenceError::UnsupportedSchema {
        schema: format!("{version:?}"),
    })?;
    Ok(Layout { version, schema })
}

impl Layout {
    /// Fail unless this release declares `entity` as instantiable.
    pub(crate) fn require_entity(self, entity: &'static str) -> OccurrenceResult<()> {
        if self
            .schema
            .entity(entity)
            .is_some_and(|found| !found.abstract_)
        {
            return Ok(());
        }
        Err(OccurrenceError::EntityNotInSchema {
            entity,
            schema: self.version,
        })
    }

    /// The release this layout binds.
    pub(crate) const fn version(self) -> SchemaVersion {
        self.version
    }

    /// The tokens of `attribute`'s enumeration on `entity` in this release,
    /// or `None` when the release declares no such attribute, or declares
    /// it with another type.
    pub(crate) fn members(self, entity: &str, attribute: &str) -> Option<Vec<&'static str>> {
        let declared = self
            .schema
            .attributes(entity)
            .into_iter()
            .find(|found| found.name.eq_ignore_ascii_case(attribute))?;
        match &self.schema.type_def(&declared.type_name)?.kind {
            TypeKind::Enumeration(members) => Some(members.iter().map(String::as_str).collect()),
            _ => None,
        }
    }

    /// Whether `found`, an entity type name, is `expected` or one of its
    /// subtypes in this release, as `TYPEOF` tests it.
    pub(crate) fn is_a(self, found: &str, expected: &str) -> bool {
        self.schema.is_a(found, expected)
    }

    /// The one type class this release pairs with `kind`, or `None` when it
    /// pairs none (#214).
    ///
    /// The bundled tables carry WHERE rule labels, not their text, so the
    /// pairing is read from the catalogue's per-release columns, generated
    /// from each release's EXPRESS source by `scripts/gen-occurrences.py`:
    /// IFC4X3 and IFC4 state it as a rule, IFC4 ADD2 TC1 for `IfcDoor` as
    ///
    /// ```text
    /// CorrectStyleAssigned : (SIZEOF(IsTypedBy) = 0)
    ///   OR ('IFC4.IFCDOORTYPE' IN TYPEOF(SELF\IfcObject.IsTypedBy[1].RelatingType));
    /// ```
    ///
    /// IFC2X3 TC1 states no such rule: `IfcRelDefinesByType.RelatingType` is
    /// any `IfcTypeObject`. Its `IfcTypeProduct` is
    /// `SUPERTYPE OF (ONEOF (IfcDoorStyle, IfcElementType, IfcWindowStyle))`:
    /// doors and windows are typed by `IfcDoorStyle` and `IfcWindowStyle`,
    /// since IFC2X3 declares no `IfcDoorType` or `IfcWindowType`, and every
    /// other occurrence by the `IfcElementType` subtype the later releases
    /// pair with it, where IFC2X3 declares that class. Where it does not
    /// (`IfcStairType`, for one), IFC2X3 pairs nothing.
    ///
    /// Rules are recorded as written: IFC4's `IfcTransformer` names
    /// `IFCTRANFORMERTYPE`, a class IFC4 does not declare, so an IFC4
    /// transformer cannot be typed. A release without a column is refused
    /// rather than paired by another release's rule.
    pub(crate) fn type_class(self, kind: Occurrence) -> OccurrenceResult<Option<&'static str>> {
        #[allow(unreachable_patterns)]
        match self.version {
            SchemaVersion::Ifc2x3 => Ok(kind.ifc2x3_type_class),
            SchemaVersion::Ifc4 => Ok(kind.ifc4_type_class),
            SchemaVersion::Ifc4x3 => Ok(kind.type_class),
            other => Err(OccurrenceError::UnsupportedSchema {
                schema: format!("{other:?}"),
            }),
        }
    }
}

impl Layout {
    /// Build a record of `entity` in this release's layout from values named
    /// by attribute; unnamed slots are `$`, and a `$` for an attribute the
    /// release does not declare is dropped.
    ///
    /// A required attribute left `$` is refused with
    /// [`OccurrenceError::AuthoringRequired`] before anything is staged, such
    /// as the IFC2X3 `OwnerHistory` or an IFC2X3 `IfcStair.ShapeType`.
    pub(crate) fn named_record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> OccurrenceResult<Entity> {
        let declared = self.schema.attributes(entity);
        let mut attributes = vec![Value::Null; declared.len()];
        for (attribute, value) in values {
            let slot = declared
                .iter()
                .position(|found| found.name.eq_ignore_ascii_case(attribute));
            match slot {
                Some(slot) => attributes[slot] = value,
                None if value == Value::Null => {}
                None => {
                    return Err(OccurrenceError::AuthoringNotInSchema {
                        entity,
                        attribute,
                        schema: self.version,
                    })
                }
            }
        }
        for (declaration, value) in declared.iter().zip(&attributes) {
            if *value == Value::Null && !declaration.optional {
                return Err(OccurrenceError::AuthoringRequired {
                    entity,
                    attribute: declaration.name.as_str(),
                    schema: self.version,
                });
            }
        }
        Ok(Entity::new(entity, attributes))
    }
}

/// Fail unless `id` is an `IfcOwnerHistory` in the model or staged on `tx`.
/// None is ever invented.
pub(crate) fn require_owner_history(
    tx: &Transaction,
    model: &Model,
    id: EntityId,
) -> OccurrenceResult<()> {
    let found =
        projected_type(tx, model, id).ok_or(OccurrenceError::UnresolvedOwnerHistory { id })?;
    if found.eq_ignore_ascii_case("IFCOWNERHISTORY") {
        return Ok(());
    }
    Err(OccurrenceError::NotAnOwnerHistory { id, found })
}

/// The type `id` has once the transaction's staged edits are applied.
fn projected_type(tx: &Transaction, model: &Model, id: EntityId) -> Option<String> {
    for edit in tx.edits().iter().rev() {
        match edit {
            Edit::Create {
                id: edit_id,
                entity,
            } if *edit_id == id => return Some(entity.type_name.to_string()),
            Edit::Remove { id: edit_id } if *edit_id == id => return None,
            Edit::Retype {
                id: edit_id,
                type_name,
            } if *edit_id == id => return Some(type_name.to_string()),
            _ => {}
        }
    }
    model.get(id).map(|entity| entity.type_name.to_string())
}
