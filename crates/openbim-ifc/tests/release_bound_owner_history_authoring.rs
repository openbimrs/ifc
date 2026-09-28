//! `IfcRoot` records authored in IFC2X3, IFC4 and IFC4X3 validate against
//! their own release (#202).
//!
//! The writers of `ifc-occurrence`, `ifc-element-type`, `ifc-approval`,
//! `ifc-constraint`, `ifc-geometry` (`IfcGrid`) and `ifc-properties`
//! (predefined sets and templates) are run with an owner history, which
//! IFC2X3 requires; the result is written to STEP, read back with
//! `ifc-step`, and checked by `ifc-validate` against the declared release's
//! table. No record this test wrote may carry an error finding. The two
//! catalogue crates are walked whole, not sampled.

#![cfg(all(
    feature = "validate",
    feature = "schema",
    feature = "step",
    feature = "occurrence",
    feature = "element-type",
    feature = "approval",
    feature = "constraint",
    feature = "properties",
    feature = "geometry-select"
))]

use ifc::approval::{associate_approval_with_owner_history, ApprovalAssociationDraft};
use ifc::constraint::{associate_constraint_with_owner_history, ConstraintAssociationDraft};
use ifc::element_type::{
    create_supertype_with_owner_history, create_type_with_owner_history, SupertypeDraft, TypeDraft,
    ALL as TYPES, ALL_SUPERTYPES,
};
use ifc::occurrence::table::ALL as OCCURRENCES;
use ifc::occurrence::{create_with_owner_history, OccurrenceDraft};
use ifc::properties::{
    add_complex_property_template_with_owner_history,
    add_door_lining_properties_with_owner_history, add_door_panel_properties_with_owner_history,
    add_property_set_template_with_owner_history, add_window_lining_properties_with_owner_history,
    DoorLiningDraft, WindowLiningDraft,
};
use ifc::schema::{for_version, TypeKind};
use ifc::{Codec, Entity, Model, SchemaVersion, StepCodec, Value};
use ifc_model::{EntityId, Transaction};

const RELEASES: [(&str, SchemaVersion); 3] = [
    ("IFC2X3", SchemaVersion::Ifc2x3),
    ("IFC4", SchemaVersion::Ifc4),
    ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
];
const OWNER: EntityId = EntityId(5);
const WALL: EntityId = EntityId(10);
const APPROVAL: EntityId = EntityId(20);
const METRIC: EntityId = EntityId(30);
const U: EntityId = EntityId(43);
const V: EntityId = EntityId(46);
const PLACEMENT: EntityId = EntityId(49);

/// Actors, an owner history (`#5`), a wall (`#10`), an approval (`#20`), a
/// metric (`#30`), two grid axes and a placement, in `schema`.
fn base(schema: &str, version: SchemaVersion) -> Model {
    let table = for_version(version).unwrap();
    let wall = ",$".repeat(table.attributes("IFCWALL").len() - 3);
    let (approval, metric) = if version == SchemaVersion::Ifc2x3 {
        (
            "#21=IFCCALENDARDATE(28,9,2026);\n#20=IFCAPPROVAL($,#21,$,$,$,'Accepted','A-1');",
            "#30=IFCMETRIC('Max',$,.HARD.,$,$,$,$,.LESSTHAN.,$,IFCTEXT('2 m'));",
        )
    } else {
        (
            "#20=IFCAPPROVAL('A-1','Accepted',$,$,$,$,$,$,$);",
            "#30=IFCMETRIC('Max',$,.HARD.,$,$,$,$,.LESSTHAN.,$,$,$);",
        )
    };
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{schema}'));\nENDSEC;\nDATA;\n\
         #1=IFCPERSON($,'Doe','Jane',$,$,$,$,$);\n\
         #2=IFCORGANIZATION($,'Acme',$,$,$);\n\
         #3=IFCPERSONANDORGANIZATION(#1,#2,$);\n\
         #4=IFCAPPLICATION(#2,'1.0','Test','test');\n\
         #5=IFCOWNERHISTORY(#3,#4,$,.NOCHANGE.,$,$,$,1700000000);\n\
         #10=IFCWALL('1xS3BCk291UvhgP2dvNsgp',#5,'W1'{wall});\n\
         {approval}\n{metric}\n\
         #40=IFCCARTESIANPOINT((0.,0.));\n\
         #41=IFCCARTESIANPOINT((10.,0.));\n\
         #42=IFCPOLYLINE((#40,#41));\n\
         #43=IFCGRIDAXIS('A',#42,.T.);\n\
         #44=IFCCARTESIANPOINT((0.,10.));\n\
         #45=IFCPOLYLINE((#40,#44));\n\
         #46=IFCGRIDAXIS('1',#45,.T.);\n\
         #47=IFCCARTESIANPOINT((0.,0.,0.));\n\
         #48=IFCAXIS2PLACEMENT3D(#47,$,$);\n\
         #49=IFCLOCALPLACEMENT($,#48);\n\
         ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec.read_bytes(text.as_bytes()).expect("parses")
}

