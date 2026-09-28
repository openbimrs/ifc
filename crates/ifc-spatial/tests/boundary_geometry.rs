//! A space boundary's `ConnectionGeometry` (#156).
//!
//! The surface a boundary contributes to its space is the whole reason
//! second-level boundaries exist for energy and coverage analysis. These
//! tests pin that it is found on every concrete boundary type in every
//! release, that its absence reads as `None`, and that a reference naming
//! nothing -- or naming the wrong thing -- is reported rather than returned
//! as a shape or silently dropped.

use ifc_model::{Codec, Entity, EntityId, Model, Transaction, Value};
use ifc_spatial::relation::boundary;
use ifc_spatial::{create_space_boundary, BoundaryDraft, BoundaryLevel, ConnectionGeometryAnomaly};

fn put(model: &mut Model, id: u64, type_name: &str, attributes: Vec<Value>) -> EntityId {
    let entity_id = EntityId(id);
    model.insert(entity_id, Entity::new(type_name, attributes));
    entity_id
}

/// Attributes of a boundary of `arity` slots with `geometry` in slot 6.
///
/// Slots: 0-3 IfcRoot, 4 RelatingSpace, 5 RelatedBuildingElement,
/// 6 ConnectionGeometry, 7 PhysicalOrVirtual, 8 InternalOrExternal,
/// 9 ParentBoundary, 10 CorrespondingBoundary.
fn attrs(arity: usize, geometry: Value) -> Vec<Value> {
    let mut a = vec![Value::Null; arity];
    a[4] = Value::Ref(EntityId(1));
    a[5] = Value::Ref(EntityId(2));
    a[6] = geometry;
    a[7] = Value::Enum("PHYSICAL".into());
    a[8] = Value::Enum("EXTERNAL".into());
    a
}

/// Space #1, wall #2, surface geometry #3.
fn base_model() -> Model {
    let mut model = Model::new();
    put(&mut model, 1, "IFCSPACE", vec![]);
    put(&mut model, 2, "IFCWALL", vec![]);
    put(
        &mut model,
        3,
        "IFCCONNECTIONSURFACEGEOMETRY",
        vec![Value::Null, Value::Null],
    );
    model
}

const TYPES: [(&str, usize); 3] = [
    ("IFCRELSPACEBOUNDARY", 9),
    ("IFCRELSPACEBOUNDARY1STLEVEL", 10),
    ("IFCRELSPACEBOUNDARY2NDLEVEL", 11),
];

fn read(model: &Model, id: u64) -> Result<Option<EntityId>, ConnectionGeometryAnomaly> {
    boundary::all(model)
        .into_iter()
        .find(|b| b.id == EntityId(id))
        .expect("boundary is listed")
        .connection_geometry(model)
}

#[test]
fn a_surface_is_returned_on_every_boundary_type() {
    for (type_name, arity) in TYPES {
        let mut model = base_model();
        put(
            &mut model,
            10,
            type_name,
            attrs(arity, Value::Ref(EntityId(3))),
        );
        assert_eq!(
            read(&model, 10),
            Ok(Some(EntityId(3))),
            "{type_name} must expose its ConnectionGeometry"
        );
    }
}

#[test]
fn no_geometry_is_none_on_every_boundary_type() {
    for (type_name, arity) in TYPES {
        let mut model = base_model();
        put(&mut model, 10, type_name, attrs(arity, Value::Null));
        assert_eq!(read(&model, 10), Ok(None), "{type_name} with `$`");
    }
}

/// A record cut off before slot 6 has no geometry, not a malformed one.
#[test]
fn a_record_ending_before_the_slot_is_none() {
    let mut model = base_model();
    let mut short = vec![Value::Null; 4];
    short.push(Value::Ref(EntityId(1)));
    short.push(Value::Ref(EntityId(2)));
    put(&mut model, 10, "IFCRELSPACEBOUNDARY", short);
    assert_eq!(read(&model, 10), Ok(None));
}

