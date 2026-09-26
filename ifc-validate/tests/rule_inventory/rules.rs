//! Global and native WHERE-rule ids.

use ifc_model::{EntityId, Model, Value};
use ifc_schema::Schema;
use ifc_validate::{validate, Report};

use super::cases::Case;
use super::fixtures::{entity, ifc4, relation, text, wall, GUID_A, GUID_B};

/// `count` projects with distinct GlobalIds.
fn projects(count: usize) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    for guid in [GUID_A, GUID_B].into_iter().take(count) {
        model.push(entity(schema, "IFCPROJECT", &[("GlobalId", text(guid))]));
    }
    ifc4(&model)
}

/// Two walls with the given GlobalIds, through `validate`.
fn two_walls(first: &str, second: &str) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(wall(schema, first, &[]));
    model.push(wall(schema, second, &[]));
    ifc4(&model)
}

/// A property-set relation whose related object is `related_type`, under
/// `schema`.
fn defines_by_properties(schema: &Schema, related_type: &str) -> Report {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        entity(schema, related_type, &[("GlobalId", text(GUID_A))]),
    );
    model.push(entity(
        schema,
        "IFCRELDEFINESBYPROPERTIES",
        &[("RelatedObjects", Value::List(vec![Value::Ref(EntityId(1))]))],
    ));
    validate(&model, schema)
}

/// A classification reference with the given identity fields set.
fn external_reference(fields: &[(&str, Value)]) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(entity(schema, "IFCCLASSIFICATIONREFERENCE", fields));
    ifc4(&model)
}

/// A sequence from task `#1` to task `#related`, under `schema`.
fn sequence(schema: &Schema, related: u64) -> Report {
    let mut model = Model::new();
    model.insert(EntityId(1), entity(schema, "IFCTASK", &[]));
    model.insert(EntityId(2), entity(schema, "IFCTASK", &[]));
    model.push(entity(
        schema,
        "IFCRELSEQUENCE",
        &[
            ("RelatingProcess", Value::Ref(EntityId(1))),
            ("RelatedProcess", Value::Ref(EntityId(related))),
        ],
    ));
    validate(&model, schema)
}

/// A material layer of `layer_type` whose `Priority` is `priority`.
fn material_layer(layer_type: &str, priority: i64) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(entity(
        schema,
        layer_type,
        &[
            ("LayerThickness", Value::Real(1.0)),
            ("Priority", Value::Integer(priority)),
        ],
    ));
    ifc4(&model)
}

/// A path connection whose `attribute` list holds `priority`.
fn path_connection(attribute: &str, priority: i64) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.push(entity(
        schema,
        "IFCRELCONNECTSPATHELEMENTS",
        &[(attribute, Value::List(vec![Value::Integer(priority)]))],
    ));
    ifc4(&model)
}

/// A 2nd-level space boundary declaring `physicality` against `element`.
fn space_boundary(element: &str, physicality: &str) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        entity(schema, element, &[("GlobalId", text(GUID_A))]),
    );
    model.push(entity(
        schema,
        "IFCRELSPACEBOUNDARY2NDLEVEL",
        &[
            ("RelatedBuildingElement", Value::Ref(EntityId(1))),
            ("PhysicalOrVirtualBoundary", Value::Enum(physicality.into())),
        ],
    ));
    ifc4(&model)
}

/// `relation` with its relating end `#relating` and related object
/// `#related`, under IFC4.
fn self_reference(relation_type: &str, relating: &str, related: u64) -> Report {
    let schema = ifc_schema::ifc4();
    ifc4(&relation(schema, relation_type, relating, 1, related))
}

