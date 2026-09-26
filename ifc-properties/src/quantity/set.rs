//! `IfcElementQuantity` and the physical quantities it carries.
//!
//! # Slots, verified against the IFC4 EXPRESS schema
//!
//! ```text
//! IfcElementQuantity     4 = MethodOfMeasurement  5 = Quantities
//! IfcPhysicalQuantity    0 = Name                 1 = Description
//! IfcPhysicalSimpleQty   2 = Unit
//! IfcQuantityLength      3 = LengthValue          4 = Formula
//! IfcQuantityArea        3 = AreaValue            4 = Formula
//! IfcQuantityVolume      3 = VolumeValue          4 = Formula
//! IfcQuantityCount       3 = CountValue           4 = Formula
//! IfcQuantityWeight      3 = WeightValue          4 = Formula
//! IfcQuantityTime        3 = TimeValue            4 = Formula
//! IfcPhysicalComplexQty  2 = HasQuantities        3 = Discrimination
//! ```
//!
//! The value slot is 3 for every simple quantity because `Unit` occupies slot
//! 2 on the shared supertype. The COMPLEX quantity has no `Unit`, so its
//! contents start at slot 2 instead -- reading it like a simple quantity
//! finds a list where a unit belongs.
//!
//! # A quantity is an assertion, not a measurement
//!
//! This crate never computes shape. `IfcQuantityArea` is what the authoring
//! tool claimed, and it may disagree with the geometry. Reporting the claim
//! faithfully -- including a negative one that breaks `WR22` -- is the job.

use std::sync::Arc;

use ifc_model::{EntityId, Model, Value};

use crate::error::PropertyAnomaly;
use crate::nesting::Nesting;
use crate::quantity::complex::complex_quantities;
use crate::unit::{unit_type, UnitKind};

const NAME: usize = 0;
const DESCRIPTION: usize = 1;
const SIMPLE_UNIT: usize = 2;
const SIMPLE_VALUE: usize = 3;
const SIMPLE_FORMULA: usize = 4;
const COMPLEX_DISCRIMINATION: usize = 3;
const SET_METHOD: usize = 4;
const SET_QUANTITIES: usize = 5;

/// What a physical quantity measures.
///
/// The kind is the entity type, not a guess from the unit: a file may omit
/// the unit entirely, and `IfcQuantityArea` still measures area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityKind {
    /// `IfcQuantityLength`.
    Length,
    /// `IfcQuantityArea`.
    Area,
    /// `IfcQuantityVolume`.
    Volume,
    /// `IfcQuantityCount`.
    Count,
    /// `IfcQuantityWeight`, measured as mass.
    Weight,
    /// `IfcQuantityTime`.
    Time,
}

impl QuantityKind {
    /// The kind named by an entity type, or `None` if it is not a simple
    /// quantity. Case-insensitive, because a type name reaching here may
    /// come from a caller rather than the upper-cased parser.
    #[must_use]
    pub fn from_type_name(name: &str) -> Option<Self> {
        Self::from_type(&name.to_ascii_uppercase())
    }

    /// The entity type name for this kind, in schema casing.
    #[must_use]
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Length => "IfcQuantityLength",
            Self::Area => "IfcQuantityArea",
            Self::Volume => "IfcQuantityVolume",
            Self::Count => "IfcQuantityCount",
            Self::Weight => "IfcQuantityWeight",
            Self::Time => "IfcQuantityTime",
        }
    }

    /// The measure type this kind's value slot carries.
    ///
    /// Fixed by the schema per subtype: an `IfcQuantityArea` holds an
    /// `IfcAreaMeasure` and nothing else.
    #[must_use]
    pub fn measure_type(self) -> &'static str {
        match self {
            Self::Length => "IfcLengthMeasure",
            Self::Area => "IfcAreaMeasure",
            Self::Volume => "IfcVolumeMeasure",
            Self::Count => "IfcCountMeasure",
            Self::Weight => "IfcMassMeasure",
            Self::Time => "IfcTimeMeasure",
        }
    }

    fn from_type(name: &str) -> Option<Self> {
        Some(match name {
            "IFCQUANTITYLENGTH" => Self::Length,
            "IFCQUANTITYAREA" => Self::Area,
            "IFCQUANTITYVOLUME" => Self::Volume,
            "IFCQUANTITYCOUNT" => Self::Count,
            "IFCQUANTITYWEIGHT" => Self::Weight,
            "IFCQUANTITYTIME" => Self::Time,
            _ => return None,
        })
    }

    /// The `IfcUnitEnum` this quantity's unit must carry, per `WR21`.
    ///
    /// `IfcQuantityCount` has no such rule -- a count is dimensionless -- so
    /// it constrains nothing.
    pub fn required_unit(self) -> Option<&'static str> {
        Some(match self {
            Self::Length => "LENGTHUNIT",
            Self::Area => "AREAUNIT",
            Self::Volume => "VOLUMEUNIT",
            Self::Weight => "MASSUNIT",
            Self::Time => "TIMEUNIT",
            Self::Count => return None,
        })
    }
}

