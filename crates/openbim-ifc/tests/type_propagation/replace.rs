//! Replacing a type, container or aggregate parent, one transaction, and the cookbook scenario.

use super::*;

// --- replace ---

#[test]
fn replacing_a_type_takes_back_what_the_previous_type_gave() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let neighbour = f.wall(vec![]);
        let op = f.assign(
            f.base.wall_type,
            &[wall, neighbour],
            AssignTypeOptions::propagate(),
        );
        f.apply(vec![op]);
        let old_shape = f.reference(wall, "Representation");
        let old_representation = f.list(old_shape, "Representations")[0];
        let old_item = f.list(old_representation, "Items")[0];
        let old_target = f.reference(old_item, "MappingTarget");
        let old_usage = usage_of(&f, wall);
        let old_relationship = f.relationships("IFCRELDEFINESBYTYPE", "RelatedObjects", wall)[0];
        let neighbour_usage = usage_of(&f, neighbour);

        let op = f.assign(
            f.base.other_wall_type,
            &[wall],
            AssignTypeOptions::propagate().replace(true),
        );
        f.apply(vec![op]);
        assert_eq!(f.type_of(wall), Some(f.base.other_wall_type));
        assert_eq!(f.type_of(neighbour), Some(f.base.wall_type), "{}", f.name());
        assert_eq!(f.list(old_relationship, "RelatedObjects"), vec![neighbour]);
        for gone in [
            old_shape,
            old_representation,
            old_item,
            old_target,
            old_usage,
        ] {
            assert!(
                f.model.get(gone).is_none(),
                "{}: #{} stays",
                f.name(),
                gone.0
            );
        }
        // The previous type keeps its map and set; the neighbour its own.
        assert_eq!(
            f.list(f.base.wall_type, "RepresentationMaps"),
            vec![f.base.wall_map]
        );
        assert!(f.model.get(f.base.wall_set).is_some());
        assert_eq!(usage_of(&f, neighbour), neighbour_usage);
        assert_mapped(&f, wall, f.base.other_wall_map);
        assert_eq!(
            f.reference(usage_of(&f, wall), "ForLayerSet"),
            f.base.other_wall_set
        );
    });
}

#[test]
fn replacing_the_last_object_removes_the_old_relationship_and_keeps_its_own_body() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let door_op = f.product("IfcDoor", vec![("Name", text("Door"))]);
        let door = f.apply(vec![door_op])[0].unwrap();
        let op = f.assign(f.base.wall_type, &[wall], AssignTypeOptions::propagate());
        f.apply(vec![op]);
        let old_relationship = f.relationships("IFCRELDEFINESBYTYPE", "RelatedObjects", wall)[0];
        // Re-typing the same object by the same type with replace maps
        // afresh rather than doubling the representation.
        let op = f.assign(
            f.base.wall_type,
            &[wall],
            AssignTypeOptions::propagate().replace(true),
        );
        f.apply(vec![op]);
        assert!(f.model.get(old_relationship).is_none(), "{}", f.name());
        assert_mapped(&f, wall, f.base.wall_map);
        assert_eq!(
            f.relationships("IFCRELASSOCIATESMATERIAL", "RelatedObjects", wall)
                .len(),
            1
        );
        // Replace on an untyped object is a plain assignment.
        let op = f.assign(
            f.base.door_type,
            &[door],
            AssignTypeOptions::propagate().replace(true),
        );
        f.apply(vec![op]);
        assert_eq!(f.type_of(door), Some(f.base.door_type));
    });
}

#[test]
fn replacing_a_container_moves_the_element() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let old = f.relationships("IFCRELCONTAINEDINSPATIALSTRUCTURE", "RelatedElements", wall);
        assert_eq!(old.len(), 1);
        let contain = |f: &Fixture, options| AuthorOp::Contain {
            structure: f.base.other_storey,
            elements: vec![wall],
            owner_history: Some(f.base.owner),
            options,
        };
        let failure = f.refuse(vec![contain(&f, RelateOptions::new())]);
        assert!(
            matches!(failure, AuthoringFailure::AlreadyRelated { id, relationship: "IFCRELCONTAINEDINSPATIALSTRUCTURE", .. } if id == wall),
            "{}: {failure:?}",
            f.name()
        );
        let op = contain(&f, RelateOptions::new().replace(true));
        let ids = f.apply(vec![op]);
        let now = f.relationships("IFCRELCONTAINEDINSPATIALSTRUCTURE", "RelatedElements", wall);
        assert_eq!(now, vec![ids[0].unwrap()]);
        assert_eq!(
            f.reference(now[0], "RelatingStructure"),
            f.base.other_storey
        );
        assert!(
            f.model.get(old[0]).is_none(),
            "{}: emptied, removed",
            f.name()
        );
    });
}

