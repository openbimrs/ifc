//! Supertype chains for `IfcProfileResource`, generated from `IFC4.exp`.
//!
//! Split out of `subtype.rs` to keep each generated table under the
//! repository's file-size rule. The profile family is its own schema in
//! IFC4 and was absent from this crate entirely until the coverage gate
//! was extended to enumerate it.

/// `(entity, its supertype chain from immediate parent upward)`.
pub(super) static PROFILE_SUPERTYPES: &[(&str, &[&str])] = &[
    ("IFCARBITRARYCLOSEDPROFILEDEF", &["IFCPROFILEDEF"]),
    ("IFCARBITRARYOPENPROFILEDEF", &["IFCPROFILEDEF"]),
    (
        "IFCARBITRARYPROFILEDEFWITHVOIDS",
        &["IFCARBITRARYCLOSEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    (
        "IFCASYMMETRICISHAPEPROFILEDEF",
        &["IFCPARAMETERIZEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    (
        "IFCCENTERLINEPROFILEDEF",
        &["IFCARBITRARYOPENPROFILEDEF", "IFCPROFILEDEF"],
    ),
    (
        "IFCCIRCLEHOLLOWPROFILEDEF",
        &[
            "IFCCIRCLEPROFILEDEF",
            "IFCPARAMETERIZEDPROFILEDEF",
            "IFCPROFILEDEF",
        ],
    ),
    (
        "IFCCIRCLEPROFILEDEF",
        &["IFCPARAMETERIZEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    ("IFCCOMPOSITEPROFILEDEF", &["IFCPROFILEDEF"]),
    (
        "IFCCSHAPEPROFILEDEF",
        &["IFCPARAMETERIZEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    ("IFCDERIVEDPROFILEDEF", &["IFCPROFILEDEF"]),
    (
        "IFCELLIPSEPROFILEDEF",
        &["IFCPARAMETERIZEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    (
        "IFCISHAPEPROFILEDEF",
        &["IFCPARAMETERIZEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    (
        "IFCLSHAPEPROFILEDEF",
        &["IFCPARAMETERIZEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    (
        "IFCMIRROREDPROFILEDEF",
        &["IFCDERIVEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    ("IFCPARAMETERIZEDPROFILEDEF", &["IFCPROFILEDEF"]),
    (
        "IFCRECTANGLEHOLLOWPROFILEDEF",
        &[
            "IFCRECTANGLEPROFILEDEF",
            "IFCPARAMETERIZEDPROFILEDEF",
            "IFCPROFILEDEF",
        ],
    ),
    (
        "IFCRECTANGLEPROFILEDEF",
        &["IFCPARAMETERIZEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    (
        "IFCROUNDEDRECTANGLEPROFILEDEF",
        &[
            "IFCRECTANGLEPROFILEDEF",
            "IFCPARAMETERIZEDPROFILEDEF",
            "IFCPROFILEDEF",
        ],
    ),
    (
        "IFCTRAPEZIUMPROFILEDEF",
        &["IFCPARAMETERIZEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    (
        "IFCTSHAPEPROFILEDEF",
        &["IFCPARAMETERIZEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    (
        "IFCUSHAPEPROFILEDEF",
        &["IFCPARAMETERIZEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
    (
        "IFCZSHAPEPROFILEDEF",
        &["IFCPARAMETERIZEDPROFILEDEF", "IFCPROFILEDEF"],
    ),
];
