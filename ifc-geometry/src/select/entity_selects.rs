//! The `SELECT` types of the three geometry schemas.
//!
//! Each resolves an entity reference to the *branch* of the select it takes,
//! so callers write an exhaustive `match` the compiler checks instead of a
//! string comparison they must keep in sync with the schema.
//!
//! Every resolver is subtype-aware via [`is_a`]: select members are usually
//! abstract, so a direct type-name comparison would reject valid files.

use crate::error::{GeometryError, GeometryResult};
use crate::select::subtype::is_a;
use ifc_model::{Entity, EntityId, Model, Value};

/// Build the "this is not a permitted member" error for a select.
fn not_a_member(entity: EntityId, actual: &str, expected: &'static str) -> GeometryError {
    GeometryError::WrongEntityType {
        entity,
        actual: actual.to_string(),
        expected,
    }
}

/// Resolve a reference, then classify it with `f`.
///
/// Shared by every select so a dangling reference reports identically
/// regardless of which attribute held it.
fn resolve_and_classify<T>(
    model: &Model,
    referrer: EntityId,
    target: EntityId,
    expected: &'static str,
    f: impl Fn(&str) -> Option<T>,
) -> GeometryResult<T> {
    let entity: &Entity = model.get(target).ok_or(GeometryError::MissingEntity {
        referrer,
        missing: target,
    })?;
    f(&entity.type_name).ok_or_else(|| not_a_member(target, &entity.type_name, expected))
}

/// `IfcAxis2Placement` = `IfcAxis2Placement2D` | `IfcAxis2Placement3D`.
///
/// The dimensionality of a placement is not stated by the attribute that holds
/// it; it is the target's type. A 2D placement inside a 3D context is a real
/// modelling error, so the distinction must survive to the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis2Placement {
    /// `IfcAxis2Placement2D`.
    TwoD(EntityId),
    /// `IfcAxis2Placement3D`.
    ThreeD(EntityId),
}

impl Axis2Placement {
    /// Classify a placement reference.
    pub fn resolve(model: &Model, referrer: EntityId, target: EntityId) -> GeometryResult<Self> {
        resolve_and_classify(model, referrer, target, "IfcAxis2Placement", |t| {
            if is_a(t, "IFCAXIS2PLACEMENT2D") {
                Some(Self::TwoD(target))
            } else if is_a(t, "IFCAXIS2PLACEMENT3D") {
                Some(Self::ThreeD(target))
            } else {
                None
            }
        })
    }

    /// The referenced entity, whichever branch was taken.
    pub fn id(&self) -> EntityId {
        match self {
            Self::TwoD(id) | Self::ThreeD(id) => *id,
        }
    }

    /// How many coordinates this placement works in.
    pub fn dimension(&self) -> usize {
        match self {
            Self::TwoD(_) => 2,
            Self::ThreeD(_) => 3,
        }
    }
}

/// `IfcBooleanOperand` = `IfcSolidModel` | `IfcHalfSpaceSolid` |
/// `IfcBooleanResult` | `IfcCsgPrimitive3D` | `IfcTessellatedFaceSet`.
///
/// # Why the branch matters
///
/// [`Self::HalfSpace`] is the one operand a kernel cannot tessellate on its
/// own, because a half space is infinite. A caller that treats all operands
/// alike will hand a kernel something unboundable; distinguishing the branch
/// here is what lets it clip first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOperand {
    /// A solid model: swept, brep, CSG or disk-swept.
    Solid(EntityId),
    /// An infinite half space. Only meaningful inside a boolean.
    HalfSpace(EntityId),
    /// A nested boolean result: the operand tree is recursive.
    BooleanResult(EntityId),
    /// An analytic CSG primitive.
    CsgPrimitive(EntityId),
    /// A tessellated face set (IFC4 ADD2 permits these as operands).
    TessellatedFaceSet(EntityId),
}

impl BooleanOperand {
    /// Classify a boolean operand reference.
    pub fn resolve(model: &Model, referrer: EntityId, target: EntityId) -> GeometryResult<Self> {
        resolve_and_classify(model, referrer, target, "IfcBooleanOperand", |t| {
            // Order matters: IfcBooleanResult and IfcCsgPrimitive3D are not
            // IfcSolidModel subtypes, but half spaces and solids overlap
            // nothing, so any order among the rest is safe.
            if is_a(t, "IFCHALFSPACESOLID") {
                Some(Self::HalfSpace(target))
            } else if is_a(t, "IFCBOOLEANRESULT") {
                Some(Self::BooleanResult(target))
            } else if is_a(t, "IFCCSGPRIMITIVE3D") {
                Some(Self::CsgPrimitive(target))
            } else if is_a(t, "IFCTESSELLATEDFACESET") {
                Some(Self::TessellatedFaceSet(target))
            } else if is_a(t, "IFCSOLIDMODEL") {
                Some(Self::Solid(target))
            } else {
                None
            }
        })
    }

