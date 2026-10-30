//! Material assignment resolution: direct vs. type-inherited, and ambiguity.

use ifc_material::{
    AssignmentSource, MaterialDefinition, MaterialIndex, MaterialView, ResolvedMaterialSelect,
};
use ifc_model::{Entity, EntityId, Model, Value};

fn text(v: &str) -> Value {
    Value::Text(v.into())
}
fn refs(ids: &[u64]) -> Value {
    Value::List(ids.iter().map(|id| Value::Ref(EntityId(*id))).collect())
}
fn relation(kind: &str, related: &[u64], target: u64) -> Entity {
    Entity::new(
        kind,
        vec![
            text("gid"),
            Value::Null,
            Value::Null,
            Value::Null,
            refs(related),
            Value::Ref(EntityId(target)),
        ],
    )
}

#[test]
fn occurrence_assignment_overrides_type_and_type_is_fallback() {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCMATERIAL", vec![text("Concrete")]),
    );
    model.insert(EntityId(2), Entity::new("IFCMATERIAL", vec![text("Wood")]));
    model.insert(EntityId(10), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(12), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(11), Entity::new("IFCWALLTYPE", vec![]));
    model.insert(EntityId(20), relation("IFCRELASSOCIATESMATERIAL", &[11], 2));
    model.insert(EntityId(21), relation("IFCRELDEFINESBYTYPE", &[10, 12], 11));
    model.insert(EntityId(22), relation("IFCRELASSOCIATESMATERIAL", &[10], 1));
    let view = MaterialView::new(&model);
    let direct = view.assigned_material(EntityId(10)).unwrap().unwrap();
    assert_eq!(direct.source, AssignmentSource::Occurrence);
    let ResolvedMaterialSelect::Definition(MaterialDefinition::Material(material)) =
        direct.material
    else {
        panic!("expected material")
    };
    assert_eq!(material.name().unwrap(), "Concrete");
    let inherited = view.assigned_material(EntityId(12)).unwrap().unwrap();
    assert_eq!(inherited.source, AssignmentSource::Type(EntityId(11)));
    let ResolvedMaterialSelect::Definition(MaterialDefinition::Material(material)) =
        inherited.material
    else {
        panic!("expected material")
    };
    assert_eq!(material.name().unwrap(), "Wood");
}

#[test]
fn mixed_case_ifcxml_type_names_resolve_like_step_names() {
    let mut model = Model::new();
    model.insert(EntityId(1), Entity::new("IfcMaterial", vec![text("Wood")]));
    model.insert(EntityId(10), Entity::new("IfcWall", vec![]));
    model.insert(EntityId(11), Entity::new("IfcWallType", vec![]));
    model.insert(EntityId(20), relation("IFCRELDEFINESBYTYPE", &[10], 11));
    model.insert(EntityId(21), relation("IFCRELASSOCIATESMATERIAL", &[11], 1));

    let view = MaterialView::new(&model);
    assert!(matches!(
        view.resolve_material_select(EntityId(1)),
        Ok(ResolvedMaterialSelect::Definition(
            MaterialDefinition::Material(_)
        ))
    ));
    let inherited = view.assigned_material(EntityId(10)).unwrap().unwrap();
    assert_eq!(inherited.source, AssignmentSource::Type(EntityId(11)));
}

// --- lookups read the relationships that name the object (#608) ---

/// Run `check` on a bare model and on a copy that keeps its reverse index.
fn both(model: &Model, check: impl Fn(MaterialView<'_>)) {
    let bare = model.clone();
    check(MaterialView::new(&bare));
    assert!(bare.reverse_index_if_built().is_none(), "left an index");
    let indexed = model.clone();
    indexed.reverse_index();
    check(MaterialView::new(&indexed));
}

/// What `assigned_material` and `assignments_for` answer for every entity.
fn answers(view: MaterialView<'_>) -> Vec<String> {
    let index = MaterialIndex::new(view);
    view.model()
        .ids()
        .map(|id| {
            let ids =
                |found: ifc_material::MaterialResult<Vec<ifc_material::MaterialAssignment<'_>>>| {
                    found.map(|found| found.iter().map(|a| a.id()).collect::<Vec<_>>())
                };
            let assignments = ids(view.assignments_for(id));
            let line = format!("{id}: {:?} {assignments:?}", view.assigned_material(id));
            let indexed = format!(
                "{id}: {:?} {:?}",
                index.assigned_material(id),
                ids(index.assignments_for(id))
            );
            assert_eq!(line, indexed, "the index answers as the view does");
            line
        })
        .collect()
}

