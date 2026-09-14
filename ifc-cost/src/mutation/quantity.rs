//! Authoring for the physical quantities a cost item is measured against.
//!
//! `IfcCostItem.CostQuantities` is what turns a rate into a sum: a value of
//! 50/m2 means nothing until something states how many square metres. The
//! quantity subtypes share a slot layout inherited through two levels --
//! `IfcPhysicalQuantity` contributes Name and Description, then
//! `IfcPhysicalSimpleQuantity` adds Unit -- so the measured number always
//! lands in slot 3, whatever the subtype calls it. The reader in
//! `crate::quantity` relies on exactly that, and these constructors must
//! agree with it or an authored quantity reads back as None.

use ifc_model::{Entity, EntityId, Model, Transaction, Value};

use super::validate::invalid;
use super::CostAuthoringResult;

/// The physical dimension a simple quantity measures.
///
/// Each variant names its IFC entity and the typed measure its value slot
/// carries. Keeping the pair together is what stops an area being written
/// with a length measure, which parses and validates but is wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityKind {
    /// `IfcQuantityLength.LengthValue`, an `IfcLengthMeasure`.
    Length,
    /// `IfcQuantityArea.AreaValue`, an `IfcAreaMeasure`.
    Area,
    /// `IfcQuantityVolume.VolumeValue`, an `IfcVolumeMeasure`.
    Volume,
    /// `IfcQuantityCount.CountValue`, an `IfcCountMeasure`.
    Count,
    /// `IfcQuantityWeight.WeightValue`, an `IfcMassMeasure`.
    Weight,
    /// `IfcQuantityTime.TimeValue`, an `IfcTimeMeasure`.
    Time,
}

impl QuantityKind {
    /// The IFC entity name for this dimension.
    pub fn entity_name(self) -> &'static str {
        match self {
            Self::Length => "IFCQUANTITYLENGTH",
            Self::Area => "IFCQUANTITYAREA",
            Self::Volume => "IFCQUANTITYVOLUME",
            Self::Count => "IFCQUANTITYCOUNT",
            Self::Weight => "IFCQUANTITYWEIGHT",
            Self::Time => "IFCQUANTITYTIME",
        }
    }

    /// The typed measure wrapper the value slot must carry.
    pub fn measure_name(self) -> &'static str {
        match self {
            Self::Length => "IFCLENGTHMEASURE",
            Self::Area => "IFCAREAMEASURE",
            Self::Volume => "IFCVOLUMEMEASURE",
            Self::Count => "IFCCOUNTMEASURE",
            Self::Weight => "IFCMASSMEASURE",
            Self::Time => "IFCTIMEMEASURE",
        }
    }
}

/// Authored fields for a simple physical quantity.
#[derive(Debug, Clone, Copy)]
pub struct QuantityDraft<'a> {
    /// The dimension being measured.
    pub kind: QuantityKind,
    /// `IfcPhysicalQuantity.Name`. Required: an unnamed quantity cannot be
    /// matched to the rate it is meant to multiply.
    pub name: &'a str,
    /// `IfcPhysicalQuantity.Description`, if given.
    pub description: Option<&'a str>,
    /// `IfcPhysicalSimpleQuantity.Unit`, an `IfcNamedUnit` reference, if
    /// given. Absent means the project unit for the dimension applies.
    pub unit: Option<EntityId>,
    /// The measured amount.
    pub value: f64,
    /// `Formula`, the derivation note, if given.
    pub formula: Option<&'a str>,
}

/// Stage one simple physical quantity.
///
/// A count is a cardinality, so a fractional or negative count is refused;
/// the remaining dimensions refuse negatives because no physical extent is
/// negative, and a sign error here silently inverts a cost sum.
pub fn create_quantity(
    tx: &mut Transaction,
    model: &Model,
    draft: QuantityDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    let entity = draft.kind.entity_name();
    if draft.name.trim().is_empty() {
        return Err(invalid(entity, "Name", "expected a non-empty name"));
    }
    if !draft.value.is_finite() || draft.value < 0.0 {
        return Err(invalid(
            entity,
            "Value",
            "expected a finite non-negative measure",
        ));
    }
    if draft.kind == QuantityKind::Count && draft.value.fract() != 0.0 {
        return Err(invalid(entity, "CountValue", "expected a whole count"));
    }
    if let Some(unit) = draft.unit {
        super::validate::reference_type(tx, model, entity, "Unit", unit, "IFCNAMEDUNIT")?;
    }
    Ok(tx.create(Entity::new(
        entity,
        vec![
            Value::Text(draft.name.into()),
            draft
                .description
                .map_or(Value::Null, |d| Value::Text(d.into())),
            draft.unit.map_or(Value::Null, Value::Ref),
            Value::Typed {
                type_name: draft.kind.measure_name().into(),
                value: Box::new(Value::Real(draft.value)),
            },
            draft.formula.map_or(Value::Null, |f| Value::Text(f.into())),
        ],
    )))
}

/// Attach quantities to an existing `IfcCostItem`.
///
/// Replaces `CostQuantities` (slot 8) wholesale rather than appending: a
/// partial update would leave the caller unable to express removal, and
/// silently accumulating duplicates would double a measured sum.
pub fn assign_cost_quantities(
    tx: &mut Transaction,
    model: &Model,
    cost_item: EntityId,
    quantities: &[EntityId],
) -> CostAuthoringResult<()> {
    super::validate::reference_type(
        tx,
        model,
        "IFCCOSTITEM",
        "CostQuantities",
        cost_item,
        "IFCCOSTITEM",
    )?;
    // Reuses the crate's own emptiness/duplicate rule rather than restating
    // it: a duplicated quantity would double the measured sum.
    super::validate::non_empty_unique("IFCCOSTITEM", "CostQuantities", quantities)?;
    for &quantity in quantities {
        require_quantity(tx, model, quantity)?;
    }
    tx.set_attribute(
        cost_item,
        crate::item::slot::COST_QUANTITIES,
        Value::List(quantities.iter().copied().map(Value::Ref).collect()),
    );
    Ok(())
}

/// Accept any concrete `IfcPhysicalQuantity` subtype.
///
/// `validate::reference_type` compares one exact name, but CostQuantities
/// is typed to the abstract supertype: an area, a volume and a count are
/// all valid there. Enumerating the concrete subtypes keeps the check
/// closed -- an unrelated entity is still refused -- without pulling a
/// schema dependency into this crate for one subtype question.
fn require_quantity(tx: &Transaction, model: &Model, target: EntityId) -> CostAuthoringResult<()> {
    const ACCEPTED: &[&str] = &[
        "IFCQUANTITYLENGTH",
        "IFCQUANTITYAREA",
        "IFCQUANTITYVOLUME",
        "IFCQUANTITYCOUNT",
        "IFCQUANTITYWEIGHT",
        "IFCQUANTITYTIME",
        "IFCPHYSICALCOMPLEXQUANTITY",
    ];
    let Some(actual) = super::validate::projected_type(tx, model, target) else {
        return Err(crate::mutation::CostAuthoringError::MissingReference {
            entity: "IFCCOSTITEM",
            attribute: "CostQuantities",
            target,
        });
    };
    if ACCEPTED.iter().any(|k| actual.eq_ignore_ascii_case(k)) {
        return Ok(());
    }
    Err(crate::mutation::CostAuthoringError::WrongReferenceType {
        entity: "IFCCOSTITEM",
        attribute: "CostQuantities",
        target,
        actual,
        expected: "IFCPHYSICALQUANTITY",
    })
}