    /// The referenced entity.
    pub fn id(&self) -> EntityId {
        match self {
            Self::Solid(id)
            | Self::HalfSpace(id)
            | Self::BooleanResult(id)
            | Self::CsgPrimitive(id)
            | Self::TessellatedFaceSet(id) => *id,
        }
    }

    /// Is this operand unbounded, and so unusable outside a boolean?
    pub fn is_unbounded(&self) -> bool {
        matches!(self, Self::HalfSpace(_))
    }
}

/// `IfcCsgSelect` = `IfcBooleanResult` | `IfcCsgPrimitive3D`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsgSelect {
    /// A nested boolean operation.
    BooleanResult(EntityId),
    /// A leaf analytic primitive.
    Primitive(EntityId),
}

impl CsgSelect {
    /// Classify a CSG tree root.
    pub fn resolve(model: &Model, referrer: EntityId, target: EntityId) -> GeometryResult<Self> {
        resolve_and_classify(model, referrer, target, "IfcCsgSelect", |t| {
            if is_a(t, "IFCBOOLEANRESULT") {
                Some(Self::BooleanResult(target))
            } else if is_a(t, "IFCCSGPRIMITIVE3D") {
                Some(Self::Primitive(target))
            } else {
                None
            }
        })
    }

    /// The referenced entity.
    pub fn id(&self) -> EntityId {
        match self {
            Self::BooleanResult(id) | Self::Primitive(id) => *id,
        }
    }
}

/// `IfcSolidOrShell` = `IfcClosedShell` | `IfcSolidModel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidOrShell {
    /// A closed shell: a boundary, not a solid.
    ClosedShell(EntityId),
    /// A solid model.
    Solid(EntityId),
}

impl SolidOrShell {
    /// Classify the reference.
    pub fn resolve(model: &Model, referrer: EntityId, target: EntityId) -> GeometryResult<Self> {
        resolve_and_classify(model, referrer, target, "IfcSolidOrShell", |t| {
            if is_a(t, "IFCCLOSEDSHELL") {
                Some(Self::ClosedShell(target))
            } else if is_a(t, "IFCSOLIDMODEL") {
                Some(Self::Solid(target))
            } else {
                None
            }
        })
    }

    /// The referenced entity.
    pub fn id(&self) -> EntityId {
        match self {
            Self::ClosedShell(id) | Self::Solid(id) => *id,
        }
    }
}

/// `IfcTrimmingSelect` = `IfcCartesianPoint` | `IfcParameterValue`.
///
/// # The one select that is not always a reference
///
/// A trim may be a *point* (an entity reference) or a *parameter* (a bare
/// number wrapped as `IFCPARAMETERVALUE(1.57)`). Both may be present on the
/// same trim, and `IfcTrimmingPreference` decides which wins. Modelling this
/// as anything less than both-optional loses information the file carries.
#[derive(Debug, Clone, PartialEq)]
pub struct TrimmingSelect {
    /// Trim given as a Cartesian point.
    pub point: Option<EntityId>,
    /// Trim given as a parameter along the basis curve.
    pub parameter: Option<f64>,
}

impl TrimmingSelect {
    /// Read a trim from the aggregate the schema declares it as.
    ///
    /// `IfcTrimmedCurve.Trim1` is `SET [1:2] OF IfcTrimmingSelect`, so one
    /// value holds up to both representations.
    pub fn from_value(value: &Value) -> Self {
        let mut out = Self {
            point: None,
            parameter: None,
        };
        let items: Vec<&Value> = match value {
            Value::List(items) => items.iter().collect(),
            single => vec![single],
        };
        for item in items {
            match item {
                Value::Ref(id) => out.point = Some(*id),
                other => {
                    if let Some(n) = other.unwrap_typed().as_f64() {
                        out.parameter = Some(n);
                    }
                }
            }
        }
        out
    }

    /// Does this trim carry anything at all?
    pub fn is_empty(&self) -> bool {
        self.point.is_none() && self.parameter.is_none()
    }
}

/// `IfcVectorOrDirection` = `IfcDirection` | `IfcVector`.
///
/// A vector carries a magnitude, a direction does not. Collapsing the two
/// loses the length of an extrusion whose direction is given as a vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorOrDirection {
    /// `IfcDirection`: orientation only.
    Direction(EntityId),
    /// `IfcVector`: orientation plus magnitude.
    Vector(EntityId),
}

impl VectorOrDirection {
    /// Classify the reference.
    pub fn resolve(model: &Model, referrer: EntityId, target: EntityId) -> GeometryResult<Self> {
        resolve_and_classify(model, referrer, target, "IfcVectorOrDirection", |t| {
            if is_a(t, "IFCVECTOR") {
                Some(Self::Vector(target))
            } else if is_a(t, "IFCDIRECTION") {
                Some(Self::Direction(target))
            } else {
                None
            }
        })
    }

    /// The referenced entity.
    pub fn id(&self) -> EntityId {
        match self {
            Self::Direction(id) | Self::Vector(id) => *id,
        }
    }

