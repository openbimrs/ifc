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
use crate::unit::{unit_type, UnitKind};

const NAME: usize = 0;
const DESCRIPTION: usize = 1;
const SIMPLE_UNIT: usize = 2;
const SIMPLE_VALUE: usize = 3;
const SIMPLE_FORMULA: usize = 4;
const COMPLEX_HAS_QUANTITIES: usize = 2;
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

/// Maximum `IfcPhysicalComplexQuantity` nesting followed.
const MAX_COMPLEX_DEPTH: usize = 16;

/// Read an `IfcElementQuantity` by id, with schema-rule anomalies.
pub fn quantity_set(model: &Model, id: EntityId) -> Option<(QuantitySet, Vec<PropertyAnomaly>)> {
    let entity = model.get(id)?;
    if !entity.type_name.eq_ignore_ascii_case("IFCELEMENTQUANTITY") {
        return None;
    }
    let mut anomalies = Vec::new();
    let quantities = entity
        .attributes
        .get(SET_QUANTITIES)
        .and_then(refs)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|q| read_quantity(model, q, 0, &mut anomalies))
        .collect();
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

fn read_quantity(
    model: &Model,
    id: EntityId,
    depth: usize,
    anomalies: &mut Vec<PropertyAnomaly>,
) -> Option<Quantity> {
    let entity = model.get(id)?;
    let ty = entity.type_name.to_ascii_uppercase();
    let name = entity.attributes.get(NAME).and_then(text);

    if ty == "IFCPHYSICALCOMPLEXQUANTITY" {
        let quantities = if depth >= MAX_COMPLEX_DEPTH {
            Vec::new()
        } else {
            entity
                .attributes
                .get(COMPLEX_HAS_QUANTITIES)
                .and_then(refs)
                .unwrap_or_default()
                .into_iter()
                .filter(|child| *child != id)
                .filter_map(|child| read_quantity(model, child, depth + 1, anomalies))
                .collect()
        };
        return Some(Quantity::Complex {
            id,
            name,
            discrimination: entity.attributes.get(COMPLEX_DISCRIMINATION).and_then(text),
            quantities,
        });
    }

    let Some(kind) = QuantityKind::from_type(&ty) else {
        return Some(Quantity::Unsupported {
            id,
            type_name: ty.as_str().into(),
        });
    };

    let value = entity
        .attributes
        .get(SIMPLE_VALUE)
        .and_then(|v| v.unwrap_typed().as_f64())?;
    let unit = entity.attributes.get(SIMPLE_UNIT).and_then(one_ref);

    // WR22: every simple quantity requires a non-negative value.
    if value < 0.0 {
        anomalies.push(PropertyAnomaly::NegativeQuantity {
            quantity: id,
            value,
        });
    }
    // WR21: a stated unit must match the quantity kind.
    if let (Some(unit_id), Some(expected)) = (unit, kind.required_unit()) {
        if let Some(found) = unit_type(model, unit_id) {
            if &*found != expected {
                anomalies.push(PropertyAnomaly::QuantityUnitMismatch {
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

fn refs(value: &Value) -> Option<Vec<EntityId>> {
    match value {
        Value::List(items) => Some(items.iter().filter_map(one_ref).collect()),
        _ => None,
    }
}
