//! The declared release, and which of its `WHERE` rules govern an entity.
//!
//! # One lookup, used by every rule
//!
//! AGENTS.md: "Validation uses the declared release's own tables". The rules
//! differ between releases in three ways, and each is answered here once:
//!
//! - **Which rules exist.** IFC2X3 TC1 declares no `FirstOperandClosed`, no
//!   `MagnitudeGreaterZero`, no `ApplicableMappedRepr`; IFC4X3 ADD2 adds
//!   `LocationIsCP` and moves `DirectrixBounded` up to
//!   `IfcDirectrixCurveSweptAreaSolid`. [`DECLARED`] lists every rule on the
//!   inventoried geometry entities with the releases that declare it, and a
//!   test checks it against each bundled release's own table.
//! - **What a rule is called.** IFC2X3 TC1 numbers its rules (`WR1`,
//!   `WR31`, ...) where IFC4 names them; a violation carries the declared
//!   release's own name ([`Declared::name_in`]).
//! - **Which entities a rule governs.** A rule declared on an entity binds
//!   every subtype, so applicability and every `'X' IN TYPEOF(...)` test
//!   are subtype tests in the release's own entity table
//!   ([`Release::is_a`]), never comparisons of names.
//!
//! Where the *text* of a rule differs between releases (`BoundaryType`,
//! `FirstOperandType`, `AxisStartInXY`), the rule itself branches on
//! [`Release::version`] and cites each release's text beside the branch.
//!
//! The table also lists the rules of entities IFC4 does not have: IFC2X3
//! TC1's `Ifc2DCompositeCurve` and `IfcRationalBezierCurve`, and the
//! linear-referencing geometry of IFC4X1 on (`rules/linear.rs`).
//!
//! The release is the header's `FILE_SCHEMA`. A file that declares none, or
//! one this crate does not know, is read as IFC4 ADD2 TC1, the crate's
//! baseline.

use ifc_model::{Entity, EntityId, Model};
use ifc_schema::{Schema, SchemaVersion};

use super::violation::{RuleViolation, ViolationKind};

/// The release a model declares, with its bundled entity table.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Release {
    version: SchemaVersion,
    /// `None` in a build that leaves this release's table out.
    schema: Option<&'static Schema>,
}

impl Release {
    /// The release `model` declares in `FILE_SCHEMA`, or IFC4 when it
    /// declares none this crate knows.
    pub(crate) fn of(model: &Model) -> Self {
        let version = model
            .header()
            .schema_token()
            .and_then(SchemaVersion::from_header_token)
            .unwrap_or(SchemaVersion::Ifc4);
        Self::new(version)
    }

    /// `version`, with its entity table when this build bundles it.
    pub(crate) fn new(version: SchemaVersion) -> Self {
        Self {
            version,
            schema: ifc_schema::for_version(version).ok(),
        }
    }

    pub(crate) fn version(&self) -> SchemaVersion {
        self.version
    }

    /// EXPRESS `'ANCESTOR' IN TYPEOF(x)` for an instance of `entity`.
    ///
    /// `TYPEOF` yields the instance's type and every supertype, so this is a
    /// subtype test, answered from the release's own entity table. A build
    /// that leaves the release's table out falls back to the compiled
    /// geometry chains ([`crate::select::is_a_in`]), which know the
    /// geometry families and answer an exact name for any other.
    pub(crate) fn is_a(&self, entity: &str, ancestor: &str) -> bool {
        match self.schema {
            Some(schema) => schema.is_a(entity, ancestor),
            None => crate::select::is_a_in(self.version, entity, ancestor),
        }
    }

    /// `'ANCESTOR' IN TYPEOF(#target)`; false when `target` does not resolve.
    pub(crate) fn ref_is_a(&self, model: &Model, target: EntityId, ancestor: &str) -> bool {
        model
            .get(target)
            .is_some_and(|e| self.is_a(&e.type_name, ancestor))
    }