    /// Does this carry a magnitude of its own?
    pub fn has_magnitude(&self) -> bool {
        matches!(self, Self::Vector(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_with(id: u64, type_name: &str) -> Model {
        let mut model = Model::new();
        model.insert(EntityId(id), Entity::new(type_name, vec![Value::Null; 4]));
        model
    }

    /// The headline property: a concrete subtype satisfies an abstract member.
    #[test]
    fn a_concrete_solid_resolves_as_the_abstract_solid_model_branch() {
        let model = model_with(5, "IFCEXTRUDEDAREASOLID");
        let operand = BooleanOperand::resolve(&model, EntityId(1), EntityId(5)).unwrap();
        assert_eq!(operand, BooleanOperand::Solid(EntityId(5)));
        assert!(!operand.is_unbounded());
    }

    /// The branch a kernel must treat specially.
    #[test]
    fn half_space_operands_are_flagged_unbounded() {
        for t in ["IFCHALFSPACESOLID", "IFCPOLYGONALBOUNDEDHALFSPACE"] {
            let model = model_with(5, t);
            let operand = BooleanOperand::resolve(&model, EntityId(1), EntityId(5)).unwrap();
            assert!(operand.is_unbounded(), "{t} must be unbounded");
        }
    }

    #[test]
    fn nested_boolean_results_are_their_own_branch() {
        let model = model_with(5, "IFCBOOLEANCLIPPINGRESULT");
        assert_eq!(
            BooleanOperand::resolve(&model, EntityId(1), EntityId(5)).unwrap(),
            BooleanOperand::BooleanResult(EntityId(5))
        );
    }

    #[test]
    fn an_entity_outside_the_select_is_rejected_with_its_actual_type() {
        let model = model_with(5, "IFCWALL");
        let err = BooleanOperand::resolve(&model, EntityId(1), EntityId(5)).unwrap_err();
        assert!(err.to_string().contains("IFCWALL"), "got {err}");
    }

    #[test]
    fn a_dangling_reference_is_reported_as_missing_not_as_wrong_type() {
        let model = Model::new();
        assert!(matches!(
            BooleanOperand::resolve(&model, EntityId(1), EntityId(99)).unwrap_err(),
            GeometryError::MissingEntity { .. }
        ));
    }

    #[test]
    fn placement_dimension_comes_from_the_target_type() {
        let two = model_with(5, "IFCAXIS2PLACEMENT2D");
        assert_eq!(
            Axis2Placement::resolve(&two, EntityId(1), EntityId(5))
                .unwrap()
                .dimension(),
            2
        );
        let three = model_with(5, "IFCAXIS2PLACEMENT3D");
        assert_eq!(
            Axis2Placement::resolve(&three, EntityId(1), EntityId(5))
                .unwrap()
                .dimension(),
            3
        );
    }

    /// Both trim representations may be present at once.
    #[test]
    fn a_trim_can_carry_a_point_and_a_parameter_together() {
        let trim = TrimmingSelect::from_value(&Value::List(vec![
            Value::Ref(EntityId(7)),
            Value::Typed {
                type_name: "IFCPARAMETERVALUE".into(),
                value: Box::new(Value::Real(0.75)),
            },
        ]));
        assert_eq!(trim.point, Some(EntityId(7)));
        assert_eq!(trim.parameter, Some(0.75));
        assert!(!trim.is_empty());
    }

    #[test]
    fn a_trim_may_be_a_bare_parameter() {
        let trim = TrimmingSelect::from_value(&Value::Typed {
            type_name: "IFCPARAMETERVALUE".into(),
            value: Box::new(Value::Real(0.0)),
        });
        assert_eq!(trim.parameter, Some(0.0));
        assert_eq!(trim.point, None);
    }

    /// A vector's magnitude must not be lost by collapsing it to a direction.
    #[test]
    fn vectors_are_distinguished_from_directions() {
        let vector = model_with(5, "IFCVECTOR");
        assert!(
            VectorOrDirection::resolve(&vector, EntityId(1), EntityId(5))
                .unwrap()
                .has_magnitude()
        );

        let direction = model_with(5, "IFCDIRECTION");
        assert!(
            !VectorOrDirection::resolve(&direction, EntityId(1), EntityId(5))
                .unwrap()
                .has_magnitude()
        );
    }

    #[test]
    fn solid_or_shell_separates_boundaries_from_solids() {
        let shell = model_with(5, "IFCCLOSEDSHELL");
        assert_eq!(
            SolidOrShell::resolve(&shell, EntityId(1), EntityId(5)).unwrap(),
            SolidOrShell::ClosedShell(EntityId(5))
        );
        let solid = model_with(5, "IFCFACETEDBREP");
        assert_eq!(
            SolidOrShell::resolve(&solid, EntityId(1), EntityId(5)).unwrap(),
            SolidOrShell::Solid(EntityId(5))
        );
    }
}
