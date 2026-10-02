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

/// A wall `#1` defined by property set definitions `#10`, `#11`, ... of
/// the given `(entity, name)`, through one relation per entry of
/// `relations`. A relation naming one definition references it; one naming
/// several writes an `IfcPropertySetDefinitionSet`.
fn object_sets(schema: &Schema, sets: &[(&str, &str)], relations: &[&[u64]]) -> Report {
    let mut model = Model::new();
    model.insert(EntityId(1), wall(schema, GUID_A, &[]));
    for (offset, (set_type, name)) in (10..).zip(sets) {
        model.insert(
            EntityId(offset),
            entity(schema, set_type, &[("Name", text(name))]),
        );
    }
    for definitions in relations {
        let definition = match definitions {
            [one] => Value::Ref(EntityId(*one)),
            many => Value::Typed {
                type_name: "IFCPROPERTYSETDEFINITIONSET".into(),
                value: Box::new(Value::List(
                    many.iter().map(|id| Value::Ref(EntityId(*id))).collect(),
                )),
            },
        };
        model.push(entity(
            schema,
            "IFCRELDEFINESBYPROPERTIES",
            &[
                ("RelatedObjects", Value::List(vec![Value::Ref(EntityId(1))])),
                ("RelatingPropertyDefinition", definition),
            ],
        ));
    }
    validate(&model, schema)
}

/// Two property sets named `first` and `second` on a wall, one relation
/// each, under IFC4.
fn two_object_sets(first: &str, second: &str) -> Report {
    let schema = ifc_schema::ifc4();
    object_sets(
        schema,
        &[("IFCPROPERTYSET", first), ("IFCPROPERTYSET", second)],
        &[&[10], &[11]],
    )
}

/// A wall type whose `HasPropertySets` holds sets named `first` and
/// `second`, under IFC4.
fn type_sets(first: &str, second: &str) -> Report {
    let schema = ifc_schema::ifc4();
    let mut model = Model::new();
    model.insert(
        EntityId(10),
        entity(schema, "IFCPROPERTYSET", &[("Name", text(first))]),
    );
    model.insert(
        EntityId(11),
        entity(schema, "IFCPROPERTYSET", &[("Name", text(second))]),
    );
    model.push(entity(
        schema,
        "IFCWALLTYPE",
        &[
            ("GlobalId", text(GUID_A)),
            ("Name", text("type")),
            (
                "HasPropertySets",
                Value::List(vec![Value::Ref(EntityId(10)), Value::Ref(EntityId(11))]),
            ),
        ],
    ));
    validate(&model, schema)
}

/// A wall type `#1` assigned to an object of `object_type` `#2` by an
/// `IfcRelDefinesByType`, under `schema`.
fn typed_object(schema: &Schema, object_type: &str) -> Report {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        entity(schema, "IFCWALLTYPE", &[("Name", text("type"))]),
    );
    model.insert(EntityId(2), entity(schema, object_type, &[]));
    model.push(entity(
        schema,
        "IFCRELDEFINESBYTYPE",
        &[
            ("RelatingType", Value::Ref(EntityId(1))),
            ("RelatedObjects", Value::List(vec![Value::Ref(EntityId(2))])),
        ],
    ));
    validate(&model, schema)
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
    Case {
        rule: "IfcObject.UniquePropertySetNames",
        form: "two property sets of one name, one relation each",
        fails: || two_object_sets("Pset_A", "Pset_A"),
        passes: || two_object_sets("Pset_A", "Pset_B"),
    },
    Case {
        rule: "IfcObject.UniquePropertySetNames",
        form: "two property sets of one name in an IfcPropertySetDefinitionSet",
        fails: || {
            let sets = [("IFCPROPERTYSET", "Pset_A"), ("IFCPROPERTYSET", "Pset_A")];
            object_sets(ifc_schema::ifc4(), &sets, &[&[10, 11]])
        },
        passes: || {
            let sets = [("IFCPROPERTYSET", "Pset_A"), ("IFCPROPERTYSET", "Pset_B")];
            object_sets(ifc_schema::ifc4(), &sets, &[&[10, 11]])
        },
    },
    Case {
        rule: "IfcObject.UniquePropertySetNames",
        form: "one set attached twice is one member of the SET",
        fails: || two_object_sets("Pset_A", "Pset_A"),
        passes: || {
            let sets = [("IFCPROPERTYSET", "Pset_A")];
            object_sets(ifc_schema::ifc4(), &sets, &[&[10], &[10]])
        },
    },
    Case {
        rule: "IfcObject.UniquePropertySetNames",
        form: "a quantity set counts as unnamed",
        fails: || two_object_sets("Qto_A", "Qto_A"),
        passes: || {
            let sets = [("IFCPROPERTYSET", "Qto_A"), ("IFCELEMENTQUANTITY", "Qto_A")];
            object_sets(ifc_schema::ifc4(), &sets, &[&[10], &[11]])
        },
    },
    Case {
        rule: "IfcObject.UniquePropertySetNames",
        form: "IFC2X3 declares no such rule",
        fails: || {
            let sets = [("IFCPROPERTYSET", "Pset_A"), ("IFCPROPERTYSET", "Pset_A")];
            object_sets(ifc_schema::ifc4x3(), &sets, &[&[10], &[11]])
        },
        passes: || {
            let sets = [("IFCPROPERTYSET", "Pset_A"), ("IFCPROPERTYSET", "Pset_A")];
            object_sets(ifc_schema::ifc2x3(), &sets, &[&[10], &[11]])
        },
    },
    Case {
        rule: "IfcTypeObject.UniquePropertySetNames",
        form: "two property sets of one name on a type",
        fails: || type_sets("Pset_A", "Pset_A"),
        passes: || type_sets("Pset_A", "Pset_B"),
    },
    Case {
        rule: "IfcTypeProduct.ApplicableOccurrence",
        form: "IFC4: a type product assigned to a task",
        fails: || typed_object(ifc_schema::ifc4(), "IFCTASK"),
        passes: || typed_object(ifc_schema::ifc4(), "IFCWALL"),
    },
    Case {
        rule: "IfcTypeProduct.ApplicableOccurrence",
        form: "IFC4X3: a type product assigned to a task",
        fails: || typed_object(ifc_schema::ifc4x3(), "IFCTASK"),
        passes: || typed_object(ifc_schema::ifc4x3(), "IFCWALL"),
    },
    Case {
        rule: "IfcTypeProduct.WR41",
        form: "IFC2X3: a type product assigned to a task",
        fails: || typed_object(ifc_schema::ifc2x3(), "IFCTASK"),
        passes: || typed_object(ifc_schema::ifc2x3(), "IFCWALL"),
    },
];
