//! What a length-unit conversion did, kept and refused.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use ifc_model::{Conflict, EntityId, Transaction};
use ifc_properties::{
    ConversionUnit, ExactPropertyError, PropertyError, UnitConversion, UnitConversionError,
};

use super::target::TargetUnits;

/// What [`super::convert_length_unit`] does beyond the length unit itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LengthUnitOptions {
    /// Replace the project's area and volume units by the square and cube
    /// of the new length unit, rescaling their values (default `true`).
    /// With `false` they and their values stay as they are.
    pub area_and_volume: bool,
    /// Remove the replaced unit entities when nothing else refers to them
    /// (default `true`).
    pub remove_replaced_units: bool,
}

impl Default for LengthUnitOptions {
    fn default() -> Self {
        Self {
            area_and_volume: true,
            remove_replaced_units: true,
        }
    }
}

impl LengthUnitOptions {
    /// The defaults: areas and volumes follow, replaced units are removed.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets [`Self::area_and_volume`].
    #[must_use]
    pub const fn area_and_volume(mut self, value: bool) -> Self {
        self.area_and_volume = value;
        self
    }

    /// Sets [`Self::remove_replaced_units`].
    #[must_use]
    pub const fn remove_replaced_units(mut self, value: bool) -> Self {
        self.remove_replaced_units = value;
        self
    }
}

/// How a converted value was rescaled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum ConvertedKind {
    /// By the length factor.
    Length,
    /// By the area factor.
    Area,
    /// By the volume factor.
    Volume,
    /// By the inverse of the length factor (a light's distance
    /// attenuation).
    InverseLength,
    /// By the inverse of the squared length factor (a light's quadric
    /// attenuation).
    InverseArea,
    /// A map conversion's `Scale`, divided by the length factor because its
    /// map side keeps its own unit.
    MapScale,
}

/// Values of one attribute of one entity type that were rescaled.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConvertedAttribute {
    /// The entity type, upper case as the file writes it.
    pub entity_type: Arc<str>,
    /// The attribute, as the release names it.
    pub attribute: Arc<str>,
    /// How its values were rescaled.
    pub kind: ConvertedKind,
    /// How many entities had a value rescaled here.
    pub entities: usize,
    /// How many numbers were rescaled (a point has three).
    pub values: usize,
}

/// Why a length-like value was left as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum KeptReason {
    /// The value states its own unit, which stays in the model.
    ExplicitUnit,
    /// A map coordinate in the target CRS's `MapUnit`.
    MapUnit,
    /// An `IfcParameterValue` in a unitless parameter space (an angle, a
    /// knot, a vector multiple, an ordinal, a direction, a texture
    /// coordinate).
    UnitlessParameter,
    /// An area or volume while the project assigns no unit of that kind, or
    /// [`LengthUnitOptions::area_and_volume`] is off.
    UnitNotFollowed,
}

/// Length-like values of one attribute of one entity type left as they are.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct KeptAttribute {
    /// The entity type, upper case.
    pub entity_type: Arc<str>,
    /// The attribute.
    pub attribute: Arc<str>,
    /// Why.
    pub reason: KeptReason,
    /// How many numbers.
    pub values: usize,
}