fn same_with_and_without_the_index(model: &Model) -> Vec<String> {
    let bare = model.clone();
    let indexed = model.clone();
    indexed.reverse_index();
    let without = answers(MaterialView::new(&bare));
    assert_eq!(without, answers(MaterialView::new(&indexed)));
    without
}

fn wall_and_type_model() -> Model {
    let mut model = Model::new();
    model.insert(
        EntityId(1),
        Entity::new("IFCMATERIAL", vec![text("Concrete")]),
    );
    model.insert(EntityId(2), Entity::new("IFCMATERIAL", vec![text("Wood")]));
    model.insert(EntityId(10), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(12), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(11), Entity::new("IFCWALLTYPE", vec![]));
    model.insert(EntityId(20), relation("IFCRELASSOCIATESMATERIAL", &[11], 2));
    model.insert(EntityId(21), relation("IFCRELDEFINESBYTYPE", &[10, 12], 11));
    model.insert(EntityId(22), relation("IFCRELASSOCIATESMATERIAL", &[10], 1));
    model
}

#[test]
fn the_index_changes_no_answer() {
    let mut mixed = Model::new();
    mixed.insert(EntityId(1), Entity::new("IfcMaterial", vec![text("Wood")]));
    mixed.insert(EntityId(10), Entity::new("IfcWall", vec![]));
    mixed.insert(EntityId(11), Entity::new("IfcWallType", vec![]));
    mixed.insert(EntityId(20), relation("IFCRELDEFINESBYTYPE", &[10], 11));
    mixed.insert(EntityId(21), relation("IFCRELASSOCIATESMATERIAL", &[11], 1));

    let mut ambiguous = wall_and_type_model();
    ambiguous.insert(EntityId(23), relation("IFCRELASSOCIATESMATERIAL", &[10], 2));
    ambiguous.insert(EntityId(24), relation("IFCRELDEFINESBYTYPE", &[12], 11));

    let mut malformed = wall_and_type_model();
    malformed.insert(
        EntityId(30),
        Entity::new(
            "IFCRELASSOCIATESMATERIAL",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(10)),
            ],
        ),
    );

    let mut resolved = 0;
    for model in [wall_and_type_model(), mixed, ambiguous, malformed] {
        let answers = same_with_and_without_the_index(&model);
        resolved += answers.iter().filter(|a| a.contains("Ok(Some")).count();
    }
    assert!(resolved >= 4, "{resolved} resolved assignments");
}

#[test]
fn the_index_changes_no_answer_on_the_fixtures() {
    use ifc_model::Codec;
    let mut resolved = 0;
    for name in [
        "ifclite-geometry/bath_csg_solid.ifc",
        "synthetic-colour/material_colour.ifc",
        "synthetic-colour/material_colour_ifc2x3.ifc",
        "synthetic-colour/material_colour_ifc4x3.ifc",
        "synthetic-relations/cascade_ifc2x3.ifc",
        "synthetic-relations/cascade_ifc4.ifc",
        "synthetic-relations/cascade_ifc4x3.ifc",
        "synthetic-relations/relations_ifc4.ifc",
        "synthetic-relations/selector_ifc2x3.ifc",
        "synthetic-relations/selector_ifc4.ifc",
        "synthetic-relations/selector_ifc4x3.ifc",
    ] {
        let path = format!("{}/../../test/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
        let model = ifc_step::StepCodec
            .read_bytes(&std::fs::read(path).expect("fixture"))
            .expect("parses");
        let answers = same_with_and_without_the_index(&model);
        resolved += answers.iter().filter(|a| a.contains("Ok(Some")).count();
    }
    assert!(resolved >= 3, "{resolved} resolved assignments");
}

#[test]
fn a_relationship_not_naming_the_object_cannot_refuse_it() {
    let mut model = wall_and_type_model();
    // Empty, non-list and out-of-range `RelatedObjects` that do not
    // reference wall #10; one that holds #10 in another slot.
    model.insert(EntityId(30), relation("IFCRELASSOCIATESMATERIAL", &[], 1));
    model.insert(
        EntityId(31),
        Entity::new(
            "IFCRELASSOCIATESMATERIAL",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Ref(EntityId(99)),
                Value::Ref(EntityId(10)),
            ],
        ),
    );
    model.insert(
        EntityId(32),
        relation("IFCRELASSOCIATESMATERIAL", &[12], 10),
    );
    model.insert(EntityId(33), relation("IFCRELDEFINESBYTYPE", &[], 11));
    model.insert(EntityId(34), relation("IFCRELDEFINESBYTYPE", &[12], 10));
    both(&model, |view| {
        let direct = view.assigned_material(EntityId(10)).unwrap().unwrap();
        assert_eq!(direct.source, AssignmentSource::Occurrence);
        assert_eq!(
            view.assignments_for(EntityId(10)).unwrap().len(),
            1,
            "only #22 names it"
        );
    });
}

