//! IFC4X3 ADD2 supertype chains, as a delta over the IFC4 tables.
//!
//! Generated from `IFC4X3_ADD2.exp`. IFC4X3 keeps every IFC4 geometry and
//! profile entity, so only two kinds of row are needed here:
//!
//! - [`ADDED`]: entities `IFC4.exp` does not declare, with their full IFC4X3
//!   chain. Their names cannot occur in an IFC4 file, so the unversioned
//!   [`super::subtype::is_a`] may consult them without changing any IFC4
//!   answer.
//! - [`REDECLARED`]: IFC4 entities whose IFC4X3 chain differs. IFC4X3 inserts
//!   an abstract supertype above them (`IfcOffsetCurve`,
//!   `IfcDirectrixCurveSweptAreaSolid`). These rows are consulted only when
//!   the caller names IFC4X3, because answering them for IFC4 would claim an
//!   ancestor IFC4 does not declare.
//!
//! `tests/schema_coverage.rs` checks every row, per release, against the
//! normative `.exp` chain.

/// `(entity, its IFC4X3 supertype chain from immediate parent upward)` for
/// entities IFC4 does not declare: the 19 concrete geometry, profile and
/// placement additions plus the abstract supertypes they introduce.
pub(super) static ADDED: &[(&str, &[&str])] = &[
    (
        "IFCAXIS2PLACEMENTLINEAR",
        &[
            "IFCPLACEMENT",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCLOTHOID",
        &[
            "IFCSPIRAL",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCOSINESPIRAL",
        &[
            "IFCSPIRAL",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCCURVESEGMENT",
        &[
            "IFCSEGMENT",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCDIRECTRIXCURVESWEPTAREASOLID",
        &[
            "IFCSWEPTAREASOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID",
        &[
            "IFCFIXEDREFERENCESWEPTAREASOLID",
            "IFCDIRECTRIXCURVESWEPTAREASOLID",
            "IFCSWEPTAREASOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCGRADIENTCURVE",
        &[
            "IFCCOMPOSITECURVE",
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    ("IFCLINEARPLACEMENT", &["IFCOBJECTPLACEMENT"]),
    (
        "IFCOFFSETCURVE",
        &[
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCOFFSETCURVEBYDISTANCES",
        &[
            "IFCOFFSETCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    ("IFCOPENCROSSPROFILEDEF", &["IFCPROFILEDEF"]),
    (
        "IFCPOINTBYDISTANCEEXPRESSION",
        &[
            "IFCPOINT",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCPOLYNOMIALCURVE",
        &[
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSECONDORDERPOLYNOMIALSPIRAL",
        &[
            "IFCSPIRAL",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSECTIONEDSOLID",
        &[
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSECTIONEDSOLIDHORIZONTAL",
        &[
            "IFCSECTIONEDSOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSECTIONEDSURFACE",
        &[
            "IFCSURFACE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSEGMENT",
        &["IFCGEOMETRICREPRESENTATIONITEM", "IFCREPRESENTATIONITEM"],
    ),
    (
        "IFCSEGMENTEDREFERENCECURVE",
        &[
            "IFCCOMPOSITECURVE",
            "IFCBOUNDEDCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSEVENTHORDERPOLYNOMIALSPIRAL",
        &[
            "IFCSPIRAL",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSINESPIRAL",
        &[
            "IFCSPIRAL",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSPIRAL",
        &[
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCTHIRDORDERPOLYNOMIALSPIRAL",
        &[
            "IFCSPIRAL",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCTRIANGULATEDIRREGULARNETWORK",
        &[
            "IFCTRIANGULATEDFACESET",
            "IFCTESSELLATEDFACESET",
            "IFCTESSELLATEDITEM",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
];

/// IFC4 entities whose IFC4X3 chain gains an abstract supertype, with that
/// IFC4X3 chain. The IFC4 tables keep the IFC4 chain for the same names.
pub(super) static REDECLARED: &[(&str, &[&str])] = &[
    (
        "IFCFIXEDREFERENCESWEPTAREASOLID",
        &[
            "IFCDIRECTRIXCURVESWEPTAREASOLID",
            "IFCSWEPTAREASOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCOFFSETCURVE2D",
        &[
            "IFCOFFSETCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCOFFSETCURVE3D",
        &[
            "IFCOFFSETCURVE",
            "IFCCURVE",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
    (
        "IFCSURFACECURVESWEPTAREASOLID",
        &[
            "IFCDIRECTRIXCURVESWEPTAREASOLID",
            "IFCSWEPTAREASOLID",
            "IFCSOLIDMODEL",
            "IFCGEOMETRICREPRESENTATIONITEM",
            "IFCREPRESENTATIONITEM",
        ],
    ),
];