#[test]
fn replacing_an_aggregate_parent_moves_the_part() {
    each_release(|mut f| {
        let mut b = Batch::default();
        let element = |b: &mut Batch, type_name: &str, container: Option<EntityId>| {
            let mut attributes = vec![("Name", text(type_name))];
            if type_name == "IfcElementAssembly" {
                // IFC2X3 requires it.
                attributes.push(("PredefinedType", enumeration("NOTDEFINED")));
            }
            b.push(AuthorOp::Product {
                type_name: type_name.into(),
                container,
                attributes: named(attributes),
                placement: Some(f.base.placement),
                type_object: None,
                owner_history: Some(f.base.owner),
            })
        };
        let first = element(&mut b, "IfcElementAssembly", Some(f.base.storey));
        let second = element(&mut b, "IfcElementAssembly", Some(f.base.storey));
        let left = element(&mut b, "IfcMember", None);
        let right = element(&mut b, "IfcMember", None);
        let old = b.push(AuthorOp::Aggregate {
            parent: first,
            parts: vec![left, right],
            owner_history: Some(f.base.owner),
            options: RelateOptions::new(),
        });
        let ids = f.apply(b.ops);
        let real =
            |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
        let (second, left, right, old) = (real(second), real(left), real(right), real(old));
        let aggregate = |f: &Fixture, options| AuthorOp::Aggregate {
            parent: second,
            parts: vec![left],
            owner_history: Some(f.base.owner),
            options,
        };
        let failure = f.refuse(vec![aggregate(&f, RelateOptions::new())]);
        assert!(
            matches!(failure, AuthoringFailure::AlreadyRelated { id, relationship: "IFCRELAGGREGATES", existing } if id == left && existing == old),
            "{}: {failure:?}",
            f.name()
        );
        let op = aggregate(&f, RelateOptions::new().replace(true));
        f.apply(vec![op]);
        let now = f.relationships("IFCRELAGGREGATES", "RelatedObjects", left);
        assert_eq!(now.len(), 1);
        assert_eq!(f.reference(now[0], "RelatingObject"), second);
        // The old aggregation still holds the other part.
        assert_eq!(f.list(old, "RelatedObjects"), vec![right], "{}", f.name());
    });
}

// --- one transaction ---

#[test]
fn a_refusal_late_in_the_batch_undoes_the_propagation() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let count = f.model.ids().count();
        let ops = vec![
            f.assign(f.base.wall_type, &[wall], AssignTypeOptions::propagate()),
            f.assign(
                f.base.other_wall_type,
                &[wall],
                AssignTypeOptions::propagate(),
            ),
        ];
        f.refuse(ops);
        assert_eq!(f.model.ids().count(), count);
        assert_eq!(f.type_of(wall), None);
        assert_eq!(f.attr(wall, "Representation"), Value::Null);
    });
}

// --- the cookbook scenario, over the #478 cascade fixtures ---

