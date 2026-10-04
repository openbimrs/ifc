//! Every refusal of schema-checked entity creation (#330) with its stable
//! code, each leaving the model byte-identical; malformed operations;
//! handles. The schema facts are those `authoring.rs` cites.
#![cfg(all(feature = "ifc4", feature = "author"))]

mod support;

use openbim_ifc_binding_core::authoring::{handle, HANDLE_BASE};
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{AuthorOp, IfcModel};
use support::*;

/// One refused single-op batch on a fresh copy of the building.
fn refused(ops: impl FnOnce(&[Option<u64>]) -> Vec<AuthorOp>) -> &'static str {
    let mut model = empty("IFC4");
    let ids = building(&mut model);
    let before = model.write().unwrap();
    let ops = ops(&ids);
    let result = match model.author(ops.clone()) {
        Ok(done) => panic!("accepted: {ops:?} -> {done:?}"),
        Err(error) => error.code(),
    };
    assert_eq!(model.write().unwrap(), before, "{result}");
    result
}

fn id(ids: &[Option<u64>], index: usize) -> Tagged {
    Tagged::Ref(ids[index].unwrap())
}

#[test]
fn every_refusal_has_its_code() {
    // The declared release does not declare the type.
    assert_eq!(
        refused(|_| vec![create("IfcWal", &[])]),
        "unsupported-schema"
    );
    // Abstract.
    assert_eq!(
        refused(|_| vec![create("IfcElement", &[])]),
        "wrong-entity-type"
    );
    // Names resolve.
    assert_eq!(
        refused(|_| vec![create("IfcWall", &[("Nmae", text("x"))])]),
        "unknown-attribute"
    );
    // Derived attributes are not settable.
    assert_eq!(
        refused(|_| vec![create(
            "IfcSIUnit",
            &[
                ("Dimensions", Tagged::Null),
                ("UnitType", token("LENGTHUNIT")),
                ("Name", token("METRE")),
            ]
        )]),
        "derived-attribute",
        "a derived slot is left unset, and written `*`"
    );
    assert_eq!(
        refused(|ids| vec![create(
            "IfcSIUnit",
            &[
                ("Dimensions", id(ids, 0)),
                ("UnitType", token("LENGTHUNIT")),
                ("Name", token("METRE")),
            ]
        )]),
        "derived-attribute"
    );
    // Required attributes present.
    assert_eq!(
        refused(|_| vec![create("IfcWallType", &[])]),
        "missing-attribute"
    );
    // Value types.
    assert_eq!(
        refused(|_| vec![create("IfcWall", &[("Name", Tagged::Integer(3))])]),
        "invalid-value"
    );
    // An attribute set twice.
    assert_eq!(
        refused(|_| vec![create(
            "IfcWall",
            &[("Name", text("a")), ("name", text("b"))]
        )]),
        "invalid-value"
    );
    // Cardinality: `Units` is `SET [1:?]`.
    assert_eq!(
        refused(|_| vec![create(
            "IfcUnitAssignment",
            &[("Units", Tagged::List(vec![]))]
        )]),
        "invalid-value"
    );
    // A malformed or duplicate GlobalId.
    assert_eq!(
        refused(|_| vec![create("IfcWall", &[("GlobalId", text("nope"))])]),
        "invalid-value"
    );
    assert_eq!(
        refused(|ids| {
            let _ = ids;
            vec![
                create("IfcWall", &[("GlobalId", text("2YvctVUKr0kugbFTf53O9L"))]),
                create("IfcWall", &[("GlobalId", text("2YvctVUKr0kugbFTf53O9L"))]),
            ]
        }),
        "invalid-value"
    );
    // References resolve, to an accepted type.
    assert_eq!(
        refused(|_| vec![create(
            "IfcWall",
            &[("ObjectPlacement", Tagged::Ref(99_999))]
        )]),
        "missing-reference"
    );
    assert_eq!(
        refused(|ids| vec![create("IfcWall", &[("ObjectPlacement", id(ids, 0))])]),
        "wrong-entity-type"
    );
    // A builder's type of the wrong kind.
    assert_eq!(
        refused(|ids| vec![op(
            "product",
            &[
                ("type", text("IfcBuildingStorey")),
                ("container", id(ids, 11))
            ]
        )]),
        "wrong-entity-type"
    );
    assert_eq!(
        refused(|ids| vec![op(
            "spatial",
            &[("type", text("IfcWall")), ("parent", id(ids, 11))]
        )]),
        "wrong-entity-type"
    );
    assert_eq!(
        refused(|_| vec![op("type_object", &[("type", text("IfcWall"))])]),
        "wrong-entity-type"
    );
    assert_eq!(
        refused(|ids| vec![op(
            "spatial",
            &[("type", text("IfcSpace")), ("parent", id(ids, 14))]
        )]),
        "wrong-entity-type"
    );
    // Relationships the schema allows once.
    assert_eq!(refused(|_| vec![op("project", &[])]), "invalid-model");
    assert_eq!(
        refused(|ids| vec![op(
            "assign_type",
            &[
                ("type_object", id(ids, 12)),
                ("objects", Tagged::List(vec![id(ids, 14)]))
            ]
        )]),
        "invalid-model"
    );
    assert_eq!(
        refused(|ids| vec![op(
            "aggregate",
            &[
                ("parent", id(ids, 9)),
                ("parts", Tagged::List(vec![id(ids, 11)]))
            ]
        )]),
        "invalid-model"
    );
    assert_eq!(
        refused(|ids| vec![op(
            "contain",
            &[
                ("structure", id(ids, 11)),
                ("elements", Tagged::List(vec![]))
            ]
        )]),
        "invalid-value"
    );
    // Edits and removals of entities that do not exist.
    assert_eq!(
        refused(|_| vec![op(
            "edit",
            &[
                ("entity", Tagged::Ref(99_999)),
                ("attributes", attributes(&[]))
            ]
        )]),
        "missing-entity"
    );
    assert_eq!(
        refused(|_| vec![op("remove", &[("entity", Tagged::Ref(99_999))])]),
        "missing-entity"
    );
    // An edit is checked like a creation.
    assert_eq!(
        refused(|ids| vec![op(
            "edit",
            &[
                ("entity", id(ids, 14)),
                ("attributes", attributes(&[("Name", Tagged::Real(1.0))]))
            ]
        )]),
        "invalid-value"
    );
    assert_eq!(
        refused(|ids| vec![op(
            "edit",
            &[
                ("entity", id(ids, 12)),
                (
                    "attributes",
                    attributes(&[("PredefinedType", Tagged::Null)])
                )
            ]
        )]),
        "missing-attribute"
    );
    assert_eq!(
        refused(|ids| vec![op(
            "edit",
            &[
                ("entity", id(ids, 0)),
                ("attributes", attributes(&[("Dimensions", Tagged::Null)]))
            ]
        )]),
        "derived-attribute"
    );
    // A removal another entity still needs: the storey's placement is
    // what the wall's placement is relative to.
    assert_eq!(
        refused(|ids| vec![op("remove", &[("entity", id(ids, 10))])]),
        "still-referenced"
    );
    // Handles name earlier operations that produced an entity.
    assert_eq!(
        refused(|_| vec![create("IfcWall", &[("ObjectPlacement", h(0))])]),
        "invalid-value"
    );
    assert_eq!(
        refused(|ids| vec![
            op("remove", &[("entity", id(ids, 14))]),
            create(
                "IfcWall",
                &[("Name", text("x")), ("Description", text("y"))]
            ),
            op("edit", &[("entity", h(0)), ("attributes", attributes(&[]))]),
        ]),
        "invalid-value"
    );
    // Placements the schema cannot hold.
    assert_eq!(
        refused(|_| vec![op(
            "placement",
            &[
                ("axis", reals(&[0.0, 0.0, 1.0])),
                ("ref_direction", reals(&[0.0, 0.0, 2.0]))
            ]
        )]),
        "invalid-value"
    );
    assert_eq!(
        refused(|_| vec![op(
            "placement",
            &[
                ("axis", reals(&[0.0, 0.0, 0.0])),
                ("ref_direction", reals(&[1.0, 0.0, 0.0]))
            ]
        )]),
        "invalid-value"
    );
}