/// Each concrete `IfcConnectionGeometry` subtype in any bundled release is
/// accepted. The list is asserted against the schemas in `slot_layout.rs`.
#[test]
fn every_connection_geometry_subtype_is_accepted() {
    for type_name in [
        "IfcConnectionCurveGeometry",
        "IfcConnectionPointEccentricity",
        "IfcConnectionPointGeometry",
        "IfcConnectionPortGeometry",
        "IfcConnectionSurfaceGeometry",
        "IfcConnectionVolumeGeometry",
    ] {
        let mut model = base_model();
        put(&mut model, 4, type_name, vec![]);
        put(
            &mut model,
            10,
            "IFCRELSPACEBOUNDARY2NDLEVEL",
            attrs(11, Value::Ref(EntityId(4))),
        );
        assert_eq!(read(&model, 10), Ok(Some(EntityId(4))), "{type_name}");
    }
}

/// A dangling reference is reported with its target, and the boundary is
/// still listed: this crate reports what the file says, never rejects it.
#[test]
fn a_dangling_reference_is_reported_not_dropped() {
    for (type_name, arity) in TYPES {
        let mut model = base_model();
        put(
            &mut model,
            10,
            type_name,
            attrs(arity, Value::Ref(EntityId(99))),
        );
        assert_eq!(boundary::all(&model).len(), 1, "{type_name} still listed");
        assert_eq!(
            read(&model, 10),
            Err(ConnectionGeometryAnomaly::Dangling {
                boundary: EntityId(10),
                target: EntityId(99),
            }),
            "{type_name}"
        );
    }
}

/// A reference to something that is not a connection geometry -- here the
/// raw face a careless exporter might write directly -- is not a shape.
#[test]
fn a_wrong_kind_reference_is_reported_with_its_type() {
    let mut model = base_model();
    put(&mut model, 4, "IfcFaceSurface", vec![]);
    put(
        &mut model,
        10,
        "IFCRELSPACEBOUNDARY2NDLEVEL",
        attrs(11, Value::Ref(EntityId(4))),
    );
    assert_eq!(
        read(&model, 10),
        Err(ConnectionGeometryAnomaly::WrongKind {
            boundary: EntityId(10),
            target: EntityId(4),
            type_name: "IFCFACESURFACE".into(),
        })
    );
}

/// The abstract supertype itself cannot be instantiated; naming it is not
/// a usable shape either.
#[test]
fn the_abstract_supertype_is_a_wrong_kind() {
    let mut model = base_model();
    put(&mut model, 4, "IFCCONNECTIONGEOMETRY", vec![]);
    put(
        &mut model,
        10,
        "IFCRELSPACEBOUNDARY",
        attrs(9, Value::Ref(EntityId(4))),
    );
    assert!(matches!(
        read(&model, 10),
        Err(ConnectionGeometryAnomaly::WrongKind { .. })
    ));
}

/// A list or a literal in a scalar reference slot is malformed. Taking the
/// first reference out of a list would guess; reporting it does not.
#[test]
fn a_non_reference_value_is_reported() {
    for value in [
        Value::List(vec![Value::Ref(EntityId(3))]),
        Value::Integer(3),
        Value::Derived,
    ] {
        let mut model = base_model();
        put(
            &mut model,
            10,
            "IFCRELSPACEBOUNDARY",
            attrs(9, value.clone()),
        );
        assert_eq!(
            read(&model, 10),
            Err(ConnectionGeometryAnomaly::NotAReference {
                boundary: EntityId(10)
            }),
            "{value:?}"
        );
    }
}

/// Asking a model that does not hold the boundary cannot read its slot, and
/// must say so rather than answer "no geometry".
#[test]
fn a_boundary_absent_from_the_model_is_reported() {
    let mut model = base_model();
    put(
        &mut model,
        10,
        "IFCRELSPACEBOUNDARY",
        attrs(9, Value::Ref(EntityId(3))),
    );
    let found = boundary::all(&model).remove(0);
    assert_eq!(
        found.connection_geometry(&Model::new()),
        Err(ConnectionGeometryAnomaly::MissingBoundary {
            boundary: EntityId(10)
        })
    );
}

