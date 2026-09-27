//! #52: every read resolves against the release `FILE_SCHEMA` declares.
//!
//! The IFC2X3 fixture is the one from openbimrs/ifc#52, parsed from STEP so
//! the header is real. The IFC4 fixture has the same shape and must keep the
//! 0.2.0 answers.

use ifc_model::{Codec, EntityId, Model};
use ifc_systems::{
    long_name_of, ports, schema_of, systems, zones, Attachment, ConnectionGraph, ElementRole,
    NotInSchema, SchemaGap, SchemaResolutionError, SchemaVersion, SystemAnomaly,
};

fn step(schema: &str, data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('t','2026-09-25T00:00:00',(''),(''),'','','');\n\
         FILE_SCHEMA(({schema}));\nENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

const OWNER: &str = "#1=IFCOWNERHISTORY($,$,$,$,$,$,$,0);";

/// The fixture from #52.
const IFC2X3_DATA: &str = "\
#10=IFCSYSTEM('s',#1,'Heating',$,$);
#11=IFCELECTRICALCIRCUIT('c',#1,'Circuit 1',$,$);
#12=IFCZONE('z',#1,'Zone A',$,$);
#20=IFCFLOWSEGMENT('p',#1,'Pipe',$,$,$,$,$);
#21=IFCFLOWSEGMENT('w',#1,'Cable',$,$,$,$,$);
#22=IFCSPACE('r',#1,'R1',$,$,$,$,$,.ELEMENT.,.INTERNAL.,$);
#30=IFCRELASSIGNSTOGROUP('a1',#1,$,$,(#20),$,#10);
#31=IFCRELASSIGNSTOGROUP('a2',#1,$,$,(#21),$,#11);
#32=IFCRELASSIGNSTOGROUP('a3',#1,$,$,(#22),$,#12);";

fn ifc2x3() -> Model {
    step("'IFC2X3'", &format!("{OWNER}\n{IFC2X3_DATA}"))
}

/// The same shape under IFC4: a zone is a system there, and a distribution
/// system replaces the IFC2X3 electrical circuit.
fn ifc4() -> Model {
    step(
        "'IFC4'",
        "#1=IFCOWNERHISTORY($,$,$,$,$,$,$,0);
#10=IFCSYSTEM('s',#1,'Heating',$,$);
#11=IFCDISTRIBUTIONSYSTEM('c',#1,'Circuit 1',$,$,$,.ELECTRICAL.);
#12=IFCZONE('z',#1,'Zone A',$,$,'Long A');
#20=IFCFLOWSEGMENT('p',#1,'Pipe',$,$,$,$,$);
#21=IFCFLOWSEGMENT('w',#1,'Cable',$,$,$,$,$);
#22=IFCSPACE('r',#1,'R1',$,$,$,$,$,.ELEMENT.,.INTERNAL.,$);
#30=IFCRELASSIGNSTOGROUP('a1',#1,$,$,(#20),$,#10);
#31=IFCRELASSIGNSTOGROUP('a2',#1,$,$,(#21),$,#11);
#32=IFCRELASSIGNSTOGROUP('a3',#1,$,$,(#22),$,#12);",
    )
}

fn ids(found: &[ifc_systems::System]) -> Vec<EntityId> {
    found.iter().map(|s| s.id).collect()
}

#[test]
fn the_declared_release_is_reported() {
    assert_eq!(schema_of(&ifc2x3()), Ok(SchemaVersion::Ifc2x3));
    assert_eq!(schema_of(&ifc4()), Ok(SchemaVersion::Ifc4));
    assert_eq!(
        schema_of(&Model::new()),
        Err(SchemaResolutionError::MissingSchema)
    );
    let ifc4x3 = step("'IFC4X3_ADD2'", OWNER);
    assert_eq!(
        schema_of(&ifc4x3),
        Err(SchemaResolutionError::UnsupportedSchema {
            schema: "IFC4X3_ADD2".into()
        })
    );
    let both = step("'IFC2X3','IFC4'", OWNER);
    assert_eq!(
        schema_of(&both),
        Err(SchemaResolutionError::MultipleSchemas { schemas: 2 })
    );
}

#[test]
fn ifc2x3_systems_are_the_system_and_the_circuit_not_the_zone() {
    let (found, anomalies) = systems(&ifc2x3());
    assert_eq!(ids(&found), [EntityId(10), EntityId(11)]);
    assert_eq!(found[1].type_name, "IFCELECTRICALCIRCUIT");
    assert_eq!(found[1].members, [EntityId(21)]);
    // The zone's IfcRelAssignsToGroup names a group that is not a system in
    // IFC2X3; that is the one expected report, not the circuit.
    assert_eq!(
        anomalies,
        [SystemAnomaly::NotASystem {
            relation: EntityId(32),
            group: EntityId(12),
            type_name: "IFCZONE".into(),
        }]
    );
}

#[test]
fn ifc4_keeps_the_zone_as_a_system() {
    let (found, anomalies) = systems(&ifc4());
    assert_eq!(ids(&found), [EntityId(10), EntityId(11), EntityId(12)]);
    assert!(anomalies.is_empty(), "{anomalies:?}");
}

#[test]
fn ifc2x3_zones_have_no_long_name_slot() {
    let model = ifc2x3();
    let (found, anomalies) = zones(&model);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, EntityId(12));
    assert_eq!(found[0].members, [EntityId(22)]);
    assert!(anomalies.is_empty(), "{anomalies:?}");
    // Not "authored empty": the release has no such attribute.
    assert!(matches!(
        long_name_of(&model, EntityId(12)),
        Err(SchemaGap::NotInSchema(NotInSchema {
            entity: EntityId(12),
            schema: SchemaVersion::Ifc2x3,
            ..
        }))
    ));
    // IFC4 reads it.
    let model = ifc4();
    assert_eq!(zones(&model).0[0].long_name.as_deref(), Some("Long A"));
    assert_eq!(
        long_name_of(&model, EntityId(12)),
        Ok(Some("Long A".into()))
    );
    // An undeclared header cannot be bound; the checked accessor says so.
    assert_eq!(
        long_name_of(&Model::new(), EntityId(12)),
        Err(SchemaGap::Schema(SchemaResolutionError::MissingSchema))
    );
}

