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
//!
//! # Release-bound, and written bare (#190)
//!
//! Unlike the rest of this crate's IFC4 authoring, [`create_quantity`] binds
//! the model's declared release (`release.rs`) and places attributes by name
//! from its table: IFC2X3 declares four attributes and no `Formula`, IFC4
//! and IFC4X3 five, and only IFC4X3 declares `IfcQuantityNumber`. The value
//! attribute is declared with a defined measure type, not a SELECT, so the
//! value is written bare (`12.5`, never `IFCAREAMEASURE(12.5)`), as ISO
//! 10303-21 requires. `ifc-properties` writes the same bytes; the facade
//! test `quantity_writer_agreement.rs` holds the two together.

use ifc_model::{EntityId, Model, Transaction, Value};

use super::release::{bind, Release};
use super::validate::invalid;
use super::{CostAuthoringError, CostAuthoringResult};

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
    /// `IfcQuantityNumber.NumberValue`, an `IfcNumericMeasure`.
    ///
    /// IFC4X3 only: a dimensionless count that is not a cardinality,
    /// such as a rating or an index.
    Number,
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
            Self::Number => "IFCQUANTITYNUMBER",
        }
    }

    /// The measure type the value slot is declared with.
    ///
    /// A defined type, not a SELECT, so [`create_quantity`] writes the value
    /// bare; the declaration already states the measure.
    pub fn measure_name(self) -> &'static str {
        match self {
            Self::Length => "IFCLENGTHMEASURE",
            Self::Area => "IFCAREAMEASURE",
            Self::Volume => "IFCVOLUMEMEASURE",
            Self::Count => "IFCCOUNTMEASURE",
            Self::Weight => "IFCMASSMEASURE",
            Self::Time => "IFCTIMEMEASURE",
            Self::Number => "IFCNUMERICMEASURE",
        }
    }

    /// The value attribute's name, the same in every release that declares
    /// the entity.
    fn value_attribute(self) -> &'static str {
        match self {
            Self::Length => "LengthValue",
            Self::Area => "AreaValue",
            Self::Volume => "VolumeValue",
            Self::Count => "CountValue",
            Self::Weight => "WeightValue",
            Self::Time => "TimeValue",
            Self::Number => "NumberValue",
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

/// Stage one simple physical quantity, in the model's declared release.
///
/// A count is a cardinality, so a fractional or negative count is refused;
/// the remaining dimensions refuse negatives because no physical extent is
/// negative, and a sign error here silently inverts a cost sum.
///
/// # Errors
///
/// Besides those refusals and a `Unit` that is not an `IfcNamedUnit`:
/// [`CostAuthoringError::MultipleSchemas`] or
/// [`CostAuthoringError::UnsupportedSchema`] when the model binds no single
/// known release (a model without `FILE_SCHEMA` binds IFC4);
/// [`CostAuthoringError::EntityNotInSchema`] for a kind the release does not
/// declare (`Number` outside IFC4X3); and
/// [`CostAuthoringError::AuthoringNotInSchema`] for a `formula` in IFC2X3.
/// Nothing is staged on an error.
///
/// [`CostAuthoringError::MultipleSchemas`]: super::CostAuthoringError::MultipleSchemas
/// [`CostAuthoringError::UnsupportedSchema`]: super::CostAuthoringError::UnsupportedSchema
/// [`CostAuthoringError::EntityNotInSchema`]: super::CostAuthoringError::EntityNotInSchema
/// [`CostAuthoringError::AuthoringNotInSchema`]: super::CostAuthoringError::AuthoringNotInSchema
pub fn create_quantity(
    tx: &mut Transaction,
    model: &Model,
    draft: QuantityDraft<'_>,
) -> CostAuthoringResult<EntityId> {
    let release = bind(model)?;
    let entity = draft.kind.entity_name();
    if draft.name.trim().is_empty() {
        return Err(invalid(entity, "Name", "expected a non-empty name"));
    }
    if !draft.value.is_finite() {
        return Err(invalid(entity, "Value", "expected a finite measure"));
    }
    // Every dimension but Number measures a physical extent, and no
    // extent is negative. A number is a plain count or index, so it may
    // be negative -- a temperature delta or a rating, for instance.
    if draft.kind != QuantityKind::Number && draft.value < 0.0 {
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
    // Bare: the declared type is a defined measure, not a SELECT. A whole
    // count is an INTEGER, valid for IfcCountMeasure as NUMBER (IFC2X3,
    // IFC4) and as INTEGER (IFC4X3); every other measure here is REAL. The
    // whole-count check above has already run, so the cast cannot lose a
    // fraction.
    let value = if draft.kind == QuantityKind::Count {
        #[allow(clippy::cast_possible_truncation)]
        Value::Integer(draft.value as i64)
    } else {
        Value::Real(draft.value)
    };
    let text = |text: Option<&str>| text.map_or(Value::Null, |text| Value::Text(text.into()));
    let record = release.record(
        entity,
        vec![
            ("Name", Value::Text(draft.name.into())),
            ("Description", text(draft.description)),
            ("Unit", draft.unit.map_or(Value::Null, Value::Ref)),
            (draft.kind.value_attribute(), value),
            ("Formula", text(draft.formula)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Attach quantities to an existing `IfcCostItem`, in the model's declared
/// release.
///
/// Replaces `CostQuantities` wholesale rather than appending: a partial
/// update would leave the caller unable to express removal, and silently
/// accumulating duplicates would double a measured sum.
///
/// Every quantity must be an instantiable subtype of the release's own
/// `CostQuantities` declaration (`IfcPhysicalQuantity`), read from its
/// table rather than a fixed list (#203): IFC4X3 adds `IfcQuantityNumber`,
/// which IFC4 does not declare. The slot is found by name, not assumed.
///
/// # Errors
///
/// A `cost_item` that is not an `IfcCostItem`; an empty or duplicated
/// quantity list; a quantity that is missing (`MissingReference`) or not a
/// physical quantity of the release (`WrongReferenceType`, such as an
/// `IfcQuantityNumber` in IFC4); a header binding no single known release
/// (`MultipleSchemas`, `UnsupportedSchema`); and an IFC2X3 model, whose
/// `IfcCostItem` declares no `CostQuantities` (`AuthoringNotInSchema`).
/// Nothing is staged on an error.
///
/// [`CostAuthoringError::AuthoringNotInSchema`]: super::CostAuthoringError::AuthoringNotInSchema
pub fn assign_cost_quantities(
    tx: &mut Transaction,
    model: &Model,
    cost_item: EntityId,
    quantities: &[EntityId],
) -> CostAuthoringResult<()> {
    const ENTITY: &str = "IFCCOSTITEM";
    let release = bind(model)?;
    super::validate::reference_type(tx, model, ENTITY, "CostQuantities", cost_item, ENTITY)?;
    let (slot, declared) = release.declared(ENTITY, "CostQuantities")?;
    // Reuses the crate's own emptiness/duplicate rule rather than restating
    // it: a duplicated quantity would double the measured sum.
    super::validate::non_empty_unique(ENTITY, "CostQuantities", quantities)?;
    for &quantity in quantities {
        require_quantity(tx, model, release, &declared.type_name, quantity)?;
    }
    tx.set_attribute(
        cost_item,
        slot,
        Value::List(quantities.iter().copied().map(Value::Ref).collect()),
    );
    Ok(())
}

/// Accept any instantiable subtype of `declared` in the bound release.
///
/// `validate::reference_type` compares one exact name, but `CostQuantities`
/// is typed to the abstract `IfcPhysicalQuantity`: an area, a volume, a
/// count and (in IFC4X3) a number are all valid there. The accepted set is
/// the release's own subtype tree, so an unrelated entity, or a subtype
/// another release added, is still refused.
fn require_quantity(
    tx: &Transaction,
    model: &Model,
    release: Release,
    declared: &str,
    target: EntityId,
) -> CostAuthoringResult<()> {
    let Some(actual) = super::validate::projected_type(tx, model, target) else {
        return Err(CostAuthoringError::MissingReference {
            entity: "IFCCOSTITEM",
            attribute: "CostQuantities",
            target,
        });
    };
    let schema = release.schema();
    let instantiable = schema.entity(&actual).is_some_and(|e| !e.abstract_);
    if instantiable && schema.accepts_type(declared, &actual) {
        return Ok(());
    }
    Err(CostAuthoringError::WrongReferenceType {
        entity: "IFCCOSTITEM",
        attribute: "CostQuantities",
        target,
        actual,
        expected: "IFCPHYSICALQUANTITY",
    })
}
