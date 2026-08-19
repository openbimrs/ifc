//! `IfcTrimmedCurve`: the single most misread entity in IFC geometry.
//!
//! # What the file gives you
//!
//! A basis curve, two trims, a sense flag and a preference. Each trim is a
//! `SET [1:2] OF IfcTrimmingSelect`, and `IfcTrimmingSelect` is a select over
//! `IfcCartesianPoint` and `IfcParameterValue`. So a single trim may carry a
//! point, a parameter, or **both**, and when both are present they can
//! disagree -- exporters round the parameter or snap the point. That is what
//! `MasterRepresentation` is for: it names which of the two is authoritative.
//!
//! # The four-arc problem
//!
//! When the basis curve is closed (`IfcCircle`, `IfcEllipse`), one basis curve
//! plus one pair of trim points defines **four** different arcs. Two come from
//! which trim is start and which is end; each of those doubles depending on
//! `SenseAgreement`, because on a closed curve you can reach the same end
//! point going either way round. The spec says so explicitly and notes that
//! informal proposition IP3 (sense implies parameter 1 < parameter 2) does
//! *not* apply to closed basis curves.
//!
//! This module therefore refuses to collapse the four into one. [`TrimSpec`]
//! keeps `Trim1`, `Trim2` and `SenseAgreement` as three independent facts, and
//! nothing here reorders trims to make parameters ascending. A kernel that
//! sorts them will silently draw the complementary arc -- 350 degrees where 10
//! were meant -- and the model will look plausible while being wrong.
//!
//! # Parameter units
//!
//! `IfcParameterValue` on a conic basis curve is an *angle* in the model's
//! declared plane-angle unit, so a file in degrees writes `90.` where a file in
//! radians writes `1.5707963`. This view returns the number as written;
//! converting requires the unit context and belongs to [`crate::units`].

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId, Value};

/// `IfcTrimmedCurve` attribute slots.
///
/// From IFC4 ADD2 TC1: `IfcBoundedCurve` and above declare no explicit
/// attributes, so all five slots belong to `IfcTrimmedCurve` itself.
mod slot {
    /// `BasisCurve`: the `IfcCurve` being trimmed.
    pub const BASIS_CURVE: usize = 0;
    /// `Trim1`: `SET [1:2] OF IfcTrimmingSelect`.
    pub const TRIM_1: usize = 1;
    /// `Trim2`: `SET [1:2] OF IfcTrimmingSelect`.
    pub const TRIM_2: usize = 2;
    /// `SenseAgreement`: `IfcBoolean`.
    pub const SENSE_AGREEMENT: usize = 3;
    /// `MasterRepresentation`: `IfcTrimmingPreference`.
    pub const MASTER_REPRESENTATION: usize = 4;
}

/// `IfcTrimmingPreference`: which trim representation is authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimmingPreference {
    /// The `IfcCartesianPoint` wins where both are supplied.
    Cartesian,
    /// The `IfcParameterValue` wins where both are supplied.
    Parameter,
    /// Neither is preferred; the consumer must choose.
    ///
    /// Common in exported files even when both forms are present, which is
    /// exactly when a choice is needed. Treat it as "use what you have, prefer
    /// the parameter for a conic" rather than as an error.
    Unspecified,
}

impl TrimmingPreference {
    /// Parse the enumeration token, e.g. `CARTESIAN`.
    ///
    /// Returns `None` for an unrecognised token rather than defaulting, so a
    /// caller can tell a typo in the file from an honest `.UNSPECIFIED.`.
    pub fn from_token(token: &str) -> Option<Self> {
        match token.to_ascii_uppercase().as_str() {
            "CARTESIAN" => Some(Self::Cartesian),
            "PARAMETER" => Some(Self::Parameter),
            "UNSPECIFIED" => Some(Self::Unspecified),
            _ => None,
        }
    }
}

