//! Booleans: `IfcBooleanResult` and the clipping specialisation.
//!
//! # A boolean is a tree, not a pair of solids
//!
//! `FirstOperand` and `SecondOperand` are both `IfcBooleanOperand`, a SELECT
//! that **includes `IfcBooleanResult` itself**. Real files nest these deeply:
//! a wall with eight openings is commonly a left-leaning chain of eight
//! `IfcBooleanClippingResult`s, each one's `FirstOperand` being the previous
//! result.
//!
//! Any consumer that assumes two leaf solids handles the first opening and
//! drops the other seven. [`BooleanResult::operands`] therefore returns
//! references to be walked recursively, and [`OperandKind`] classifies what was
//! found so a walker knows when to recurse.
//!
//! Depth is bounded in practice but not by the schema, and a self-referencing
//! tree is expressible, so any recursive walk needs its own depth limit and
//! cycle check -- see [`crate::GeometryError::CyclicChain`].
//!
//! # `IfcBooleanClippingResult` is a constrained `IfcBooleanResult`
//!
//! It adds no attributes. Its EXPRESS WHERE rules require the operator to be
//! DIFFERENCE, the first operand to be a swept solid or another clipping
//! result, and the second to be an `IfcHalfSpaceSolid`. It exists so a consumer
//! can recognise "solid minus half space" -- the cheap, always-implementable
//! case -- without inspecting the operands.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId, Model};

/// `IfcBooleanResult` attribute slots.
///
/// EXPRESS (IFC4 ADD2 TC1): subtypes `IfcGeometricRepresentationItem`, which
/// declares no explicit attributes, so all three are absolute slots 0-2.
/// `IfcBooleanClippingResult` adds none, so it shares this exact layout.
mod slot {
    /// `Operator : IfcBooleanOperator`.
    pub const OPERATOR: usize = 0;
    /// `FirstOperand : IfcBooleanOperand`.
    pub const FIRST_OPERAND: usize = 1;
    /// `SecondOperand : IfcBooleanOperand`.
    pub const SECOND_OPERAND: usize = 2;
}

/// `IfcBooleanOperator`: the three set operations IFC defines.
///
/// The IFC-file enumeration is distinct from the neutral
/// [`axiolid_core::BooleanOperator`], with an explicit lossless conversion at the
/// adapter boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IfcBooleanOperator {
    /// `.UNION.` -- everything in either operand.
    Union,
    /// `.INTERSECTION.` -- everything in both operands.
    Intersection,
    /// `.DIFFERENCE.` -- first operand minus second. **Not commutative**;
    /// swapping the operands of a clipping result deletes the wall instead of
    /// the opening.
    Difference,
}

/// Legacy source-compatible name for the IFC-file enumeration.
pub use IfcBooleanOperator as BooleanOperator;

/// Invalid `IfcBooleanOperator` token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("unknown IfcBooleanOperator token")]
pub struct ParseIfcBooleanOperatorError;

impl core::str::FromStr for IfcBooleanOperator {
    type Err = ParseIfcBooleanOperatorError;

    fn from_str(token: &str) -> Result<Self, Self::Err> {
        let bare = token.trim_matches('.');
        if bare.eq_ignore_ascii_case("UNION") {
            Ok(Self::Union)
        } else if bare.eq_ignore_ascii_case("INTERSECTION") {
            Ok(Self::Intersection)
        } else if bare.eq_ignore_ascii_case("DIFFERENCE") {
            Ok(Self::Difference)
        } else {
            Err(ParseIfcBooleanOperatorError)
        }
    }
}

impl core::fmt::Display for IfcBooleanOperator {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, ".{}.", self.as_token())
    }
}

impl From<IfcBooleanOperator> for axiolid_core::BooleanOperator {
    fn from(value: IfcBooleanOperator) -> Self {
        match value {
            IfcBooleanOperator::Union => Self::Union,
            IfcBooleanOperator::Intersection => Self::Intersection,
            IfcBooleanOperator::Difference => Self::Difference,
        }
    }
}

