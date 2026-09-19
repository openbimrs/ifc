//! Shared attribute access for typed geometry views.
//!
//! Every one of the 112 geometry entities reads positional attributes and must
//! report the same way when one is missing or malformed. Without this, each
//! view re-implements the same error construction and they drift.
//!
//! # The pattern
//!
//! A view is a newtype over `(EntityId, &Entity)`. It declares its slots as
//! `mod slot` constants citing the EXPRESS declaration, then uses [`Slots`] to
//! read them. Views own nothing and cost nothing to construct.

use crate::error::{GeometryError, GeometryResult};
use ifc_model::{Entity, EntityId, Model, Value};

/// Attribute reader bound to one entity.
///
/// Carries the id and type name so every error is self-locating without the
/// call site repeating them.
#[derive(Debug, Clone, Copy)]
pub struct Slots<'m> {
    id: EntityId,
    entity: &'m Entity,
}

impl<'m> Slots<'m> {
    /// Wrap an entity for attribute access.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self { id, entity }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// The underlying entity.
    pub fn entity(&self) -> &'m Entity {
        self.entity
    }

    /// The IFC type name.
    pub fn type_name(&self) -> &'m str {
        &self.entity.type_name
    }

    /// Raw attribute, `None` when absent or `$`.
    ///
    /// Treats a missing slot and an explicit `$` alike: both mean "not
    /// provided", and real files disagree about which to write for trailing
    /// optionals.
    pub fn opt(&self, index: usize) -> Option<&'m Value> {
        match self.entity.attribute(index) {
            None | Some(Value::Null) => None,
            other => other,
        }
    }

    /// Required attribute.
    pub fn req(&self, index: usize, name: &'static str) -> GeometryResult<&'m Value> {
        self.opt(index)
            .ok_or_else(|| GeometryError::MissingAttribute {
                entity: self.id,
                type_name: self.type_name().to_string(),
                attribute: name,
            })
    }

    /// Required real number, accepting an integer literal.
    ///
    /// STEP writes `0` where a real is declared often enough that rejecting it
    /// would fail on conforming-in-practice files.
    pub fn req_f64(&self, index: usize, name: &'static str) -> GeometryResult<f64> {
        let value = self.req(index, name)?;
        value
            .unwrap_typed()
            .as_f64()
            .ok_or_else(|| self.kind_error(name, "a number", value))
    }

    /// Optional real number.
    pub fn opt_f64(&self, index: usize) -> Option<f64> {
        self.opt(index)?.unwrap_typed().as_f64()
    }

    /// Required integer.
    pub fn req_i64(&self, index: usize, name: &'static str) -> GeometryResult<i64> {
        let value = self.req(index, name)?;
        match value.unwrap_typed() {
            Value::Integer(i) => Ok(*i),
            other => Err(self.kind_error(name, "an integer", other)),
        }
    }

    /// Required entity reference.
    pub fn req_ref(&self, index: usize, name: &'static str) -> GeometryResult<EntityId> {
        let value = self.req(index, name)?;
        value
            .as_ref_id()
            .ok_or_else(|| self.kind_error(name, "an entity reference", value))
    }

    /// Optional entity reference.
    pub fn opt_ref(&self, index: usize) -> Option<EntityId> {
        self.opt(index)?.as_ref_id()
    }

    /// Required list of reals, e.g. `Coordinates` on `IfcCartesianPoint`.
    pub fn req_f64_list(&self, index: usize, name: &'static str) -> GeometryResult<Vec<f64>> {
        let value = self.req(index, name)?;
        let items = value
            .as_list()
            .ok_or_else(|| self.kind_error(name, "a list", value))?;
        items
            .iter()
            .map(|v| {
                v.unwrap_typed()
                    .as_f64()
                    .ok_or_else(|| self.kind_error(name, "a list of numbers", v))
            })
            .collect()
    }

    /// Required list of entity references.
    pub fn req_ref_list(&self, index: usize, name: &'static str) -> GeometryResult<Vec<EntityId>> {
        let value = self.req(index, name)?;
        let items = value
            .as_list()
            .ok_or_else(|| self.kind_error(name, "a list", value))?;
        items
            .iter()
            .map(|v| {
                v.as_ref_id()
                    .ok_or_else(|| self.kind_error(name, "a list of references", v))
            })
            .collect()
    }

    /// Optional list of entity references; absent becomes empty.
    pub fn opt_ref_list(&self, index: usize) -> Vec<EntityId> {
        self.opt(index)
            .and_then(|v| v.as_list())
            .map(|items| items.iter().filter_map(|v| v.as_ref_id()).collect())
            .unwrap_or_default()
    }

    /// Enumeration token without its dots, e.g. `.CARTESIAN.` -> `CARTESIAN`.
    /// A text attribute, absent when unset or not a string.
    ///
    /// Labels like RepresentationIdentifier are optional in the schema and
    /// authors do omit them, so a missing value is data rather than an error.
    pub fn opt_text(&self, index: usize) -> Option<String> {
        match self.opt(index)?.unwrap_typed() {
            Value::Text(text) => Some(text.to_string()),
            _ => None,
        }
    }

    /// The enumeration token at `index`, when the slot holds one.
    ///
    /// Returns `None` for an omitted slot and for a value that is present but
    /// not an enumeration, so a caller cannot mistake a type error for an
    /// authored absence.
    pub fn opt_enum(&self, index: usize) -> Option<&'m str> {
        match self.opt(index)? {
            Value::Enum(e) => Some(e),
            _ => None,
        }
    }

    /// Boolean or logical value.
    ///
    /// Returns `None` for `.U.` (logical unknown), which is a real third state
    /// in IFC and must not silently become `false`.
    pub fn opt_bool(&self, index: usize) -> Option<bool> {
        match self.opt(index)? {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Required boolean.
    pub fn req_bool(&self, index: usize, name: &'static str) -> GeometryResult<bool> {
        let value = self.req(index, name)?;
        match value {
            Value::Bool(b) => Ok(*b),
            other => Err(self.kind_error(name, "a boolean", other)),
        }
    }

    /// Resolve a referenced entity, failing if it dangles.
    pub fn resolve(&self, model: &'m Model, id: EntityId) -> GeometryResult<&'m Entity> {
        model.get(id).ok_or(GeometryError::MissingEntity {
            referrer: self.id,
            missing: id,
        })
    }

    /// Build a kind mismatch error naming what was actually found.
    fn kind_error(
        &self,
        attribute: &'static str,
        expected: &'static str,
        found: &Value,
    ) -> GeometryError {
        GeometryError::WrongValueKind {
            entity: self.id,
            type_name: self.type_name().to_string(),
            attribute,
            expected,
            found: describe(found),
        }
    }

    /// Report a valid-but-unhandled entity.
    pub fn unsupported(&self, detail: &'static str) -> GeometryError {
        GeometryError::Unsupported {
            entity: self.id,
            type_name: self.type_name().to_string(),
            detail,
        }
    }

    /// Report geometry that cannot exist.
    pub fn degenerate(&self, detail: impl Into<String>) -> GeometryError {
        GeometryError::Degenerate {
            entity: self.id,
            type_name: self.type_name().to_string(),
            detail: detail.into(),
        }
    }
}

/// Short human description of a value's kind, for error messages.
fn describe(value: &Value) -> String {
    match value {
        Value::Null => "$".into(),
        Value::Derived => "*".into(),
        Value::Bool(b) => format!(".{}.", if *b { "T" } else { "F" }),
        Value::LogicalUnknown => ".U.".into(),
        Value::Integer(i) => format!("integer {i}"),
        Value::Real(r) => format!("real {r}"),
        Value::Text(t) => format!("text {t:?}"),
        Value::Binary(_) => "binary".into(),
        Value::Enum(e) => format!(".{e}."),
        Value::Ref(id) => format!("reference {id}"),
        Value::List(items) => format!("list of {}", items.len()),
        Value::Typed { type_name, .. } => format!("{type_name}(...)"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point() -> Entity {
        Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(vec![
                Value::Real(1.0),
                Value::Real(2.0),
                Value::Real(3.0),
            ])],
        )
    }

    #[test]
    fn reads_a_coordinate_list() {
        let e = point();
        let s = Slots::new(EntityId(1), &e);
        assert_eq!(
            s.req_f64_list(0, "Coordinates").unwrap(),
            vec![1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn missing_required_attribute_names_the_entity_and_slot() {
        let e = point();
        let s = Slots::new(EntityId(7), &e);
        let err = s.req(3, "Missing").unwrap_err();
        assert!(err.to_string().contains("#7"));
        assert!(err.to_string().contains("Missing"));
    }

    /// `$` and an absent slot mean the same thing to a consumer.
    #[test]
    fn explicit_null_reads_as_absent() {
        let e = Entity::new("IFCTEST", vec![Value::Null, Value::Real(5.0)]);
        let s = Slots::new(EntityId(1), &e);
        assert!(s.opt(0).is_none(), "$ is absent");
        assert!(s.opt(99).is_none(), "past the end is absent");
        assert_eq!(s.opt_f64(1), Some(5.0));
    }

    /// A measure wrapper must not hide the number from a consumer.
    #[test]
    fn unwraps_typed_measures() {
        let e = Entity::new(
            "IFCCIRCLE",
            vec![Value::Typed {
                type_name: "IFCPOSITIVELENGTHMEASURE".into(),
                value: Box::new(Value::Real(2.5)),
            }],
        );
        let s = Slots::new(EntityId(1), &e);
        assert_eq!(s.req_f64(0, "Radius").unwrap(), 2.5);
    }

    /// STEP writes `0` for a real often enough that rejecting it breaks files.
    #[test]
    fn integer_literal_is_accepted_where_a_real_is_declared() {
        let e = Entity::new("IFCTEST", vec![Value::Integer(0)]);
        let s = Slots::new(EntityId(1), &e);
        assert_eq!(s.req_f64(0, "Depth").unwrap(), 0.0);
    }

    /// `.U.` is a third state and must not collapse into `false`.
    #[test]
    fn logical_unknown_is_not_false() {
        let e = Entity::new("IFCTEST", vec![Value::LogicalUnknown]);
        let s = Slots::new(EntityId(1), &e);
        assert_eq!(s.opt_bool(0), None);
    }

    #[test]
    fn wrong_kind_reports_what_was_actually_found() {
        let e = Entity::new("IFCTEST", vec![Value::Text("nope".into())]);
        let s = Slots::new(EntityId(1), &e);
        let err = s.req_f64(0, "Radius").unwrap_err();
        assert!(err.to_string().contains("text"), "got: {err}");
    }
}

/// Attribute slots for the profile families, shared by both directions.
///
/// Lives here rather than in `lower/` because authoring needs the same
/// numbers and must compile with `--no-default-features`, where `lower/` is
/// absent (ADR 0011). One definition serves reading and writing, so a
/// correction cannot land on one side only.
///
/// `IfcParameterizedProfileDef` contributes ProfileType, ProfileName and
/// Position, so subtype attributes start at slot 3. Every index was read
/// from the IFC4 ADD2 TC1 schema, not inferred.
pub mod profile_slot {
    /// `ProfileType`, `.AREA.` or `.CURVE.` -- declared by `IfcProfileDef`.
    pub const PROFILE_TYPE: usize = 0;
    /// `ProfileName`.
    pub const PROFILE_NAME: usize = 1;
    /// `Position : IfcAxis2Placement2D`.
    pub const POSITION: usize = 2;
    /// `IfcRectangleProfileDef.XDim`.
    pub const X_DIM: usize = 3;
    /// `IfcRectangleProfileDef.YDim`.
    pub const Y_DIM: usize = 4;
    /// `IfcCircleProfileDef.Radius`.
    pub const RADIUS: usize = 3;
    /// `IfcArbitraryClosedProfileDef.OuterCurve`.
    pub const OUTER_CURVE: usize = 2;
    /// `IfcArbitraryProfileDefWithVoids.InnerCurves`.
    pub const INNER_CURVES: usize = 3;
    /// `IfcCircleHollowProfileDef.WallThickness`.
    pub const CIRCLE_WALL_THICKNESS: usize = 4;
    /// `IfcRectangleHollowProfileDef.WallThickness`.
    pub const RECT_WALL_THICKNESS: usize = 5;
    /// `IfcRectangleHollowProfileDef.InnerFilletRadius`.
    pub const RECT_INNER_RADIUS: usize = 6;
    /// `IfcRectangleHollowProfileDef.OuterFilletRadius`.
    pub const RECT_OUTER_RADIUS: usize = 7;
    /// `IfcRoundedRectangleProfileDef.RoundingRadius`.
    pub const ROUNDED_RECT_RADIUS: usize = 5;
}

/// Absolute attribute slots for the parameterized profile families.
///
/// Lives here, not beside the lowerer, because `lower` is behind the
/// `lowering` feature while authoring is kernel-free: a writer that
/// imported these from `lower` would not compile with the feature off.
/// One definition, both directions.
///
/// Every index below was read from the IFC4 ADD2 TC1 schema, not inferred:
/// `IfcParameterizedProfileDef` contributes ProfileType, ProfileName and
/// Position, so subtype attributes start at slot 3.
///
/// Public because it is the schema itself, not an implementation detail:
/// a caller assembling a profile record by hand needs the same indices,
/// and some entries here serve only the lowering direction, so they are
/// unreferenced in a kernel-free build without being dead.
pub mod section_slot {
    // IfcIShapeProfileDef
    /// `IfcIShapeProfileDef.OverallWidth`.
    pub const I_WIDTH: usize = 3;
    /// `IfcIShapeProfileDef.OverallDepth`.
    pub const I_DEPTH: usize = 4;
    /// `IfcIShapeProfileDef.WebThickness`.
    pub const I_WEB: usize = 5;
    /// `IfcIShapeProfileDef.FlangeThickness`.
    pub const I_FLANGE: usize = 6;
    /// `IfcIShapeProfileDef.FilletRadius`.
    pub const I_FILLET: usize = 7;
    /// `IfcIShapeProfileDef.FlangeEdgeRadius`.
    pub const I_EDGE: usize = 8;
    /// `IfcIShapeProfileDef.FlangeSlope`.
    pub const I_SLOPE: usize = 9;

    // IfcAsymmetricIShapeProfileDef
    /// `IfcAsymmetricIShapeProfileDef.BottomFlangeWidth`.
    pub const AI_BOTTOM_WIDTH: usize = 3;
    /// `IfcAsymmetricIShapeProfileDef.OverallDepth`.
    pub const AI_DEPTH: usize = 4;
    /// `IfcAsymmetricIShapeProfileDef.WebThickness`.
    pub const AI_WEB: usize = 5;
    /// `IfcAsymmetricIShapeProfileDef.BottomFlangeThickness`.
    pub const AI_BOTTOM_FLANGE: usize = 6;
    /// `IfcAsymmetricIShapeProfileDef.BottomFlangeFilletRadius`.
    pub const AI_BOTTOM_FILLET: usize = 7;
    /// `IfcAsymmetricIShapeProfileDef.TopFlangeWidth`.
    pub const AI_TOP_WIDTH: usize = 8;
    /// `IfcAsymmetricIShapeProfileDef.TopFlangeThickness`.
    pub const AI_TOP_FLANGE: usize = 9;
    /// `IfcAsymmetricIShapeProfileDef.TopFlangeFilletRadius`.
    pub const AI_TOP_FILLET: usize = 10;
    /// `IfcAsymmetricIShapeProfileDef.BottomFlangeEdgeRadius`.
    pub const AI_BOTTOM_EDGE: usize = 11;
    /// `IfcAsymmetricIShapeProfileDef.BottomFlangeSlope`.
    pub const AI_BOTTOM_SLOPE: usize = 12;
    /// `IfcAsymmetricIShapeProfileDef.TopFlangeEdgeRadius`.
    pub const AI_TOP_EDGE: usize = 13;
    /// `IfcAsymmetricIShapeProfileDef.TopFlangeSlope`.
    pub const AI_TOP_SLOPE: usize = 14;

    // IfcLShapeProfileDef
    /// `IfcLShapeProfileDef.Depth`.
    pub const L_DEPTH: usize = 3;
    /// `IfcLShapeProfileDef.Width`.
    pub const L_WIDTH: usize = 4;
    /// `IfcLShapeProfileDef.Thickness`.
    pub const L_THICKNESS: usize = 5;
    /// `IfcLShapeProfileDef.FilletRadius`.
    pub const L_FILLET: usize = 6;
    /// `IfcLShapeProfileDef.EdgeRadius`.
    pub const L_EDGE: usize = 7;
    /// `IfcLShapeProfileDef.LegSlope`.
    pub const L_SLOPE: usize = 8;

    // IfcTShapeProfileDef
    /// `IfcTShapeProfileDef.Depth`.
    pub const T_DEPTH: usize = 3;
    /// `IfcTShapeProfileDef.FlangeWidth`.
    pub const T_WIDTH: usize = 4;
    /// `IfcTShapeProfileDef.WebThickness`.
    pub const T_WEB: usize = 5;
    /// `IfcTShapeProfileDef.FlangeThickness`.
    pub const T_FLANGE: usize = 6;
    /// `IfcTShapeProfileDef.FilletRadius`.
    pub const T_FILLET: usize = 7;
    /// `IfcTShapeProfileDef.FlangeEdgeRadius`.
    pub const T_FLANGE_EDGE: usize = 8;
    /// `IfcTShapeProfileDef.WebEdgeRadius`.
    pub const T_WEB_EDGE: usize = 9;
    /// `IfcTShapeProfileDef.WebSlope`.
    pub const T_WEB_SLOPE: usize = 10;
    /// `IfcTShapeProfileDef.FlangeSlope`.
    pub const T_FLANGE_SLOPE: usize = 11;

    // IfcUShapeProfileDef
    /// `IfcUShapeProfileDef.Depth`.
    pub const U_DEPTH: usize = 3;
    /// `IfcUShapeProfileDef.FlangeWidth`.
    pub const U_WIDTH: usize = 4;
    /// `IfcUShapeProfileDef.WebThickness`.
    pub const U_WEB: usize = 5;
    /// `IfcUShapeProfileDef.FlangeThickness`.
    pub const U_FLANGE: usize = 6;
    /// `IfcUShapeProfileDef.FilletRadius`.
    pub const U_FILLET: usize = 7;
    /// `IfcUShapeProfileDef.EdgeRadius`.
    pub const U_EDGE: usize = 8;
    /// `IfcUShapeProfileDef.FlangeSlope`.
    pub const U_SLOPE: usize = 9;

    // IfcCShapeProfileDef
    /// `IfcCShapeProfileDef.Depth`.
    pub const C_DEPTH: usize = 3;
    /// `IfcCShapeProfileDef.Width`.
    pub const C_WIDTH: usize = 4;
    /// `IfcCShapeProfileDef.WallThickness`.
    pub const C_WALL: usize = 5;
    /// `IfcCShapeProfileDef.Girth`.
    pub const C_GIRTH: usize = 6;
    /// `IfcCShapeProfileDef.InternalFilletRadius`.
    pub const C_FILLET: usize = 7;

    // IfcZShapeProfileDef
    /// `IfcZShapeProfileDef.Depth`.
    pub const Z_DEPTH: usize = 3;
    /// `IfcZShapeProfileDef.FlangeWidth`.
    pub const Z_FLANGE_WIDTH: usize = 4;
    /// `IfcZShapeProfileDef.WebThickness`.
    pub const Z_WEB: usize = 5;
    /// `IfcZShapeProfileDef.FlangeThickness`.
    pub const Z_FLANGE: usize = 6;
    /// `IfcZShapeProfileDef.FilletRadius`.
    pub const Z_FILLET: usize = 7;
    /// `IfcZShapeProfileDef.EdgeRadius`.
    pub const Z_EDGE: usize = 8;

    // IfcEllipseProfileDef
    /// `IfcEllipseProfileDef.SemiAxis1`.
    pub const E_SEMI1: usize = 3;
    /// `IfcEllipseProfileDef.SemiAxis2`.
    pub const E_SEMI2: usize = 4;

    // IfcTrapeziumProfileDef
    /// `IfcTrapeziumProfileDef.BottomXDim`.
    pub const TZ_BOTTOM: usize = 3;
    /// `IfcTrapeziumProfileDef.TopXDim`.
    pub const TZ_TOP: usize = 4;
    /// `IfcTrapeziumProfileDef.YDim`.
    pub const TZ_Y: usize = 5;
    /// `IfcCenterLineProfileDef`: Curve at 2, Thickness at 3.
    /// Thickness is the FULL width across the path, not a half-width.
    pub const CL_CURVE: usize = 2;
    /// Full width across the centre line.
    pub const CL_THICKNESS: usize = 3;

    /// `IfcTrapeziumProfileDef.TopXOffset`.
    pub const TZ_OFFSET: usize = 6;

    // IfcCompositeProfileDef / IfcDerivedProfileDef
    /// `IfcCompositeProfileDef.Profiles`.
    pub const COMPOSITE_PROFILES: usize = 2;
    /// `IfcDerivedProfileDef.ParentProfile`.
    pub const DERIVED_PARENT: usize = 2;
    /// `IfcDerivedProfileDef.Operator`.
    pub const DERIVED_OPERATOR: usize = 3;
}
