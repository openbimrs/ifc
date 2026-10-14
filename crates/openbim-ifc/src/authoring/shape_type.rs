//! The `RepresentationType` of an `IfcShapeRepresentation`, checked against
//! its items or inferred from them.
//!
//! Each release states the rule as the function
//! `IfcShapeRepresentationTypes`, called by `CorrectItemsForType` (IFC2X3
//! `WR24`): a type admits its items when every one counts. The tables
//! below restate it per release family, transcribed from
//! `references/ifc-spec/` (IFC2X3 TC1, IFC4 ADD2 TC1, whose function IFC4X1
//! and IFC4X2 repeat, and IFC4X3 ADD2). A type the function does not list
//! returns `?`, which does not violate the rule, so such a label is
//! written as given.
//!
//! The tables are in inference order: the first type that admits every
//! item is the inferred one, so a more specific type precedes the one
//! that contains it (`SweptSolid` before `AdvancedSweptSolid` before
//! `SolidModel`; `Tessellation` before `SurfaceModel`). `Clipping`, which
//! admits a CSG solid too, is inferred only when an item is a clipping
//! result.
//!
//! `Curve2D` and `Curve3D` count curves by `IfcCurve.Dim`, which the
//! release derives through `IfcCurveDim`; [`Dims`] follows that function
//! (and the `Dim` of points, point lists, placements and composite curve
//! segments it reaches). A curve whose dimension it cannot derive leaves
//! the check undecided.

use ifc_model::{EntityId, Value};
use ifc_schema::{Schema, SchemaVersion};

use super::kinds::Kinds;
use super::overlay::Overlay;

/// How a type counts one item.
#[derive(Debug, Clone, Copy)]
enum Test {
    /// An instance of any of these.
    Any(&'static [&'static str]),
    /// An instance of any of the first and none of the second.
    AnyBut(&'static [&'static str], &'static [&'static str]),
    /// An instance of exactly one of these (`Annotation2D`).
    One(&'static [&'static str]),
    /// An `IfcCurve` of this dimension.
    CurveDim(i64),
    /// An `IfcSurface` of this dimension.
    SurfaceDim(i64),
    /// An `IfcBoundingBox`, alone in the representation.
    Alone(&'static str),
    /// Any of these, except an `IfcGeometricSet` holding a surface
    /// (`GeometricCurveSet`).
    CurveSet(&'static [&'static str]),
}

type Table = &'static [(&'static str, Test)];

const MAPPED: (&str, Test) = ("MappedRepresentation", Test::Any(&["IfcMappedItem"]));
const BOX: (&str, Test) = ("BoundingBox", Test::Alone("IfcBoundingBox"));
const SPINE: (&str, Test) = ("SectionedSpine", Test::Any(&["IfcSectionedSpine"]));
const LIGHT: (&str, Test) = ("LightSource", Test::Any(&["IfcLightSource"]));
const SOLID: (&str, Test) = ("SolidModel", Test::Any(&["IfcSolidModel"]));
const CURVE_SET: (&str, Test) = (
    "GeometricCurveSet",
    Test::CurveSet(&[
        "IfcGeometricCurveSet",
        "IfcGeometricSet",
        "IfcPoint",
        "IfcCurve",
    ]),
);
const SET: (&str, Test) = (
    "GeometricSet",
    Test::Any(&["IfcGeometricSet", "IfcPoint", "IfcCurve", "IfcSurface"]),
);

/// IFC2X3 TC1.
const IFC2X3: Table = &[
    MAPPED,
    BOX,
    SPINE,
    (
        "AdvancedSweptSolid",
        Test::Any(&["IfcSurfaceCurveSweptAreaSolid", "IfcSweptDiskSolid"]),
    ),
    ("SweptSolid", Test::Any(&["IfcSweptAreaSolid"])),
    (
        "Brep",
        Test::Any(&["IfcFacetedBrep", "IfcFacetedBrepWithVoids"]),
    ),
    ("Clipping", Test::Any(&["IfcBooleanClippingResult"])),
    ("CSG", Test::Any(&["IfcBooleanResult"])),
    SOLID,
    (
        "SurfaceModel",
        Test::Any(&[
            "IfcShellBasedSurfaceModel",
            "IfcFaceBasedSurfaceModel",
            "IfcFacetedBrep",
            "IfcFacetedBrepWithVoids",
        ]),
    ),
    ("Curve2D", Test::CurveDim(2)),
    CURVE_SET,
    SET,
    (
        "Annotation2D",
        Test::One(&[
            "IfcPoint",
            "IfcCurve",
            "IfcGeometricCurveSet",
            "IfcAnnotationFillArea",
            "IfcDefinedSymbol",
            "IfcTextLiteral",
            "IfcDraughtingCallout",
        ]),
    ),
];