/// One physical quantity.
#[derive(Debug, Clone, PartialEq)]
pub enum Quantity {
    /// A simple measured value.
    Simple {
        /// The entity.
        id: EntityId,
        /// `Name`, required by the schema.
        name: Option<Arc<str>>,
        /// `Description`.
        description: Option<Arc<str>>,
        /// What it measures.
        kind: QuantityKind,
        /// The stated value.
        value: f64,
        /// The unit override, when stated.
        unit: Option<EntityId>,
        /// `Formula`: how the author says it was derived. Free text.
        formula: Option<Arc<str>>,
    },
    /// A nested group of quantities.
    Complex {
        /// The entity.
        id: EntityId,
        /// `Name`.
        name: Option<Arc<str>>,
        /// `Discrimination`: what distinguishes the parts.
        discrimination: Option<Arc<str>>,
        /// Contained quantities.
        quantities: Vec<Quantity>,
    },
    /// A concrete quantity type this crate does not model.
    Unsupported {
        /// The entity.
        id: EntityId,
        /// Declared type, upper-cased.
        type_name: Arc<str>,
    },
}

impl Quantity {
    /// The entity id, whatever the variant.
    pub fn id(&self) -> EntityId {
        match self {
            Self::Simple { id, .. } | Self::Complex { id, .. } | Self::Unsupported { id, .. } => {
                *id
            }
        }
    }

    /// The name, whatever the variant.
    pub fn name(&self) -> Option<&str> {
        match self {
            Self::Simple { name, .. } | Self::Complex { name, .. } => name.as_deref(),
            Self::Unsupported { .. } => None,
        }
    }
}

/// An `IfcElementQuantity` with its quantities resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantitySet {
    /// The entity.
    pub id: EntityId,
    /// `Name`.
    pub name: Option<Arc<str>>,
    /// `MethodOfMeasurement`: the standard the author measured against.
    ///
    /// Free text such as `BaseQuantities`. It is the only statement about
    /// HOW a quantity was arrived at, so it is carried rather than dropped.
    pub method: Option<Arc<str>>,
    /// Quantities in file order.
    pub quantities: Vec<Quantity>,
}

impl QuantitySet {
    /// Look up a quantity by name.
    ///
    /// `UniqueQuantityNames` makes the first match the only match in a
    /// well-formed file.
    pub fn quantity(&self, name: &str) -> Option<&Quantity> {
        self.quantities.iter().find(|q| q.name() == Some(name))
    }
}

/// Read an `IfcElementQuantity` by id, with schema-rule anomalies.
///
/// Returns `None` when `id` is absent or is not an `IfcElementQuantity`.
/// A member that cannot be represented is left out of `quantities` and
/// reported, never dropped silently: an absent id
/// ([`PropertyAnomaly::MissingMember`]), a simple quantity with no value
/// ([`PropertyAnomaly::QuantityValueMissing`]) or a non-numeric one
/// ([`PropertyAnomaly::QuantityValueNotNumeric`]). Nested complex
/// quantities are followed along a tracked path; a cycle, over-deep nesting
/// or an exhausted member budget is reported as
/// [`PropertyAnomaly::ComplexCycle`],
/// [`PropertyAnomaly::ComplexTooDeep`] or
/// [`PropertyAnomaly::ComplexBudgetExceeded`].
pub fn quantity_set(model: &Model, id: EntityId) -> Option<(QuantitySet, Vec<PropertyAnomaly>)> {
    let entity = model.get(id)?;
    if !entity.type_name.eq_ignore_ascii_case("IFCELEMENTQUANTITY") {
        return None;
    }
    let mut anomalies = Vec::new();
    let mut nesting = Nesting::new(&mut anomalies);
    let mut quantities = Vec::new();
    for member in entity
        .attributes
        .get(SET_QUANTITIES)
        .and_then(refs)
        .unwrap_or_default()
    {
        if !nesting.admit(model, id, member) {
            continue;
        }
        if let Some(quantity) = read_quantity(model, member, &mut nesting) {
            quantities.push(quantity);
        }
    }
    Some((
        QuantitySet {
            id,
            name: entity.attributes.get(2).and_then(text),
            method: entity.attributes.get(SET_METHOD).and_then(text),
            quantities,
        },
        anomalies,
    ))
}

