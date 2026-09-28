//! #215: the system, port, connectivity, flow and placement readers under
//! IFC4X3, each verified against the IFC4X3 ADD2 table.
//!
//! Every record is laid out by attribute name from the bundled IFC4X3 table,
//! written to STEP text with an `IFC4X3_ADD2` header and read back, so what
//! is checked is the reader on a real IFC4X3 file, not an in-memory shape.

use std::sync::Arc;

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use ifc_schema::{ifc4x3, Schema};
use ifc_step::StepCodec;
use ifc_systems::{
    ports, role_inconsistencies, schema_of, spatial_placements, systems, Attachment,
    ConnectionGraph, ElementRole, FlowDirection, SchemaResolutionError, SchemaVersion,
    SystemAnomaly,
};

/// `type_name` laid out by name in `schema`; unnamed slots are `$`.
fn named(schema: &Schema, type_name: &str, fields: &[(&str, Value)]) -> Entity {
    let names = schema.attribute_names(type_name);
    assert!(!names.is_empty(), "{type_name} is declared");
    let mut values = vec![Value::Null; names.len()];
    for (name, value) in fields {
        let slot = names
            .iter()
            .position(|declared| declared.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("{type_name}.{name}"));
        values[slot] = value.clone();
    }
    Entity::new(type_name, values)
}

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}

fn id(n: usize) -> Value {
    Value::Text(Arc::from(format!("0000000000000000000{n:03}")))
}

fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

/// Ids of the records `network` pushes, by role.
struct Ids {
    system: EntityId,
    built: EntityId,
    pump: EntityId,
    pipe: EntityId,
    outlet: EntityId,
    inlet: EntityId,
    connection: EntityId,
    storey: EntityId,
    not_a_port: EntityId,
}

/// A pump nesting an outlet port, a pipe attached to an inlet port by the
/// legacy relationship, the two ports connected, both elements in a
/// distribution system, an IFC4X3-only `IfcBuiltSystem`, and the pump
/// contained in a storey.
fn network() -> (Model, Ids) {
    let schema = ifc4x3();
    let mut model = Model::new();
    let owner = model.push(Entity::new(
        "IFCOWNERHISTORY",
        vec![Value::Null; schema.attribute_names("IFCOWNERHISTORY").len()],
    ));
    let rooted = |n: usize, name: &str| {
        vec![
            ("GlobalId", id(n)),
            ("OwnerHistory", Value::Ref(owner)),
            ("Name", text(name)),
        ]
    };
    let push = |model: &mut Model, type_name: &str, fields: Vec<(&str, Value)>| {
        model.push(named(schema, type_name, &fields))
    };

    let pump = push(&mut model, "IFCPUMP", rooted(1, "Pump"));
    let pipe = push(&mut model, "IFCPIPESEGMENT", rooted(2, "Pipe"));
    let mut outlet_fields = rooted(3, "Outlet");
    outlet_fields.push(("FlowDirection", Value::Enum(Arc::from("SOURCE"))));
    let outlet = push(&mut model, "IFCDISTRIBUTIONPORT", outlet_fields);
    let mut inlet_fields = rooted(4, "Inlet");
    inlet_fields.push(("FlowDirection", Value::Enum(Arc::from("SINK"))));
    let inlet = push(&mut model, "IFCDISTRIBUTIONPORT", inlet_fields);
    let mut nests = rooted(5, "Nests");
    nests.extend([
        ("RelatingObject", Value::Ref(pump)),
        ("RelatedObjects", refs(&[outlet])),
    ]);
    push(&mut model, "IFCRELNESTS", nests);
    let mut legacy = rooted(6, "PortToElement");
    legacy.extend([
        ("RelatingPort", Value::Ref(inlet)),
        ("RelatedElement", Value::Ref(pipe)),
    ]);
    push(&mut model, "IFCRELCONNECTSPORTTOELEMENT", legacy);
    let mut connects = rooted(7, "Connects");
    connects.extend([
        ("RelatingPort", Value::Ref(outlet)),
        ("RelatedPort", Value::Ref(inlet)),
    ]);
    let connection = push(&mut model, "IFCRELCONNECTSPORTS", connects);
    // A connection naming a non-port is reported, not followed.
    let mut bad = rooted(8, "Bad");
    bad.extend([
        ("RelatingPort", Value::Ref(outlet)),
        ("RelatedPort", Value::Ref(pipe)),
    ]);
    let not_a_port = push(&mut model, "IFCRELCONNECTSPORTS", bad);

    let mut system_fields = rooted(9, "Heating");
    system_fields.push(("PredefinedType", Value::Enum(Arc::from("HEATING"))));
    let system = push(&mut model, "IFCDISTRIBUTIONSYSTEM", system_fields);
    let mut built_fields = rooted(10, "Facade");
    built_fields.push(("PredefinedType", Value::Enum(Arc::from("FENESTRATION"))));
    let built = push(&mut model, "IFCBUILTSYSTEM", built_fields);
    let mut assigns = rooted(11, "Members");
    assigns.extend([
        ("RelatedObjects", refs(&[pump, pipe])),
        ("RelatingGroup", Value::Ref(system)),
    ]);
    push(&mut model, "IFCRELASSIGNSTOGROUP", assigns);

    let storey = push(&mut model, "IFCBUILDINGSTOREY", rooted(12, "Level 1"));
    let mut contained = rooted(13, "Contained");
    contained.extend([
        ("RelatedElements", refs(&[pump])),
        ("RelatingStructure", Value::Ref(storey)),
    ]);
    push(&mut model, "IFCRELCONTAINEDINSPATIALSTRUCTURE", contained);
    let mut referenced = rooted(14, "Referenced");
    referenced.extend([
        ("RelatedElements", refs(&[pipe])),
        ("RelatingStructure", Value::Ref(storey)),
    ]);
    push(&mut model, "IFCRELREFERENCEDINSPATIALSTRUCTURE", referenced);

    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let bytes = StepCodec.write_bytes(&model).expect("written");
    let back = StepCodec.read_bytes(&bytes).expect("read back");
    assert!(back.diagnostics().is_empty(), "{:?}", back.diagnostics());
    (
        back,
        Ids {
            system,
            built,
            pump,
            pipe,
            outlet,
            inlet,
            connection,
            storey,
            not_a_port,
        },
    )
}