#[test]
fn a_group_that_is_not_a_system_is_still_an_anomaly_under_ifc2x3() {
    let model = step(
        "'IFC2X3'",
        "#1=IFCOWNERHISTORY($,$,$,$,$,$,$,0);
#13=IFCGROUP('g',#1,'Group',$,$);
#20=IFCFLOWSEGMENT('p',#1,'Pipe',$,$,$,$,$);
#33=IFCRELASSIGNSTOGROUP('a4',#1,$,$,(#20),$,#13);",
    );
    let (found, anomalies) = systems(&model);
    assert!(found.is_empty());
    assert_eq!(
        anomalies,
        [SystemAnomaly::NotASystem {
            relation: EntityId(33),
            group: EntityId(13),
            type_name: "IFCGROUP".into(),
        }]
    );
}

/// IFC2X3 attaches ports only through `IfcRelConnectsPortToElement`, and
/// its `IfcDistributionPort` has eight attributes.
#[test]
fn ifc2x3_ports_attach_and_connect() {
    let model = step(
        "'IFC2X3'",
        "#1=IFCOWNERHISTORY($,$,$,$,$,$,$,0);
#20=IFCFLOWSEGMENT('p',#1,'Pipe',$,$,$,$,$);
#21=IFCFLOWTERMINAL('t',#1,'Radiator',$,$,$,$,$);
#40=IFCDISTRIBUTIONPORT('o',#1,'Out',$,$,$,$,.SOURCE.);
#41=IFCDISTRIBUTIONPORT('i',#1,'In',$,$,$,$,.SINK.);
#50=IFCRELCONNECTSPORTTOELEMENT('e1',#1,$,$,#40,#20);
#51=IFCRELCONNECTSPORTTOELEMENT('e2',#1,$,$,#41,#21);
#60=IFCRELCONNECTSPORTS('c',#1,$,$,#40,#41,$);",
    );
    let (found, anomalies) = ports(&model);
    assert!(anomalies.is_empty(), "{anomalies:?}");
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].element, Some(EntityId(20)));
    assert_eq!(found[0].attachment, Some(Attachment::ConnectsPortToElement));
    let (graph, anomalies) = ConnectionGraph::build(&model);
    assert!(anomalies.is_empty(), "{anomalies:?}");
    assert_eq!(graph.connections().len(), 1);
    assert_eq!(
        ElementRole::of(&model, EntityId(20)),
        Some(ElementRole::Segment)
    );
    assert_eq!(
        ElementRole::of(&model, EntityId(21)),
        Some(ElementRole::Terminal)
    );
}

/// IFC4-only records in an IFC2X3 file are not read with IFC4 ancestry.
///
/// Ports need no such case: both releases declare exactly one `IfcPort`
/// subtype, `IfcDistributionPort`, so port discovery and the connection
/// graph's port check cannot differ between the two tables.
#[test]
fn ifc4_only_records_have_no_ifc2x3_role_system_or_zone_membership() {
    let model = step(
        "'IFC2X3'",
        "#1=IFCOWNERHISTORY($,$,$,$,$,$,$,0);
#20=IFCPIPESEGMENT('p',#1,'Pipe',$,$,$,$,$,$);
#44=IFCSPATIALZONE('sz',#1,'SZ',$,$,$,$,$,$,.NOTDEFINED.);
#11=IFCDISTRIBUTIONSYSTEM('c',#1,'Circuit',$,$,$,.ELECTRICAL.);
#12=IFCZONE('z',#1,'Zone',$,$);
#32=IFCRELASSIGNSTOGROUP('a',#1,$,$,(#44),$,#12);",
    );
    // IfcPipeSegment does not exist in IFC2X3.
    assert_eq!(ElementRole::of(&model, EntityId(20)), None);
    // Nor does IfcDistributionSystem: it is no IFC2X3 system.
    assert!(!ids(&systems(&model).0).contains(&EntityId(11)));
    // IfcSpatialZone is IFC4-only, so it is not a permitted zone member in
    // an IFC2X3 file.
    let (_, anomalies) = zones(&model);
    assert!(
        anomalies.iter().any(|a| matches!(
            a,
            SystemAnomaly::ZoneMemberNotSpatial {
                member: EntityId(44),
                ..
            }
        )),
        "{anomalies:?}"
    );
    // Under IFC4 the same records are a segment, a system, and a zone member.
    let model = step(
        "'IFC4'",
        "#1=IFCOWNERHISTORY($,$,$,$,$,$,$,0);
#20=IFCPIPESEGMENT('p',#1,'Pipe',$,$,$,$,$,$);
#44=IFCSPATIALZONE('sz',#1,'SZ',$,$,$,$,$,$,.NOTDEFINED.);
#12=IFCZONE('z',#1,'Zone',$,$,$);
#32=IFCRELASSIGNSTOGROUP('a',#1,$,$,(#44),$,#12);
#11=IFCDISTRIBUTIONSYSTEM('c',#1,'Circuit',$,$,$,.ELECTRICAL.);",
    );
    assert_eq!(
        ElementRole::of(&model, EntityId(20)),
        Some(ElementRole::Segment)
    );
    assert!(ids(&systems(&model).0).contains(&EntityId(11)));
    assert!(zones(&model).1.is_empty(), "{:?}", zones(&model).1);
}
