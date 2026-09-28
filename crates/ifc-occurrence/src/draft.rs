//! The authored fields of an occurrence.
//!
//! Beyond the attributes every occurrence shares, a few classes require
//! more in IFC2X3 TC1 than in IFC4 ADD2 TC1 and IFC4X3 ADD2, and the draft
//! carries them so those records can be authored (#214). From
//! `references/ifc-spec`:
//!
//! ```text
//! IFC2X3  IfcRamp / IfcRoof / IfcStair
//!           ShapeType : IfcRampTypeEnum / IfcRoofTypeEnum / IfcStairTypeEnum;
//! IFC2X3  IfcReinforcingBar
//!           NominalDiameter : IfcPositiveLengthMeasure;
//!           CrossSectionArea : IfcAreaMeasure;
//!           BarRole : IfcReinforcingBarRoleEnum;
//! IFC2X3  IfcTendon
//!           NominalDiameter : IfcPositiveLengthMeasure;
//!           CrossSectionArea : IfcAreaMeasure;
//! IFC2X3  IfcReinforcingMesh
//!           Longitudinal/TransverseBarNominalDiameter : IfcPositiveLengthMeasure;
//!           Longitudinal/TransverseBarCrossSectionArea : IfcAreaMeasure;
//!           Longitudinal/TransverseBarSpacing : IfcPositiveLengthMeasure;
//! ```
//!
//! IFC4 and IFC4X3 declare the reinforcement measures `OPTIONAL` and have
//! no `ShapeType` or `BarRole`. A value is written wherever the bound
//! release declares the attribute on the class and refused where it does
//! not.

use ifc_model::{EntityId, Value};

use crate::error::{OccurrenceError, OccurrenceResult};
use crate::release::Layout;

/// Attributes of an occurrence: those every class shares, and the few that
/// one class requires on top.
///
/// The struct is `#[non_exhaustive]`: build it with
/// [`OccurrenceDraft::new`] and the setters, so a field a later release
/// needs can be added without breaking callers.
#[derive(Debug, Clone, Copy, Default)]
#[non_exhaustive]
pub struct OccurrenceDraft<'a> {
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `ObjectType`; required when `PredefinedType` is `USERDEFINED`.
    pub object_type: Option<&'a str>,
    /// `ObjectPlacement`.
    pub placement: Option<EntityId>,
    /// `Representation`.
    pub representation: Option<EntityId>,
    /// `Tag`, slot 7 on every class in this catalogue.
    pub tag: Option<&'a str>,
    /// `ShapeType` of an IFC2X3 `IfcRamp`, `IfcRoof` or `IfcStair`, a token
    /// of that release's enumeration; required there.
    pub shape_type: Option<&'a str>,
    /// `NominalDiameter` of an `IfcReinforcingBar` or `IfcTendon`, an
    /// `IfcPositiveLengthMeasure`; required in IFC2X3.
    pub nominal_diameter: Option<f64>,
    /// `CrossSectionArea` of an `IfcReinforcingBar` or `IfcTendon`, an
    /// `IfcAreaMeasure`; required in IFC2X3.
    pub cross_section_area: Option<f64>,
    /// `BarRole` of an IFC2X3 `IfcReinforcingBar`, an
    /// `IfcReinforcingBarRoleEnum` token; required there.
    pub bar_role: Option<&'a str>,
    /// The longitudinal bars of an `IfcReinforcingMesh`; required in
    /// IFC2X3.
    pub longitudinal_bars: Option<MeshBars>,
    /// The transverse bars of an `IfcReinforcingMesh`; required in IFC2X3.
    pub transverse_bars: Option<MeshBars>,
}

/// One bar direction of an `IfcReinforcingMesh`: its
/// `...BarNominalDiameter`, `...BarCrossSectionArea` and `...BarSpacing`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct MeshBars {
    /// `...BarNominalDiameter`, an `IfcPositiveLengthMeasure`.
    pub nominal_diameter: f64,
    /// `...BarCrossSectionArea`, an `IfcAreaMeasure`.
    pub cross_section_area: f64,
    /// `...BarSpacing`, an `IfcPositiveLengthMeasure`.
    pub spacing: f64,
}

impl MeshBars {
    /// The three measures of one bar direction.
    #[must_use]
    pub const fn new(nominal_diameter: f64, cross_section_area: f64, spacing: f64) -> Self {
        Self {
            nominal_diameter,
            cross_section_area,
            spacing,
        }
    }
}

impl<'a> OccurrenceDraft<'a> {
    /// Starts an empty draft with every attribute unset.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets `Name`.
    #[must_use]
    pub fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `Description`.
    #[must_use]
    pub fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `ObjectType`.
    #[must_use]
    pub fn object_type(mut self, value: &'a str) -> Self {
        self.object_type = Some(value);
        self
    }

