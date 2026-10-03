//! The `GlobalId` and `Name` of an `IfcRoot` entity.
//!
//! Every object, type, relationship and property definition is an
//! `IfcRoot`, and a binding or report that lists them wants to key them by
//! `GlobalId` and label them by `Name`. Both are `IfcRoot` attributes, but
//! whether an entity *is* an `IfcRoot` is a question for the schema, which
//! ADR 0003 keeps out of `ifc-model`, so the join lives here. The slots are
//! looked up by attribute name in the table the caller passes, never
//! assumed.
//!
//! ```
//! let model = ifc::Model::new();
//! assert!(ifc::root_identity(&model, ifc::schema::ifc4(), ifc::EntityId(1)).is_none());
//! ```
#![cfg(feature = "schema-api")]

use ifc_model::{EntityId, Model, Value};
use ifc_schema::Schema;

/// The identity attributes of one `IfcRoot` entity, borrowed from the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RootIdentity<'m> {
    /// `GlobalId`, when it holds text. The schema requires it; `None` means
    /// the file left it unset or wrote something that is not a string.
    pub global_id: Option<&'m str>,
    /// `Name`, when it holds text.
    pub name: Option<&'m str>,
}

/// The `GlobalId` and `Name` of entity `id`, read against `schema`.
///
/// `None` when `id` is not in the model or `schema` does not declare its
/// type an `IfcRoot`: a `IfcCartesianPoint` has no identity to report, and
/// one is never guessed from the first slot. A typed wrapper such as
/// `IFCGLOBALLYUNIQUEID('...')` is read through to its text.
#[must_use]
pub fn root_identity<'m>(
    model: &'m Model,
    schema: &Schema,
    id: EntityId,
) -> Option<RootIdentity<'m>> {
    let entity = model.get(id)?;
    if !schema.is_a(&entity.type_name, "IFCROOT") {
        return None;
    }
    let names = schema.attribute_names(&entity.type_name);
    let text = |attribute: &str| {
        let slot = names
            .iter()
            .position(|name| name.eq_ignore_ascii_case(attribute))?;
        match entity.attributes.get(slot)?.unwrap_typed() {
            Value::Text(text) => Some(&**text),
            _ => None,
        }
    };
    Some(RootIdentity {
        global_id: text("GlobalId"),
        name: text("Name"),
    })
}

#[cfg(all(test, feature = "ifc4"))]
mod tests {
    use super::*;
    use ifc_model::Entity;

    fn model() -> Model {
        let mut model = Model::new();
        model.insert(
            EntityId(1),
            Entity::new(
                "IFCWALL",
                vec![
                    Value::Text("2O2Fr$t4X7Zf8NOew3FLOH".into()),
                    Value::Null,
                    Value::Text("Wall".into()),
                ],
            ),
        );
        model.insert(
            EntityId(2),
            Entity::new("IFCCARTESIANPOINT", vec![Value::Text("x".into())]),
        );
        model
    }

    #[test]
    fn a_root_entity_reports_its_global_id_and_name() {
        let model = model();
        let identity = root_identity(&model, ifc_schema::ifc4(), EntityId(1)).unwrap();
        assert_eq!(identity.global_id, Some("2O2Fr$t4X7Zf8NOew3FLOH"));
        assert_eq!(identity.name, Some("Wall"));
    }

    #[test]
    fn a_non_root_or_absent_entity_has_no_identity() {
        let model = model();
        assert_eq!(
            root_identity(&model, ifc_schema::ifc4(), EntityId(2)),
            None,
            "a point's first slot is not a GlobalId"
        );
        assert_eq!(root_identity(&model, ifc_schema::ifc4(), EntityId(9)), None);
    }
}