/// One member of an `IfcTrimmingSelect` set.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrimPoint {
    /// An `IfcCartesianPoint` reference giving a position on the basis curve.
    // TODO: `resource::point` will provide a typed point view to resolve this.
    Cartesian(EntityId),
    /// An `IfcParameterValue` in the basis curve's own parameterisation.
    ///
    /// For a conic this is an angle in the model's plane-angle unit, not a
    /// length and not necessarily radians.
    Parameter(f64),
}

/// One end of the trim: up to one point and up to one parameter.
///
/// The schema's `SET [1:2]` means "at least one, at most one of each kind".
/// Modelling it as two `Option`s rather than a list makes the both-present
/// case impossible to overlook and the duplicate-kind case impossible to
/// represent.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Trim {
    /// The Cartesian form, if the file supplied one.
    pub cartesian: Option<EntityId>,
    /// The parameter form, if the file supplied one.
    pub parameter: Option<f64>,
}

impl Trim {
    /// The representation `preference` selects, falling back to whichever
    /// exists.
    ///
    /// Under `Unspecified` the parameter is preferred: it is exact for conics,
    /// whereas a Cartesian trim point must be inverted back to a parameter
    /// with a tolerance the schema never specifies.
    pub fn preferred(&self, preference: TrimmingPreference) -> Option<TrimPoint> {
        let cartesian = self.cartesian.map(TrimPoint::Cartesian);
        let parameter = self.parameter.map(TrimPoint::Parameter);
        match preference {
            TrimmingPreference::Cartesian => cartesian.or(parameter),
            TrimmingPreference::Parameter | TrimmingPreference::Unspecified => {
                parameter.or(cartesian)
            }
        }
    }

    /// Does this trim carry both a point and a parameter?
    ///
    /// When true, `MasterRepresentation` is not decoration: the two forms may
    /// disagree by more than any sane tolerance in a rounded export.
    pub fn is_over_specified(&self) -> bool {
        self.cartesian.is_some() && self.parameter.is_some()
    }

    /// Is this trim empty, i.e. the file supplied neither form?
    ///
    /// The schema forbids it (`SET [1:2]`), so an empty trim means a broken
    /// file, not a defaultable value.
    pub fn is_empty(&self) -> bool {
        self.cartesian.is_none() && self.parameter.is_none()
    }
}

/// The complete, uncollapsed trim of a curve.
///
/// Every field is kept because dropping any one of them loses a distinguishable
/// arc. See the module docs on the four-arc problem.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrimSpec {
    /// The `IfcCurve` being trimmed.
    pub basis_curve: EntityId,
    /// The trim at the **start** of the resulting curve.
    pub trim1: Trim,
    /// The trim at the **end** of the resulting curve.
    pub trim2: Trim,
    /// Does the trimmed curve run along the basis curve's own direction?
    ///
    /// `false` means traverse from `trim1` to `trim2` the other way round the
    /// basis curve. On an open basis curve this only reverses the direction;
    /// on a closed one it selects the complementary arc.
    pub sense_agreement: bool,
    /// Which trim form is authoritative.
    pub master_representation: TrimmingPreference,
}

impl TrimSpec {
    /// The start and end trims in traversal order.
    ///
    /// Always `(trim1, trim2)`: `SenseAgreement` changes *which way round the
    /// basis curve* the traversal goes, it does not swap the endpoints. Files
    /// are frequently misread the other way, so this accessor exists to state
    /// the rule once.
    pub fn endpoints(&self) -> (Trim, Trim) {
        (self.trim1, self.trim2)
    }

    /// The start trim resolved through `MasterRepresentation`.
    pub fn start(&self) -> Option<TrimPoint> {
        self.trim1.preferred(self.master_representation)
    }

    /// The end trim resolved through `MasterRepresentation`.
    pub fn end(&self) -> Option<TrimPoint> {
        self.trim2.preferred(self.master_representation)
    }

