//! #202: `IfcGrid` written in the model's declared release.
//!
//! From the EXPRESS sources: `IfcRoot.OwnerHistory` is `IfcOwnerHistory` in
//! IFC2X3 TC1 and `OPTIONAL IfcOwnerHistory` in IFC4 ADD2 TC1 and IFC4X3
//! ADD2; IFC2X3 `IfcGrid` declares `UAxes`, `VAxes`, `WAxes` after the
//! seven `IfcProduct` attributes and no `PredefinedType`, so it has 10
//! attributes where IFC4 and IFC4X3 have 11. `openbim-ifc`'s
//! `release_bound_owner_history_authoring` runs the same record through
//! `ifc-validate`.

use ifc_geometry::authoring::{grid, grid_with_owner_history};
use ifc_geometry::GeometryError;
use ifc_model::{Codec, Edit, Entity, EntityId, Model, Transaction, Value};
use ifc_schema::{for_version, SchemaVersion};
use ifc_step::StepCodec;

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];
const OWNER: EntityId = EntityId(5);
const U: EntityId = EntityId(23);
const V: EntityId = EntityId(26);
const PLACEMENT: EntityId = EntityId(32);
const GUID: &str = "1jQ2A$rnvCJhUvFV5RxFtz";

/// An owner history (`#5`), two grid axes (`#23`, `#26`) and a placement
/// (`#32`) declaring `schema`; an empty `schema` leaves `FILE_SCHEMA` empty.
fn model(schema: &[&str]) -> Model {
    let declared = schema
        .iter()
        .map(|token| format!("'{token}'"))
        .collect::<Vec<_>>()
        .join(",");
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(({declared}));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #20=IFCCARTESIANPOINT((0.,0.));\n\
         #21=IFCCARTESIANPOINT((10.,0.));\n\
         #22=IFCPOLYLINE((#20,#21));\n\
         #23=IFCGRIDAXIS('A',#22,.T.);\n\
         #24=IFCCARTESIANPOINT((0.,10.));\n\
         #25=IFCPOLYLINE((#20,#24));\n\
         #26=IFCGRIDAXIS('1',#25,.T.);\n\
         #30=IFCCARTESIANPOINT((0.,0.,0.));\n\
         #31=IFCAXIS2PLACEMENT3D(#30,$,$);\n\
         #32=IFCLOCALPLACEMENT($,#31);\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

fn staged(tx: &Transaction) -> &Entity {
    match tx.edits() {
        [Edit::Create { entity, .. }] => entity,
        other => panic!("expected one create, got {other:?}"),
    }
}

/// Refused before anything is staged.
fn refused(
    model: &Model,
    author: impl Fn(&mut Transaction) -> Result<EntityId, GeometryError>,
) -> GeometryError {
    let mut tx = Transaction::new(model);
    let error = author(&mut tx).expect_err("refused");
    assert!(tx.is_empty(), "a refusal staged {:?}", tx.edits());
    error
}

/// Each release: the release's arity (10 in IFC2X3), the owner history
/// and the axes by attribute name; written as STEP, read back, and read by
/// the grid-axis rule, which finds each axis in exactly one list.
#[test]
fn every_release_round_trips_with_its_own_layout() {
    for (schema, version) in RELEASES {
        let table = for_version(version).unwrap();
        let names = table.attribute_names("IFCGRID");
        assert_eq!(
            names.len(),
            if version == SchemaVersion::Ifc2x3 {
                10
            } else {
                11
            }
        );
        let at = |name: &str| names.iter().position(|found| *found == name).unwrap();
        let mut model = model(&[schema]);
        let token = (version != SchemaVersion::Ifc2x3).then_some("RECTANGULAR");
        let mut tx = Transaction::new(&model);
        let id = grid_with_owner_history(
            &mut tx,
            &model,
            GUID,
            Some(PLACEMENT),
            (&[U], &[V], &[]),
            token,
            OWNER,
        )
        .expect(schema);
        tx.commit(&mut model).expect("commit");

        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let record = back.get(id).expect("read back");
        assert_eq!(record.attributes.len(), names.len(), "{schema}");
        assert_eq!(record.attributes[at("OwnerHistory")], Value::Ref(OWNER));
        assert_eq!(
            record.attributes[at("ObjectPlacement")],
            Value::Ref(PLACEMENT)
        );
        assert_eq!(
            record.attributes[at("UAxes")],
            Value::List(vec![Value::Ref(U)])
        );
        assert_eq!(
            record.attributes[at("VAxes")],
            Value::List(vec![Value::Ref(V)])
        );
        assert_eq!(record.attributes[at("WAxes")], Value::Null);
        if let Some(token) = token {
            assert_eq!(
                record.attributes[at("PredefinedType")],
                Value::Enum(token.into())
            );
        }
        for axis in [U, V] {
            let found = ifc_geometry::rules::validate(&back, axis);
            assert!(found.is_empty(), "{schema}: {found:?}");
        }
    }
}

/// In IFC4 and IFC4X3 the record is the one `grid` writes, with the owner
/// history in its optional slot; `grid` itself is unchanged.
#[test]
fn ifc4_and_ifc4x3_output_is_unchanged() {
    for schema in [&["IFC4"][..], &["IFC4X3_ADD2"], &[]] {
        let model = model(schema);
        let axes: (&[EntityId], &[EntityId], &[EntityId]) = (&[U], &[V], &[V]);
        let mut tx = Transaction::new(&model);
        grid(&mut tx, GUID, Some(PLACEMENT), axes, Some("RADIAL")).expect("grid");
        let mut expected = staged(&tx).clone();
        assert_eq!(expected.attributes.len(), 11);
        let mut tx = Transaction::new(&model);
        grid_with_owner_history(
            &mut tx,
            &model,
            GUID,
            Some(PLACEMENT),
            axes,
            Some("RADIAL"),
            OWNER,
        )
        .expect("grid");
        expected.attributes[1] = Value::Ref(OWNER);
        assert_eq!(*staged(&tx), expected, "{schema:?}");
    }
}

/// IFC2X3 declares no `PredefinedType`; a token outside the release's
/// `IfcGridTypeEnum` is refused in the others.
#[test]
fn what_the_release_lacks_is_refused() {
    let ifc2x3 = model(&["IFC2X3"]);
    assert_eq!(
        refused(&ifc2x3, |tx| grid_with_owner_history(
            tx,
            &ifc2x3,
            GUID,
            Some(PLACEMENT),
            (&[U], &[V], &[]),
            Some("RECTANGULAR"),
            OWNER
        )),
        GeometryError::InvalidAuthoredValue {
            type_name: "IFCGRID",
            attribute: "PredefinedType",
            detail: "Ifc2x3 declares no such attribute".into(),
        }
    );
    let ifc4 = model(&["IFC4"]);
    let error = refused(&ifc4, |tx| {
        grid_with_owner_history(
            tx,
            &ifc4,
            GUID,
            None,
            (&[U], &[V], &[]),
            Some("HEXAGONAL"),
            OWNER,
        )
    });
    assert!(
        matches!(
            error,
            GeometryError::InvalidAuthoredValue {
                attribute: "PredefinedType",
                ..
            }
        ),
        "{error:?}"
    );
}

#[test]
fn a_wrong_type_or_missing_owner_history_is_refused() {
    for (schema, _) in RELEASES {
        let model = model(&[schema]);
        for (owner, detail) in [(U, "IFCGRIDAXIS"), (EntityId(999), "does not resolve")] {
            let error = refused(&model, |tx| {
                grid_with_owner_history(tx, &model, GUID, None, (&[U], &[V], &[]), None, owner)
            });
            assert!(
                matches!(
                    &error,
                    GeometryError::InvalidAuthoredValue {
                        type_name: "IFCGRID",
                        attribute: "OwnerHistory",
                        detail: found,
                    } if found.contains(detail)
                ),
                "{schema}: {error:?}"
            );
        }
    }
}

/// An owner history staged in the same transaction is accepted.
#[test]
fn a_staged_owner_history_is_accepted() {
    let model = model(&["IFC2X3"]);
    let mut tx = Transaction::new(&model);
    let staged = tx.create(model.get(OWNER).unwrap().clone());
    grid_with_owner_history(&mut tx, &model, GUID, None, (&[U], &[V], &[]), None, staged)
        .expect("a staged IfcOwnerHistory");
}

#[test]
fn multiple_or_unknown_schemas_are_refused() {
    let author = |model: &Model| {
        refused(model, |tx| {
            grid_with_owner_history(tx, model, GUID, None, (&[U], &[V], &[]), None, OWNER)
        })
    };
    for header in [&["IFC4", "IFC2X3"][..], &["IFC9"]] {
        let error = author(&model(header));
        assert!(
            matches!(
                error,
                GeometryError::AuthoringSchemaUnbound {
                    type_name: "IFCGRID",
                    ..
                }
            ),
            "{header:?}: {error:?}"
        );
    }
}