impl IfcBooleanOperator {
    /// Parse an enumeration token, with or without its surrounding dots.
    ///
    /// Accepts any case because STEP keywords are case-insensitive and real
    /// exporters are inconsistent about it.
    pub fn parse(token: &str) -> Option<Self> {
        token.parse().ok()
    }

    /// The EXPRESS token, without dots.
    pub fn as_token(self) -> &'static str {
        match self {
            Self::Union => "UNION",
            Self::Intersection => "INTERSECTION",
            Self::Difference => "DIFFERENCE",
        }
    }

    /// Does operand order change the result?
    ///
    /// Only DIFFERENCE is order-sensitive, and it is also the most common
    /// operator in building models, so a walker that normalises operand order
    /// for caching must check this first.
    pub fn is_order_sensitive(self) -> bool {
        matches!(self, Self::Difference)
    }
}

/// What an `IfcBooleanOperand` reference actually points at.
///
/// The SELECT permits five families. A walker needs to know whether to recurse
/// (a nested result) or to hand the entity to a solid builder, and classifying
/// once here keeps that decision out of every call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperandKind {
    /// Another `IfcBooleanResult` (or `IfcBooleanClippingResult`): **recurse**.
    BooleanResult,
    /// An `IfcCsgPrimitive3D` subtype.
    CsgPrimitive,
    /// An `IfcHalfSpaceSolid` subtype: infinite, only valid here.
    HalfSpace,
    /// An `IfcSolidModel` subtype: swept, brep or CSG solid.
    SolidModel,
    /// An `IfcTessellatedFaceSet` subtype. The schema requires it to be closed
    /// when used as a boolean operand.
    TessellatedFaceSet,
}

impl OperandKind {
    /// Classify by IFC type name.
    ///
    /// Name-based because the model is untyped by design; the alternative is a
    /// generated subtype table per schema version, which is what this codebase
    /// exists to avoid.
    pub fn classify(type_name: &str) -> Option<Self> {
        let n = type_name.to_ascii_uppercase();
        let kind = match n.as_str() {
            "IFCBOOLEANRESULT" | "IFCBOOLEANCLIPPINGRESULT" => Self::BooleanResult,
            "IFCBLOCK"
            | "IFCRECTANGULARPYRAMID"
            | "IFCRIGHTCIRCULARCONE"
            | "IFCRIGHTCIRCULARCYLINDER"
            | "IFCSPHERE"
            | "IFCCSGPRIMITIVE3D" => Self::CsgPrimitive,
            "IFCHALFSPACESOLID" | "IFCBOXEDHALFSPACE" | "IFCPOLYGONALBOUNDEDHALFSPACE" => {
                Self::HalfSpace
            }
            "IFCTRIANGULATEDFACESET" | "IFCPOLYGONALFACESET" => Self::TessellatedFaceSet,
            "IFCCSGSOLID"
            | "IFCFACETEDBREP"
            | "IFCFACETEDBREPWITHVOIDS"
            | "IFCADVANCEDBREP"
            | "IFCADVANCEDBREPWITHVOIDS"
            | "IFCEXTRUDEDAREASOLID"
            | "IFCEXTRUDEDAREASOLIDTAPERED"
            | "IFCREVOLVEDAREASOLID"
            | "IFCREVOLVEDAREASOLIDTAPERED"
            | "IFCSURFACECURVESWEPTAREASOLID"
            | "IFCFIXEDREFERENCESWEPTAREASOLID"
            | "IFCSWEPTDISKSOLID"
            | "IFCSWEPTDISKSOLIDPOLYGONAL" => Self::SolidModel,
            _ => return None,
        };
        Some(kind)
    }

    /// Must a walker descend into this operand?
    pub fn is_nested_boolean(self) -> bool {
        matches!(self, Self::BooleanResult)
    }
}

/// `IfcBooleanResult`: two operands combined by a set operation.
///
/// See the module docs: the operands form a **tree** and may themselves be
/// boolean results.
#[derive(Debug, Clone, Copy)]
pub struct BooleanResult<'m> {
    slots: Slots<'m>,
}