    /// Both trims carry a parameter, so the arc is fully defined numerically.
    ///
    /// A kernel can take the fast path only when this holds; otherwise it must
    /// project the Cartesian trim points onto the basis curve.
    pub fn is_parametrically_complete(&self) -> bool {
        self.trim1.parameter.is_some() && self.trim2.parameter.is_some()
    }
}

/// A borrowed view of an `IfcTrimmedCurve`.
#[derive(Debug, Clone, Copy)]
pub struct TrimmedCurve<'m> {
    slots: Slots<'m>,
}

impl<'m> TrimmedCurve<'m> {
    /// Wrap an entity known to be an `IfcTrimmedCurve`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The `IfcCurve` being trimmed.
    ///
    /// May itself be trimmed, offset or composite; nesting is legal and this
    /// view does not follow the chain.
    pub fn basis_curve_ref(&self) -> GeometryResult<EntityId> {
        self.slots.req_ref(slot::BASIS_CURVE, "BasisCurve")
    }

    /// Does the trimmed curve follow the basis curve's own direction?
    pub fn sense_agreement(&self) -> GeometryResult<bool> {
        self.slots.req_bool(slot::SENSE_AGREEMENT, "SenseAgreement")
    }

    /// Which trim form wins, defaulting to `Unspecified` when absent.
    ///
    /// A missing or unreadable token is not worth failing the whole curve for:
    /// `Unspecified` is the schema's own "no preference" and callers already
    /// have to handle it.
    pub fn master_representation(&self) -> TrimmingPreference {
        self.slots
            .opt_enum(slot::MASTER_REPRESENTATION)
            .and_then(TrimmingPreference::from_token)
            .unwrap_or(TrimmingPreference::Unspecified)
    }

    /// The start trim.
    pub fn trim1(&self) -> GeometryResult<Trim> {
        self.read_trim(slot::TRIM_1, "Trim1")
    }

    /// The end trim.
    pub fn trim2(&self) -> GeometryResult<Trim> {
        self.read_trim(slot::TRIM_2, "Trim2")
    }

    /// The whole trim, with nothing collapsed.
    ///
    /// Prefer this over the individual accessors: a consumer that reads the
    /// trims without also reading `SenseAgreement` will draw the wrong arc for
    /// a closed basis curve, and gathering them together makes forgetting it
    /// harder.
    pub fn spec(&self) -> GeometryResult<TrimSpec> {
        Ok(TrimSpec {
            basis_curve: self.basis_curve_ref()?,
            trim1: self.trim1()?,
            trim2: self.trim2()?,
            sense_agreement: self.sense_agreement()?,
            master_representation: self.master_representation(),
        })
    }