#[cfg(feature = "step")]
#[test]
fn door_one_is_retyped_and_wall_b_moved_in_the_cascade_fixtures() {
    use ifc::{Codec, StepCodec};
    for (release, name) in [
        (SchemaVersion::Ifc2x3, "cascade_ifc2x3.ifc"),
        (SchemaVersion::Ifc4, "cascade_ifc4.ifc"),
        (SchemaVersion::Ifc4x3, "cascade_ifc4x3.ifc"),
    ] {
        let path = format!(
            "{}/../../test/fixtures/synthetic-relations/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        let mut model = StepCodec
            .read_path(std::path::Path::new(&path))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let schema = ifc::schema::for_version(release).expect("bundled");
        let id = EntityId;
        // Door 1 (#100) keeps its type D-90 (#111), assigned again with
        // replace: the mapped body #105 with its item #106 and operator
        // #107 is taken back and mapped afresh; the type's map #112 stays.
        let ops = vec![
            AuthorOp::AssignType {
                type_object: id(111),
                objects: vec![id(100)],
                owner_history: Some(id(5)),
                options: AssignTypeOptions::new()
                    .map_representations(true)
                    .predefined_type(PredefinedTypeMode::Sync)
                    .replace(true),
            },
            AuthorOp::Contain {
                structure: id(13),
                elements: vec![id(40)],
                owner_history: Some(id(5)),
                options: RelateOptions::new().replace(true),
            },
        ];
        apply_authoring(&mut model, schema, &ops, 480)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_valid(name, &model, schema);
        for gone in [104, 105, 106, 107, 108] {
            assert!(model.get(id(gone)).is_none(), "{name}: #{gone} stays");
        }
        assert!(model.get(id(112)).is_some());
        assert_eq!(ifc::type_of(&model, schema, id(100)), Ok(Some(id(111))));
        let door = model.get(id(100)).unwrap();
        let slot = |name: &str| {
            schema
                .attributes(&door.type_name)
                .iter()
                .position(|attribute| attribute.name == name)
        };
        if let Some(predefined) = slot("PredefinedType") {
            assert_eq!(
                door.attributes[predefined],
                Value::Null,
                "{name}: DOOR left to the type"
            );
        }
        let shape = door.attributes[slot("Representation").unwrap()]
            .as_ref_id()
            .unwrap();
        assert!(shape.0 > 174, "{name}: a fresh product definition shape");
    }
}

// --- joining a type's relationship (#534) ---

#[test]
fn a_second_assignment_joins_the_types_existing_relationship() {
    each_release(|mut f| {
        let first = f.wall(vec![]);
        let second = f.wall(vec![]);
        let third = f.wall(vec![]);
        let op = f.assign(f.base.wall_type, &[first], AssignTypeOptions::new());
        let relationship = f.apply(vec![op])[0].unwrap();
        // Later, and twice in one batch: all join the first relationship.
        let op = f.assign(f.base.wall_type, &[second], AssignTypeOptions::new());
        let joined = f.apply(vec![op])[0].unwrap();
        assert_eq!(joined, relationship, "{}", f.name());
        let op = f.assign(f.base.wall_type, &[third], AssignTypeOptions::propagate());
        f.apply(vec![op]);
        let typing = f.of_type("IFCRELDEFINESBYTYPE");
        assert_eq!(typing, vec![relationship], "{}", f.name());
        assert_eq!(
            f.list(relationship, "RelatedObjects"),
            vec![first, second, third],
            "{}",
            f.name()
        );
    });
}

#[test]
fn assignments_in_one_batch_share_one_relationship() {
    each_release(|mut f| {
        let first = f.wall(vec![]);
        let second = f.wall(vec![]);
        let ops = vec![
            f.assign(f.base.wall_type, &[first], AssignTypeOptions::new()),
            f.assign(f.base.wall_type, &[second], AssignTypeOptions::new()),
        ];
        let ids = f.apply(ops);
        assert_eq!(ids[0], ids[1], "{}", f.name());
        assert_eq!(f.of_type("IFCRELDEFINESBYTYPE").len(), 1, "{}", f.name());
    });
}

#[test]
fn a_typed_object_is_refused_by_another_type_and_the_model_stays() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let op = f.assign(f.base.wall_type, &[wall], AssignTypeOptions::new());
        f.apply(vec![op]);
        let fresh = f.wall(vec![]);
        // `fresh` is fine, `wall` is not: nothing of the batch lands.
        let op = f.assign(
            f.base.other_wall_type,
            &[fresh, wall],
            AssignTypeOptions::new(),
        );
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(failure, AuthoringFailure::AlreadyRelated { id, relationship: "IFCRELDEFINESBYTYPE", .. } if id == wall),
            "{failure:?}"
        );
    });
}

#[test]
fn replacing_into_a_type_with_a_relationship_joins_it() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let neighbour = f.wall(vec![]);
        let ops = vec![
            f.assign(f.base.wall_type, &[wall], AssignTypeOptions::new()),
            f.assign(
                f.base.other_wall_type,
                &[neighbour],
                AssignTypeOptions::new(),
            ),
        ];
        f.apply(ops);
        let op = f.assign(
            f.base.other_wall_type,
            &[wall],
            AssignTypeOptions::new().replace(true),
        );
        f.apply(vec![op]);
        assert_eq!(f.type_of(wall), Some(f.base.other_wall_type));
        assert_eq!(f.of_type("IFCRELDEFINESBYTYPE").len(), 1, "{}", f.name());
    });
}
