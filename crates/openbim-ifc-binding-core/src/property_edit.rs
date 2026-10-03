//! Writing property sets and quantities (feature `properties-write`, #123).
//!
//! The write side of [`IfcModel::property_sets`]: a property is addressed
//! by the same object, set name and property name the read side reports,
//! and its value is the read side's `value`, a typed `IfcValue` (or `$`), a
//! list of them for an enumerated or list value, or a typed measure for a
//! quantity. The facade's checked edit (`ifc::apply_property_edits`) does
//! the work: it plans the whole batch, validates every value against the
//! declared release and, with the `property-catalog` feature, the release's
//! PSD/QTO catalog, and commits one transaction, or refuses and changes
//! nothing.
//!
//! An occurrence's inherited value is overridden on the occurrence, never
//! changed on the shared type set; address the type object to change that.
//! A set or a property entity the object shares is copied before it
//! changes.
//!
//! Hosts carry an edit in their own idiom. The C tape form, read by
//! [`PropertyEdit::from_tagged`], is one `LIST`: an `ENUM` `SET` or
//! `REMOVE`, the object (`REF` or `INTEGER`), the set name and the property
//! name (`TEXT`), then, for `SET`, the value and an optional set type
//! (`TEXT` or `NULL`).

use crate::record::{Field, Record, ToRecord};
use crate::value::Tagged;
use crate::{BindingError, IfcModel};

/// One edit of a batch.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyEdit {
    /// The occurrence or type object whose own set is edited.
    pub object: u64,
    /// The set's name.
    pub set: String,
    /// The property's name.
    pub name: String,
    /// What the edit does.
    pub action: EditAction,
}

/// What a [`PropertyEdit`] does.
#[derive(Debug, Clone, PartialEq)]
pub enum EditAction {
    /// Write `value`, creating the property and its set when absent.
    Set {
        /// The value in the read side's form.
        value: Tagged,
        /// `IfcPropertySet` or `IfcElementQuantity` (any case), for a set
        /// the edit creates and nothing else describes.
        set_type: Option<String>,
    },
    /// Remove the property from the object's own set.
    Remove,
}

/// What a committed batch did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyEditResult {
    /// Per edit: the entity holding the property after the batch, `None`
    /// when the batch leaves none.
    pub properties: Vec<Option<u64>>,
    /// Entities created, in commit order.
    pub created: Vec<u64>,
    /// Entities removed.
    pub removed: Vec<u64>,
}

impl PropertyEdit {
    /// An edit writing `value`.
    pub fn set(object: u64, set: &str, name: &str, value: Tagged) -> Self {
        Self {
            object,
            set: set.to_owned(),
            name: name.to_owned(),
            action: EditAction::Set {
                value,
                set_type: None,
            },
        }
    }

    /// An edit removing the property.
    pub fn remove(object: u64, set: &str, name: &str) -> Self {
        Self {
            object,
            set: set.to_owned(),
            name: name.to_owned(),
            action: EditAction::Remove,
        }
    }

    /// Read the C tape form (see the module docs).
    ///
    /// # Errors
    ///
    /// `invalid-value` for anything else.
    pub fn from_tagged(value: &Tagged) -> Result<Self, BindingError> {
        let invalid =
            |detail: &str| BindingError::InvalidValue(format!("a property edit {detail}"));
        let Tagged::List(fields) = value else {
            return Err(invalid("is a LIST"));
        };
        let text = |index: usize, what: &str| match fields.get(index) {
            Some(Tagged::Text(text)) => Ok(text.clone()),
            _ => Err(invalid(&format!(
                "has its {what} as TEXT at position {index}"
            ))),
        };
        let object = match fields.get(1) {
            Some(Tagged::Ref(id)) => *id,
            Some(Tagged::Integer(id)) => u64::try_from(*id)
                .map_err(|_| BindingError::OutOfRange(format!("object id {id}")))?,
            _ => return Err(invalid("has its object as REF or INTEGER at position 1")),
        };
        let (set, name) = (text(2, "set name")?, text(3, "property name")?);
        let action = match fields.first() {
            Some(Tagged::Enum(op))
                if op.eq_ignore_ascii_case("SET") && (5..=6).contains(&fields.len()) =>
            {
                let set_type = match fields.get(5) {
                    None | Some(Tagged::Null) => None,
                    Some(Tagged::Text(text)) => Some(text.clone()),
                    _ => return Err(invalid("has its set type as TEXT or NULL at position 5")),
                };
                EditAction::Set {
                    value: fields[4].clone(),
                    set_type,
                }
            }
            Some(Tagged::Enum(op)) if op.eq_ignore_ascii_case("REMOVE") && fields.len() == 4 => {
                EditAction::Remove
            }
            _ => {
                return Err(invalid(
                    "starts with ENUM SET (5 or 6 fields) or REMOVE (4 fields)",
                ))
            }
        };
        Ok(Self {
            object,
            set,
            name,
            action,
        })
    }
}