    /// Read one `SET [1:2] OF IfcTrimmingSelect`.
    fn read_trim(&self, index: usize, name: &'static str) -> GeometryResult<Trim> {
        let value = self.slots.req(index, name)?;
        // Conforming files write a set, but a single-member set is sometimes
        // written unwrapped. Accepting the bare member costs nothing and
        // rejecting it would fail on files every other viewer reads.
        let members: &[Value] = match value {
            Value::List(items) => items,
            single => std::slice::from_ref(single),
        };

        let mut trim = Trim::default();
        for member in members {
            match member {
                Value::Ref(id) => {
                    if trim.cartesian.replace(*id).is_some() {
                        return Err(self.slots.degenerate(format!(
                            "{name} holds two Cartesian trim points; \
                             IfcTrimmingSelect allows at most one of each kind"
                        )));
                    }
                }
                other => {
                    // `IFCPARAMETERVALUE(1.5)`, or a bare number where a
                    // writer dropped the measure wrapper.
                    let Some(p) = other.unwrap_typed().as_f64() else {
                        return Err(self.slots.degenerate(format!(
                            "{name} member is neither an IfcCartesianPoint reference \
                             nor an IfcParameterValue"
                        )));
                    };
                    if trim.parameter.replace(p).is_some() {
                        return Err(self.slots.degenerate(format!(
                            "{name} holds two parameter values; \
                             IfcTrimmingSelect allows at most one of each kind"
                        )));
                    }
                }
            }
        }

        if trim.is_empty() {
            return Err(self
                .slots
                .degenerate(format!("{name} is empty; SET [1:2] requires a member")));
        }
        Ok(trim)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameter(v: f64) -> Value {
        Value::Typed {
            type_name: "IFCPARAMETERVALUE".into(),
            value: Box::new(Value::Real(v)),
        }
    }

    fn curve(trim1: Value, trim2: Value, sense: bool, preference: &str) -> Entity {
        Entity::new(
            "IFCTRIMMEDCURVE",
            vec![
                Value::Ref(EntityId(50)),
                trim1,
                trim2,
                Value::Bool(sense),
                Value::Enum(preference.into()),
            ],
        )
    }

    #[test]
    fn a_trim_may_carry_a_point_and_a_parameter_at_once() {
        let e = curve(
            Value::List(vec![Value::Ref(EntityId(1)), parameter(0.0)]),
            Value::List(vec![Value::Ref(EntityId(2)), parameter(90.0)]),
            true,
            "PARAMETER",
        );
        let spec = TrimmedCurve::new(EntityId(9), &e).spec().unwrap();
        assert_eq!(spec.trim1.cartesian, Some(EntityId(1)));
        assert_eq!(spec.trim1.parameter, Some(0.0));
        assert!(spec.trim1.is_over_specified());
        assert!(spec.is_parametrically_complete());
    }

    /// The whole point of MasterRepresentation: with both forms present it
    /// decides, and the two forms genuinely disagree in rounded exports.
    #[test]
    fn master_representation_decides_between_the_two_trim_forms() {
        let both = Value::List(vec![Value::Ref(EntityId(1)), parameter(0.25)]);

        let cartesian = curve(both.clone(), both.clone(), true, "CARTESIAN");
        assert_eq!(
            TrimmedCurve::new(EntityId(1), &cartesian)
                .spec()
                .unwrap()
                .start(),
            Some(TrimPoint::Cartesian(EntityId(1)))
        );

        let param = curve(both.clone(), both.clone(), true, "PARAMETER");
        assert_eq!(
            TrimmedCurve::new(EntityId(1), &param)
                .spec()
                .unwrap()
                .start(),
            Some(TrimPoint::Parameter(0.25))
        );
    }

    /// Under UNSPECIFIED the parameter is exact while inverting a point needs
    /// an undefined tolerance, so the parameter is chosen.
    #[test]
    fn unspecified_preference_falls_back_to_the_parameter_form() {
        let both = Value::List(vec![Value::Ref(EntityId(1)), parameter(0.5)]);
        let e = curve(both.clone(), both, true, "UNSPECIFIED");
        assert_eq!(
            TrimmedCurve::new(EntityId(1), &e).spec().unwrap().start(),
            Some(TrimPoint::Parameter(0.5))
        );
    }

    /// A preference for a form the file did not supply must not yield None.
    #[test]
    fn a_preference_for_an_absent_form_falls_back_to_the_present_one() {
        let only_point = Value::List(vec![Value::Ref(EntityId(1))]);
        let e = curve(only_point.clone(), only_point, true, "PARAMETER");
        assert_eq!(
            TrimmedCurve::new(EntityId(1), &e).spec().unwrap().start(),
            Some(TrimPoint::Cartesian(EntityId(1)))
        );
    }

    /// One circle and one pair of trim points describe four distinct arcs.
    /// If any two of these compare equal, the model has collapsed an arc a
    /// file can legitimately express.
    #[test]
    fn four_distinct_arcs_come_from_one_basis_curve_and_one_trim_pair() {
        let a = Value::List(vec![parameter(0.0)]);
        let b = Value::List(vec![parameter(90.0)]);

        let mut specs = Vec::new();
        for (t1, t2) in [(a.clone(), b.clone()), (b.clone(), a.clone())] {
            for sense in [true, false] {
                let e = curve(t1.clone(), t2.clone(), sense, "PARAMETER");
                specs.push(TrimmedCurve::new(EntityId(1), &e).spec().unwrap());
            }
        }

        assert_eq!(specs.len(), 4);
        for i in 0..specs.len() {
            for j in (i + 1)..specs.len() {
                assert_ne!(
                    specs[i], specs[j],
                    "arcs {i} and {j} collapsed into one specification"
                );
            }
        }
    }

    /// Trim1 is the start and Trim2 the end regardless of sense; sense picks
    /// the way round, not the endpoints.
    #[test]
    fn sense_agreement_does_not_swap_the_endpoints() {
        let a = Value::List(vec![parameter(10.0)]);
        let b = Value::List(vec![parameter(350.0)]);
        for sense in [true, false] {
            let e = curve(a.clone(), b.clone(), sense, "PARAMETER");
            let spec = TrimmedCurve::new(EntityId(1), &e).spec().unwrap();
            assert_eq!(spec.start(), Some(TrimPoint::Parameter(10.0)));
            assert_eq!(spec.end(), Some(TrimPoint::Parameter(350.0)));
            assert_eq!(spec.sense_agreement, sense);
        }
    }

    /// Descending parameters are legal on a closed basis curve: informal
    /// proposition IP3 does not apply there, so no reordering may happen.
    #[test]
    fn descending_trim_parameters_are_preserved_not_sorted() {
        let e = curve(
            Value::List(vec![parameter(270.0)]),
            Value::List(vec![parameter(45.0)]),
            true,
            "PARAMETER",
        );
        let spec = TrimmedCurve::new(EntityId(1), &e).spec().unwrap();
        assert_eq!(spec.trim1.parameter, Some(270.0));
        assert_eq!(spec.trim2.parameter, Some(45.0));
    }

    #[test]
    fn an_empty_trim_set_is_rejected() {
        let e = curve(
            Value::List(vec![]),
            Value::List(vec![parameter(1.0)]),
            true,
            "PARAMETER",
        );
        let err = TrimmedCurve::new(EntityId(3), &e).spec().unwrap_err();
        assert!(err.to_string().contains("Trim1"), "got: {err}");
    }

    #[test]
    fn two_parameters_in_one_trim_set_is_rejected() {
        let e = curve(
            Value::List(vec![parameter(1.0), parameter(2.0)]),
            Value::List(vec![parameter(3.0)]),
            true,
            "PARAMETER",
        );
        assert!(TrimmedCurve::new(EntityId(1), &e).spec().is_err());
    }

    /// Some writers omit the set brackets for a single member; every other
    /// viewer reads those files.
    #[test]
    fn a_single_unwrapped_trim_member_is_accepted() {
        let e = curve(parameter(0.0), Value::Ref(EntityId(2)), true, "UNSPECIFIED");
        let spec = TrimmedCurve::new(EntityId(1), &e).spec().unwrap();
        assert_eq!(spec.trim1.parameter, Some(0.0));
        assert_eq!(spec.trim2.cartesian, Some(EntityId(2)));
    }

    #[test]
    fn an_absent_master_representation_reads_as_unspecified() {
        let e = Entity::new(
            "IFCTRIMMEDCURVE",
            vec![
                Value::Ref(EntityId(50)),
                Value::List(vec![parameter(0.0)]),
                Value::List(vec![parameter(1.0)]),
                Value::Bool(true),
            ],
        );
        assert_eq!(
            TrimmedCurve::new(EntityId(1), &e).master_representation(),
            TrimmingPreference::Unspecified
        );
    }

    #[test]
    fn unknown_preference_tokens_are_not_silently_accepted() {
        assert_eq!(TrimmingPreference::from_token("NONSENSE"), None);
        assert_eq!(
            TrimmingPreference::from_token("cartesian"),
            Some(TrimmingPreference::Cartesian)
        );
    }
}