#[test]
fn malformed_operations_are_invalid_values() {
    let read = |tape: Vec<Tagged>| code(AuthorOp::from_tagged(&Tagged::List(tape)));
    assert_eq!(read(vec![token("BUILD")]), "invalid-value");
    assert_eq!(read(vec![text("create")]), "invalid-value");
    assert_eq!(read(vec![token("CREATE")]), "invalid-value", "needs a type");
    assert_eq!(
        read(vec![
            token("CREATE"),
            text("type"),
            text("IfcWall"),
            text("colour"),
            text("red")
        ]),
        "invalid-value"
    );
    assert_eq!(
        read(vec![
            token("CREATE"),
            text("type"),
            text("IfcWall"),
            text("type"),
            text("IfcSlab")
        ]),
        "invalid-value"
    );
    assert_eq!(
        read(vec![
            token("PLACEMENT"),
            text("location"),
            reals(&[1.0, 2.0])
        ]),
        "invalid-value"
    );
    assert_eq!(
        read(vec![
            token("PLACEMENT"),
            text("axis"),
            reals(&[0.0, 0.0, 1.0])
        ]),
        "invalid-value",
        "axis without ref_direction"
    );
    assert_eq!(
        read(vec![
            token("CONTAIN"),
            text("structure"),
            Tagged::Integer(-1)
        ]),
        "invalid-value"
    );
    // Both spellings of a name are accepted.
    assert!(AuthorOp::from_tagged(&Tagged::List(vec![
        token("assignType"),
        text("typeObject"),
        Tagged::Ref(1),
        text("objects"),
        Tagged::List(vec![Tagged::Ref(2)]),
    ]))
    .is_ok());
}

#[test]
fn a_header_without_a_bundled_release_is_refused() {
    let mut model = IfcModel::empty();
    assert_eq!(
        code(model.author(vec![op("project", &[])])),
        "unsupported-schema"
    );
    let mut model = empty("IFC9");
    assert_eq!(
        code(model.create_entity("IfcWall", vec![])),
        "unsupported-schema"
    );
}

#[test]
fn handles_live_in_their_own_range() {
    assert_eq!(handle(0).unwrap(), HANDLE_BASE);
    assert_eq!(HANDLE_BASE, 1 << 62);
    assert_eq!(code(handle(HANDLE_BASE)), "out-of-range");
}
