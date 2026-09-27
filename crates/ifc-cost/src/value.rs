//! `IfcCostValue` and the applied-value tree beneath it.
//!
//! # Slots, verified against IFC4 EXPRESS
//!
//! `IfcCostValue` adds nothing of its own; every slot comes from
//! `IfcAppliedValue`:
//!
//! ```text
//! 0 Name                1 Description        2 AppliedValue
//! 3 UnitBasis           4 ApplicableDate     5 FixedUntilDate
//! 6 Category            7 Condition          8 ArithmeticOperator
//! 9 Components
//! ```
//!
//! # A cost value is a tree, not a number
//!
//! `Components` holds nested `IfcAppliedValue`s and `ArithmeticOperator` says
//! how to combine them. A value with components and an `ADD` operator is the
//! sum of its children; the `AppliedValue` slot may then be absent entirely.
//! Reading only slot 2 and calling that "the amount" silently reports nothing
//! for every composed rate in the file.
//!
//! # What this module refuses to do
//!
//! It does not evaluate the tree. `ArithmeticOperator` is `ADD`, `DIVIDE`,
//! `MULTIPLY` or `SUBTRACT` over a LIST whose order the schema does not
//! constrain to be meaningful for non-commutative operators: `SUBTRACT` over
//! `[a, b, c]` has no defined bracketing in the standard. Folding it anyway
//! would produce a number that looks authoritative and is not. So the shape is
//! reported and evaluation is left to a caller who knows their own convention.

use ifc_model::{Entity, EntityId, Model, Value};

/// `IfcAppliedValue` slots. `IfcCostValue` adds none of its own.
mod slot {
    /// `Name`.
    pub const NAME: usize = 0;
    /// `Description`.
    pub const DESCRIPTION: usize = 1;
    /// `AppliedValue`, usually `IFCMONETARYMEASURE`.
    pub const APPLIED_VALUE: usize = 2;
    /// `UnitBasis`, an `IfcMeasureWithUnit`.
    pub const UNIT_BASIS: usize = 3;
    /// `ApplicableDate`.
    pub const APPLICABLE_DATE: usize = 4;
    /// `FixedUntilDate`.
    pub const FIXED_UNTIL_DATE: usize = 5;
    /// `Category`.
    pub const CATEGORY: usize = 6;
    /// `Condition`.
    pub const CONDITION: usize = 7;
    /// `ArithmeticOperator`.
    pub const ARITHMETIC_OPERATOR: usize = 8;
    /// `Components`.
    pub const COMPONENTS: usize = 9;
}

/// How an applied value combines its components.
///
/// `IfcArithmeticOperatorEnum`, verified against IFC4 EXPRESS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOperator {
    /// `.ADD.`
    Add,
    /// `.DIVIDE.`
    Divide,
    /// `.MULTIPLY.`
    Multiply,
    /// `.SUBTRACT.`
    Subtract,
}

impl ArithmeticOperator {
    /// Parse the enum token, without its dots.
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        Some(match token {
            "ADD" => Self::Add,
            "DIVIDE" => Self::Divide,
            "MULTIPLY" => Self::Multiply,
            "SUBTRACT" => Self::Subtract,
            _ => return None,
        })
    }

    /// Whether operand order changes the result.
    ///
    /// `ADD` and `MULTIPLY` are commutative, so folding a component list in
    /// file order is safe. `SUBTRACT` and `DIVIDE` are not, and IFC does not
    /// define the bracketing, so a caller folding them is choosing a
    /// convention rather than reading one.
    #[must_use]
    pub fn is_order_sensitive(self) -> bool {
        matches!(self, Self::Divide | Self::Subtract)
    }
}

/// How a rate is expressed per unit of something.
///
/// `UnitBasis` is an `IfcMeasureWithUnit`: "45.50 per 1 cubic metre". Without
/// it a cost value is a lump sum; with it, it is a rate and multiplying it by
/// a quantity is meaningful.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitBasis {
    /// The entity holding the basis.
    pub id: EntityId,
    /// The numeric component of the basis measure.
    pub value: Option<f64>,
    /// The measure type wrapping it, e.g. `IFCVOLUMEMEASURE`.
    pub measure: Option<&'static str>,
    /// The unit entity the basis names, if it resolves.
    pub unit: Option<EntityId>,
}

/// A borrowed view of an `IfcCostValue` entity.
#[derive(Debug, Clone, Copy)]
pub struct CostValue<'m> {
    id: EntityId,
    entity: &'m Entity,
}

