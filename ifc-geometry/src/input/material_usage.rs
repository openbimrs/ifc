//! Geometry-only projections of MaterialResource usage entities.
//!
//! These views intentionally overlap a few authored slots with `ifc-material`.
//! Sibling domain crates may not depend on one another: `ifc-material` owns material
//! identity/quantities; this module owns only shape inputs. Values stay in project
//! units until lowering. No material semantics enter Axiolid.

use ifc_model::{Entity, EntityId, Value};

use crate::error::{GeometryError, GeometryResult};
use crate::slots::Slots;

/// Axis along which an IFC material layer set affects product geometry.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerSetDirection {
    Axis1,
    Axis2,
    Axis3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectionSense {
    Positive,
    Negative,
}

/// Positive IFC cardinal reference. Standard values are 1 through 19.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardinalPoint(u64);
impl CardinalPoint {
    pub fn get(self) -> u64 {
        self.0
    }
    pub fn standard(self) -> Option<u8> {
        u8::try_from(self.0)
            .ok()
            .filter(|value| (1..=19).contains(value))
    }
}

fn checked<'m>(
    id: EntityId,
    entity: &'m Entity,
    expected: &'static str,
) -> GeometryResult<Slots<'m>> {
    if !entity.is_type(expected) {
        return Err(GeometryError::WrongEntityType {
            entity: id,
            actual: entity.type_name.to_string(),
            expected,
        });
    }
    Ok(Slots::new(id, entity))
}

fn optional_i64(
    slots: &Slots<'_>,
    index: usize,
    name: &'static str,
) -> GeometryResult<Option<i64>> {
    match slots.opt(index) {
        None => Ok(None),
        Some(_) => slots.req_i64(index, name).map(Some),
    }
}

fn optional_f64(
    slots: &Slots<'_>,
    index: usize,
    name: &'static str,
) -> GeometryResult<Option<f64>> {
    match slots.opt(index) {
        None => Ok(None),
        Some(_) => slots.req_f64(index, name).map(Some),
    }
}

fn cardinal(
    slots: &Slots<'_>,
    index: usize,
    name: &'static str,
) -> GeometryResult<Option<CardinalPoint>> {
    let Some(value) = optional_i64(slots, index, name)? else {
        return Ok(None);
    };
    let value = u64::try_from(value)
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| slots.degenerate(format!("{name} must be positive")))?;
    Ok(Some(CardinalPoint(value)))
}

fn positive_optional(
    slots: &Slots<'_>,
    index: usize,
    name: &'static str,
) -> GeometryResult<Option<f64>> {
    let value = optional_f64(slots, index, name)?;
    if value.is_some_and(|value| !value.is_finite() || value <= 0.0) {
        return Err(slots.degenerate(format!("{name} must be finite and positive")));
    }
    Ok(value)
}