pub const CASES: &[Case] = &[
    Case {
        rule: "global.IfcSingleProjectInstance",
        form: "two IfcProject instances",
        fails: || projects(2),
        passes: || projects(1),
    },
    Case {
        rule: "global.UniqueGlobalId",
        form: "two roots sharing a GlobalId",
        fails: || two_walls(GUID_A, GUID_A),
        passes: || two_walls(GUID_A, GUID_B),
    },
    Case {
        rule: "IfcRelDefinesByProperties.NoRelatedTypeObject",
        form: "a type object among the related objects",
        fails: || defines_by_properties(ifc_schema::ifc4(), "IFCWALLTYPE"),
        passes: || defines_by_properties(ifc_schema::ifc4(), "IFCWALL"),
    },
    Case {
        rule: "IfcRelDefinesByProperties.NoRelatedTypeObject",
        form: "IFC2X3 declares no such rule",
        fails: || defines_by_properties(ifc_schema::ifc4x3(), "IFCWALLTYPE"),
        passes: || defines_by_properties(ifc_schema::ifc2x3(), "IFCWALLTYPE"),
    },
    Case {
        rule: "IfcExternalReference.WR1",
        form: "no identification, location or name",
        fails: || external_reference(&[]),
        passes: || external_reference(&[("Identification", text("21.01"))]),
    },
    Case {
        rule: "IfcRelSequence.WR1",
        form: "IFC2X3: a sequence from a task to itself",
        fails: || sequence(ifc_schema::ifc2x3(), 1),
        passes: || sequence(ifc_schema::ifc2x3(), 2),
    },
    Case {
        rule: "IfcRelSequence.AvoidInconsistentSequence",
        form: "IFC4: a sequence from a task to itself",
        fails: || sequence(ifc_schema::ifc4(), 1),
        passes: || sequence(ifc_schema::ifc4(), 2),
    },
    Case {
        rule: "IfcRelAggregates.NoSelfReference",
        form: "a whole that is its own part",
        fails: || self_reference("IFCRELAGGREGATES", "RelatingObject", 1),
        passes: || self_reference("IFCRELAGGREGATES", "RelatingObject", 2),
    },
    Case {
        rule: "IfcRelNests.NoSelfReference",
        form: "a host nested in itself",
        fails: || self_reference("IFCRELNESTS", "RelatingObject", 1),
        passes: || self_reference("IFCRELNESTS", "RelatingObject", 2),
    },
    Case {
        rule: "IfcMaterialLayer.NormalizedPriority",
        form: "priority above 100",
        fails: || material_layer("IFCMATERIALLAYER", 101),
        passes: || material_layer("IFCMATERIALLAYER", 100),
    },
    Case {
        rule: "IfcMaterialLayer.NormalizedPriority",
        form: "priority below 0",
        fails: || material_layer("IFCMATERIALLAYER", -1),
        passes: || material_layer("IFCMATERIALLAYER", 0),
    },
    Case {
        rule: "IfcMaterialLayer.NormalizedPriority",
        form: "the IfcMaterialLayerWithOffsets subtype inherits the rule",
        fails: || material_layer("IFCMATERIALLAYERWITHOFFSETS", 101),
        passes: || material_layer("IFCMATERIALLAYERWITHOFFSETS", 100),
    },
    Case {
        rule: "IfcRelAssignsToActor.NoSelfReference",
        form: "an object assigned to itself",
        fails: || self_reference("IFCRELASSIGNSTOACTOR", "RelatingActor", 1),
        passes: || self_reference("IFCRELASSIGNSTOACTOR", "RelatingActor", 2),
    },
    Case {
        rule: "IfcRelAssignsToProcess.NoSelfReference",
        form: "an object assigned to itself",
        fails: || self_reference("IFCRELASSIGNSTOPROCESS", "RelatingProcess", 1),
        passes: || self_reference("IFCRELASSIGNSTOPROCESS", "RelatingProcess", 2),
    },
    Case {
        rule: "IfcRelAssignsToProduct.NoSelfReference",
        form: "an object assigned to itself",
        fails: || self_reference("IFCRELASSIGNSTOPRODUCT", "RelatingProduct", 1),
        passes: || self_reference("IFCRELASSIGNSTOPRODUCT", "RelatingProduct", 2),
    },
    Case {
        rule: "IfcRelAssignsToGroup.NoSelfReference",
        form: "an object assigned to itself",
        fails: || self_reference("IFCRELASSIGNSTOGROUP", "RelatingGroup", 1),
        passes: || self_reference("IFCRELASSIGNSTOGROUP", "RelatingGroup", 2),
    },
    Case {
        rule: "IfcRelAssignsToGroup.NoSelfReference",
        form: "the IfcRelAssignsToGroupByFactor subtype inherits the rule",
        fails: || self_reference("IFCRELASSIGNSTOGROUPBYFACTOR", "RelatingGroup", 1),
        passes: || self_reference("IFCRELASSIGNSTOGROUPBYFACTOR", "RelatingGroup", 2),
    },
    Case {
        rule: "IfcRelConnectsPathElements.NormalizedRelatingPriorities",
        form: "a relating priority above 100",
        fails: || path_connection("RelatingPriorities", 101),
        passes: || path_connection("RelatingPriorities", 100),
    },
    Case {
        rule: "IfcRelConnectsPathElements.NormalizedRelatedPriorities",
        form: "a related priority below 0",
        fails: || path_connection("RelatedPriorities", -1),
        passes: || path_connection("RelatedPriorities", 0),
    },
    Case {
        rule: "IfcRelSpaceBoundary.CorrectPhysOrVirt",
        form: "PHYSICAL against a virtual element",
        fails: || space_boundary("IFCVIRTUALELEMENT", "PHYSICAL"),
        passes: || space_boundary("IFCWALL", "PHYSICAL"),
    },
    Case {
        rule: "IfcRelSpaceBoundary.CorrectPhysOrVirt",
        form: "VIRTUAL against a physical element",
        fails: || space_boundary("IFCWALL", "VIRTUAL"),
        passes: || space_boundary("IFCOPENINGELEMENT", "VIRTUAL"),
    },
];