const SWEPT: (&str, Test) = (
    "SweptSolid",
    Test::AnyBut(
        &["IfcExtrudedAreaSolid", "IfcRevolvedAreaSolid"],
        &["IfcExtrudedAreaSolidTapered", "IfcRevolvedAreaSolidTapered"],
    ),
);
const BREP: (&str, Test) = ("Brep", Test::Any(&["IfcFacetedBrep"]));
const ADVANCED_BREP: (&str, Test) = ("AdvancedBrep", Test::Any(&["IfcManifoldSolidBrep"]));
const CLIPPING: (&str, Test) = (
    "Clipping",
    Test::Any(&["IfcCsgSolid", "IfcBooleanClippingResult"]),
);
const CSG: (&str, Test) = (
    "CSG",
    Test::Any(&["IfcBooleanResult", "IfcCsgPrimitive3D", "IfcCsgSolid"]),
);
const TESSELLATION: (&str, Test) = ("Tessellation", Test::Any(&["IfcTessellatedItem"]));
const SURFACE_MODEL: (&str, Test) = (
    "SurfaceModel",
    Test::Any(&[
        "IfcTessellatedItem",
        "IfcShellBasedSurfaceModel",
        "IfcFaceBasedSurfaceModel",
    ]),
);
const SURFACE_OR_SOLID: (&str, Test) = (
    "SurfaceOrSolidModel",
    Test::Any(&[
        "IfcTessellatedItem",
        "IfcShellBasedSurfaceModel",
        "IfcFaceBasedSurfaceModel",
        "IfcSolidModel",
    ]),
);
const POINT_CLOUD: (&str, Test) = ("PointCloud", Test::Any(&["IfcCartesianPointList3D"]));
const CURVES: [(&str, Test); 3] = [
    ("Curve2D", Test::CurveDim(2)),
    ("Curve3D", Test::CurveDim(3)),
    ("Curve", Test::Any(&["IfcCurve"])),
];
const SURFACES: [(&str, Test); 4] = [
    ("AdvancedSurface", Test::Any(&["IfcBSplineSurface"])),
    ("Surface2D", Test::SurfaceDim(2)),
    ("Surface3D", Test::SurfaceDim(3)),
    ("Surface", Test::Any(&["IfcSurface"])),
];
const FILL: (&str, Test) = ("FillArea", Test::Any(&["IfcAnnotationFillArea"]));
const TEXT: (&str, Test) = ("Text", Test::Any(&["IfcTextLiteral"]));
const ANNOTATION: (&str, Test) = (
    "Annotation2D",
    Test::One(&[
        "IfcPoint",
        "IfcCurve",
        "IfcGeometricCurveSet",
        "IfcAnnotationFillArea",
        "IfcTextLiteral",
    ]),
);

/// IFC4 ADD2 TC1; IFC4X1 and IFC4X2 state the same function.
const IFC4: Table = &[
    MAPPED,
    BOX,
    LIGHT,
    SPINE,
    SWEPT,
    (
        "AdvancedSweptSolid",
        Test::Any(&["IfcSweptAreaSolid", "IfcSweptDiskSolid"]),
    ),
    BREP,
    ADVANCED_BREP,
    CLIPPING,
    CSG,
    SOLID,
    TESSELLATION,
    SURFACE_MODEL,
    SURFACE_OR_SOLID,
    POINT_CLOUD,
    ("Point", Test::Any(&["IfcPoint"])),
    CURVES[0],
    CURVES[1],
    CURVES[2],
    SURFACES[0],
    SURFACES[1],
    SURFACES[2],
    SURFACES[3],
    FILL,
    TEXT,
    CURVE_SET,
    SET,
    ANNOTATION,
];

/// IFC4X3 ADD2: `Point` admits point lists, `AdvancedSweptSolid` the
/// horizontal sectioned solid, and `Segment` and `SectionedSurface` are new.
const IFC4X3: Table = &[
    MAPPED,
    BOX,
    LIGHT,
    SPINE,
    SWEPT,
    (
        "AdvancedSweptSolid",
        Test::Any(&[
            "IfcSweptAreaSolid",
            "IfcSweptDiskSolid",
            "IfcSectionedSolidHorizontal",
        ]),
    ),
    BREP,
    ADVANCED_BREP,
    CLIPPING,
    CSG,
    SOLID,
    TESSELLATION,
    SURFACE_MODEL,
    SURFACE_OR_SOLID,
    POINT_CLOUD,
    ("Point", Test::Any(&["IfcPoint", "IfcCartesianPointList"])),
    CURVES[0],
    CURVES[1],
    CURVES[2],
    ("Segment", Test::Any(&["IfcSegment"])),
    SURFACES[0],
    SURFACES[1],
    SURFACES[2],
    SURFACES[3],
    ("SectionedSurface", Test::Any(&["IfcSectionedSurface"])),
    FILL,
    TEXT,
    CURVE_SET,
    SET,
    ANNOTATION,
];