/// Why the conversion cannot carry a value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RefusalReason {
    /// An `IfcParameterValue` on a curve or surface whose parameter space
    /// IFC leaves undefined or this conversion cannot scale.
    ParameterSpace {
        /// The curve or surface whose parameterisation decides.
        host: EntityId,
        /// Its type.
        host_type: Arc<str>,
        /// Why it has no parameterisation to carry.
        detail: Arc<str>,
    },
    /// An `IfcPcurve` whose reference curve lies in the parameter space of
    /// a surface that is not a plane: its coordinates are not all lengths.
    ParameterSpaceCurve {
        /// The surface.
        surface: EntityId,
        /// Its type.
        surface_type: Arc<str>,
    },
    /// `IfcPolynomialCurve` coefficients are `IfcReal`s whose unit depends
    /// on a parameterisation IFC4.3 ADD2 does not state.
    PolynomialCoefficients,
    /// `IfcTextureCoordinateGenerator.Parameter`, whose meaning depends on
    /// the X3D mode.
    TextureGeneratorParameters,
    /// A number in a `SELECT` attribute without the typed wrapper that
    /// would say which measure it is.
    UntypedSelectValue {
        /// The select type.
        select: Arc<str>,
    },
    /// A map conversion or rigid operation whose target is not a CRS this
    /// conversion knows the unit of.
    MapTarget {
        /// The target's type.
        target_type: Arc<str>,
    },
    /// A value two owners give different units (a time series value or
    /// table row shared between owners, a georeferencing set shared by
    /// hosts with and without a `MapUnit`).
    AmbiguousUnit,
    /// An entity or record the declared release does not define, or whose
    /// slot count is not its arity.
    NotInSchema,
    /// A reference that does not resolve, or a chain deeper than the budget.
    Unresolved {
        /// What.
        detail: Arc<str>,
    },
    /// An `IfcProjectLibrary` (or other context than the project) with its
    /// own `UnitsInContext`: its values are in units this conversion does
    /// not change.
    ContextUnits,
}

/// One value the conversion cannot carry.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LengthUnitRefusal {
    /// The entity holding it.
    pub entity: EntityId,
    /// Its type.
    pub entity_type: Arc<str>,
    /// The attribute, when one is concerned.
    pub attribute: Option<Arc<str>>,
    /// Why.
    pub reason: RefusalReason,
}

/// What a conversion did or, staged, will do.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LengthUnitReport {
    /// The project length unit before.
    pub from: ConversionUnit,
    /// The project length unit after.
    pub to: ConversionUnit,
    /// The conversion of lengths.
    pub length: UnitConversion,
    /// The conversion of areas, when the project has an area unit and it
    /// follows the length unit.
    pub area: Option<UnitConversion>,
    /// The conversion of volumes, likewise.
    pub volume: Option<UnitConversion>,
    /// The new length unit entity (an existing one when reused); `None` when
    /// nothing changes.
    pub length_unit: Option<EntityId>,
    /// The new area unit entity.
    pub area_unit: Option<EntityId>,
    /// The new volume unit entity.
    pub volume_unit: Option<EntityId>,
    /// Rescaled values per entity type and attribute.
    pub converted: Vec<ConvertedAttribute>,
    /// Length-like values left as they are, with the reason.
    pub kept: Vec<KeptAttribute>,
    /// Entities with at least one rescaled value.
    pub entities_changed: usize,
    /// Numbers rescaled.
    pub values_converted: usize,
    /// Entities created, in commit order (empty until committed).
    pub created: Vec<EntityId>,
    /// Entities removed (staged: the replaced units planned for removal).
    pub removed: Vec<EntityId>,
    /// The model revision after the commit.
    pub revision: Option<u64>,
    /// Whether the model was changed.
    pub applied: bool,
}

/// A planned conversion: its transaction, not yet committed, and its report.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct StagedLengthUnitConversion {
    /// Every edit, opened against the planned model.
    pub transaction: Transaction,
    /// What the commit will do.
    pub report: LengthUnitReport,
}

/// Why a conversion was refused. Nothing was written.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LengthUnitError {
    /// The target is not a length unit this operation can assign.
    Target {
        /// The target as named.
        target: Arc<str>,
        /// Why.
        reason: &'static str,
    },
    /// The model's release is not IFC2X3, IFC4 or IFC4X3, or the model is
    /// incomplete.
    Schema(ExactPropertyError),
    /// The project's unit of this type cannot be resolved exactly (no
    /// project, no length unit, two of them, an offset or context-dependent
    /// unit ...).
    ProjectUnit {
        /// `LENGTHUNIT`, `AREAUNIT` or `VOLUMEUNIT`.
        unit_type: Arc<str>,
        /// Why.
        error: UnitConversionError,
    },
    /// Values the conversion cannot carry, every one of them.
    Refused(Vec<LengthUnitRefusal>),
    /// A new unit could not be written.
    Authoring(PropertyError),
    /// The staged transaction failed its preflight.
    Conflict(Vec<Conflict>),
}

