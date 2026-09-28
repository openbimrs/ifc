//! Spatial records authored in IFC2X3, IFC4 and IFC4X3 validate against
//! their own release (#202).
//!
//! Every record the `ifc-spatial` `*_with_owner_history` writers can stage
//! in a release is written to STEP, read back with `ifc-step`, and checked
//! by `ifc-validate` against the declared release's table. No record this
//! test wrote may carry an error finding. The fixture mirrors
//! `crates/ifc-spatial/tests/release_fixture/mod.rs`.

#![cfg(all(
    feature = "validate",
    feature = "spatial",
    feature = "schema",
    feature = "step"
))]
#![allow(dead_code)] // the fixture names more than this test reads

use ifc::schema::{for_version, SchemaVersion};
use ifc::spatial::authoring::{
    adhere_to_element_with_owner_history, assign_to_actor_with_owner_history,
    assign_to_group_by_factor_with_owner_history, assign_to_process_with_owner_history,
    assign_to_product_with_owner_history, assign_to_resource_with_owner_history,
    associate_profile_def_with_owner_history, connect_elements_with_owner_history,
    connect_with_realizing_elements_with_owner_history, control_flow_element_with_owner_history,
    cover_elements_with_owner_history, cover_spaces_with_owner_history, declare_with_owner_history,
    define_by_object_with_owner_history, fill_element_with_owner_history,
    interfere_elements_with_owner_history, position_products_with_owner_history,
    project_element_with_owner_history, serve_buildings_with_owner_history,
    void_element_with_owner_history,
};
use ifc::spatial::facility::IFCBRIDGE;
use ifc::spatial::{
    aggregate_with_owner_history, connect_path_elements_with_owner_history,
    contain_with_owner_history, create_external_spatial_element_with_owner_history,
    create_facility_with_owner_history, create_project_library_with_owner_history,
    create_project_with_owner_history, create_space_boundary_with_owner_history,
    create_spatial_element_with_owner_history, BoundaryDraft, BoundaryLevel, ExternalSpatialDraft,
    FacilityDraft, ProjectLibraryDraft, SpatialDraft, SpatialKind,
};
use ifc_model::{Codec, EntityId, Model, Transaction};

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

const OWNER: EntityId = EntityId(5);
const CONTEXT: EntityId = EntityId(7);
const UNITS: EntityId = EntityId(8);
const WALL: EntityId = EntityId(10);
const WALL2: EntityId = EntityId(11);
const COVERING: EntityId = EntityId(12);
const SPACE: EntityId = EntityId(13);
const OPENING: EntityId = EntityId(14);
const DOOR: EntityId = EntityId(15);
const SYSTEM: EntityId = EntityId(16);
const ACTOR: EntityId = EntityId(17);
const TASK: EntityId = EntityId(18);
const SEGMENT: EntityId = EntityId(19);
const CONTROL: EntityId = EntityId(20);
const PROJECTION: EntityId = EntityId(21);
const RESOURCE: EntityId = EntityId(22);
const PROXY: EntityId = EntityId(23);
const GROUP: EntityId = EntityId(24);
const WALL_TYPE: EntityId = EntityId(26);
const FEATURE: EntityId = EntityId(27);
const ALIGNMENT: EntityId = EntityId(28);
const PROFILE: EntityId = EntityId(29);

const ROOTED: [(EntityId, &str); 17] = [
    (WALL, "IFCWALL"),
    (WALL2, "IFCWALL"),
    (COVERING, "IFCCOVERING"),
    (SPACE, "IFCSPACE"),
    (OPENING, "IFCOPENINGELEMENT"),
    (DOOR, "IFCDOOR"),
    (SYSTEM, "IFCSYSTEM"),
    (ACTOR, "IFCACTOR"),
    (TASK, "IFCTASK"),
    (SEGMENT, "IFCFLOWSEGMENT"),
    (CONTROL, "IFCDISTRIBUTIONCONTROLELEMENT"),
    (PROJECTION, "IFCPROJECTIONELEMENT"),
    (RESOURCE, "IFCLABORRESOURCE"),
    (PROXY, "IFCBUILDINGELEMENTPROXY"),
    (GROUP, "IFCGROUP"),
    (WALL_TYPE, "IFCWALLTYPE"),
    (FEATURE, "IFCSURFACEFEATURE"),
];

/// A distinct compressed GUID per index.
fn guid(n: usize) -> String {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let (hi, lo) = (DIGITS[n / 64 % 64] as char, DIGITS[n % 64] as char);
    format!("0YvctVUKr0kugbFTf53O{hi}{lo}")
}

