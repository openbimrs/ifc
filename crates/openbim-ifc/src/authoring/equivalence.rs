//! When an appended entity is one the target holds already (#479): equal
//! by value (a unit, context, owner history, material or style the
//! options reuse), or the holder of the values a `UNIQUE` clause makes an
//! identity of.

use std::collections::BTreeMap;

use ifc_model::{Entity, EntityId, Value};
use ifc_schema::Schema;

use super::append::Source;
use super::kinds::unique_keys;
use super::plan::Planner;
use super::{release, AppendOptions};

/// The owner history chain, reused with `owner_history`.
const OWNER_CHAIN: &[&str] = &[
    "IfcOwnerHistory",
    "IfcPerson",
    "IfcOrganization",
    "IfcPersonAndOrganization",
    "IfcApplication",
    "IfcActorRole",
    "IfcAddress",
];

const UNITS: &[&str] = &[
    "IfcNamedUnit",
    "IfcDerivedUnit",
    "IfcDerivedUnitElement",
    "IfcMonetaryUnit",
    "IfcDimensionalExponents",
    "IfcMeasureWithUnit",
];

const MATERIALS: &[&str] = &[
    "IfcMaterialDefinition",
    "IfcMaterial",
    "IfcMaterialLayer",
    "IfcMaterialLayerSet",
    "IfcMaterialList",
    "IfcMaterialProfile",
    "IfcMaterialProfileSet",
    "IfcMaterialConstituent",
    "IfcMaterialConstituentSet",
];

const STYLES: &[&str] = &[
    "IfcPresentationStyle",
    "IfcPresentationStyleAssignment",
    "IfcColourSpecification",
];

impl Planner<'_, '_> {
    /// Whether `type_name` is a resource the options reuse.
    pub(super) fn reusable(&self, source: &Source<'_>, options: &AppendOptions, type_name: &str) -> bool {
        let any = |classes: &[&str]| classes.iter().any(|c| source.schema.is_a(type_name, c));
        let reuse = options.reuse;
        (reuse.units && any(UNITS))
            || (reuse.contexts && source.schema.is_a(type_name, "IfcRepresentationContext"))
            || (reuse.owner_history && any(OWNER_CHAIN))
            || (reuse.materials && any(MATERIALS))
            || (reuse.styles && any(STYLES))
    }

    /// A target entity equal to source entity `id`, when the two releases
    /// lay its type out alike.
    pub(super) fn find_equal(
        &self,
        source: &Source<'_>,
        reused: &BTreeMap<EntityId, EntityId>,
        id: EntityId,
    ) -> Option<EntityId> {
        let entity = source.get(id)?;
        if !release::same_layout(source.schema, self.schema, &entity.type_name) {
            return None;
        }
        let upper = entity.type_name.to_ascii_uppercase();
        self.view.ids_of_type(&upper).into_iter().find(|candidate| {
            let mut memo = BTreeMap::new();
            self.equal(source, reused, &mut memo, id, *candidate)
        })
    }

    /// The target entity holding the values of a `UNIQUE` clause source
    /// entity `id` holds (an `IfcApplication`'s identifier, an
    /// `IfcPropertyEnumeration`'s name).
    pub(super) fn find_by_key(
        &self,
        source: &Source<'_>,
        reused: &BTreeMap<EntityId, EntityId>,
        id: EntityId,
    ) -> Option<EntityId> {
        let entity = source.get(id)?;
        let key = |schema: &Schema, entity: &Entity, names: &[String]| -> Option<Vec<Value>> {
            let layout = schema.attributes(&entity.type_name);
            names
                .iter()
                .map(|name| {
                    let slot = layout.iter().position(|a| a.name.eq_ignore_ascii_case(name))?;
                    entity
                        .attributes
                        .get(slot)
                        .filter(|value| !matches!(value, Value::Null | Value::Derived))
                        .cloned()
                })
                .collect()
        };
        let types: Vec<String> = self
            .view
            .base()
            .type_histogram()
            .into_iter()
            .map(|(type_name, _)| type_name.to_owned())
            .collect();
        for (declaring, names) in unique_keys(source.schema, &entity.type_name) {
            let Some(wanted) = key(source.schema, entity, &names) else {
                continue;
            };
            for type_name in types.iter().filter(|t| self.schema.is_a(t, &declaring)) {
                for candidate in self.view.ids_of_type(type_name) {
                    let Some(found) = self.view.get(candidate) else {
                        continue;
                    };
                    let Some(held) = key(self.schema, found, &names) else {
                        continue;
                    };
                    let mut memo = BTreeMap::new();
                    if wanted
                        .iter()
                        .zip(&held)
                        .all(|(a, b)| self.equal_values(source, reused, &mut memo, a, b))
                    {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    }

    /// Whether source entity `s` and target entity `t` are equal by value.
    pub(super) fn equal(
        &self,
        source: &Source<'_>,
        reused: &BTreeMap<EntityId, EntityId>,
        memo: &mut BTreeMap<(EntityId, EntityId), bool>,
        s: EntityId,
        t: EntityId,
    ) -> bool {
        if reused.get(&s) == Some(&t) {
            return true;
        }
        if let Some(known) = memo.get(&(s, t)) {
            return *known;
        }
        // Assumed equal while compared, so a cycle closes.
        memo.insert((s, t), true);
        let equal = match (source.get(s), self.view.get(t)) {
            (Some(a), Some(b)) => {
                a.type_name.eq_ignore_ascii_case(&b.type_name)
                    && a.attributes.len() == b.attributes.len()
                    && a
                        .attributes
                        .iter()
                        .zip(&b.attributes)
                        .all(|(x, y)| self.equal_values(source, reused, memo, x, y))
            }
            _ => false,
        };
        memo.insert((s, t), equal);
        equal
    }

    pub(super) fn equal_values(
        &self,
        source: &Source<'_>,
        reused: &BTreeMap<EntityId, EntityId>,
        memo: &mut BTreeMap<(EntityId, EntityId), bool>,
        a: &Value,
        b: &Value,
    ) -> bool {
        match (a, b) {
            (Value::Ref(x), Value::Ref(y)) => self.equal(source, reused, memo, *x, *y),
            (Value::List(xs), Value::List(ys)) => {
                xs.len() == ys.len()
                    && xs
                        .iter()
                        .zip(ys)
                        .all(|(x, y)| self.equal_values(source, reused, memo, x, y))
            }
            (
                Value::Typed {
                    type_name: n,
                    value: x,
                },
                Value::Typed {
                    type_name: m,
                    value: y,
                },
            ) => n.eq_ignore_ascii_case(m) && self.equal_values(source, reused, memo, x, y),
            (Value::Enum(x), Value::Enum(y)) => x.eq_ignore_ascii_case(y),
            #[allow(clippy::cast_precision_loss)]
            (Value::Real(x), Value::Integer(y)) | (Value::Integer(y), Value::Real(x)) => {
                *x == *y as f64
            }
            _ => a == b,
        }
    }
}