/// The same answer through the STEP codec for each declared release. IFC2x3
/// has only the plain boundary type; IFC4 and IFC4X3 write the 2nd level.
#[test]
fn every_release_reads_the_slot_from_a_step_file() {
    let cases = [
        (
            "IFC2X3",
            "#10=IFCRELSPACEBOUNDARY('0jQ2A$rnvCJhUvFV5RxFtz',$,$,$,#1,#2,#3,.PHYSICAL.,.EXTERNAL.);\n\
             #11=IFCRELSPACEBOUNDARY('1jQ2A$rnvCJhUvFV5RxFtz',$,$,$,#1,#2,$,.PHYSICAL.,.EXTERNAL.);",
        ),
        (
            "IFC4",
            "#10=IFCRELSPACEBOUNDARY2NDLEVEL('0jQ2A$rnvCJhUvFV5RxFtz',$,$,$,#1,#2,#3,.PHYSICAL.,.EXTERNAL.,$,$);\n\
             #11=IFCRELSPACEBOUNDARY1STLEVEL('1jQ2A$rnvCJhUvFV5RxFtz',$,$,$,#1,#2,$,.PHYSICAL.,.EXTERNAL.,$);",
        ),
        (
            "IFC4X3_ADD2",
            "#10=IFCRELSPACEBOUNDARY2NDLEVEL('0jQ2A$rnvCJhUvFV5RxFtz',$,$,$,#1,#2,#3,.PHYSICAL.,.EXTERNAL.,$,$);\n\
             #11=IFCRELSPACEBOUNDARY2NDLEVEL('1jQ2A$rnvCJhUvFV5RxFtz',$,$,$,#1,#2,$,.PHYSICAL.,.EXTERNAL.,$,$);",
        ),
    ];
    for (schema, boundaries) in cases {
        let text = format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
             FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
             #1=IFCSPACE('2jQ2A$rnvCJhUvFV5RxFtz',$,$,$,$,$,$,$,.ELEMENT.,.INTERNAL.,$);\n\
             #2=IFCWALL('3jQ2A$rnvCJhUvFV5RxFtz',$,$,$,$,$,$,$);\n\
             #3=IFCCONNECTIONSURFACEGEOMETRY($,$);\n{boundaries}\nENDSEC;\nEND-ISO-10303-21;\n"
        );
        let model = ifc_step::StepCodec
            .read_bytes(text.as_bytes())
            .unwrap_or_else(|error| panic!("{schema}: {error:?}"));
        assert_eq!(read(&model, 10), Ok(Some(EntityId(3))), "{schema}");
        assert_eq!(read(&model, 11), Ok(None), "{schema}");
    }
}

/// What the authoring side writes, the reader returns.
#[test]
fn an_authored_geometry_reads_back() {
    let mut model = Model::new();
    let mut tx = Transaction::new(&model);
    let space = tx.create(Entity::new("IFCSPACE", vec![Value::Null; 8]));
    let wall = tx.create(Entity::new("IFCWALL", vec![Value::Null; 8]));
    let surface = tx.create(Entity::new(
        "IFCCONNECTIONSURFACEGEOMETRY",
        vec![Value::Null, Value::Null],
    ));
    let id = create_space_boundary(
        &mut tx,
        &model,
        BoundaryLevel::Second,
        "1jQ2A$rnvCJhUvFV5RxFtz",
        BoundaryDraft::new(space, wall, "PHYSICAL", "EXTERNAL").connection_geometry(surface),
    )
    .expect("stage");
    tx.commit(&mut model).expect("commit");
    assert_eq!(read(&model, id.0), Ok(Some(surface)));
}