impl<'m> CostValue<'m> {
    /// Wrap an entity known to be an `IfcCostValue`.
    #[must_use]
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self { id, entity }
    }

    /// The entity id in the file.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The value's name, e.g. `Estimate`.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.entity.text(slot::NAME)
    }

    /// The value's description.
    #[must_use]
    pub fn description(&self) -> Option<&'m str> {
        self.entity.text(slot::DESCRIPTION)
    }

    /// The directly stated amount.
    ///
    /// `None` for a composed value that states only `Components`. Use
    /// [`CostValue::component_refs`] and [`CostValue::operator`] for those, or
    /// [`CostValue::is_composed`] to tell the two shapes apart.
    #[must_use]
    pub fn amount(&self) -> Option<f64> {
        self.entity
            .attribute(slot::APPLIED_VALUE)?
            .unwrap_typed()
            .as_f64()
    }

    /// The measure type wrapping the applied value, e.g. `IFCMONETARYMEASURE`.
    ///
    /// A cost value is not required to be monetary: `IfcAppliedValueSelect`
    /// also admits ratios and plain measures. Reporting the wrapper lets a
    /// caller refuse to add a ratio to a currency.
    #[must_use]
    pub fn measure(&self) -> Option<&'m str> {
        match self.entity.attribute(slot::APPLIED_VALUE)? {
            Value::Typed { type_name, .. } => Some(type_name),
            _ => None,
        }
    }

    /// Whether the applied value is a monetary measure.
    #[must_use]
    pub fn is_monetary(&self) -> bool {
        self.measure()
            .is_some_and(|m| m.eq_ignore_ascii_case("IFCMONETARYMEASURE"))
    }

    /// The rate category, e.g. `Labour` or `Material`.
    #[must_use]
    pub fn category(&self) -> Option<&'m str> {
        self.entity.text(slot::CATEGORY)
    }

    /// The condition under which the value applies.
    #[must_use]
    pub fn condition(&self) -> Option<&'m str> {
        self.entity.text(slot::CONDITION)
    }

    /// The date from which the value applies, as authored.
    #[must_use]
    pub fn applicable_date(&self) -> Option<&'m str> {
        self.entity.text(slot::APPLICABLE_DATE)
    }

    /// The date after which the value no longer holds, as authored.
    #[must_use]
    pub fn fixed_until_date(&self) -> Option<&'m str> {
        self.entity.text(slot::FIXED_UNTIL_DATE)
    }

    /// How this value's components combine, if stated.
    #[must_use]
    pub fn operator(&self) -> Option<ArithmeticOperator> {
        match self.entity.attribute(slot::ARITHMETIC_OPERATOR)? {
            Value::Enum(token) => ArithmeticOperator::parse(token),
            _ => None,
        }
    }

    /// Ids of the nested `IfcAppliedValue` components, in file order.
    ///
    /// Order is preserved because it is the only ordering information the file
    /// carries, and an order-sensitive operator needs it.
    #[must_use]
    pub fn component_refs(&self) -> Vec<EntityId> {
        let mut out = Vec::new();
        if let Some(v) = self.entity.attribute(slot::COMPONENTS) {
            v.for_each_ref(&mut |id| out.push(id));
        }
        out
    }

    /// Whether this value is composed from components rather than stated.
    #[must_use]
    pub fn is_composed(&self) -> bool {
        !self.component_refs().is_empty()
    }

    /// The rate basis, if this value is a rate rather than a lump sum.
    ///
    /// Resolves the `IfcMeasureWithUnit`: slot 0 is `ValueComponent`, slot 1
    /// is `UnitComponent`.
    #[must_use]
    pub fn unit_basis(&self, model: &Model) -> Option<UnitBasis> {
        let id = match self.entity.attribute(slot::UNIT_BASIS)? {
            Value::Ref(id) => *id,
            _ => return None,
        };
        let measure_with_unit = model.get(id)?;
        let component = measure_with_unit.attribute(0);
        Some(UnitBasis {
            id,
            value: component.and_then(|v| v.unwrap_typed().as_f64()),
            measure: component.and_then(|v| match v {
                // Leaked as 'static only when the name is one we know; a
                // borrowed lifetime would tie UnitBasis to the model borrow
                // and stop it being Copy.
                Value::Typed { type_name, .. } => KNOWN_MEASURES
                    .iter()
                    .find(|m| type_name.eq_ignore_ascii_case(m))
                    .copied(),
                _ => None,
            }),
            unit: match measure_with_unit.attribute(1) {
                Some(Value::Ref(unit)) => Some(*unit),
                _ => None,
            },
        })
    }
}

/// Measure names a unit basis is expected to carry.
///
/// Anything outside this list reports `None` rather than being invented: the
/// point of the field is to let a caller check dimensional agreement, and an
/// unrecognised name is not evidence of agreement.
const KNOWN_MEASURES: &[&str] = &[
    "IFCVOLUMEMEASURE",
    "IFCAREAMEASURE",
    "IFCLENGTHMEASURE",
    "IFCMASSMEASURE",
    "IFCCOUNTMEASURE",
    "IFCTIMEMEASURE",
    "IFCMONETARYMEASURE",
];
