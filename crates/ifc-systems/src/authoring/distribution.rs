//! Distribution element occurrences and the two grouping forms.
//!
//! # Why one writer per class and not one taking a type name
//!
//! `IfcFlowSegment`, `IfcFlowFitting` and their siblings share an
//! identical eight-slot layout inherited from `IfcDistributionElement`.
//! A single `create_distribution_element(type_name, ..)` would be less
//! code, but it would also accept `"IFCWALL"` or a typo, and the caller
//! would learn about it from a validator rather than the compiler. The
//! enum below keeps the set closed: every variant is a class the flow
//! reader in this crate already understands.
//!
//! # `IfcZone` and `IfcSpatialZone` are not the same thing
//!
//! Both group spaces, but `IfcZone` is an `IfcGroup` -- membership is
//! an objectified relationship, and it carries no placement. An
//! `IfcSpatialZone` is an `IfcSpatialElement`: it has its own
//! placement, representation and a `PredefinedType`, so it can occupy
//! geometry a zone cannot. Authoring the wrong one produces a file that
//! parses and then answers spatial queries incorrectly, which is why
//! they get separate constructors rather than a flag.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::authoring::{invalid, SystemAuthoringResult};

/// `IfcDistributionElement` slots, from IFC4X3 ADD2.
///
/// All eight are inherited: `IfcRoot` contributes 0-3, `IfcObject`
/// contributes `ObjectType` at 4, `IfcProduct` contributes placement
/// and representation at 5-6, and `IfcElement` contributes `Tag` at 7.
/// None of the concrete subtypes in [`DistributionElementKind`] add an
/// attribute of their own, so the width is the same for every one.
pub(crate) mod distribution_slot {
    /// `GlobalId`.
    pub const GLOBAL_ID: usize = 0;
    /// `Name`.
    pub const NAME: usize = 2;
    /// `Description`.
    pub const DESCRIPTION: usize = 3;
    /// `ObjectPlacement`.
    pub const OBJECT_PLACEMENT: usize = 5;
    /// `Representation`.
    pub const REPRESENTATION: usize = 6;
    /// `Tag`.
    pub const TAG: usize = 7;
    /// Total attribute count.
    pub const WIDTH: usize = 8;
}

/// `IfcZone` slots. `IfcGroup` has no placement, so the list is short.
pub(crate) mod zone_slot {
    /// `GlobalId`.
    pub const GLOBAL_ID: usize = 0;
    /// `Name`.
    pub const NAME: usize = 2;
    /// `Description`.
    pub const DESCRIPTION: usize = 3;
    /// `LongName`, declared by `IfcZone` itself.
    pub const LONG_NAME: usize = 5;
    /// Total attribute count.
    pub const WIDTH: usize = 6;
}

/// `IfcSpatialZone` slots.
pub(crate) mod spatial_zone_slot {
    /// `GlobalId`.
    pub const GLOBAL_ID: usize = 0;
    /// `Name`.
    pub const NAME: usize = 2;
    /// `Description`.
    pub const DESCRIPTION: usize = 3;
    /// `ObjectPlacement`.
    pub const OBJECT_PLACEMENT: usize = 5;
    /// `Representation`.
    pub const REPRESENTATION: usize = 6;
    /// `LongName`, from `IfcSpatialElement`.
    pub const LONG_NAME: usize = 7;
    /// `PredefinedType`, declared by `IfcSpatialZone`.
    pub const PREDEFINED_TYPE: usize = 8;
    /// Total attribute count.
    pub const WIDTH: usize = 9;
}

/// A concrete distribution element class.
///
/// The variants are exactly the classes this crate's flow reader
/// traverses. Holding them in a closed enum means a caller cannot
/// stage a class the reader will later fail to recognise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributionElementKind {
    /// `IfcDistributionElement`: the general form.
    DistributionElement,
    /// `IfcEnergyConversionDevice`: boilers, chillers, coils.
    EnergyConversionDevice,
    /// `IfcFlowController`: valves, dampers, switches.
    FlowController,
    /// `IfcFlowFitting`: junctions, bends, transitions.
    FlowFitting,
    /// `IfcFlowMovingDevice`: pumps, fans, compressors.
    FlowMovingDevice,
    /// `IfcFlowSegment`: a run of duct, pipe or cable.
    FlowSegment,
    /// `IfcFlowStorageDevice`: tanks and batteries.
    FlowStorageDevice,
    /// `IfcFlowTerminal`: the point where flow leaves the system.
    FlowTerminal,
    /// `IfcFlowTreatmentDevice`: filters, interceptors.
    FlowTreatmentDevice,
}

impl DistributionElementKind {
    /// The IFC entity name this kind stages.
    pub fn type_name(self) -> &'static str {
        match self {
            Self::DistributionElement => "IFCDISTRIBUTIONELEMENT",
            Self::EnergyConversionDevice => "IFCENERGYCONVERSIONDEVICE",
            Self::FlowController => "IFCFLOWCONTROLLER",
            Self::FlowFitting => "IFCFLOWFITTING",
            Self::FlowMovingDevice => "IFCFLOWMOVINGDEVICE",
            Self::FlowSegment => "IFCFLOWSEGMENT",
            Self::FlowStorageDevice => "IFCFLOWSTORAGEDEVICE",
            Self::FlowTerminal => "IFCFLOWTERMINAL",
            Self::FlowTreatmentDevice => "IFCFLOWTREATMENTDEVICE",
        }
    }
}