    /// Can [`Self::is_a`] place `entity` in the hierarchy at all?
    ///
    /// Always, with the release's own table: a name it does not declare is
    /// in no `TYPEOF` set but its own, as EXPRESS reads it. Without the
    /// table, only for the families the compiled geometry chains carry; a
    /// rule that demands a type must not report an entity it cannot
    /// classify (an `IfcShapeRepresentation`, say), or a reduced build
    /// would flag conforming files.
    fn classifies(&self, entity: &str) -> bool {
        self.schema.is_some() || !crate::select::supertypes_of_in(self.version, entity).is_empty()
    }

    /// Every rule of this release that governs an instance of `entity`.
    pub(crate) fn governing(&self, entity: &str) -> Vec<&'static Declared> {
        DECLARED
            .iter()
            .filter(|d| d.releases.contains(&self.version) && self.is_a(entity, d.entity))
            .collect()
    }
}

/// One `WHERE` rule as the bundled releases declare it.
#[derive(Debug)]
pub(crate) struct Declared {
    /// The entity that declares the rule, upper case.
    pub(crate) entity: &'static str,
    /// The rule's label from IFC4 ADD2 TC1 on, or its IFC2X3 TC1 label for
    /// a rule only IFC2X3 declares. Rules look themselves up by it.
    pub(crate) label: &'static str,
    /// The label IFC2X3 TC1 gives the same rule, when it declares one.
    pub(crate) ifc2x3: Option<&'static str>,
    /// The releases that declare the rule on [`Self::entity`].
    pub(crate) releases: &'static [SchemaVersion],
}

impl Declared {
    /// The rule's name in `version`, as that release's schema spells it.
    pub(crate) fn name_in(&self, version: SchemaVersion) -> &'static str {
        match (version, self.ifc2x3) {
            (SchemaVersion::Ifc2x3, Some(name)) => name,
            _ => self.label,
        }
    }
}

use SchemaVersion::{Ifc2x3, Ifc4, Ifc4x1, Ifc4x2, Ifc4x3};

/// Every bundled release.
const ALL: &[SchemaVersion] = &[Ifc2x3, Ifc4, Ifc4x1, Ifc4x2, Ifc4x3];
/// IFC4 ADD2 TC1 and every later bundled release, not IFC2X3 TC1.
const IFC4_ON: &[SchemaVersion] = &[Ifc4, Ifc4x1, Ifc4x2, Ifc4x3];
/// IFC4 ADD2 TC1, IFC4X1 and IFC4X2.
const IFC4_TO_IFC4X2: &[SchemaVersion] = &[Ifc4, Ifc4x1, Ifc4x2];
/// IFC4X1 and every later bundled release.
const IFC4X1_ON: &[SchemaVersion] = &[Ifc4x1, Ifc4x2, Ifc4x3];
const IFC4X3_ONLY: &[SchemaVersion] = &[Ifc4x3];
const IFC2X3_ONLY: &[SchemaVersion] = &[Ifc2x3];

/// A rule only IFC2X3 TC1 declares, under its label `wr`.
const fn ifc2x3_only(entity: &'static str, wr: &'static str) -> Declared {
    Declared {
        entity,
        label: wr,
        ifc2x3: Some(wr),
        releases: IFC2X3_ONLY,
    }
}

/// A rule every bundled release declares, under `wr` in IFC2X3 TC1.
const fn all(entity: &'static str, label: &'static str, wr: &'static str) -> Declared {
    Declared {
        entity,
        label,
        ifc2x3: Some(wr),
        releases: ALL,
    }
}

/// A rule declared only in `releases`, none of them IFC2X3 TC1.
const fn only(
    entity: &'static str,
    label: &'static str,
    releases: &'static [SchemaVersion],
) -> Declared {
    Declared {
        entity,
        label,
        ifc2x3: None,
        releases,
    }
}