impl IfcModel {
    /// Apply `edits` as one checked transaction: every edit, in order, or,
    /// when any is refused, none, and the model is unchanged.
    ///
    /// Refused with `missing-entity` for an object not in the model,
    /// `wrong-entity-type` for an object that carries no property sets or a
    /// set type that disagrees with the set, `unsupported-schema` for a
    /// release other than IFC2X3, IFC4 or IFC4X3, `invalid-value` for a
    /// value the release does not admit, `template-violation` for one the
    /// catalog or the property's enumeration does not, `missing-property`
    /// for a removal of a property the object does not state (also one it
    /// only inherits), `unsupported` for a value form this writer does not
    /// write, `invalid-model` when the file's records prevent the edit, and
    /// `feature-disabled` without the `properties-write` feature, or without
    /// a catalog feature for a `Pset_` or `Qto_` set, and
    /// `catalog-not-loaded` for such a set in a `property-catalog-runtime`
    /// build before [`crate::catalog::load`] loaded its release. The message
    /// names the refused edit's position.
    pub fn set_properties(
        &mut self,
        edits: Vec<PropertyEdit>,
    ) -> Result<PropertyEditResult, BindingError> {
        #[cfg(feature = "properties-write")]
        {
            write::apply(self, edits)
        }
        #[cfg(not(feature = "properties-write"))]
        {
            let _ = edits;
            Err(BindingError::FeatureDisabled("properties-write"))
        }
    }

    /// Write one value; [`Self::set_properties`] with one edit. Returns the
    /// entity that holds the property afterwards.
    pub fn set_property(
        &mut self,
        object: u64,
        set: &str,
        name: &str,
        value: Tagged,
        set_type: Option<String>,
    ) -> Result<u64, BindingError> {
        let mut edit = PropertyEdit::set(object, set, name, value);
        if let EditAction::Set { set_type: slot, .. } = &mut edit.action {
            *slot = set_type;
        }
        let result = self.set_properties(vec![edit])?;
        result
            .properties
            .first()
            .copied()
            .flatten()
            .ok_or_else(|| BindingError::InvalidModel("the edit left no property".into()))
    }

    /// Remove one property; [`Self::set_properties`] with one edit.
    pub fn remove_property(
        &mut self,
        object: u64,
        set: &str,
        name: &str,
    ) -> Result<(), BindingError> {
        self.set_properties(vec![PropertyEdit::remove(object, set, name)])
            .map(drop)
    }
}

impl ToRecord for PropertyEditResult {
    fn to_record(&self) -> Record {
        Record::new(
            "PropertyEditResult",
            vec![
                (
                    "properties",
                    Field::List(self.properties.iter().map(|id| Field::id(*id)).collect()),
                ),
                ("created", Field::ids(self.created.iter().copied())),
                ("removed", Field::ids(self.removed.iter().copied())),
            ],
        )
    }
}

#[cfg(feature = "properties-write")]
mod write {
    use ifc::{apply_property_edits, EntityId, PropertyEditError, PropertyEditFailure, SetType};

    use super::{EditAction, PropertyEdit, PropertyEditResult};
    use crate::properties::read::property_error;
    use crate::{BindingError, IfcModel};

    pub(super) fn apply(
        model: &mut IfcModel,
        edits: Vec<PropertyEdit>,
    ) -> Result<PropertyEditResult, BindingError> {
        let edits = edits
            .into_iter()
            .enumerate()
            .map(|(index, edit)| facade_edit(edit).map_err(|error| at(index, error)))
            .collect::<Result<Vec<_>, _>>()?;
        let outcome = apply_property_edits(&mut model.inner, &edits).map_err(edit_error)?;
        let ids = |ids: Vec<EntityId>| ids.into_iter().map(|EntityId(id)| id).collect();
        Ok(PropertyEditResult {
            properties: outcome
                .properties
                .into_iter()
                .map(|id| id.map(|EntityId(id)| id))
                .collect(),
            created: ids(outcome.created),
            removed: ids(outcome.removed),
        })
    }