#[test]
fn ifc4x3_binds_its_own_table() {
    let (model, _) = network();
    assert_eq!(schema_of(&model), Ok(SchemaVersion::Ifc4x3));
}

#[test]
fn ifc4x3_systems_include_the_built_system_and_their_members() {
    let (model, ids) = network();
    let (found, anomalies) = systems(&model).unwrap();
    assert_eq!(
        found.iter().map(|s| s.id).collect::<Vec<_>>(),
        [ids.system, ids.built]
    );
    assert_eq!(found[0].type_name, "IFCDISTRIBUTIONSYSTEM");
    assert_eq!(found[0].name.as_deref(), Some("Heating"));
    assert_eq!(found[0].members, [ids.pump, ids.pipe]);
    assert_eq!(found[1].type_name, "IFCBUILTSYSTEM");
    assert!(found[1].members.is_empty());
    assert!(anomalies.is_empty(), "{anomalies:?}");
}

#[test]
fn ifc4x3_ports_resolve_both_attachments_and_directions() {
    let (model, ids) = network();
    let (found, anomalies) = ports(&model).unwrap();
    assert!(anomalies.is_empty(), "{anomalies:?}");
    assert_eq!(found.len(), 2);
    let outlet = found.iter().find(|p| p.id == ids.outlet).unwrap();
    assert_eq!(outlet.element, Some(ids.pump));
    assert_eq!(outlet.attachment, Some(Attachment::Nests));
    assert_eq!(outlet.flow, FlowDirection::Source);
    assert_eq!(outlet.name.as_deref(), Some("Outlet"));
    let inlet = found.iter().find(|p| p.id == ids.inlet).unwrap();
    assert_eq!(inlet.element, Some(ids.pipe));
    assert_eq!(inlet.attachment, Some(Attachment::ConnectsPortToElement));
    assert_eq!(inlet.flow, FlowDirection::Sink);
}

#[test]
fn ifc4x3_connections_and_roles_read_against_the_ifc4x3_table() {
    let (model, ids) = network();
    let (graph, anomalies) = ConnectionGraph::build(&model).unwrap();
    assert_eq!(graph.connections().len(), 1);
    assert_eq!(graph.connections()[0].id, ids.connection);
    assert_eq!(graph.neighbours(ids.outlet), [ids.inlet]);
    assert_eq!(
        anomalies,
        [SystemAnomaly::NotAPort {
            relation: ids.not_a_port,
            entity: ids.pipe,
            type_name: "IFCPIPESEGMENT".to_owned(),
        }]
    );

    assert_eq!(
        ElementRole::of(&model, ids.pump),
        Ok(Some(ElementRole::MovingDevice))
    );
    assert_eq!(
        ElementRole::of(&model, ids.pipe),
        Ok(Some(ElementRole::Segment))
    );
    assert_eq!(ElementRole::of(&model, ids.storey), Ok(None));

    let (found, _) = ports(&model).unwrap();
    assert_eq!(role_inconsistencies(&model, &found), Ok(Vec::new()));
}

#[test]
fn ifc4x3_spatial_placements_keep_containment_and_reference_apart() {
    let (model, ids) = network();
    let (placements, anomalies) = spatial_placements(&model).unwrap();
    assert!(anomalies.is_empty());
    assert_eq!(placements[&ids.pump].contained_in, Some(ids.storey));
    assert!(placements[&ids.pump].referenced_in.is_empty());
    assert_eq!(placements[&ids.pipe].contained_in, None);
    assert_eq!(placements[&ids.pipe].referenced_in, [ids.storey]);
}

/// The same file declaring an unverified or no release is refused by every
/// reader with the same error; none reads it as IFC4.
#[test]
fn every_reader_refuses_an_unbound_release() {
    let (mut model, ids) = network();
    for (schema, expected) in [
        (
            vec!["IFC4X1".to_owned()],
            SchemaResolutionError::UnsupportedSchema {
                schema: "IFC4X1".to_owned(),
            },
        ),
        (Vec::new(), SchemaResolutionError::MissingSchema),
    ] {
        model.header_mut().schema = schema;
        assert_eq!(systems(&model).unwrap_err(), expected);
        assert_eq!(ports(&model).unwrap_err(), expected);
        assert_eq!(ConnectionGraph::build(&model).unwrap_err(), expected);
        assert_eq!(ElementRole::of(&model, ids.pump).unwrap_err(), expected);
        assert_eq!(role_inconsistencies(&model, &[]).unwrap_err(), expected);
        assert_eq!(spatial_placements(&model).unwrap_err(), expected);
        assert_eq!(ifc_systems::zones(&model).unwrap_err(), expected);
    }
}
