//! Unit tests for the unreachable-product lint.
//!
//! The interesting half is what must *not* be reported: a lint that fires on
//! openings or space boundaries is one a user switches off, at which point it
//! protects nobody. Each false-positive case below is a real pattern measured
//! in `AC20-FZK-Haus.ifc`.

use ifc_model::{Entity, EntityId, Model, Value};

use super::{unreachable_products, Unreachable};

/// Model under construction with increasing ids, mirroring how a STEP file
/// numbers its records.
struct Builder {
    model: Model,
    next: u64,
}

impl Builder {
    fn new() -> Self {
        Self {
            model: Model::new(),
            next: 1,
        }
    }

    fn add(&mut self, type_name: &str, attributes: Vec<Value>) -> EntityId {
        let id = EntityId(self.next);
        self.next += 1;
        self.model.insert(
            id,
            Entity {
                type_name: type_name.into(),
                attributes,
            },
        );
        id
    }

    /// `IfcRoot`'s four slots, so products land at the right offsets.
    fn root_slots() -> Vec<Value> {
        vec![
            Value::Text("guid".into()),
            Value::Null,
            Value::Null,
            Value::Null,
        ]
    }

    /// A representation context, optionally carrying a target view.
    fn context(&mut self, target_view: Option<&str>) -> EntityId {
        let mut a = vec![Value::Null; 10];
        a[0] = Value::Text("Body".into());
        a[1] = Value::Text("Model".into());
        if let Some(view) = target_view {
            a[8] = Value::Enum(view.into());
        }
        self.add("IFCGEOMETRICREPRESENTATIONSUBCONTEXT", a)
    }

    /// An `IfcProductDefinitionShape` holding one representation in `context`.
    fn shape(&mut self, context: Option<EntityId>) -> EntityId {
        let mut r = vec![Value::Null; 4];
        if let Some(c) = context {
            r[0] = Value::Ref(c);
        }
        r[1] = Value::Text("Body".into());
        r[2] = Value::Text("SweptSolid".into());
        let representation = self.add("IFCSHAPEREPRESENTATION", r);

        let mut s = vec![Value::Null; 3];
        s[2] = Value::List(vec![Value::Ref(representation)]);
        self.add("IFCPRODUCTDEFINITIONSHAPE", s)
    }

    /// A product carrying `shape`, or none.
    fn product(&mut self, type_name: &str, shape: Option<EntityId>) -> EntityId {
        let mut a = Self::root_slots();
        a.push(Value::Null); // 4 ObjectType
        a.push(Value::Null); // 5 ObjectPlacement
        a.push(shape.map_or(Value::Null, Value::Ref)); // 6 Representation
        self.add(type_name, a)
    }

    /// Place `product` into a new storey via containment.
    fn contain(&mut self, product: EntityId) {
        let storey = self.add("IFCBUILDINGSTOREY", Self::root_slots());
        let mut a = vec![Value::Null; 6];
        a[4] = Value::List(vec![Value::Ref(product)]);
        a[5] = Value::Ref(storey);
        self.add("IFCRELCONTAINEDINSPATIALSTRUCTURE", a);
    }

    /// A drawable product in the model context, uncontained.
    fn drawable(&mut self, type_name: &str, view: Option<&str>) -> EntityId {
        let context = self.context(view);
        let shape = self.shape(Some(context));
        self.product(type_name, Some(shape))
    }
}

#[test]
fn an_orphan_product_with_geometry_is_reported() {
    let mut b = Builder::new();
    let wall = b.drawable("IFCWALL", Some(".MODEL_VIEW."));

    assert_eq!(
        unreachable_products(&b.model),
        vec![(wall, Unreachable::NotContainedInSpatialStructure)],
        "a wall no relationship attaches to the tree is invisible"
    );
}

#[test]
fn a_contained_product_is_not_reported() {
    let mut b = Builder::new();
    let wall = b.drawable("IFCWALL", Some(".MODEL_VIEW."));
    b.contain(wall);

    assert!(
        unreachable_products(&b.model).is_empty(),
        "a contained wall in the model context is drawable"
    );
}