fn required_enum<'a>(
    slots: &'a Slots<'_>,
    index: usize,
    name: &'static str,
) -> GeometryResult<&'a str> {
    let value = slots.req(index, name)?;
    match value {
        Value::Enum(token) => Ok(token),
        _ => Err(GeometryError::WrongValueKind {
            entity: slots.id(),
            type_name: slots.type_name().to_string(),
            attribute: name,
            expected: "an enumeration",
            found: format!("{value:?}"),
        }),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MaterialProfileSetUsageGeometry<'m> {
    slots: Slots<'m>,
}
impl<'m> MaterialProfileSetUsageGeometry<'m> {
    pub fn new(id: EntityId, entity: &'m Entity) -> GeometryResult<Self> {
        Ok(Self {
            slots: checked(id, entity, "IFCMATERIALPROFILESETUSAGE")?,
        })
    }
    pub fn profile_set_id(self) -> GeometryResult<EntityId> {
        self.slots.req_ref(0, "ForProfileSet")
    }
    pub fn cardinal_point(self) -> GeometryResult<Option<CardinalPoint>> {
        cardinal(&self.slots, 1, "CardinalPoint")
    }
    pub fn reference_extent(self) -> GeometryResult<Option<f64>> {
        positive_optional(&self.slots, 2, "ReferenceExtent")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MaterialProfileSetUsageTaperingGeometry<'m> {
    slots: Slots<'m>,
}
impl<'m> MaterialProfileSetUsageTaperingGeometry<'m> {
    pub fn new(id: EntityId, entity: &'m Entity) -> GeometryResult<Self> {
        Ok(Self {
            slots: checked(id, entity, "IFCMATERIALPROFILESETUSAGETAPERING")?,
        })
    }
    pub fn profile_set_id(self) -> GeometryResult<EntityId> {
        self.slots.req_ref(0, "ForProfileSet")
    }
    pub fn cardinal_point(self) -> GeometryResult<Option<CardinalPoint>> {
        cardinal(&self.slots, 1, "CardinalPoint")
    }
    pub fn reference_extent(self) -> GeometryResult<Option<f64>> {
        positive_optional(&self.slots, 2, "ReferenceExtent")
    }
    pub fn end_profile_set_id(self) -> GeometryResult<EntityId> {
        self.slots.req_ref(3, "ForProfileEndSet")
    }
    pub fn cardinal_end_point(self) -> GeometryResult<Option<CardinalPoint>> {
        cardinal(&self.slots, 4, "CardinalEndPoint")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MaterialProfileGeometry<'m> {
    slots: Slots<'m>,
}
impl<'m> MaterialProfileGeometry<'m> {
    pub fn new(id: EntityId, entity: &'m Entity) -> GeometryResult<Self> {
        if !entity.is_type("IFCMATERIALPROFILE") && !entity.is_type("IFCMATERIALPROFILEWITHOFFSETS")
        {
            return Err(GeometryError::WrongEntityType {
                entity: id,
                actual: entity.type_name.to_string(),
                expected: "IfcMaterialProfile",
            });
        }
        Ok(Self {
            slots: Slots::new(id, entity),
        })
    }
    pub fn profile_id(self) -> GeometryResult<EntityId> {
        self.slots.req_ref(3, "Profile")
    }
    pub fn offset_values(self) -> GeometryResult<Option<[f64; 2]>> {
        if !self
            .slots
            .type_name()
            .eq_ignore_ascii_case("IFCMATERIALPROFILEWITHOFFSETS")
        {
            return Ok(None);
        }
        let values = self.slots.req_f64_list(6, "OffsetValues")?;
        let values: [f64; 2] = values.try_into().map_err(|_| {
            self.slots
                .degenerate("OffsetValues must contain exactly two lengths")
        })?;
        if !values.iter().all(|value| value.is_finite()) {
            return Err(self.slots.degenerate("OffsetValues must be finite"));
        }
        Ok(Some(values))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MaterialLayerSetUsageGeometry<'m> {
    slots: Slots<'m>,
}
impl<'m> MaterialLayerSetUsageGeometry<'m> {
    pub fn new(id: EntityId, entity: &'m Entity) -> GeometryResult<Self> {
        Ok(Self {
            slots: checked(id, entity, "IFCMATERIALLAYERSETUSAGE")?,
        })
    }
    pub fn layer_set_id(self) -> GeometryResult<EntityId> {
        self.slots.req_ref(0, "ForLayerSet")
    }
    pub fn layer_set_direction(self) -> GeometryResult<LayerSetDirection> {
        match required_enum(&self.slots, 1, "LayerSetDirection")? {
            token if token.eq_ignore_ascii_case("AXIS1") => Ok(LayerSetDirection::Axis1),
            token if token.eq_ignore_ascii_case("AXIS2") => Ok(LayerSetDirection::Axis2),
            token if token.eq_ignore_ascii_case("AXIS3") => Ok(LayerSetDirection::Axis3),
            _ => Err(self
                .slots
                .degenerate("LayerSetDirection must be AXIS1, AXIS2, or AXIS3")),
        }
    }
    pub fn direction_sense(self) -> GeometryResult<DirectionSense> {
        match required_enum(&self.slots, 2, "DirectionSense")? {
            token if token.eq_ignore_ascii_case("POSITIVE") => Ok(DirectionSense::Positive),
            token if token.eq_ignore_ascii_case("NEGATIVE") => Ok(DirectionSense::Negative),
            _ => Err(self
                .slots
                .degenerate("DirectionSense must be POSITIVE or NEGATIVE")),
        }
    }
    pub fn offset_from_reference_line(self) -> GeometryResult<f64> {
        let value = self.slots.req_f64(3, "OffsetFromReferenceLine")?;
        if !value.is_finite() {
            return Err(self
                .slots
                .degenerate("OffsetFromReferenceLine must be finite"));
        }
        Ok(value)
    }
    pub fn reference_extent(self) -> GeometryResult<Option<f64>> {
        positive_optional(&self.slots, 4, "ReferenceExtent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn r(id: u64) -> Value {
        Value::Ref(EntityId(id))
    }

    #[test]
    fn profile_usage_reads_absolute_cardinal_and_taper_slots() {
        let entity = Entity::new(
            "IFCMATERIALPROFILESETUSAGETAPERING",
            vec![
                r(10),
                Value::Integer(9),
                Value::Real(4.0),
                r(11),
                Value::Integer(5),
            ],
        );
        let view = MaterialProfileSetUsageTaperingGeometry::new(EntityId(1), &entity).unwrap();
        assert_eq!(view.profile_set_id().unwrap(), EntityId(10));
        assert_eq!(view.cardinal_point().unwrap().unwrap().standard(), Some(9));
        assert_eq!(view.reference_extent().unwrap(), Some(4.0));
        assert_eq!(view.end_profile_set_id().unwrap(), EntityId(11));
        assert_eq!(view.cardinal_end_point().unwrap().unwrap().get(), 5);
    }

    #[test]
    fn layer_usage_preserves_axis_sense_signed_offset_and_extent() {
        let entity = Entity::new(
            "IFCMATERIALLAYERSETUSAGE",
            vec![
                r(20),
                Value::Enum("AXIS2".into()),
                Value::Enum("NEGATIVE".into()),
                Value::Real(-0.25),
                Value::Real(8.0),
            ],
        );
        let view = MaterialLayerSetUsageGeometry::new(EntityId(2), &entity).unwrap();
        assert_eq!(view.layer_set_id().unwrap(), EntityId(20));
        assert_eq!(
            view.layer_set_direction().unwrap(),
            LayerSetDirection::Axis2
        );
        assert_eq!(view.direction_sense().unwrap(), DirectionSense::Negative);
        assert_eq!(view.offset_from_reference_line().unwrap(), -0.25);
        assert_eq!(view.reference_extent().unwrap(), Some(8.0));
    }

    #[test]
    fn profile_offsets_are_kept_as_two_signed_project_lengths() {
        let entity = Entity::new(
            "IFCMATERIALPROFILEWITHOFFSETS",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                r(30),
                Value::Null,
                Value::Null,
                Value::List(vec![Value::Real(-1.0), Value::Real(2.0)]),
            ],
        );
        let view = MaterialProfileGeometry::new(EntityId(3), &entity).unwrap();
        assert_eq!(view.profile_id().unwrap(), EntityId(30));
        assert_eq!(view.offset_values().unwrap(), Some([-1.0, 2.0]));
    }

    #[test]
    fn invalid_geometry_inputs_report_the_source_entity() {
        let entity = Entity::new(
            "IFCMATERIALPROFILESETUSAGE",
            vec![r(1), Value::Integer(0), Value::Real(-2.0)],
        );
        let view = MaterialProfileSetUsageGeometry::new(EntityId(77), &entity).unwrap();
        for error in [
            view.cardinal_point().unwrap_err(),
            view.reference_extent().unwrap_err(),
        ] {
            assert_eq!(error.entity(), Some(EntityId(77)));
        }
    }
}