impl fmt::Display for LengthUnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Target { target, reason } => write!(f, "{target}: {reason}"),
            Self::Schema(error) => write!(f, "{error}"),
            Self::ProjectUnit { unit_type, error } => {
                write!(f, "the project {unit_type} cannot be resolved: {error}")
            }
            Self::Refused(refusals) => write!(
                f,
                "{} value(s) cannot be converted, the first: {:?}",
                refusals.len(),
                refusals.first()
            ),
            Self::Authoring(error) => write!(f, "{error}"),
            Self::Conflict(conflicts) => write!(f, "{} conflict(s)", conflicts.len()),
        }
    }
}

impl std::error::Error for LengthUnitError {}

/// Entity type and attribute.
type Key = (Arc<str>, Arc<str>);

/// Counts gathered while planning.
pub(super) struct Tally {
    report: LengthUnitReport,
    /// Entities and values per attribute and kind.
    converted: BTreeMap<(Key, ConvertedKind), (usize, usize)>,
    kept: BTreeMap<(Arc<str>, Arc<str>, KeptReason), usize>,
}

impl Tally {
    pub(super) fn new(from: &ConversionUnit, to: &ConversionUnit, units: &TargetUnits) -> Self {
        Self {
            report: LengthUnitReport {
                from: from.clone(),
                to: to.clone(),
                length: units.length,
                area: units.area.as_ref().map(|(_, conversion)| *conversion),
                volume: units.volume.as_ref().map(|(_, conversion)| *conversion),
                length_unit: None,
                area_unit: None,
                volume_unit: None,
                converted: Vec::new(),
                kept: Vec::new(),
                entities_changed: 0,
                values_converted: 0,
                created: Vec::new(),
                removed: Vec::new(),
                revision: None,
                applied: false,
            },
            converted: BTreeMap::new(),
            kept: BTreeMap::new(),
        }
    }

    /// One entity's `values` rescaled in one attribute; call once per
    /// entity, attribute and kind.
    pub(super) fn converted(
        &mut self,
        entity_type: &Arc<str>,
        attribute: &Arc<str>,
        kind: ConvertedKind,
        values: usize,
    ) {
        let entry = self
            .converted
            .entry(((entity_type.clone(), attribute.clone()), kind))
            .or_default();
        entry.0 += 1;
        entry.1 += values;
        self.report.values_converted += values;
    }

    pub(super) fn kept(
        &mut self,
        entity_type: &Arc<str>,
        attribute: &Arc<str>,
        reason: KeptReason,
        values: usize,
    ) {
        *self
            .kept
            .entry((entity_type.clone(), attribute.clone(), reason))
            .or_default() += values;
    }

    pub(super) fn entity_changed(&mut self) {
        self.report.entities_changed += 1;
    }

    pub(super) fn units(
        &mut self,
        length: EntityId,
        area: Option<EntityId>,
        volume: Option<EntityId>,
    ) {
        self.report.length_unit = Some(length);
        self.report.area_unit = area;
        self.report.volume_unit = volume;
    }

    pub(super) fn removing(&mut self, removed: Vec<EntityId>) {
        self.report.removed = removed;
    }

    pub(super) fn staged(mut self, transaction: Transaction) -> StagedLengthUnitConversion {
        self.report.converted = self
            .converted
            .into_iter()
            .map(
                |(((entity_type, attribute), kind), (entities, values))| ConvertedAttribute {
                    entity_type,
                    attribute,
                    kind,
                    entities,
                    values,
                },
            )
            .collect();
        self.report.kept = self
            .kept
            .into_iter()
            .map(|((entity_type, attribute, reason), values)| KeptAttribute {
                entity_type,
                attribute,
                reason,
                values,
            })
            .collect();
        StagedLengthUnitConversion {
            transaction,
            report: self.report,
        }
    }
}