    fn facade_edit(edit: PropertyEdit) -> Result<ifc::PropertyEdit, BindingError> {
        let object = EntityId(edit.object);
        Ok(match edit.action {
            EditAction::Set { value, set_type } => {
                let mut facade =
                    ifc::PropertyEdit::set(object, edit.set, edit.name, value.into_value()?);
                if let Some(name) = set_type {
                    let set_type = SetType::from_entity(&name).ok_or_else(|| {
                        BindingError::InvalidValue(format!(
                            "set type {name:?} is neither IfcPropertySet nor IfcElementQuantity"
                        ))
                    })?;
                    facade = facade.with_set_type(set_type);
                }
                facade
            }
            EditAction::Remove => ifc::PropertyEdit::remove(object, edit.set, edit.name),
        })
    }

    /// `error` with the edit's position in front of its detail.
    fn at(index: usize, error: BindingError) -> BindingError {
        match error {
            BindingError::InvalidValue(detail) => {
                BindingError::InvalidValue(format!("edit {index}: {detail}"))
            }
            BindingError::OutOfRange(detail) => {
                BindingError::OutOfRange(format!("edit {index}: {detail}"))
            }
            other => other,
        }
    }

    fn edit_error(error: PropertyEditError) -> BindingError {
        use PropertyEditFailure as F;
        let detail = error.to_string();
        let position = error.edit.map(|edit| format!("edit {edit}: "));
        let prefixed = |inner: String| match &position {
            Some(position) => format!("{position}{inner}"),
            None => inner,
        };
        match error.failure {
            F::MissingEntity(EntityId(id)) => BindingError::MissingEntity(id),
            F::Resolve(resolve) => match property_error(resolve) {
                // The resolver's codes, with the edit's position.
                BindingError::UnsupportedSchema(inner) => {
                    BindingError::UnsupportedSchema(prefixed(inner))
                }
                BindingError::WrongEntityType(inner) => {
                    BindingError::WrongEntityType(prefixed(inner))
                }
                BindingError::InvalidModel(inner) => BindingError::InvalidModel(prefixed(inner)),
                BindingError::MissingReference(inner) => {
                    BindingError::MissingReference(prefixed(inner))
                }
                BindingError::BudgetExceeded(inner) => {
                    BindingError::BudgetExceeded(prefixed(inner))
                }
                BindingError::Unsupported(inner) => BindingError::Unsupported(prefixed(inner)),
                other => other,
            },
            F::Authoring(authoring) => authoring_error(authoring, detail),
            F::InvalidValue(_) => BindingError::InvalidValue(detail),
            F::Template(_) => BindingError::TemplateViolation(detail),
            F::MissingProperty { .. } => BindingError::MissingProperty(detail),
            F::WrongSetType(_) => BindingError::WrongEntityType(detail),
            F::Unsupported(_) => BindingError::Unsupported(detail),
            F::CatalogUnavailable(_) => BindingError::FeatureDisabled("property-catalog"),
            F::CatalogNotLoaded { .. } => BindingError::CatalogNotLoaded(detail),
            // The model's own records: an object without the GlobalId or
            // owner history a new record needs, or a planned transaction
            // whose preflight found a reference the file keeps.
            _ => BindingError::InvalidModel(detail),
        }
    }

    fn authoring_error(error: ifc::properties::PropertyError, detail: String) -> BindingError {
        use ifc::properties::PropertyError as E;
        match error {
            E::MissingEntity { id } => BindingError::MissingEntity(id.0),
            E::NotAQuantity { .. } | E::NotAQuantitySet { .. } => {
                BindingError::WrongEntityType(detail)
            }
            E::MultipleSchemas { .. } | E::UnsupportedSchema { .. } => {
                BindingError::UnsupportedSchema(detail)
            }
            E::MalformedEntitySlots { .. } => BindingError::InvalidModel(detail),
            _ => BindingError::InvalidValue(detail),
        }
    }
}