/// Actors, an owner history (`#5`), a representation context, units and
/// the elements the relationships name, in `schema`. The project is
/// authored, with the representation contexts IFC2X3 requires (#214).
fn base(schema: &str, version: SchemaVersion) -> Model {
    let table = for_version(version).unwrap();
    let mut data = String::from(
        "#1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #7=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#9,$);\n\
         #8=IFCUNITASSIGNMENT((#91));\n\
         #9=IFCAXIS2PLACEMENT3D(#90,$,$);\n\
         #90=IFCCARTESIANPOINT((0.,0.,0.));\n\
         #91=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n",
    );
    let mut rooted = ROOTED.to_vec();
    if version == SchemaVersion::Ifc4x3 {
        rooted.push((ALIGNMENT, "IFCALIGNMENT"));
        data.push_str("#29=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,1.,1.);\n");
    }
    for (n, (id, entity)) in rooted.into_iter().enumerate() {
        let arity = table.attributes(entity).len();
        if arity == 0 {
            continue;
        }
        let unset = ",$".repeat(arity - 2);
        data.push_str(&format!(
            "#{}={entity}('{}',#5{unset});\n",
            id.0,
            guid(800 + n)
        ));
    }
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         {data}ENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = ifc::StepCodec.read_bytes(text.as_bytes()).expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

/// What authoring staged, by role.
struct Authored {
    project: EntityId,
    /// The authored `IfcSpace`: in IFC2X3 with its required
    /// `InteriorOrExteriorSpace` and aggregated into the storey (#214).
    room: EntityId,
    site: EntityId,
    building: EntityId,
    storey: EntityId,
    contained: EntityId,
    covers_spaces: EntityId,
    boundary: EntityId,
    /// Every record staged, in order.
    written: Vec<EntityId>,
}

/// Author, with the variants, every spatial record `version` can hold, and
/// commit.
#[allow(clippy::too_many_lines)]
fn author(model: &mut Model, version: SchemaVersion) -> Authored {
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let ifc4x3 = version == SchemaVersion::Ifc4x3;
    let snapshot = model.clone();
    let m = &snapshot;
    let mut staged = Transaction::new(model);
    let tx = &mut staged;
    let mut n = 0;
    let mut next = || {
        n += 1;
        guid(n)
    };
    let mut written = Vec::new();
    let mut keep = |id: EntityId| {
        written.push(id);
        id
    };
    // IFC2X3 requires `RepresentationContexts`; IFC4 and IFC4X3 records
    // are kept as they were before #214, without them.
    let contexts: &[EntityId] = if ifc4 { &[] } else { &[CONTEXT] };
    let project = keep(
        create_project_with_owner_history(tx, m, &next(), Some("P"), Some(UNITS), contexts, OWNER)
            .unwrap(),
    );
    let element = |name| SpatialDraft::new().name(name).composition("ELEMENT");
    let spatial = |tx: &mut Transaction, kind, id: &str, name| {
        create_spatial_element_with_owner_history(tx, m, kind, id, element(name), OWNER).unwrap()
    };
    let site = keep(spatial(tx, SpatialKind::Site, &next(), "Site"));
    let building = keep(spatial(tx, SpatialKind::Building, &next(), "Building"));
    let storey = keep(spatial(tx, SpatialKind::Storey, &next(), "Storey"));
    keep(aggregate_with_owner_history(tx, m, &next(), project, &[site], OWNER).unwrap());
    keep(aggregate_with_owner_history(tx, m, &next(), site, &[building], OWNER).unwrap());
    keep(aggregate_with_owner_history(tx, m, &next(), building, &[storey], OWNER).unwrap());
    let contained =
        keep(contain_with_owner_history(tx, m, &next(), storey, &[WALL, WALL2], OWNER).unwrap());
    keep(cover_elements_with_owner_history(tx, m, &next(), WALL, &[COVERING], OWNER).unwrap());
    let covers_spaces =
        keep(cover_spaces_with_owner_history(tx, m, &next(), SPACE, &[COVERING], OWNER).unwrap());
    keep(serve_buildings_with_owner_history(tx, m, &next(), SYSTEM, &[building], OWNER).unwrap());
    keep(
        control_flow_element_with_owner_history(tx, m, &next(), SEGMENT, &[CONTROL], OWNER)
            .unwrap(),
    );
    keep(assign_to_actor_with_owner_history(tx, m, &next(), ACTOR, &[WALL], OWNER).unwrap());
    keep(assign_to_product_with_owner_history(tx, m, &next(), PROXY, &[WALL], OWNER).unwrap());
    keep(assign_to_process_with_owner_history(tx, m, &next(), TASK, &[WALL], OWNER).unwrap());
    keep(assign_to_resource_with_owner_history(tx, m, &next(), RESOURCE, &[WALL], OWNER).unwrap());
    keep(connect_elements_with_owner_history(tx, m, &next(), WALL, WALL2, OWNER).unwrap());
    keep(
        connect_with_realizing_elements_with_owner_history(
            tx,
            m,
            &next(),
            WALL,
            WALL2,
            &[PROXY],
            OWNER,
        )
        .unwrap(),
    );
    keep(void_element_with_owner_history(tx, m, &next(), WALL, OPENING, OWNER).unwrap());
    keep(fill_element_with_owner_history(tx, m, &next(), OPENING, DOOR, OWNER).unwrap());
    keep(project_element_with_owner_history(tx, m, &next(), WALL, PROJECTION, OWNER).unwrap());
    keep(
        connect_path_elements_with_owner_history(
            tx,
            m,
            &next(),
            WALL,
            WALL2,
            (&[0], &[100]),
            ("ATSTART", "ATEND"),
            OWNER,
        )
        .unwrap(),
    );
    let boundary = |space, element| BoundaryDraft::new(space, element, "PHYSICAL", "INTERNAL");
    let base_boundary = keep(
        create_space_boundary_with_owner_history(
            tx,
            m,
            BoundaryLevel::Base,
            &next(),
            boundary(SPACE, WALL),
            OWNER,
        )
        .unwrap(),
    );
    let room;
    if !ifc4 {
        // IFC2X3 requires `CompositionType` and `InteriorOrExteriorSpace`.
        let space_draft = element("Room").interior_or_exterior("INTERNAL");
        room = keep(
            create_spatial_element_with_owner_history(
                tx,
                m,
                SpatialKind::Space,
                &next(),
                space_draft,
                OWNER,
            )
            .unwrap(),
        );
        keep(aggregate_with_owner_history(tx, m, &next(), storey, &[room], OWNER).unwrap());
    } else {
        let space_draft = SpatialDraft::new().name("Room");
        room = keep(
            create_spatial_element_with_owner_history(
                tx,
                m,
                SpatialKind::Space,
                &next(),
                space_draft,
                OWNER,
            )
            .unwrap(),
        );
        keep(declare_with_owner_history(tx, m, &next(), project, &[WALL_TYPE], OWNER).unwrap());
        keep(define_by_object_with_owner_history(tx, m, &next(), WALL, &[WALL2], OWNER).unwrap());
        keep(
            assign_to_group_by_factor_with_owner_history(
                tx,
                m,
                &next(),
                GROUP,
                &[WALL],
                0.5,
                OWNER,
            )
            .unwrap(),
        );
        keep(
            interfere_elements_with_owner_history(tx, m, &next(), WALL, WALL2, None, OWNER)
                .unwrap(),
        );
        let external = ExternalSpatialDraft::new()
            .name("Outside")
            .predefined_type("EXTERNAL");
        keep(
            create_external_spatial_element_with_owner_history(tx, m, &next(), external, OWNER)
                .unwrap(),
        );
        let library = ProjectLibraryDraft::new().name("Library");
        keep(
            create_project_library_with_owner_history(tx, m, &next(), library, &[CONTEXT], OWNER)
                .unwrap(),
        );
        let first = keep(
            create_space_boundary_with_owner_history(
                tx,
                m,
                BoundaryLevel::First,
                &next(),
                boundary(SPACE, WALL2),
                OWNER,
            )
            .unwrap(),
        );
        let second = boundary(SPACE, WALL2).parent(first);
        keep(
            create_space_boundary_with_owner_history(
                tx,
                m,
                BoundaryLevel::Second,
                &next(),
                second,
                OWNER,
            )
            .unwrap(),
        );
    }
    if ifc4x3 {
        keep(
            adhere_to_element_with_owner_history(tx, m, &next(), WALL, &[FEATURE], OWNER).unwrap(),
        );
        keep(
            position_products_with_owner_history(tx, m, &next(), ALIGNMENT, &[PROXY], OWNER)
                .unwrap(),
        );
        keep(
            associate_profile_def_with_owner_history(tx, m, &next(), PROFILE, &[WALL_TYPE], OWNER)
                .unwrap(),
        );
        let bridge = FacilityDraft::new().name("Bridge").composition("ELEMENT");
        keep(
            create_facility_with_owner_history(
                tx,
                m,
                IFCBRIDGE,
                &next(),
                Some("GIRDER"),
                bridge,
                OWNER,
            )
            .unwrap(),
        );
    }
    staged.commit(model).expect("commit");
    Authored {
        project,
        room,
        site,
        building,
        storey,
        contained,
        covers_spaces,
        boundary: base_boundary,
        written,
    }
}

/// Error findings `ifc-validate` reports on `ids`, against `version`.
fn errors(model: &Model, version: SchemaVersion, ids: &[EntityId]) -> Vec<String> {
    let report = ifc_validate::validate(model, for_version(version).expect("bundled"));
    report
        .findings()
        .iter()
        .filter(|finding| finding.severity == ifc_validate::Severity::Error)
        .filter(|finding| match &finding.path {
            ifc_validate::Path::Entity(id) | ifc_validate::Path::Attribute { entity: id, .. } => {
                ids.contains(id)
            }
            ifc_validate::Path::File => false,
            // A path kind a later ifc-validate adds names no authored record.
            _ => false,
        })
        .map(|finding| format!("{} at {}: {}", finding.rule, finding.path, finding.message))
        .collect()
}

#[test]
fn authored_spatial_records_validate_in_their_release() {
    for (schema, version) in RELEASES {
        let mut model = base(schema, version);
        let authored = author(&mut model, version);
        let bytes = ifc::StepCodec.write_bytes(&model).expect("written");
        let back = ifc::StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let found = errors(&back, version, &authored.written);
        assert!(found.is_empty(), "{schema}:\n  {}", found.join("\n  "));
    }
}

/// #214: an IFC2X3 `IfcSpace` with its required `InteriorOrExteriorSpace`
/// and an IFC2X3 `IfcProject` with its required `RepresentationContexts`
/// and `UnitsInContext` are authored, survive a STEP round trip, read back
/// through the spatial tree, and validate without an error finding.
#[test]
fn ifc2x3_spaces_and_projects_round_trip_with_their_required_attributes() {
    let version = SchemaVersion::Ifc2x3;
    let table = for_version(version).expect("bundled");
    let mut model = base("IFC2X3", version);
    let authored = author(&mut model, version);
    let bytes = ifc::StepCodec.write_bytes(&model).expect("written");
    let back = ifc::StepCodec.read_bytes(&bytes).expect("read back");
    assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());

    let slot = |entity: &str, attribute: &str| {
        table
            .attribute_names(entity)
            .iter()
            .position(|name| *name == attribute)
            .expect("declared")
    };
    let room = back.get(authored.room).expect("space");
    assert_eq!(
        room.attributes[slot("IFCSPACE", "InteriorOrExteriorSpace")],
        ifc_model::Value::Enum("INTERNAL".into())
    );
    let project = back.get(authored.project).expect("project");
    assert_eq!(
        project.attributes[slot("IFCPROJECT", "RepresentationContexts")],
        ifc_model::Value::List(vec![ifc_model::Value::Ref(CONTEXT)])
    );
    assert_eq!(
        project.attributes[slot("IFCPROJECT", "UnitsInContext")],
        ifc_model::Value::Ref(UNITS)
    );

    let tree = ifc::spatial::SpatialTree::build(&back);
    assert_eq!(tree.release(), Some(version));
    // The fixture's unrelated `#13` space is a root of its own.
    assert!(tree.roots().contains(&authored.project));
    let space = tree.node(authored.room).expect("the space is a node");
    assert_eq!(space.kind, SpatialKind::Space);
    assert_eq!(space.parent, Some(authored.storey));
    assert!(tree.ancestors(authored.room).contains(&authored.project));

    let found = errors(&back, version, &[authored.room, authored.project]);
    assert!(found.is_empty(), "{}", found.join("\n  "));
}

/// The oracle is trusted because it fails when it should: an IFC2X3
/// aggregation with `$` for its required `OwnerHistory`, as the plain
/// writers stage, is an error finding.
#[test]
fn the_validator_catches_an_unowned_ifc2x3_aggregation() {
    let mut model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let relation = ifc::spatial::aggregate(&mut tx, &guid(1), WALL, &[WALL2]).expect("staged");
    tx.commit(&mut model).expect("commit");
    assert!(
        !errors(&model, SchemaVersion::Ifc2x3, &[relation]).is_empty(),
        "IFC2X3 requires IfcRoot.OwnerHistory"
    );
}