/// Optional descriptive attributes shared by every distribution element.
///
/// Grouped into a struct because all six are optional and positional
/// arguments at that count invite silent transposition -- `name` and
/// `description` are both `Option<&str>` and swapping them compiles.
#[derive(Debug, Clone, Copy, Default)]
pub struct ElementAttributes<'a> {
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ObjectPlacement`, usually an `IfcLocalPlacement`.
    pub placement: Option<EntityId>,
    /// `Representation`, an `IfcProductDefinitionShape`.
    pub representation: Option<EntityId>,
    /// `Tag`, the authoring tool's own identifier.
    pub tag: Option<&'a str>,
}

fn text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |v| Value::Text(v.into()))
}

fn reference(value: Option<EntityId>) -> Value {
    value.map_or(Value::Null, Value::Ref)
}

/// Stage a distribution element occurrence.
///
/// # Errors
///
/// Refuses a malformed GlobalId.
pub fn create_distribution_element(
    tx: &mut Transaction,
    kind: DistributionElementKind,
    global_id: &str,
    attributes: ElementAttributes<'_>,
) -> SystemAuthoringResult<EntityId> {
    let type_name = kind.type_name();
    super::guid(type_name, global_id)?;

    let mut attrs = vec![Value::Null; distribution_slot::WIDTH];
    attrs[distribution_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attrs[distribution_slot::NAME] = text(attributes.name);
    attrs[distribution_slot::DESCRIPTION] = text(attributes.description);
    attrs[distribution_slot::OBJECT_PLACEMENT] = reference(attributes.placement);
    attrs[distribution_slot::REPRESENTATION] = reference(attributes.representation);
    attrs[distribution_slot::TAG] = text(attributes.tag);
    Ok(tx.create(Entity::new(type_name, attrs)))
}

/// Stage an `IfcZone`: spaces grouped for a shared purpose.
///
/// A zone holds no members directly. Membership is an
/// `IfcRelAssignsToGroup` pointing at this entity, which is why this
/// takes no member list -- staging one here would imply the zone
/// owns its contents, and it does not.
///
/// # Errors
///
/// Refuses a malformed GlobalId.
pub fn create_zone(
    tx: &mut Transaction,
    global_id: &str,
    name: Option<&str>,
    description: Option<&str>,
    long_name: Option<&str>,
) -> SystemAuthoringResult<EntityId> {
    super::guid("IFCZONE", global_id)?;

    let mut attrs = vec![Value::Null; zone_slot::WIDTH];
    attrs[zone_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attrs[zone_slot::NAME] = text(name);
    attrs[zone_slot::DESCRIPTION] = text(description);
    attrs[zone_slot::LONG_NAME] = text(long_name);
    Ok(tx.create(Entity::new("IFCZONE", attrs)))
}

/// Stage an `IfcSpatialZone`: a zone that occupies space.
///
/// # The USERDEFINED rule
///
/// `IfcSpatialZoneTypeEnum` includes `USERDEFINED`, which means
/// "the predefined list does not cover this". The schema pairs that
/// with `ObjectType` on `IfcObject`: choosing USERDEFINED without
/// naming the user-defined kind leaves the zone's purpose
/// unrecoverable, so it is refused here rather than written out as
/// a file nobody can interpret.
///
/// # Errors
///
/// Refuses a malformed GlobalId, and USERDEFINED without an
/// `object_type` label.
pub fn create_spatial_zone(
    tx: &mut Transaction,
    global_id: &str,
    attributes: ElementAttributes<'_>,
    long_name: Option<&str>,
    predefined_type: Option<&str>,
    object_type: Option<&str>,
) -> SystemAuthoringResult<EntityId> {
    super::guid("IFCSPATIALZONE", global_id)?;

    if predefined_type == Some("USERDEFINED") && object_type.is_none() {
        return Err(invalid(
            "IFCSPATIALZONE",
            "PredefinedType",
            "USERDEFINED requires an ObjectType naming the kind",
        ));
    }

    let mut attrs = vec![Value::Null; spatial_zone_slot::WIDTH];
    attrs[spatial_zone_slot::GLOBAL_ID] = Value::Text(global_id.into());
    attrs[spatial_zone_slot::NAME] = text(attributes.name);
    attrs[spatial_zone_slot::DESCRIPTION] = text(attributes.description);
    attrs[4] = text(object_type);
    attrs[spatial_zone_slot::OBJECT_PLACEMENT] = reference(attributes.placement);
    attrs[spatial_zone_slot::REPRESENTATION] = reference(attributes.representation);
    attrs[spatial_zone_slot::LONG_NAME] = text(long_name);
    attrs[spatial_zone_slot::PREDEFINED_TYPE] =
        predefined_type.map_or(Value::Null, |t| Value::Enum(t.into()));
    Ok(tx.create(Entity::new("IFCSPATIALZONE", attrs)))
}