/// The release's table; `None` for tables of no recognised release.
fn table(schema: &Schema) -> Option<Table> {
    Some(match schema.version()? {
        SchemaVersion::Ifc2x3 => IFC2X3,
        SchemaVersion::Ifc4 | SchemaVersion::Ifc4x1 | SchemaVersion::Ifc4x2 => IFC4,
        SchemaVersion::Ifc4x3 => IFC4X3,
        // A release added later states its own function; until it is
        // transcribed, its labels are written as given.
        _ => return None,
    })
}

/// What checking a type against the items found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Verdict {
    /// Every item counts.
    Admits,
    /// The release does not list the type: its rule does not constrain it.
    Unlisted,
    /// These items do not count; what the type admits.
    Rejects(Vec<EntityId>, String),
    /// This item's dimension cannot be derived.
    Undecided(EntityId),
}

/// The checker over the batch's view.
pub(super) struct ShapeTypes<'a, 'm> {
    schema: &'a Schema,
    view: &'a Overlay<'m>,
    kinds: Kinds<'a>,
}

impl<'a, 'm> ShapeTypes<'a, 'm> {
    pub(super) fn new(schema: &'a Schema, view: &'a Overlay<'m>) -> Self {
        Self {
            schema,
            view,
            kinds: Kinds::new(schema),
        }
    }

    /// Whether `representation_type` admits `items`, by the release's
    /// rule.
    pub(super) fn check(&self, representation_type: &str, items: &[EntityId]) -> Verdict {
        let Some(table) = table(self.schema) else {
            return Verdict::Unlisted;
        };
        // `CASE` compares strings exactly: a label in another case is not
        // the listed type, but it is far likelier a typo than a new type.
        let Some((name, test)) = table
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(representation_type))
        else {
            return Verdict::Unlisted;
        };
        if *name != representation_type {
            return Verdict::Rejects(
                items.to_vec(),
                format!("nothing under that spelling; the release lists '{name}'"),
            );
        }
        let mut rejected = Vec::new();
        for item in items {
            match self.counts(*test, *item, items.len()) {
                Some(true) => {}
                Some(false) => rejected.push(*item),
                None => return Verdict::Undecided(*item),
            }
        }
        if rejected.is_empty() {
            Verdict::Admits
        } else {
            Verdict::Rejects(rejected, describe(*test))
        }
    }

    /// The first type of the release's table that admits every item.
    pub(super) fn infer(&self, items: &[EntityId]) -> Option<&'static str> {
        let table = table(self.schema)?;
        let clipping = items
            .iter()
            .any(|item| self.is(*item, "IfcBooleanClippingResult"));
        table
            .iter()
            .filter(|(name, _)| *name != "Clipping" || clipping)
            .find(|(_, test)| {
                items
                    .iter()
                    .all(|item| self.counts(*test, *item, items.len()) == Some(true))
            })
            .map(|(name, _)| *name)
    }

    fn is(&self, id: EntityId, ancestor: &str) -> bool {
        self.view
            .get(id)
            .is_some_and(|entity| self.schema.is_a(&entity.type_name, ancestor))
    }

    fn any(&self, id: EntityId, types: &[&str]) -> bool {
        types.iter().any(|t| self.is(id, t))
    }

    /// Whether `test` counts `item`; `None` when a dimension is unknown.
    fn counts(&self, test: Test, item: EntityId, count: usize) -> Option<bool> {
        Some(match test {
            Test::Any(types) => self.any(item, types),
            Test::AnyBut(types, but) => self.any(item, types) && !self.any(item, but),
            Test::One(types) => types.iter().filter(|t| self.is(item, t)).count() == 1,
            Test::CurveDim(dim) => {
                if !self.is(item, "IfcCurve") {
                    return Some(false);
                }
                self.dims().of(item)? == dim
            }
            Test::SurfaceDim(dim) => {
                if !self.is(item, "IfcSurface") {
                    return Some(false);
                }
                self.dims().of(item)? == dim
            }
            Test::Alone(kind) => count == 1 && self.is(item, kind),
            Test::CurveSet(types) => {
                self.any(item, types)
                    && !(self.is(item, "IfcGeometricSet") && self.holds_surface(item))
            }
        })
    }

    fn holds_surface(&self, set: EntityId) -> bool {
        self.view.get(set).is_some_and(|entity| {
            self.kinds
                .references(entity, "Elements")
                .into_iter()
                .any(|element| self.is(element, "IfcSurface"))
        })
    }

    fn dims(&self) -> Dims<'_, 'a, 'm> {
        Dims { types: self }
    }
}

