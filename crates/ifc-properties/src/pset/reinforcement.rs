//! Reinforcement, section, and profile property families.
//!
//! These five carry no WHERE rules. What constrains them is structural:
//! required slots with no unset fallback, aggregate lower bounds of
//! `[1:?]`, closed enumerations, and the measure type each numeric slot
//! declares -- including `IfcCountMeasure`, which is an EXPRESS INTEGER.
//!
//! Only `IfcReinforcementDefinitionProperties` is an
//! `IfcPropertySetDefinition` with the four `IfcRoot` slots. The other
//! four are `IfcPropertyAbstraction` subtypes and start at their own
//! first attribute.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::PropertyResult;

use super::authoring::optional_text;
use super::predefined::{invalid, measure, root_slots, Measure};
const BAR_SURFACE: &[&str] = &["PLAIN", "TEXTURED"];

/// Attributes of an `IfcReinforcementBarProperties`.
///
/// `total_cross_section_area` and `steel_grade` are required by the
/// schema, so they are plain fields rather than options.
#[derive(Debug, Clone, Copy)]
pub struct ReinforcementBarDraft<'a> {
    /// `TotalCrossSectionArea`, an area measure. Required.
    pub total_cross_section_area: f64,
    /// `SteelGrade`. Required.
    pub steel_grade: &'a str,
    /// `BarSurface`: `PLAIN` or `TEXTURED`.
    pub bar_surface: Option<&'a str>,
    /// `EffectiveDepth`, a length. May be negative.
    pub effective_depth: Option<f64>,
    /// `NominalBarDiameter`, a positive length.
    pub nominal_bar_diameter: Option<f64>,
    /// `BarCount`, an integer count.
    pub bar_count: Option<i64>,
}