/// The rules on the inventoried geometry entities, and on the geometry
/// entities IFC4 does not declare, per release.
///
/// Read from `references/ifc-spec/` (IFC2X3_TC1.exp, IFC4.exp, IFC4x1.exp,
/// IFC4x2.exp, IFC4X3_ADD2.exp) and checked against each bundled release's
/// own table by `the_declared_rules_match_every_bundled_release` below, so a
/// row cannot claim a rule, a release or an IFC2X3 label the schema does
/// not declare. Labels only: the expressions are CC BY-ND schema text.
pub(crate) static DECLARED: &[Declared] = &[
    ifc2x3_only("IFC2DCOMPOSITECURVE", "WR1"),
    ifc2x3_only("IFC2DCOMPOSITECURVE", "WR2"),
    only("IFCADVANCEDBREP", "HasAdvancedFaces", IFC4_ON),
    only(
        "IFCADVANCEDBREPWITHVOIDS",
        "VoidsHaveAdvancedFaces",
        IFC4_ON,
    ),
    all("IFCAXIS1PLACEMENT", "AxisIs3D", "WR1"),
    all("IFCAXIS1PLACEMENT", "LocationIs3D", "WR2"),
    only("IFCAXIS1PLACEMENT", "LocationIsCP", IFC4X3_ONLY),
    all("IFCAXIS2PLACEMENT2D", "RefDirIs2D", "WR1"),
    all("IFCAXIS2PLACEMENT2D", "LocationIs2D", "WR2"),
    only("IFCAXIS2PLACEMENT2D", "LocationIsCP", IFC4X3_ONLY),
    only("IFCAXIS2PLACEMENTLINEAR", "WR1", IFC4X3_ONLY),
    only("IFCAXIS2PLACEMENTLINEAR", "WR2", IFC4X3_ONLY),
    all("IFCAXIS2PLACEMENT3D", "LocationIs3D", "WR1"),
    all("IFCAXIS2PLACEMENT3D", "AxisIs3D", "WR2"),
    all("IFCAXIS2PLACEMENT3D", "RefDirIs3D", "WR3"),
    all("IFCAXIS2PLACEMENT3D", "AxisToRefDirPosition", "WR4"),
    all("IFCAXIS2PLACEMENT3D", "AxisAndRefDirProvision", "WR5"),
    only("IFCAXIS2PLACEMENT3D", "LocationIsCP", IFC4X3_ONLY),
    all("IFCBOOLEANCLIPPINGRESULT", "FirstOperandType", "WR1"),
    all("IFCBOOLEANCLIPPINGRESULT", "SecondOperandType", "WR2"),
    all("IFCBOOLEANCLIPPINGRESULT", "OperatorType", "WR3"),
    all("IFCBOOLEANRESULT", "SameDim", "WR1"),
    only("IFCBOOLEANRESULT", "FirstOperandClosed", IFC4_ON),
    only("IFCBOOLEANRESULT", "SecondOperandClosed", IFC4_ON),
    only("IFCBOUNDARYCURVE", "IsClosed", IFC4_ON),
    all("IFCBOXEDHALFSPACE", "UnboundedSurface", "WR1"),
    all("IFCBSPLINECURVE", "SameDim", "WR41"),
    only("IFCBSPLINECURVEWITHKNOTS", "ConsistentBSpline", IFC4_ON),
    only(
        "IFCBSPLINECURVEWITHKNOTS",
        "CorrespondingKnotLists",
        IFC4_ON,
    ),
    only(
        "IFCBSPLINESURFACEWITHKNOTS",
        "UDirectionConstraints",
        IFC4_ON,
    ),
    only(
        "IFCBSPLINESURFACEWITHKNOTS",
        "VDirectionConstraints",
        IFC4_ON,
    ),
    only("IFCBSPLINESURFACEWITHKNOTS", "CorrespondingULists", IFC4_ON),
    only("IFCBSPLINESURFACEWITHKNOTS", "CorrespondingVLists", IFC4_ON),
    all("IFCCARTESIANPOINT", "CP2Dor3D", "WR1"),
    all(
        "IFCCARTESIANTRANSFORMATIONOPERATOR",
        "ScaleGreaterZero",
        "WR1",
    ),
    all("IFCCARTESIANTRANSFORMATIONOPERATOR2D", "DimEqual2", "WR1"),
    all("IFCCARTESIANTRANSFORMATIONOPERATOR2D", "Axis1Is2D", "WR2"),
    all("IFCCARTESIANTRANSFORMATIONOPERATOR2D", "Axis2Is2D", "WR3"),
    all(
        "IFCCARTESIANTRANSFORMATIONOPERATOR2DNONUNIFORM",
        "Scale2GreaterZero",
        "WR1",
    ),
    all("IFCCARTESIANTRANSFORMATIONOPERATOR3D", "DimIs3D", "WR1"),
    all("IFCCARTESIANTRANSFORMATIONOPERATOR3D", "Axis1Is3D", "WR2"),
    all("IFCCARTESIANTRANSFORMATIONOPERATOR3D", "Axis2Is3D", "WR3"),
    all("IFCCARTESIANTRANSFORMATIONOPERATOR3D", "Axis3Is3D", "WR4"),
    all(
        "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM",
        "Scale2GreaterZero",
        "WR1",
    ),
    all(
        "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM",
        "Scale3GreaterZero",
        "WR2",
    ),
    all("IFCCOMPOSITECURVE", "CurveContinuous", "WR41"),
    all("IFCCOMPOSITECURVE", "SameDim", "WR42"),
    only("IFCCOMPOSITECURVEONSURFACE", "SameSurface", IFC4_ON),
    all("IFCCOMPOSITECURVESEGMENT", "ParentIsBoundedCurve", "WR1"),
    only("IFCDIRECTION", "MagnitudeGreaterZero", IFC4_ON),
    only(
        "IFCDIRECTRIXCURVESWEPTAREASOLID",
        "DirectrixBounded",
        IFC4X3_ONLY,
    ),
    all("IFCEXTRUDEDAREASOLID", "ValidExtrusionDirection", "WR31"),
    only(
        "IFCEXTRUDEDAREASOLIDTAPERED",
        "CorrectProfileAssignment",
        IFC4_ON,
    ),
    only(
        "IFCFIXEDREFERENCESWEPTAREASOLID",
        "DirectrixBounded",
        IFC4_TO_IFC4X2,
    ),
    all("IFCGEOMETRICCURVESET", "NoSurfaces", "WR1"),
    all("IFCGEOMETRICSET", "ConsistentDim", "WR21"),
    all("IFCGRIDAXIS", "WR1", "WR1"),
    all("IFCGRIDAXIS", "WR2", "WR2"),
    only("IFCINDEXEDPOLYCURVE", "Consecutive", IFC4_ON),
    only("IFCINTERSECTIONCURVE", "TwoPCurves", IFC4_ON),
    only("IFCINTERSECTIONCURVE", "DistinctSurfaces", IFC4_ON),
    all("IFCLINE", "SameDim", "WR1"),
    all("IFCLOCALPLACEMENT", "WR21", "WR21"),
    all("IFCOFFSETCURVE2D", "DimIs2D", "WR1"),
    all("IFCOFFSETCURVE3D", "DimIs2D", "WR1"),
    only("IFCPCURVE", "DimIs2D", IFC4_ON),
    all("IFCPOLYGONALBOUNDEDHALFSPACE", "BoundaryDim", "WR41"),
    all("IFCPOLYGONALBOUNDEDHALFSPACE", "BoundaryType", "WR42"),
    all("IFCPOLYLINE", "SameDim", "WR41"),
    only("IFCPOLYNOMIALCURVE", "CorrectPositionDim", IFC4X3_ONLY),
    only("IFCPOLYNOMIALCURVE", "ValidCoefficients", IFC4X3_ONLY),
    ifc2x3_only("IFCRATIONALBEZIERCURVE", "WR1"),
    ifc2x3_only("IFCRATIONALBEZIERCURVE", "WR2"),
    only(
        "IFCRATIONALBSPLINECURVEWITHKNOTS",
        "SameNumOfWeightsAndPoints",
        IFC4_ON,
    ),
    only(
        "IFCRATIONALBSPLINECURVEWITHKNOTS",
        "WeightsGreaterZero",
        IFC4_ON,
    ),
    only(
        "IFCRATIONALBSPLINESURFACEWITHKNOTS",
        "CorrespondingWeightsDataLists",
        IFC4_ON,
    ),
    only(
        "IFCRATIONALBSPLINESURFACEWITHKNOTS",
        "WeightValuesGreaterZero",
        IFC4_ON,
    ),
    all("IFCRECTANGULARTRIMMEDSURFACE", "U1AndU2Different", "WR1"),
    all("IFCRECTANGULARTRIMMEDSURFACE", "V1AndV2Different", "WR2"),
    all("IFCRECTANGULARTRIMMEDSURFACE", "UsenseCompatible", "WR3"),
    all("IFCRECTANGULARTRIMMEDSURFACE", "VsenseCompatible", "WR4"),
    only(
        "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT",
        "PositiveLengthParameter",
        IFC4_ON,
    ),
    only("IFCREPRESENTATIONMAP", "ApplicableMappedRepr", IFC4_ON),
    all("IFCREVOLVEDAREASOLID", "AxisStartInXY", "WR31"),
    all("IFCREVOLVEDAREASOLID", "AxisDirectionInXY", "WR32"),
    only(
        "IFCREVOLVEDAREASOLIDTAPERED",
        "CorrectProfileAssignment",
        IFC4_ON,
    ),
    only("IFCSEAMCURVE", "TwoPCurves", IFC4_ON),
    only("IFCSEAMCURVE", "SameSurface", IFC4_ON),
    only("IFCSECTIONEDSOLID", "ConsistentProfileTypes", IFC4X1_ON),
    only("IFCSECTIONEDSOLID", "DirectrixIs3D", IFC4X1_ON),
    only("IFCSECTIONEDSOLID", "SectionsSameType", IFC4X1_ON),
    only(
        "IFCSECTIONEDSOLIDHORIZONTAL",
        "CorrespondingSectionPositions",
        IFC4X1_ON,
    ),
    only(
        "IFCSECTIONEDSOLIDHORIZONTAL",
        "NoLongitudinalOffsets",
        IFC4X1_ON,
    ),
    all("IFCSECTIONEDSPINE", "CorrespondingSectionPositions", "WR1"),
    all("IFCSECTIONEDSPINE", "ConsistentProfileTypes", "WR2"),
    all("IFCSECTIONEDSPINE", "SpineCurveDim", "WR3"),
    only("IFCSECTIONEDSURFACE", "AreaProfileTypes", IFC4X3_ONLY),
    only(
        "IFCSECTIONEDSURFACE",
        "CorrespondingSectionPositions",
        IFC4X3_ONLY,
    ),
    only("IFCSECTIONEDSURFACE", "DirectrixIs3D", IFC4X3_ONLY),
    only("IFCSECTIONEDSURFACE", "NoOffsets", IFC4X3_ONLY),
    only("IFCSECTIONEDSURFACE", "SectionsSameType", IFC4X3_ONLY),
    only("IFCSURFACECURVE", "CurveIs3D", IFC4_ON),
    only("IFCSURFACECURVE", "CurveIsNotPcurve", IFC4_ON),
    only(
        "IFCSURFACECURVESWEPTAREASOLID",
        "DirectrixBounded",
        IFC4_TO_IFC4X2,
    ),
    all("IFCSURFACEOFLINEAREXTRUSION", "DepthGreaterZero", "WR41"),
    all("IFCSWEPTAREASOLID", "SweptAreaType", "WR22"),
    all("IFCSWEPTDISKSOLID", "DirectrixDim", "WR1"),
    all("IFCSWEPTDISKSOLID", "InnerRadiusSize", "WR2"),
    only("IFCSWEPTDISKSOLID", "DirectrixBounded", IFC4_ON),
    only("IFCSWEPTDISKSOLIDPOLYGONAL", "CorrectRadii", IFC4_ON),
    only("IFCSWEPTDISKSOLIDPOLYGONAL", "DirectrixIsPolyline", IFC4_ON),
    // IFC2X3 TC1 alone: `WR1 : NOT('IFC2X3.IFCDERIVEDPROFILEDEF' IN
    // TYPEOF(SweptCurve))`. IFC4 dropped it.
    Declared {
        entity: "IFCSWEPTSURFACE",
        label: "WR1",
        ifc2x3: Some("WR1"),
        releases: IFC2X3_ONLY,
    },
    all("IFCSWEPTSURFACE", "SweptCurveType", "WR2"),
    only("IFCTOROIDALSURFACE", "MajorLargerMinor", IFC4_ON),
    only("IFCTRIANGULATEDIRREGULARNETWORK", "NotClosed", IFC4X1_ON),
    all("IFCTRIMMEDCURVE", "Trim1ValuesConsistent", "WR41"),
    all("IFCTRIMMEDCURVE", "Trim2ValuesConsistent", "WR42"),
    all("IFCTRIMMEDCURVE", "NoTrimOfBoundedCurves", "WR43"),
    all("IFCVECTOR", "MagGreaterOrEqualZero", "WR1"),
];