/// `IfcCurve.Dim` and the `Dim` of what it derives from, as the releases
/// derive them (`IfcCurveDim`, `IfcPointDim`, `IfcPointListDim`,
/// `IfcSegmentDim` and the `DERIVE` clauses).
struct Dims<'t, 'a, 'm> {
    types: &'t ShapeTypes<'a, 'm>,
}

/// Deeper than any curve nests; a cycle is undecided, not a hang.
const DEPTH: usize = 64;

impl Dims<'_, '_, '_> {
    fn of(&self, id: EntityId) -> Option<i64> {
        self.at(id, 0)
    }

    fn at(&self, id: EntityId, depth: usize) -> Option<i64> {
        if depth > DEPTH {
            return None;
        }
        let t = self.types;
        let entity = t.view.get(id)?;
        let is = |kind: &str| t.schema.is_a(&entity.type_name, kind);
        let follow = |name: &str| -> Option<i64> {
            let next = t.kinds.reference(entity, name)?;
            self.at(next, depth + 1)
        };
        let first = |name: &str| -> Option<i64> {
            let next = *t.kinds.references(entity, name).first()?;
            self.at(next, depth + 1)
        };
        let length = |name: &str| -> Option<i64> {
            let slot = t.kinds.slot(&entity.type_name, name)?;
            let items = entity.attributes.get(slot)?.as_list()?;
            i64::try_from(items.len()).ok()
        };
        let ifc2x3 = t.schema.version() == Some(SchemaVersion::Ifc2x3);
        // Points and what locates them.
        if is("IfcCartesianPoint") {
            return length("Coordinates");
        }
        if is("IfcDirection") {
            return length("DirectionRatios");
        }
        if is("IfcPointOnCurve") || is("IfcPointByDistanceExpression") {
            return follow("BasisCurve");
        }
        if is("IfcPointOnSurface") {
            return follow("BasisSurface");
        }
        if is("IfcCartesianPointList2D") {
            return Some(2);
        }
        if is("IfcCartesianPointList3D") {
            return Some(3);
        }
        if is("IfcPlacement") {
            return follow("Location");
        }
        if is("IfcVector") {
            return follow("Orientation");
        }
        if is("IfcCartesianTransformationOperator") {
            return follow("LocalOrigin");
        }
        if is("IfcCompositeCurveSegment") || is("IfcCurveSegment") {
            return follow("ParentCurve");
        }
        if is("IfcSurface") {
            if !ifc2x3 {
                return Some(3);
            }
            if is("IfcElementarySurface") || is("IfcSweptSurface") {
                return follow("Position");
            }
            if is("IfcCurveBoundedPlane") || is("IfcRectangularTrimmedSurface") {
                return follow("BasisSurface");
            }
            return None;
        }
        if !is("IfcCurve") {
            return None;
        }
        // `IfcCurveDim`, in the release's order.
        if is("IfcLine") {
            return follow("Pnt");
        }
        if is("IfcConic") {
            return follow("Position");
        }
        if is("IfcPolyline") {
            return first("Points");
        }
        if is("IfcTrimmedCurve") {
            return follow("BasisCurve");
        }
        if is("IfcGradientCurve") || is("IfcSegmentedReferenceCurve") {
            return Some(3);
        }
        if is("IfcCompositeCurve") {
            return first("Segments");
        }
        if is("IfcBSplineCurve") {
            return first("ControlPointsList");
        }
        if is("IfcOffsetCurve2D") {
            return Some(2);
        }
        if is("IfcOffsetCurve3D") || is("IfcOffsetCurveByDistances") {
            return Some(3);
        }
        if is("IfcPolynomialCurve") {
            let no_z = t
                .kinds
                .slot(&entity.type_name, "CoefficientsZ")
                .and_then(|slot| entity.attributes.get(slot))
                .is_none_or(|value| matches!(value, Value::Null));
            return Some(if no_z && follow("Position")? == 2 {
                2
            } else {
                3
            });
        }
        if is("IfcPcurve") {
            return Some(3);
        }
        if is("IfcIndexedPolyCurve") {
            return follow("Points");
        }
        if is("IfcSpiral") {
            return follow("Position");
        }
        None
    }
}

fn describe(test: Test) -> String {
    let list = |types: &[&str]| types.join(", ");
    match test {
        Test::Any(types) => list(types),
        Test::AnyBut(types, but) => format!("{} but not {}", list(types), list(but)),
        Test::One(types) => format!("exactly one of {}", list(types)),
        Test::CurveDim(dim) => format!("IfcCurve of dimension {dim}"),
        Test::SurfaceDim(dim) => format!("IfcSurface of dimension {dim}"),
        Test::Alone(kind) => format!("one {kind}, alone"),
        Test::CurveSet(types) => format!("{}, without surfaces", list(types)),
    }
}
