//! A model per release and every spatial record it can hold, authored
//! through the `*_with_owner_history` writers (#202).
//!
//! Included with `mod release_fixture;` by the release test binaries.

#![allow(dead_code)] // each binary uses its own part

use ifc_model::{Codec, EntityId, Model, Transaction};
use ifc_schema::{for_version, SchemaVersion};
use ifc_spatial::authoring::{
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
use ifc_spatial::facility::IFCBRIDGE;
use ifc_spatial::{
    aggregate_with_owner_history, connect_path_elements_with_owner_history,
    contain_with_owner_history, create_external_spatial_element_with_owner_history,
    create_facility_with_owner_history, create_project_library_with_owner_history,
    create_project_with_owner_history, create_space_boundary_with_owner_history,
    create_spatial_element_with_owner_history, BoundaryDraft, BoundaryLevel, ExternalSpatialDraft,
    FacilityDraft, ProjectLibraryDraft, SpatialDraft, SpatialKind,
};

pub const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];

pub const OWNER: EntityId = EntityId(5);
pub const CONTEXT: EntityId = EntityId(7);
pub const UNITS: EntityId = EntityId(8);
pub const WALL: EntityId = EntityId(10);
pub const WALL2: EntityId = EntityId(11);
pub const COVERING: EntityId = EntityId(12);
pub const SPACE: EntityId = EntityId(13);
pub const OPENING: EntityId = EntityId(14);
pub const DOOR: EntityId = EntityId(15);
pub const SYSTEM: EntityId = EntityId(16);
pub const ACTOR: EntityId = EntityId(17);
pub const TASK: EntityId = EntityId(18);
pub const SEGMENT: EntityId = EntityId(19);
pub const CONTROL: EntityId = EntityId(20);
pub const PROJECTION: EntityId = EntityId(21);
pub const RESOURCE: EntityId = EntityId(22);
pub const PROXY: EntityId = EntityId(23);
pub const GROUP: EntityId = EntityId(24);
pub const WALL_TYPE: EntityId = EntityId(26);
pub const FEATURE: EntityId = EntityId(27);
pub const ALIGNMENT: EntityId = EntityId(28);
pub const PROFILE: EntityId = EntityId(29);

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
pub fn guid(n: usize) -> String {
    const DIGITS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let (hi, lo) = (DIGITS[n / 64 % 64] as char, DIGITS[n % 64] as char);
    format!("0YvctVUKr0kugbFTf53O{hi}{lo}")
}

/// Actors, an owner history (`#5`), a representation context, units and
/// the elements the relationships name, in `schema`. The project is
/// authored, with the representation contexts IFC2X3 requires (#214).
pub fn base(schema: &str, version: SchemaVersion) -> Model {
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
    let model = ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

/// What authoring staged, by role.
pub struct Authored {
    pub project: EntityId,
    /// The authored `IfcSpace`: in IFC2X3 with its required
    /// `InteriorOrExteriorSpace` and aggregated into the storey (#214).
    pub room: EntityId,
    pub site: EntityId,
    pub building: EntityId,
    pub storey: EntityId,
    pub contained: EntityId,
    pub covers_spaces: EntityId,
    pub boundary: EntityId,
    /// Every record staged, in order.
    pub written: Vec<EntityId>,
}

/// Author, with the variants, every spatial record `version` can hold, and
/// commit.
#[allow(clippy::too_many_lines)]
pub fn author(model: &mut Model, version: SchemaVersion) -> Authored {
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