/// The entity a rule is being evaluated against, in its declared release.
#[derive(Clone, Copy)]
pub(crate) struct Subject<'a> {
    pub(crate) model: &'a Model,
    pub(crate) release: &'a Release,
    pub(crate) id: EntityId,
    pub(crate) entity: &'a Entity,
    /// The entity's type, upper case.
    pub(crate) name: &'a str,
    governing: &'a [&'static Declared],
}

impl<'a> Subject<'a> {
    /// `governing` is [`Release::governing`] for `name`.
    pub(crate) fn new(
        model: &'a Model,
        release: &'a Release,
        id: EntityId,
        entity: &'a Entity,
        name: &'a str,
        governing: &'a [&'static Declared],
    ) -> Self {
        Self {
            model,
            release,
            id,
            entity,
            name,
            governing,
        }
    }

    /// Does any rule of the release govern this entity?
    pub(crate) fn is_governed(&self) -> bool {
        !self.governing.is_empty()
    }

    /// The release's own name for the rule `label` declared on
    /// `declared_on`, when the release declares it and this entity is that
    /// entity or a subtype of it; `None` when the rule does not apply.
    pub(crate) fn rule(&self, declared_on: &str, label: &str) -> Option<&'static str> {
        self.governing
            .iter()
            .find(|d| d.entity == declared_on && d.label == label)
            .map(|d| d.name_in(self.release.version))
    }

    /// `'ANCESTOR' IN TYPEOF(SELF)`.
    pub(crate) fn is_a(&self, ancestor: &str) -> bool {
        self.release.is_a(self.name, ancestor)
    }

    /// `'ANCESTOR' IN TYPEOF(#target)`.
    pub(crate) fn ref_is_a(&self, target: EntityId, ancestor: &str) -> bool {
        self.release.ref_is_a(self.model, target, ancestor)
    }

    /// Does a rule demanding `#target` be one of `kinds` fail?
    ///
    /// True when `target` resolves, can be classified in the release, and
    /// `TYPEOF(#target)` holds none of `kinds`. An unresolved reference is
    /// not evidence of a wrong type.
    pub(crate) fn ref_is_none_of(&self, target: EntityId, kinds: &[&str]) -> bool {
        self.model.get(target).is_some_and(|e| {
            self.release.classifies(&e.type_name)
                && !kinds
                    .iter()
                    .any(|kind| self.release.is_a(&e.type_name, kind))
        })
    }

    /// A violation of `rule` by this entity.
    pub(crate) fn violation(
        &self,
        rule: &'static str,
        kind: ViolationKind,
        detail: impl Into<String>,
    ) -> RuleViolation {
        RuleViolation::new(self.id, self.name, rule, kind, detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const INVENTORY: &str = include_str!("../../data/ifc4-where-rules.tsv");

    /// `DECLARED` is each bundled release's own rule list, exactly.
    ///
    /// For every inventoried entity and every entity a row names, in every
    /// release: the labels the release's table declares on that entity are
    /// exactly the rows' names in that release. A rule a release adds,
    /// drops or renames fails here rather than being checked with another
    /// release's text.
    #[test]
    fn the_declared_rules_match_every_bundled_release() {
        let mut scope: BTreeSet<String> = INVENTORY
            .lines()
            .skip(1)
            .filter_map(|line| line.split('\t').next())
            .filter(|entity| !entity.is_empty())
            .map(str::to_ascii_uppercase)
            .collect();
        scope.extend(DECLARED.iter().map(|d| d.entity.to_owned()));
        for version in SchemaVersion::ALL {
            let schema = ifc_schema::for_version(version).expect("default build bundles all");
            for entity in &scope {
                let declared: BTreeSet<&str> = schema
                    .entity(entity)
                    .map(|def| def.where_rules.iter().map(|r| r.label.as_str()).collect())
                    .unwrap_or_default();
                let listed: BTreeSet<&str> = DECLARED
                    .iter()
                    .filter(|d| d.entity == entity && d.releases.contains(&version))
                    .map(|d| d.name_in(version))
                    .collect();
                assert_eq!(declared, listed, "{} {entity}", version.release_id());
            }
        }
    }

    #[test]
    fn rows_are_unique_and_upper_case() {
        let mut seen = BTreeSet::new();
        for d in DECLARED {
            assert_eq!(d.entity, d.entity.to_ascii_uppercase());
            assert!(seen.insert((d.entity, d.label)), "{d:?} twice");
        }
    }

    /// The five release schemas under this repository's
    /// `references/ifc-spec` (or the superproject's, when it has all five),
    /// or `None` -- a failure when `IFC_SPEC_REQUIRED` is set.
    fn spec_root(files: &[(SchemaVersion, &str)]) -> Option<std::path::PathBuf> {
        let crate_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let found = [
            "../../references/ifc-spec",
            "../../../../references/ifc-spec",
        ]
        .into_iter()
        .map(|rel| crate_dir.join(rel))
        .find(|root| files.iter().all(|(_, rel)| root.join(rel).is_file()));
        assert!(
            found.is_some() || std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec lacks a release; \
             run scripts/fetch-ifc-schemas.sh"
        );
        found
    }

    /// The rules whose text a release changes, which the rule code branches
    /// on and cites, rather than reading that release as IFC4's text.
    const BRANCHED: &[(&str, &str, SchemaVersion)] = &[
        // IFC2X3 WR1 lacks the swept-disk disjunct.
        ("IFCBOOLEANCLIPPINGRESULT", "FirstOperandType", Ifc2x3),
        // IFC4X3 admits IfcIndexedPolyCurve.
        ("IFCPOLYGONALBOUNDEDHALFSPACE", "BoundaryType", Ifc4x3),
        // IFC4X3 adds the IfcCartesianPoint clause.
        ("IFCREVOLVEDAREASOLID", "AxisStartInXY", Ifc4x3),
        // IFC4X3 guards with NOT(EXISTS(Segments)) for SIZEOF(Segments) = 0;
        // equivalent on every writable list (rules/typing.rs).
        ("IFCINDEXEDPOLYCURVE", "Consecutive", Ifc4x3),
        // IFC4X3 qualifies Directrix\IfcIndexedPolyCurve.Segments; the same
        // attribute.
        ("IFCSWEPTDISKSOLIDPOLYGONAL", "DirectrixIsPolyline", Ifc4x3),
        // IFC4X1 and IFC4X2 read temp.OffsetLongitudinal on an
        // IfcDistanceExpression, IFC4X3 temp.Location.OffsetLongitudinal on
        // a linear placement (rules/linear.rs).
        (
            "IFCSECTIONEDSOLIDHORIZONTAL",
            "NoLongitudinalOffsets",
            Ifc4x3,
        ),
    ];

    /// Every rule's text in every release that declares it is IFC4 ADD2
    /// TC1's, except where [`BRANCHED`] says the code reads it otherwise.
    ///
    /// Read from the normative EXPRESS under `references/ifc-spec/`, schema
    /// prefixes and whitespace removed. This is what lets the rule modules
    /// apply one text to every release: a repin that changes a rule's text
    /// fails here, and an entry in `BRANCHED` that no longer differs fails
    /// too.
    #[test]
    fn each_release_states_the_text_the_rules_read() {
        let files = [
            (Ifc2x3, "ifc2x3-tc1/IFC2X3_TC1.exp"),
            (Ifc4, "ifc4-add2-tc1/IFC4.exp"),
            (Ifc4x1, "ifc4x1-final/IFC4x1.exp"),
            (Ifc4x2, "ifc4x2-final/IFC4x2.exp"),
            (Ifc4x3, "ifc4x3-add2/IFC4X3_ADD2.exp"),
        ];
        let Some(root) = spec_root(&files) else {
            return;
        };
        let schemas: Vec<(SchemaVersion, Schema)> = files
            .into_iter()
            .map(|(version, rel)| {
                let bytes = std::fs::read(root.join(rel)).expect("read the schema");
                (version, Schema::from_express_bytes(&bytes))
            })
            .collect();
        let text = |version: SchemaVersion, entity: &str, label: &str| -> Option<String> {
            let schema = &schemas.iter().find(|(v, _)| *v == version)?.1;
            let rule = schema
                .entity(entity)?
                .where_rules
                .iter()
                .find(|r| r.label == label)?;
            let mut expression: String = rule
                .expression
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            for prefix in [
                "'IFC2X3.",
                "'IFC4X3_ADD2.",
                "'IFC4X2.",
                "'IFC4X1.",
                "'IFC4.",
            ] {
                expression = expression.replace(prefix, "'");
            }
            Some(expression.to_ascii_uppercase())
        };
        for d in DECLARED {
            // IFC4X3 declares DirectrixBounded on the supertype it introduced;
            // its text is IFC4's on the entities it replaces.
            let (ref_entity, ref_label) = match (d.entity, d.label) {
                ("IFCDIRECTRIXCURVESWEPTAREASOLID", label) => {
                    ("IFCFIXEDREFERENCESWEPTAREASOLID", label)
                }
                key => key,
            };
            let reference = text(Ifc4, ref_entity, ref_label).or_else(|| {
                let first = d.releases[0];
                text(first, d.entity, d.name_in(first))
            });
            let reference = reference.unwrap_or_else(|| panic!("{d:?}: no text"));
            for &version in d.releases {
                let found = text(version, d.entity, d.name_in(version))
                    .unwrap_or_else(|| panic!("{d:?} undeclared in {version:?}"));
                if BRANCHED.contains(&(d.entity, d.label, version)) {
                    assert_ne!(found, reference, "{d:?} {version:?} no longer differs");
                } else {
                    assert_eq!(found, reference, "{d:?} {version:?}");
                }
            }
        }
    }

    /// A rule declared on a supertype governs its subtypes, in the
    /// release's own hierarchy.
    #[test]
    fn governing_rules_follow_the_release_hierarchy() {
        let ifc4 = Release::new(Ifc4);
        let ifc4x3 = Release::new(Ifc4x3);
        let names = |r: &Release, e: &str| -> Vec<(&str, &str)> {
            r.governing(e)
                .iter()
                .map(|d| (d.entity, d.name_in(r.version())))
                .collect()
        };
        let bounded = ("IFCDIRECTRIXCURVESWEPTAREASOLID", "DirectrixBounded");
        assert!(names(&ifc4x3, "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID").contains(&bounded));
        assert!(!names(&ifc4, "IFCFIXEDREFERENCESWEPTAREASOLID").contains(&bounded));
        assert!(names(&Release::new(Ifc2x3), "IFCAXIS2PLACEMENT3D")
            .contains(&("IFCAXIS2PLACEMENT3D", "WR4")));
        assert!(Release::new(Ifc2x3).governing("IFCADVANCEDBREP").is_empty());
    }
}