    /// Sets `ObjectPlacement`.
    #[must_use]
    pub fn placement(mut self, value: EntityId) -> Self {
        self.placement = Some(value);
        self
    }

    /// Sets `Representation`.
    #[must_use]
    pub fn representation(mut self, value: EntityId) -> Self {
        self.representation = Some(value);
        self
    }

    /// Sets `Tag`.
    #[must_use]
    pub fn tag(mut self, value: &'a str) -> Self {
        self.tag = Some(value);
        self
    }

    /// Sets the IFC2X3 `ShapeType`.
    #[must_use]
    pub fn shape_type(mut self, value: &'a str) -> Self {
        self.shape_type = Some(value);
        self
    }

    /// Sets `NominalDiameter`.
    #[must_use]
    pub fn nominal_diameter(mut self, value: f64) -> Self {
        self.nominal_diameter = Some(value);
        self
    }

    /// Sets `CrossSectionArea`.
    #[must_use]
    pub fn cross_section_area(mut self, value: f64) -> Self {
        self.cross_section_area = Some(value);
        self
    }

    /// Sets the IFC2X3 `BarRole`.
    #[must_use]
    pub fn bar_role(mut self, value: &'a str) -> Self {
        self.bar_role = Some(value);
        self
    }

    /// Sets the longitudinal bars of a mesh.
    #[must_use]
    pub fn longitudinal_bars(mut self, value: MeshBars) -> Self {
        self.longitudinal_bars = Some(value);
        self
    }

    /// Sets the transverse bars of a mesh.
    #[must_use]
    pub fn transverse_bars(mut self, value: MeshBars) -> Self {
        self.transverse_bars = Some(value);
        self
    }
}

/// How a measure is checked.
#[derive(Clone, Copy)]
enum Measure {
    /// `IfcPositiveLengthMeasure`: finite and greater than zero.
    PositiveLength,
    /// `IfcAreaMeasure`: finite.
    Area,
}

/// The class-specific values of `draft`, named by attribute and checked.
///
/// A token must be a member of the enumeration `layout` declares for the
/// attribute on `entity` ([`OccurrenceError::UnknownToken`]); a token for
/// an attribute `entity` does not declare there is
/// [`OccurrenceError::AuthoringNotInSchema`], as is a measure, through
/// [`Layout::named_record`]. Unset fields yield no value, so a record
/// without them is laid out exactly as before.
pub(crate) fn specific_values(
    layout: Layout,
    entity: &'static str,
    draft: &OccurrenceDraft<'_>,
) -> OccurrenceResult<Vec<(&'static str, Value)>> {
    let mut values = Vec::new();
    for (attribute, token) in [("ShapeType", draft.shape_type), ("BarRole", draft.bar_role)] {
        let Some(token) = token else {
            continue;
        };
        let Some(members) = layout.members(entity, attribute) else {
            return Err(OccurrenceError::AuthoringNotInSchema {
                entity,
                attribute,
                schema: layout.version(),
            });
        };
        if !members.contains(&token) {
            return Err(OccurrenceError::UnknownToken {
                entity,
                attribute,
                token: token.into(),
            });
        }
        values.push((attribute, Value::Enum(token.into())));
    }

    let mut measures = vec![
        (
            "NominalDiameter",
            draft.nominal_diameter,
            Measure::PositiveLength,
        ),
        ("CrossSectionArea", draft.cross_section_area, Measure::Area),
    ];
    for (bars, [diameter, area, spacing]) in [
        (
            draft.longitudinal_bars,
            [
                "LongitudinalBarNominalDiameter",
                "LongitudinalBarCrossSectionArea",
                "LongitudinalBarSpacing",
            ],
        ),
        (
            draft.transverse_bars,
            [
                "TransverseBarNominalDiameter",
                "TransverseBarCrossSectionArea",
                "TransverseBarSpacing",
            ],
        ),
    ] {
        measures.extend([
            (
                diameter,
                bars.map(|b| b.nominal_diameter),
                Measure::PositiveLength,
            ),
            (area, bars.map(|b| b.cross_section_area), Measure::Area),
            (spacing, bars.map(|b| b.spacing), Measure::PositiveLength),
        ]);
    }
    for (attribute, value, measure) in measures {
        let Some(value) = value else {
            continue;
        };
        let admitted = match measure {
            Measure::PositiveLength => value.is_finite() && value > 0.0,
            Measure::Area => value.is_finite(),
        };
        if !admitted {
            return Err(OccurrenceError::InvalidMeasure {
                entity,
                attribute,
                value: format!("{value:?}"),
            });
        }
        values.push((attribute, Value::Real(value)));
    }
    Ok(values)
}