#[test]
fn a_malformed_relationship_naming_the_object_still_refuses_it() {
    let bad = |kind: &str, related: Value| {
        Entity::new(
            kind,
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                related,
                Value::Ref(EntityId(1)),
            ],
        )
    };
    for (kind, related) in [
        ("IFCRELASSOCIATESMATERIAL", Value::Ref(EntityId(10))),
        ("IFCRELDEFINESBYTYPE", Value::Ref(EntityId(10))),
        (
            "IFCRELASSOCIATESMATERIAL",
            Value::List(vec![Value::Ref(EntityId(10)), Value::Text("x".into())]),
        ),
    ] {
        let mut model = Model::new();
        model.insert(EntityId(1), Entity::new("IFCMATERIAL", vec![text("M")]));
        model.insert(EntityId(10), Entity::new("IFCWALL", vec![]));
        model.insert(EntityId(30), bad(kind, related));
        both(&model, |view| {
            assert!(view.assigned_material(EntityId(10)).is_err(), "{kind}");
        });
        // Other objects are untouched.
        both(&model, |view| {
            assert!(
                matches!(view.assigned_material(EntityId(1)), Ok(None)),
                "{kind}"
            );
        });
    }
}

#[test]
fn two_relationships_on_one_object_are_ambiguous_on_both_paths() {
    let mut model = wall_and_type_model();
    model.insert(EntityId(23), relation("IFCRELASSOCIATESMATERIAL", &[10], 2));
    model.insert(EntityId(24), relation("IFCRELDEFINESBYTYPE", &[12], 11));
    both(&model, |view| {
        assert!(matches!(
            view.assigned_material(EntityId(10)),
            Err(ifc_material::MaterialError::AmbiguousAssignment { count: 2, .. })
        ));
        assert!(matches!(
            view.assigned_material(EntityId(12)),
            Err(ifc_material::MaterialError::AmbiguousType { count: 2, .. })
        ));
        let ids: Vec<EntityId> = view
            .assignments_for(EntityId(10))
            .unwrap()
            .iter()
            .map(|a| a.id())
            .collect();
        assert_eq!(ids, [EntityId(22), EntityId(23)], "ascending");
    });
}

#[test]
fn several_relationships_are_read_in_ascending_id() {
    let mut model = wall_and_type_model();
    // Inserted at higher ids first.
    model.insert(EntityId(41), relation("IFCRELASSOCIATESMATERIAL", &[10], 2));
    model.insert(EntityId(40), relation("IFCRELASSOCIATESMATERIAL", &[10], 2));
    both(&model, |view| {
        let ids: Vec<EntityId> = view
            .assignments_for(EntityId(10))
            .unwrap()
            .iter()
            .map(|a| a.id())
            .collect();
        assert_eq!(ids, [EntityId(22), EntityId(40), EntityId(41)]);
    });
}

#[test]
fn the_material_index_leaves_no_model_index() {
    let model = wall_and_type_model();
    let index = MaterialIndex::new(MaterialView::new(&model));
    assert!(index.assigned_material(EntityId(12)).unwrap().is_some());
    assert!(model.reverse_index_if_built().is_none(), "left an index");
}

#[test]
fn material_index_is_send_and_sync() {
    fn check<T: Send + Sync + Clone + std::fmt::Debug>() {}
    check::<MaterialIndex<'static>>();
}

#[test]
fn a_malformed_shared_list_refuses_every_object_it_names_through_the_index() {
    let mut model = wall_and_type_model();
    model.insert(EntityId(13), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(14), Entity::new("IFCWALL", vec![]));
    model.insert(EntityId(15), relation("IFCRELASSOCIATESMATERIAL", &[14], 1));
    model.insert(
        EntityId(30),
        Entity::new(
            "IFCRELDEFINESBYTYPE",
            vec![
                Value::Null,
                Value::Null,
                Value::Null,
                Value::Null,
                Value::List(vec![
                    Value::Ref(EntityId(12)),
                    Value::Ref(EntityId(13)),
                    Value::Text("x".into()),
                ]),
                Value::Ref(EntityId(11)),
            ],
        ),
    );
    both(&model, |view| {
        let index = MaterialIndex::new(view);
        for wall in [12, 13] {
            let through_view = view.assigned_material(EntityId(wall));
            assert!(through_view.is_err(), "wall {wall}");
            assert_eq!(
                format!("{through_view:?}"),
                format!("{:?}", index.assigned_material(EntityId(wall))),
            );
        }
        assert!(index.assigned_material(EntityId(14)).unwrap().is_some());
    });
}