/// A fresh, unique GlobalId per record.
fn guid(n: usize) -> String {
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&(n as u64 + 1).to_le_bytes());
    ifc_model::guid::Guid::from_uuid(bytes).to_string()
}

/// The first token `version` declares for `entity`'s `PredefinedType`,
/// other than `USERDEFINED`.
fn token(version: SchemaVersion, entity: &str) -> Option<String> {
    let table = for_version(version).unwrap();
    let declared = table
        .attributes(entity)
        .into_iter()
        .find(|attribute| attribute.name == "PredefinedType")?;
    match &table.type_def(&declared.type_name)?.kind {
        TypeKind::Enumeration(members) => members.iter().find(|m| *m != "USERDEFINED").cloned(),
        _ => None,
    }
}

/// Author through every writer of #202 that `version` can hold.
fn author(model: &mut Model, version: SchemaVersion) -> Vec<EntityId> {
    let mut tx = Transaction::new(model);
    let mut written = Vec::new();
    let mut n = 0;
    let mut next = || {
        n += 1;
        guid(n)
    };
    for kind in OCCURRENCES {
        let token = token(version, kind.type_name);
        let draft = OccurrenceDraft::new().name("Sweep");
        let gid = next();
        // Rows the release lacks, or whose required attributes this draft
        // leaves unset, are refused; `ifc-occurrence` tests which, and
        // `release_bound_occurrence_authoring` writes them (#214).
        if let Ok(id) = create_with_owner_history(
            &mut tx,
            model,
            *kind,
            &gid,
            token.as_deref(),
            None,
            draft,
            OWNER,
        ) {
            written.push(id);
        }
    }
    for kind in TYPES {
        let token = token(version, kind.type_name);
        let draft = TypeDraft::new().name("Sweep");
        let gid = next();
        if let Ok(id) = create_type_with_owner_history(
            &mut tx,
            model,
            *kind,
            &gid,
            token.as_deref(),
            draft,
            OWNER,
        ) {
            written.push(id);
        }
    }
    for kind in ALL_SUPERTYPES {
        let gid = next();
        if let Ok(id) = create_supertype_with_owner_history(
            &mut tx,
            model,
            *kind,
            &gid,
            "Sweep",
            SupertypeDraft::default(),
            OWNER,
        ) {
            written.push(id);
        }
    }
    let gid = next();
    let approval = ApprovalAssociationDraft::new(&gid, &[WALL], APPROVAL).name("Approved");
    written
        .push(associate_approval_with_owner_history(&mut tx, model, approval, OWNER).expect("a"));
    let gid = next();
    let constraint = ConstraintAssociationDraft::new(&gid, &[WALL], METRIC).intent("DESIGN");
    written.push(
        associate_constraint_with_owner_history(&mut tx, model, constraint, OWNER).expect("c"),
    );
    let gid = next();
    let predefined = (version != SchemaVersion::Ifc2x3).then_some("RECTANGULAR");
    written.push(
        ifc::geometry::authoring::grid_with_owner_history(
            &mut tx,
            model,
            &gid,
            Some(PLACEMENT),
            (&[U], &[V], &[]),
            predefined,
            OWNER,
        )
        .expect("grid"),
    );
    let ifc4 = version != SchemaVersion::Ifc2x3;
    let gid = next();
    let mut lining = DoorLiningDraft::new()
        .lining_depth(120.0)
        .lining_thickness(40.0);
    lining.lining_to_panel_offset_x = ifc4.then_some(5.0);
    written.push(
        add_door_lining_properties_with_owner_history(&mut tx, model, &gid, lining, OWNER)
            .expect("dl"),
    );
    let gid = next();
    let mut window = WindowLiningDraft::new()
        .lining_depth(100.0)
        .lining_thickness(50.0);
    window.lining_offset = ifc4.then_some(-5.0);
    written.push(
        add_window_lining_properties_with_owner_history(&mut tx, model, &gid, window, OWNER)
            .expect("wl"),
    );
    let gid = next();
    written.push(
        add_door_panel_properties_with_owner_history(
            &mut tx,
            model,
            &gid,
            Some("Leaf"),
            "SWINGING",
            "LEFT",
            (Some(40.0), Some(0.5)),
            OWNER,
        )
        .expect("dp"),
    );
    if ifc4 {
        let names = for_version(version)
            .unwrap()
            .attribute_names("IFCSIMPLEPROPERTYTEMPLATE");
        let mut simple = vec![Value::Null; names.len()];
        simple[0] = Value::Text(next().into());
        simple[2] = Value::Text("Width".into());
        let simple = tx.create(Entity::new("IFCSIMPLEPROPERTYTEMPLATE", simple));
        let gid = next();
        let complex = add_complex_property_template_with_owner_history(
            &mut tx,
            model,
            &gid,
            Some("Frame"),
            (Some("Frame"), Some("P_COMPLEX")),
            &[],
            OWNER,
        )
        .expect("complex");
        let gid = next();
        let set = add_property_set_template_with_owner_history(
            &mut tx,
            model,
            &gid,
            "Pset_Custom",
            Some("IfcDoor"),
            &[simple, complex],
            OWNER,
        )
        .expect("set template");
        written.extend([complex, set]);
    }
    tx.commit(model).expect("commit");
    written
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
        })
        .map(|finding| format!("{} at {}: {}", finding.rule, finding.path, finding.message))
        .collect()
}