/// Stage an `IfcReinforcementBarProperties`.
///
/// # Errors
///
/// Refuses a blank steel grade, a non-finite or negative cross-section
/// area, a bar surface outside the enumeration, and a measure that
/// violates its schema measure type.
pub fn add_reinforcement_bar_properties(
    tx: &mut Transaction,
    draft: ReinforcementBarDraft<'_>,
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCREINFORCEMENTBARPROPERTIES";

    if draft.steel_grade.trim().is_empty() {
        return Err(invalid(ENTITY, "SteelGrade", "blank"));
    }
    // An area is a magnitude: negative has no reading, and the slot is
    // required, so there is no "unset" to fall back to.
    if !draft.total_cross_section_area.is_finite() || draft.total_cross_section_area < 0.0 {
        return Err(invalid(
            ENTITY,
            "TotalCrossSectionArea",
            format!("{}", draft.total_cross_section_area),
        ));
    }
    if let Some(surface) = draft.bar_surface {
        if !BAR_SURFACE.contains(&surface) {
            return Err(invalid(ENTITY, "BarSurface", surface));
        }
    }

    let mut attributes = vec![Value::Null; 6];
    attributes[0] = Value::Real(draft.total_cross_section_area);
    attributes[1] = Value::Text(draft.steel_grade.into());
    attributes[2] = draft
        .bar_surface
        .map_or(Value::Null, |s| Value::Enum(s.into()));
    attributes[3] = measure(
        ENTITY,
        "EffectiveDepth",
        draft.effective_depth,
        Measure::Length,
    )?;
    attributes[4] = measure(
        ENTITY,
        "NominalBarDiameter",
        draft.nominal_bar_diameter,
        Measure::Positive,
    )?;
    // IfcCountMeasure is declared INTEGER in EXPRESS. Writing it as a
    // real produces a record of the right arity that states the wrong
    // type, which the structural validator does not catch.
    attributes[5] = draft.bar_count.map_or(Value::Null, Value::Integer);

    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

const SECTION_TYPE: &[&str] = &["TAPERED", "UNIFORM"];

/// Stage an `IfcSectionProperties`.
///
/// `section_type` is `TAPERED` or `UNIFORM`. A uniform section may still
/// carry an end profile; the schema does not tie the two, and refusing
/// the combination here would reject files it permits.
///
/// # Errors
///
/// Refuses a section type outside the enumeration.
pub fn add_section_properties(
    tx: &mut Transaction,
    section_type: &str,
    start_profile: EntityId,
    end_profile: Option<EntityId>,
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCSECTIONPROPERTIES";

    if !SECTION_TYPE.contains(&section_type) {
        return Err(invalid(ENTITY, "SectionType", section_type));
    }

    let mut attributes = vec![Value::Null; 3];
    attributes[0] = Value::Enum(section_type.into());
    attributes[1] = Value::Ref(start_profile);
    attributes[2] = end_profile.map_or(Value::Null, Value::Ref);

    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

const BAR_ROLE: &[&str] = &[
    "ANCHORING",
    "EDGE",
    "LIGATURE",
    "MAIN",
    "PUNCHING",
    "RING",
    "SHEAR",
    "STUD",
    "USERDEFINED",
    "NOTDEFINED",
];

/// Attributes of an `IfcSectionReinforcementProperties`.
#[derive(Debug, Clone, Copy)]
pub struct SectionReinforcementDraft<'a> {
    /// `LongitudinalStartPosition`, a length. Required.
    pub longitudinal_start_position: f64,
    /// `LongitudinalEndPosition`, a length. Required.
    pub longitudinal_end_position: f64,
    /// `TransversePosition`, a length.
    pub transverse_position: Option<f64>,
    /// `ReinforcementRole`, from `IfcReinforcingBarRoleEnum`. Required.
    pub reinforcement_role: &'a str,
    /// `SectionDefinition`: an `IfcSectionProperties`. Required.
    pub section_definition: EntityId,
    /// `CrossSectionReinforcementDefinitions`: at least one
    /// `IfcReinforcementBarProperties`.
    pub cross_section_reinforcement_definitions: &'a [EntityId],
}

/// Stage an `IfcSectionReinforcementProperties`.
///
/// Both longitudinal positions are `IfcLengthMeasure`, which admits
/// negatives, and the schema states no ordering between them: a section
/// measured from a datum may legitimately start at a negative station.
/// Only non-finite values are refused.
///
/// # Errors
///
/// Refuses a role outside the enumeration, a non-finite position, and an
/// empty reinforcement-definition set, whose schema bound is `[1:?]`.
pub fn add_section_reinforcement_properties(
    tx: &mut Transaction,
    draft: SectionReinforcementDraft<'_>,
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCSECTIONREINFORCEMENTPROPERTIES";

    if !BAR_ROLE.contains(&draft.reinforcement_role) {
        return Err(invalid(
            ENTITY,
            "ReinforcementRole",
            draft.reinforcement_role,
        ));
    }
    if draft.cross_section_reinforcement_definitions.is_empty() {
        return Err(invalid(
            ENTITY,
            "CrossSectionReinforcementDefinitions",
            "empty",
        ));
    }

    let mut attributes = vec![Value::Null; 6];
    attributes[0] = measure(
        ENTITY,
        "LongitudinalStartPosition",
        Some(draft.longitudinal_start_position),
        Measure::Length,
    )?;
    attributes[1] = measure(
        ENTITY,
        "LongitudinalEndPosition",
        Some(draft.longitudinal_end_position),
        Measure::Length,
    )?;
    attributes[2] = measure(
        ENTITY,
        "TransversePosition",
        draft.transverse_position,
        Measure::Length,
    )?;
    attributes[3] = Value::Enum(draft.reinforcement_role.into());
    attributes[4] = Value::Ref(draft.section_definition);
    attributes[5] = Value::List(
        draft
            .cross_section_reinforcement_definitions
            .iter()
            .copied()
            .map(Value::Ref)
            .collect(),
    );

    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

/// Stage an `IfcReinforcementDefinitionProperties`.
///
/// This is the only member of the family that is an
/// `IfcPropertySetDefinition`, so it carries the four `IfcRoot` slots and
/// a GlobalId. The others are `IfcPropertyAbstraction` subtypes with no
/// identity of their own.
///
/// # Errors
///
/// Refuses a malformed GlobalId and an empty section-definition list,
/// whose schema bound is `[1:?]`.
pub fn add_reinforcement_definition_properties(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    description: Option<&str>,
    definition_type: Option<&str>,
    reinforcement_section_definitions: &[EntityId],
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCREINFORCEMENTDEFINITIONPROPERTIES";

    if reinforcement_section_definitions.is_empty() {
        return Err(invalid(ENTITY, "ReinforcementSectionDefinitions", "empty"));
    }

    let mut attributes = root_slots(ENTITY, global_id, name, description, 6)?;
    attributes[4] = optional_text(definition_type);
    attributes[5] = Value::List(
        reinforcement_section_definitions
            .iter()
            .copied()
            .map(Value::Ref)
            .collect(),
    );

    Ok(tx.create(Entity::new(ENTITY, attributes)))
}

/// Stage an `IfcProfileProperties`.
///
/// `properties` are `IfcProperty` instances, not measures: this is the
/// extended-property form, where the profile's characteristics are
/// expressed as named properties rather than fixed typed slots.
///
/// # Errors
///
/// Refuses an empty property set, whose schema bound is `[1:?]`.
pub fn add_profile_properties(
    tx: &mut Transaction,
    name: Option<&str>,
    description: Option<&str>,
    properties: &[EntityId],
    profile_definition: EntityId,
) -> PropertyResult<EntityId> {
    const ENTITY: &str = "IFCPROFILEPROPERTIES";

    if properties.is_empty() {
        return Err(invalid(ENTITY, "Properties", "empty"));
    }

    let mut attributes = vec![Value::Null; 4];
    attributes[0] = optional_text(name);
    attributes[1] = optional_text(description);
    attributes[2] = Value::List(properties.iter().copied().map(Value::Ref).collect());
    attributes[3] = Value::Ref(profile_definition);

    Ok(tx.create(Entity::new(ENTITY, attributes)))
}