/// Read one physical quantity. `None` means it cannot be represented; the
/// reason has already been reported through `nesting`.
pub(super) fn read_quantity(
    model: &Model,
    id: EntityId,
    nesting: &mut Nesting<'_>,
) -> Option<Quantity> {
    let entity = model.get(id)?;
    let ty = entity.type_name.to_ascii_uppercase();
    let name = entity.attributes.get(NAME).and_then(text);

    if ty == "IFCPHYSICALCOMPLEXQUANTITY" {
        return Some(Quantity::Complex {
            id,
            name,
            discrimination: entity.attributes.get(COMPLEX_DISCRIMINATION).and_then(text),
            quantities: complex_quantities(model, id, entity, nesting),
        });
    }

    let Some(kind) = QuantityKind::from_type(&ty) else {
        return Some(Quantity::Unsupported {
            id,
            type_name: ty.as_str().into(),
        });
    };

    // The value attribute is not OPTIONAL on any IfcQuantity*. A quantity
    // without a number has nothing to report as `Simple`, so it is named
    // instead of silently vanishing from its set.
    let value = match entity.attributes.get(SIMPLE_VALUE) {
        None | Some(Value::Null) => {
            nesting.report(PropertyAnomaly::QuantityValueMissing { quantity: id });
            return None;
        }
        Some(stated) => match stated.unwrap_typed().as_f64() {
            Some(value) => value,
            None => {
                nesting.report(PropertyAnomaly::QuantityValueNotNumeric {
                    quantity: id,
                    found: format!("{stated:?}"),
                });
                return None;
            }
        },
    };
    let unit = entity.attributes.get(SIMPLE_UNIT).and_then(one_ref);

    // WR22: every simple quantity requires a non-negative value.
    if value < 0.0 {
        nesting.report(PropertyAnomaly::NegativeQuantity {
            quantity: id,
            value,
        });
    }
    // WR21: a stated unit must match the quantity kind.
    if let (Some(unit_id), Some(expected)) = (unit, kind.required_unit()) {
        if let Some(found) = unit_type(model, unit_id) {
            if &*found != expected {
                nesting.report(PropertyAnomaly::QuantityUnitMismatch {
                    quantity: id,
                    unit: unit_id,
                    expected,
                    found: found.to_string(),
                });
            }
        }
    }

    Some(Quantity::Simple {
        id,
        name,
        description: entity.attributes.get(DESCRIPTION).and_then(text),
        kind,
        value,
        unit,
        formula: entity.attributes.get(SIMPLE_FORMULA).and_then(text),
    })
}

/// Every `IfcElementQuantity` in the file, with anomalies.
pub fn quantity_sets(model: &Model) -> (Vec<QuantitySet>, Vec<PropertyAnomaly>) {
    let mut sets = Vec::new();
    let mut anomalies = Vec::new();
    let mut ids: Vec<_> = model.ids_of_type("IFCELEMENTQUANTITY").to_vec();
    ids.sort_unstable();
    for id in ids {
        if let Some((set, mut found)) = quantity_set(model, id) {
            sets.push(set);
            anomalies.append(&mut found);
        }
    }
    (sets, anomalies)
}

/// Resolve the unit kind for a quantity, following its explicit unit only.
///
/// Project-default units are NOT applied here: falling back to the project
/// context would report a unit the quantity never stated. Callers that want
/// the effective unit combine this with [`crate::unit::project_units`].
pub fn stated_unit(model: &Model, quantity: &Quantity) -> Option<UnitKind> {
    match quantity {
        Quantity::Simple { unit, .. } => {
            let id = (*unit)?;
            crate::unit::unit(model, id)
        }
        _ => None,
    }
}

fn text(value: &Value) -> Option<Arc<str>> {
    match value.unwrap_typed() {
        Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}

fn one_ref(value: &Value) -> Option<EntityId> {
    match value.unwrap_typed() {
        Value::Ref(id) => Some(*id),
        _ => None,
    }
}

pub(super) fn refs(value: &Value) -> Option<Vec<EntityId>> {
    match value {
        Value::List(items) => Some(items.iter().filter_map(one_ref).collect()),
        _ => None,
    }
}