#[test]
fn authored_root_records_validate_in_their_release() {
    for (schema, version) in RELEASES {
        let mut model = base(schema, version);
        let written = author(&mut model, version);
        // Minimum counts: a sweep that silently wrote nothing would pass.
        let floor = if version == SchemaVersion::Ifc2x3 {
            100
        } else {
            230
        };
        assert!(written.len() >= floor, "{schema}: {}", written.len());
        let bytes = StepCodec.write_bytes(&model).expect("written");
        let back = StepCodec.read_bytes(&bytes).expect("read back");
        assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
        let found = errors(&back, version, &written);
        assert!(found.is_empty(), "{schema}:\n  {}", found.join("\n  "));
    }
}

/// The oracle above is only trusted because it fails when it should: an
/// IFC2X3 `IfcWall` with `OwnerHistory` `$`, as the occurrence writer
/// wrote before #202, and the IFC4 eleven-attribute `IfcGrid`, as `grid`
/// writes, are both error findings in IFC2X3.
#[test]
fn the_validator_catches_the_pre_202_records_in_ifc2x3() {
    let mut model = base("IFC2X3", SchemaVersion::Ifc2x3);
    let mut tx = Transaction::new(&model);
    let mut wall = model.get(WALL).unwrap().clone();
    wall.attributes[0] = Value::Text(guid(900).into());
    wall.attributes[1] = Value::Null;
    let wall = tx.create(wall);
    let grid = ifc::geometry::authoring::grid(
        &mut tx,
        &guid(901),
        Some(PLACEMENT),
        (&[U], &[V], &[]),
        None,
    )
    .expect("IFC4 layout");
    tx.commit(&mut model).expect("commit");
    for id in [wall, grid] {
        assert!(
            !errors(&model, SchemaVersion::Ifc2x3, &[id]).is_empty(),
            "{:?} is not IFC2X3",
            model.get(id)
        );
    }
}