impl<'m> BooleanResult<'m> {
    /// Wrap an entity assumed to be an `IfcBooleanResult` or a subtype.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The IFC type name, naming the concrete subtype.
    pub fn type_name(&self) -> &'m str {
        self.slots.type_name()
    }

    /// The set operation, parsed.
    ///
    /// An unrecognised token is an error rather than a silent UNION, because
    /// substituting the wrong operator produces a solid that looks built.
    pub fn operator(&self) -> GeometryResult<IfcBooleanOperator> {
        let token = self
            .slots
            .opt_enum(slot::OPERATOR)
            .ok_or_else(|| self.missing("Operator"))?;
        IfcBooleanOperator::parse(token).ok_or_else(|| {
            self.slots
                .degenerate(format!("unknown IfcBooleanOperator '.{token}.'"))
        })
    }

    /// The first operand's reference. For DIFFERENCE this is the minuend.
    pub fn first_operand(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::FIRST_OPERAND, "FirstOperand")
    }

    /// The second operand's reference. For DIFFERENCE this is the subtrahend.
    pub fn second_operand(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::SECOND_OPERAND, "SecondOperand")
    }

    /// Both operands in schema order.
    ///
    /// Order is preserved and must never be normalised for a DIFFERENCE; see
    /// [`IfcBooleanOperator::is_order_sensitive`].
    pub fn operands(&self) -> GeometryResult<(EntityId, EntityId)> {
        Ok((self.first_operand()?, self.second_operand()?))
    }

    /// Classify an operand, resolving it in `model`.
    ///
    /// Returns `None` for an entity type that is not a legal operand, letting a
    /// caller report the file's own inconsistency with its own context.
    pub fn operand_kind(
        &self,
        model: &'m Model,
        operand: EntityId,
    ) -> GeometryResult<Option<OperandKind>> {
        let entity = self.slots.resolve(model, operand)?;
        Ok(OperandKind::classify(&entity.type_name))
    }

    /// Is this the `IfcBooleanClippingResult` specialisation?
    pub fn is_clipping(&self) -> bool {
        self.type_name()
            .eq_ignore_ascii_case("IFCBOOLEANCLIPPINGRESULT")
    }

    /// A `MissingAttribute` error for this entity.
    fn missing(&self, attribute: &'static str) -> crate::GeometryError {
        crate::GeometryError::MissingAttribute {
            entity: self.slots.id(),
            type_name: self.slots.type_name().to_string(),
            attribute,
        }
    }
}

/// `IfcBooleanClippingResult`: a solid clipped by a half space.
///
/// Adds no attributes over [`BooleanResult`]; it is a promise about the
/// operands, backed by EXPRESS WHERE rules:
///
/// - `Operator` is DIFFERENCE.
/// - `FirstOperand` is an `IfcSweptAreaSolid`, an `IfcSweptDiskSolid` or
///   another `IfcBooleanClippingResult`.
/// - `SecondOperand` is an `IfcHalfSpaceSolid`.
///
/// That recursion in the first operand is the point: an element clipped N
/// times is a chain of N of these, and the chain must be walked to the bottom.
#[derive(Debug, Clone, Copy)]
pub struct BooleanClippingResult<'m> {
    slots: Slots<'m>,
}

impl<'m> BooleanClippingResult<'m> {
    /// Wrap an entity assumed to be an `IfcBooleanClippingResult`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcBooleanResult` attributes.
    pub fn base(&self) -> BooleanResult<'m> {
        BooleanResult { slots: self.slots }
    }

    /// Check the schema's operator constraint.
    ///
    /// Files do violate it. Reporting rather than assuming DIFFERENCE means a
    /// mislabelled union is visible instead of quietly cutting the element.
    pub fn checked_operator(&self) -> GeometryResult<IfcBooleanOperator> {
        let op = self.base().operator()?;
        if op == IfcBooleanOperator::Difference {
            Ok(op)
        } else {
            Err(self.slots.degenerate(format!(
                "IfcBooleanClippingResult requires DIFFERENCE, found {}",
                op.as_token()
            )))
        }
    }
}

#[cfg(test)]
mod tests;
