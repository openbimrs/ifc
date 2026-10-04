//! Shared by the authoring test files (#330): the tape helpers and the
//! host tests' model, built from nothing.
#![allow(dead_code)]

use openbim_ifc_binding_core::header::Header;
use openbim_ifc_binding_core::value::Tagged;
#[cfg(feature = "author")]
use openbim_ifc_binding_core::{authoring::handle, AuthorOp};
use openbim_ifc_binding_core::{BindingError, IfcModel};

pub fn token(value: &str) -> Tagged {
    Tagged::Enum(value.into())
}

pub fn empty(schema: &str) -> IfcModel {
    let mut model = IfcModel::empty();
    model.set_header(Header {
        schema: vec![schema.to_owned()],
        ..Header::default()
    });
    model
}

pub fn code<T: std::fmt::Debug>(result: Result<T, BindingError>) -> &'static str {
    result.expect_err("refused").code()
}

#[cfg(feature = "author")]
mod enabled {
    use super::*;

    pub fn text(value: &str) -> Tagged {
        Tagged::Text(value.into())
    }

    pub fn h(index: u64) -> Tagged {
        Tagged::Ref(handle(index).unwrap())
    }

    pub fn reals(values: &[f64]) -> Tagged {
        Tagged::List(values.iter().copied().map(Tagged::Real).collect())
    }

    pub fn attributes(pairs: &[(&str, Tagged)]) -> Tagged {
        Tagged::List(
            pairs
                .iter()
                .map(|(name, value)| Tagged::List(vec![text(name), value.clone()]))
                .collect(),
        )
    }

    /// One operation in the tape form every host converts to.
    pub fn op(name: &str, fields: &[(&str, Tagged)]) -> AuthorOp {
        let mut tape = vec![token(name)];
        for (key, value) in fields {
            tape.push(text(key));
            tape.push(value.clone());
        }
        AuthorOp::from_tagged(&Tagged::List(tape)).expect("well-formed op")
    }

    /// Units, a context, the project, site, building and storey, a wall type
    /// and a placed wall contained in the storey and typed: the host tests'
    /// model. Returns the batch result's ids.
    pub fn building(model: &mut IfcModel) -> Vec<Option<u64>> {
        let ops = vec![
            // 0..=4: a length unit and the 3D model context.
            op(
                "create",
                &[
                    ("type", text("IfcSIUnit")),
                    (
                        "attributes",
                        attributes(&[("UnitType", token("LENGTHUNIT")), ("Name", token("METRE"))]),
                    ),
                ],
            ),
            op(
                "create",
                &[
                    ("type", text("IfcUnitAssignment")),
                    (
                        "attributes",
                        attributes(&[("Units", Tagged::List(vec![h(0)]))]),
                    ),
                ],
            ),
            op(
                "create",
                &[
                    ("type", text("IfcCartesianPoint")),
                    (
                        "attributes",
                        attributes(&[("Coordinates", reals(&[0.0, 0.0, 0.0]))]),
                    ),
                ],
            ),
            op(
                "create",
                &[
                    ("type", text("IfcAxis2Placement3D")),
                    ("attributes", attributes(&[("Location", h(2))])),
                ],
            ),
            op(
                "create",
                &[
                    ("type", text("IfcGeometricRepresentationContext")),
                    (
                        "attributes",
                        attributes(&[
                            ("ContextType", text("Model")),
                            ("CoordinateSpaceDimension", Tagged::Integer(3)),
                            ("Precision", Tagged::Real(1e-5)),
                            ("WorldCoordinateSystem", h(3)),
                        ]),
                    ),
                ],
            ),
            // 5: the project.
            op(
                "project",
                &[(
                    "attributes",
                    attributes(&[
                        ("Name", text("Demo")),
                        ("UnitsInContext", h(1)),
                        ("RepresentationContexts", Tagged::List(vec![h(4)])),
                    ]),
                )],
            ),
            // 6..=11: site, building and storey, each placed in its parent.
            op("placement", &[]),
            op(
                "spatial",
                &[
                    ("type", text("IfcSite")),
                    ("parent", h(5)),
                    ("placement", h(6)),
                    ("attributes", attributes(&[("Name", text("Site"))])),
                ],
            ),
            op("placement", &[("relative_to", h(6))]),
            op(
                "spatial",
                &[
                    ("type", text("IfcBuilding")),
                    ("parent", h(7)),
                    ("placement", h(8)),
                ],
            ),
            op("placement", &[("relative_to", h(8))]),
            op(
                "spatial",
                &[
                    ("type", text("IfcBuildingStorey")),
                    ("parent", h(9)),
                    ("placement", h(10)),
                    (
                        "attributes",
                        attributes(&[("Name", text("Level 0")), ("Elevation", Tagged::Real(0.0))]),
                    ),
                ],
            ),
            // 12: the wall type.
            op(
                "type_object",
                &[
                    ("type", text("IfcWallType")),
                    (
                        "attributes",
                        attributes(&[
                            ("Name", text("Basic 200")),
                            ("PredefinedType", token("STANDARD")),
                        ]),
                    ),
                ],
            ),
            // 13, 14: the wall, placed in the storey, contained and typed.
            op(
                "placement",
                &[
                    ("relative_to", h(10)),
                    ("location", reals(&[1.0, 2.0, 0.0])),
                    ("axis", reals(&[0.0, 0.0, 1.0])),
                    ("ref_direction", reals(&[1.0, 0.0, 0.0])),
                ],
            ),
            op(
                "product",
                &[
                    ("type", text("IfcWall")),
                    ("container", h(11)),
                    ("placement", h(13)),
                    ("type_object", h(12)),
                    (
                        "attributes",
                        attributes(&[
                            ("Name", text("Wall")),
                            ("PredefinedType", token("STANDARD")),
                        ]),
                    ),
                ],
            ),
        ];
        model.author(ops).expect("the building authors").ids
    }

    pub fn create(type_name: &str, pairs: &[(&str, Tagged)]) -> AuthorOp {
        op(
            "create",
            &[("type", text(type_name)), ("attributes", attributes(pairs))],
        )
    }
}
#[cfg(feature = "author")]
pub use enabled::*;