#[test]
fn an_opening_is_not_reported_although_it_is_uncontained() {
    let mut b = Builder::new();
    let opening = b.drawable("IFCOPENINGELEMENT", Some(".MODEL_VIEW."));
    let wall = b.add("IFCWALL", Builder::root_slots());
    let mut a = vec![Value::Null; 6];
    a[4] = Value::Ref(wall);
    a[5] = Value::Ref(opening);
    b.add("IFCRELVOIDSELEMENT", a);

    assert!(
        unreachable_products(&b.model).is_empty(),
        "openings reach the model through IfcRelVoidsElement, never containment; \
         flagging them reports 17 non-problems on the reference file"
    );
}

#[test]
fn an_aggregated_part_is_not_reported() {
    let mut b = Builder::new();
    let part = b.drawable("IFCMEMBER", Some(".MODEL_VIEW."));
    let assembly = b.add("IFCELEMENTASSEMBLY", Builder::root_slots());
    let mut a = vec![Value::Null; 6];
    a[4] = Value::Ref(assembly);
    a[5] = Value::List(vec![Value::Ref(part)]);
    b.add("IFCRELAGGREGATES", a);

    assert!(
        unreachable_products(&b.model).is_empty(),
        "assembly parts hang off IfcRelAggregates, not containment"
    );
}

#[test]
fn a_product_without_any_representation_is_not_reported() {
    let mut b = Builder::new();
    b.product("IFCVIRTUALELEMENT", None);

    assert!(
        unreachable_products(&b.model).is_empty(),
        "nothing to draw is not a drawing defect: the reference file has 3 of these"
    );
}

#[test]
fn a_spatial_container_is_not_reported() {
    let mut b = Builder::new();
    b.drawable("IFCBUILDINGSTOREY", Some(".MODEL_VIEW."));

    assert!(
        unreachable_products(&b.model).is_empty(),
        "containers are aggregated into the tree, not contained in it"
    );
}

#[test]
fn geometry_only_in_the_plan_context_is_reported() {
    let mut b = Builder::new();
    let annotation = b.drawable("IFCANNOTATION", Some(".PLAN_VIEW."));
    b.contain(annotation);

    let found = unreachable_products(&b.model);

    assert_eq!(found.len(), 1, "one finding, got {found:?}");
    assert_eq!(found[0].0, annotation);
    assert!(
        matches!(
            &found[0].1,
            Unreachable::NoRepresentationInModelContext { found } if found == &["PlanView"]
        ),
        "a model viewer renders Model and skips Plan, got {:?}",
        found[0].1
    );
}

#[test]
fn a_missing_target_view_is_treated_as_drawable() {
    let mut b = Builder::new();
    let wall = b.drawable("IFCWALL", None);
    b.contain(wall);

    assert!(
        unreachable_products(&b.model).is_empty(),
        "TargetView is optional and absent in IFC2x3 files; assuming invisible \
         would report every product in every older model"
    );
}

#[test]
fn a_representation_with_an_unresolvable_context_is_reported() {
    let mut b = Builder::new();
    let shape = b.shape(None);
    let wall = b.product("IFCWALL", Some(shape));
    b.contain(wall);

    assert_eq!(
        unreachable_products(&b.model),
        vec![(wall, Unreachable::RepresentationWithoutContext)],
        "geometry a viewer cannot schedule is still invisible"
    );
}

#[test]
fn containment_is_reported_before_the_context_problem() {
    // Both defects at once. Only the blocking one is reported: fixing
    // containment first is what makes the file render at all.
    let mut b = Builder::new();
    let annotation = b.drawable("IFCANNOTATION", Some(".PLAN_VIEW."));

    assert_eq!(
        unreachable_products(&b.model),
        vec![(annotation, Unreachable::NotContainedInSpatialStructure)],
        "reporting both would bury the one that must be fixed first"
    );
}

#[test]
fn findings_are_in_stable_id_order() {
    // Two orphans: the report must not depend on hash iteration order, or a
    // caller diffing two runs sees phantom changes.
    let mut b = Builder::new();
    let first = b.drawable("IFCWALL", Some(".MODEL_VIEW."));
    let second = b.drawable("IFCDOOR", Some(".MODEL_VIEW."));

    let found = unreachable_products(&b.model);

    assert_eq!(found.len(), 2, "both orphans reported");
    assert!(found[0].0 < found[1].0, "ascending id order");
    assert_eq!((found[0].0, found[1].0), (first, second));
}
