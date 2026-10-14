//! Propagating a type to its occurrences on assignment, and replacing a
//! type, container or aggregate parent (#480), over door and wall types
//! in IFC2X3, IFC4 and IFC4X3.
//!
//! Each case builds its model through the checked authoring batch, so
//! the same names hold in every release, and validates the model after
//! every applied batch: a propagated type leaves no validation error.

#![cfg(all(
    feature = "author",
    feature = "validate",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

use ifc::{
    apply_authoring, authoring_handle, AssignTypeOptions, AuthorOp, AuthoringFailure, EntityId,
    LayerAxis, Model, OwnerHistoryOp, PredefinedTypeMode, RelateOptions, Schema, SchemaVersion,
    Value, HANDLE_BASE,
};

const RELEASES: [(SchemaVersion, &str); 3] = [
    (SchemaVersion::Ifc2x3, "IFC2X3"),
    (SchemaVersion::Ifc4, "IFC4"),
    (SchemaVersion::Ifc4x3, "IFC4X3_ADD2"),
];

// --- building the models ---

/// A batch under construction: each push returns the handle of the
/// entity its operation produces.
#[derive(Default)]
struct Batch {
    ops: Vec<AuthorOp>,
}

impl Batch {
    fn push(&mut self, op: AuthorOp) -> EntityId {
        self.ops.push(op);
        authoring_handle(self.ops.len() - 1)
    }

    fn create(&mut self, type_name: &str, attributes: Vec<(&str, Value)>) -> EntityId {
        self.push(AuthorOp::Create {
            type_name: type_name.into(),
            attributes: named(attributes),
        })
    }
}

fn named(attributes: Vec<(&str, Value)>) -> Vec<(String, Value)> {
    attributes
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect()
}

fn text(value: &str) -> Value {
    Value::Text(value.into())
}

fn enumeration(value: &str) -> Value {
    Value::Enum(value.into())
}

fn reals(values: &[f64]) -> Value {
    Value::List(values.iter().copied().map(Value::Real).collect())
}

fn refs(ids: &[EntityId]) -> Value {
    Value::List(ids.iter().copied().map(Value::Ref).collect())
}

/// The ids of the base model every case starts from.
#[derive(Debug, Clone, Copy)]
struct Base {
    owner: EntityId,
    context: EntityId,
    storey: EntityId,
    other_storey: EntityId,
    placement: EntityId,
    wall_type: EntityId,
    wall_map: EntityId,
    wall_set: EntityId,
    other_wall_type: EntityId,
    other_wall_map: EntityId,
    other_wall_set: EntityId,
    door_type: EntityId,
    door_map: EntityId,
}

struct Fixture {
    /// Two `IfcWallTypeEnum` members of the release: the first type's
    /// and the second's.
    kinds: (&'static str, &'static str),
    model: Model,
    schema: &'static Schema,
    release: SchemaVersion,
    base: Base,
}

/// A body representation map: a box `x` by `y` by `z` extruded from a
/// rectangle, in `context`.
fn body_map(batch: &mut Batch, context: EntityId, x: f64, y: f64, z: f64) -> EntityId {
    let origin = batch.create(
        "IfcCartesianPoint",
        vec![("Coordinates", reals(&[0.0, 0.0, 0.0]))],
    );
    let placement = batch.create(
        "IfcAxis2Placement3D",
        vec![("Location", Value::Ref(origin))],
    );
    let centre = batch.create(
        "IfcCartesianPoint",
        vec![("Coordinates", reals(&[x / 2.0, y / 2.0]))],
    );
    let position = batch.create(
        "IfcAxis2Placement2D",
        vec![("Location", Value::Ref(centre))],
    );
    let profile = batch.create(
        "IfcRectangleProfileDef",
        vec![
            ("ProfileType", enumeration("AREA")),
            ("Position", Value::Ref(position)),
            ("XDim", Value::Real(x)),
            ("YDim", Value::Real(y)),
        ],
    );
    let up = batch.create(
        "IfcDirection",
        vec![("DirectionRatios", reals(&[0.0, 0.0, 1.0]))],
    );
    let solid = batch.create(
        "IfcExtrudedAreaSolid",
        vec![
            ("SweptArea", Value::Ref(profile)),
            ("Position", Value::Ref(placement)),
            ("ExtrudedDirection", Value::Ref(up)),
            ("Depth", Value::Real(z)),
        ],
    );
    let representation = batch.create(
        "IfcShapeRepresentation",
        vec![
            ("ContextOfItems", Value::Ref(context)),
            ("RepresentationIdentifier", text("Body")),
            ("RepresentationType", text("SweptSolid")),
            ("Items", refs(&[solid])),
        ],
    );
    batch.create(
        "IfcRepresentationMap",
        vec![
            ("MappingOrigin", Value::Ref(placement)),
            ("MappedRepresentation", Value::Ref(representation)),
        ],
    )
}

/// A one-layer `IfcMaterialLayerSet` of `thickness`.
fn layer_set(batch: &mut Batch, name: &str, thickness: f64) -> EntityId {
    let material = batch.create("IfcMaterial", vec![("Name", text(name))]);
    let layer = batch.create(
        "IfcMaterialLayer",
        vec![
            ("Material", Value::Ref(material)),
            ("LayerThickness", Value::Real(thickness)),
        ],
    );
    batch.create(
        "IfcMaterialLayerSet",
        vec![
            ("MaterialLayers", refs(&[layer])),
            ("LayerSetName", text(name)),
        ],
    )
}

fn associate(batch: &mut Batch, owner: EntityId, objects: &[EntityId], material: EntityId) {
    batch.create(
        "IfcRelAssociatesMaterial",
        vec![
            ("OwnerHistory", Value::Ref(owner)),
            ("RelatedObjects", refs(objects)),
            ("RelatingMaterial", Value::Ref(material)),
        ],
    );
}

fn wall_type(
    batch: &mut Batch,
    owner: EntityId,
    name: &str,
    map: EntityId,
    predefined: &str,
) -> EntityId {
    batch.push(AuthorOp::TypeObject {
        type_name: "IfcWallType".into(),
        attributes: named(vec![
            ("Name", text(name)),
            ("RepresentationMaps", refs(&[map])),
            ("PredefinedType", enumeration(predefined)),
        ]),
        owner_history: Some(owner),
    })
}

fn fixture(release: SchemaVersion, token: &str) -> Fixture {
    let schema = ifc::schema::for_version(release).expect("bundled");
    let mut model = Model::new();
    model.header_mut().schema = vec![token.to_owned()];
    let mut b = Batch::default();
    let mut history = OwnerHistoryOp::new("openbim.rs", "type propagation", "1", "tp", 0);
    history.change_action = Some("ADDED".into());
    history.family_name = Some("Author".into());
    let owner = b.push(AuthorOp::OwnerHistory(history));
    let origin = b.create(
        "IfcCartesianPoint",
        vec![("Coordinates", reals(&[0.0, 0.0, 0.0]))],
    );
    let world = b.create(
        "IfcAxis2Placement3D",
        vec![("Location", Value::Ref(origin))],
    );
    let context = b.create(
        "IfcGeometricRepresentationContext",
        vec![
            ("ContextType", text("Model")),
            ("CoordinateSpaceDimension", Value::Integer(3)),
            ("Precision", Value::Real(1e-5)),
            ("WorldCoordinateSystem", Value::Ref(world)),
        ],
    );
    let metre = b.create(
        "IfcSIUnit",
        vec![
            ("UnitType", enumeration("LENGTHUNIT")),
            ("Name", enumeration("METRE")),
        ],
    );
    let units = b.create("IfcUnitAssignment", vec![("Units", refs(&[metre]))]);
    let project = b.push(AuthorOp::Project {
        attributes: named(vec![
            ("Name", text("Type propagation")),
            ("RepresentationContexts", refs(&[context])),
            ("UnitsInContext", Value::Ref(units)),
        ]),
        owner_history: Some(owner),
    });
    let site_placement = b.push(AuthorOp::Placement {
        relative_to: None,
        location: [0.0; 3],
        axes: None,
    });
    let spatial = |b: &mut Batch, type_name: &str, name: &str, parent: EntityId| {
        b.push(AuthorOp::Spatial {
            type_name: type_name.into(),
            parent,
            attributes: named(vec![
                ("Name", text(name)),
                ("CompositionType", enumeration("ELEMENT")),
            ]),
            placement: Some(site_placement),
            owner_history: Some(owner),
        })
    };
    let building = spatial(&mut b, "IfcBuilding", "Building", project);
    let storey = spatial(&mut b, "IfcBuildingStorey", "Level 0", building);
    let other_storey = spatial(&mut b, "IfcBuildingStorey", "Level 1", building);
    let placement = b.push(AuthorOp::Placement {
        relative_to: Some(site_placement),
        location: [1.0, 2.0, 0.0],
        axes: None,
    });
    // IFC2X3's `IfcWallTypeEnum` has neither SOLIDWALL nor PARTITIONING.
    let kinds = if release == SchemaVersion::Ifc2x3 {
        ("STANDARD", "SHEAR")
    } else {
        ("SOLIDWALL", "PARTITIONING")
    };
    let wall_map = body_map(&mut b, context, 5.0, 0.2, 3.0);
    let wall_type_id = wall_type(&mut b, owner, "WT-200", wall_map, kinds.0);
    let wall_set = layer_set(&mut b, "Concrete", 0.2);
    associate(&mut b, owner, &[wall_type_id], wall_set);
    let other_wall_map = body_map(&mut b, context, 5.0, 0.1, 3.0);
    let other_wall_type = wall_type(&mut b, owner, "WT-100", other_wall_map, kinds.1);
    let other_wall_set = layer_set(&mut b, "Gypsum", 0.1);
    associate(&mut b, owner, &[other_wall_type], other_wall_set);
    let door_map = body_map(&mut b, context, 0.9, 0.05, 2.1);
    let door_type = if release == SchemaVersion::Ifc2x3 {
        b.push(AuthorOp::TypeObject {
            type_name: "IfcDoorStyle".into(),
            attributes: named(vec![
                ("Name", text("D-90")),
                ("RepresentationMaps", refs(&[door_map])),
                ("OperationType", enumeration("SINGLE_SWING_LEFT")),
                ("ConstructionType", enumeration("WOOD")),
                ("ParameterTakesPrecedence", Value::Bool(false)),
                ("Sizeable", Value::Bool(false)),
            ]),
            owner_history: Some(owner),
        })
    } else {
        b.push(AuthorOp::TypeObject {
            type_name: "IfcDoorType".into(),
            attributes: named(vec![
                ("Name", text("D-90")),
                ("RepresentationMaps", refs(&[door_map])),
                ("PredefinedType", enumeration("DOOR")),
                ("OperationType", enumeration("SINGLE_SWING_LEFT")),
            ]),
            owner_history: Some(owner),
        })
    };
    let outcome = apply_authoring(&mut model, schema, &b.ops, 480).expect("the base model builds");
    let real =
        |handle: EntityId| outcome.ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
    let base = Base {
        owner: real(owner),
        context: real(context),
        storey: real(storey),
        other_storey: real(other_storey),
        placement: real(placement),
        wall_type: real(wall_type_id),
        wall_map: real(wall_map),
        wall_set: real(wall_set),
        other_wall_type: real(other_wall_type),
        other_wall_map: real(other_wall_map),
        other_wall_set: real(other_wall_set),
        door_type: real(door_type),
        door_map: real(door_map),
    };
    assert_valid(token, &model, schema);
    Fixture {
        kinds,
        model,
        schema,
        release,
        base,
    }
}

fn each_release(mut check: impl FnMut(Fixture)) {
    for (release, token) in RELEASES {
        check(fixture(release, token));
    }
}

impl Fixture {
    fn name(&self) -> String {
        format!("{:?}", self.release)
    }

    /// Apply `ops`, validate, and return the id each produced.
    fn apply(&mut self, ops: Vec<AuthorOp>) -> Vec<Option<EntityId>> {
        let outcome = apply_authoring(&mut self.model, self.schema, &ops, 7)
            .unwrap_or_else(|error| panic!("{}: {error}", self.name()));
        assert_valid(&self.name(), &self.model, self.schema);
        outcome.ids
    }

    /// Apply `ops`, expect a refusal, and check the model is unchanged.
    fn refuse(&mut self, ops: Vec<AuthorOp>) -> AuthoringFailure {
        let before = self.model.revision();
        let count = self.model.ids().count();
        let error = apply_authoring(&mut self.model, self.schema, &ops, 7)
            .expect_err("the batch is refused");
        assert_eq!(self.model.revision(), before, "a refusal changes nothing");
        assert_eq!(self.model.ids().count(), count);
        *error.failure
    }

    /// A product of `type_name`, contained in the storey, placed.
    fn product(&self, type_name: &str, attributes: Vec<(&str, Value)>) -> AuthorOp {
        AuthorOp::Product {
            type_name: type_name.into(),
            container: Some(self.base.storey),
            attributes: named(attributes),
            placement: Some(self.base.placement),
            type_object: None,
            owner_history: Some(self.base.owner),
        }
    }

    fn wall(&mut self, attributes: Vec<(&str, Value)>) -> EntityId {
        let mut attributes = attributes;
        attributes.push(("Name", text("Wall")));
        let op = self.product("IfcWall", attributes);
        self.apply(vec![op])[0].unwrap()
    }

    fn assign(
        &self,
        type_object: EntityId,
        objects: &[EntityId],
        options: AssignTypeOptions,
    ) -> AuthorOp {
        AuthorOp::AssignType {
            type_object,
            objects: objects.to_vec(),
            owner_history: Some(self.base.owner),
            options,
        }
    }

    fn get(&self, id: EntityId) -> &ifc::Entity {
        self.model
            .get(id)
            .unwrap_or_else(|| panic!("{}: #{} is missing", self.name(), id.0))
    }

    fn attr(&self, id: EntityId, name: &str) -> Value {
        let entity = self.get(id);
        let slot = self
            .schema
            .attributes(&entity.type_name)
            .iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("{} has no {name}", entity.type_name));
        entity.attributes[slot].clone()
    }

    fn reference(&self, id: EntityId, name: &str) -> EntityId {
        self.attr(id, name)
            .as_ref_id()
            .unwrap_or_else(|| panic!("#{}.{name} is not a reference", id.0))
    }

    fn list(&self, id: EntityId, name: &str) -> Vec<EntityId> {
        let mut found = Vec::new();
        self.attr(id, name).for_each_ref(&mut |id| found.push(id));
        found
    }

    /// Ids of every entity of exactly `type_name`, upper case.
    fn of_type(&self, type_name: &str) -> Vec<EntityId> {
        self.model.ids_of_type(type_name).to_vec()
    }

    /// The relationship of `type_name` whose `attribute` lists `object`.
    fn relationships(&self, type_name: &str, attribute: &str, object: EntityId) -> Vec<EntityId> {
        self.of_type(type_name)
            .into_iter()
            .filter(|rel| self.list(*rel, attribute).contains(&object))
            .collect()
    }

    fn type_of(&self, object: EntityId) -> Option<EntityId> {
        ifc::type_of(&self.model, self.schema, object).expect("answers")
    }

    fn has_predefined(&self, type_name: &str) -> bool {
        self.schema
            .attributes(type_name)
            .iter()
            .any(|attribute| attribute.name == "PredefinedType")
    }
}

/// The model has no validation error and no dangling reference.
fn assert_valid(what: &str, model: &Model, schema: &Schema) {
    let report = ifc::validate::validate(model, schema);
    let errors: Vec<String> = report
        .sorted()
        .iter()
        .filter(|finding| finding.severity == ifc::validate::Severity::Error)
        .map(|finding| {
            format!(
                "{}: {} at {:?}",
                finding.rule, finding.message, finding.path
            )
        })
        .collect();
    assert!(errors.is_empty(), "{what}:\n  {}", errors.join("\n  "));
    assert!(model.dangling_references().is_empty(), "{what}: dangling");
}

// --- the default: only the relationship, refusing a typed object ---

#[test]
fn assignment_without_options_writes_only_the_relationship() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let count = f.model.ids().count();
        let op = f.assign(f.base.wall_type, &[wall], AssignTypeOptions::new());
        f.apply(vec![op]);
        assert_eq!(f.type_of(wall), Some(f.base.wall_type));
        assert_eq!(f.model.ids().count(), count + 1, "{}", f.name());
        assert_eq!(f.attr(wall, "Representation"), Value::Null);
    });
}

#[test]
fn retyping_is_refused_by_default() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let op = f.assign(f.base.wall_type, &[wall], AssignTypeOptions::propagate());
        f.apply(vec![op]);
        let op = f.assign(
            f.base.other_wall_type,
            &[wall],
            AssignTypeOptions::propagate(),
        );
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(failure, AuthoringFailure::AlreadyRelated { id, relationship: "IFCRELDEFINESBYTYPE", .. } if id == wall),
            "{}: {failure:?}",
            f.name()
        );
        assert_eq!(f.type_of(wall), Some(f.base.wall_type));
    });
}

// --- representations ---

/// The one mapped representation of `object`, checked against `map`.
fn assert_mapped(f: &Fixture, object: EntityId, map: EntityId) -> EntityId {
    let shape = f.reference(object, "Representation");
    assert_eq!(f.get(shape).type_name.as_ref(), "IFCPRODUCTDEFINITIONSHAPE");
    let representations = f.list(shape, "Representations");
    assert_eq!(representations.len(), 1, "{}", f.name());
    let representation = representations[0];
    assert_eq!(
        f.get(representation).type_name.as_ref(),
        "IFCSHAPEREPRESENTATION"
    );
    assert_eq!(
        f.reference(representation, "ContextOfItems"),
        f.base.context
    );
    assert_eq!(
        f.attr(representation, "RepresentationIdentifier"),
        text("Body")
    );
    assert_eq!(
        f.attr(representation, "RepresentationType"),
        text("MappedRepresentation")
    );
    let items = f.list(representation, "Items");
    assert_eq!(items.len(), 1);
    let item = items[0];
    assert_eq!(f.get(item).type_name.as_ref(), "IFCMAPPEDITEM");
    assert_eq!(f.reference(item, "MappingSource"), map);
    let target = f.reference(item, "MappingTarget");
    assert_eq!(
        f.get(target).type_name.as_ref(),
        "IFCCARTESIANTRANSFORMATIONOPERATOR3D"
    );
    for name in ["Axis1", "Axis2", "Scale", "Axis3"] {
        assert_eq!(f.attr(target, name), Value::Null, "identity: no {name}");
    }
    let origin = f.reference(target, "LocalOrigin");
    assert_eq!(f.attr(origin, "Coordinates"), reals(&[0.0, 0.0, 0.0]));
    representation
}

#[test]
fn wall_and_door_types_map_their_representations() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let door_op = f.product("IfcDoor", vec![("Name", text("Door"))]);
        let door = f.apply(vec![door_op])[0].unwrap();
        let options = AssignTypeOptions::new().map_representations(true);
        let ops = vec![
            f.assign(f.base.wall_type, &[wall], options),
            f.assign(f.base.door_type, &[door], options),
        ];
        f.apply(ops);
        assert_mapped(&f, wall, f.base.wall_map);
        assert_mapped(&f, door, f.base.door_map);
        // The type's own representation is shared, not copied.
        let maps = f.list(f.base.door_type, "RepresentationMaps");
        assert_eq!(maps, vec![f.base.door_map]);
    });
}

#[test]
fn one_assignment_maps_every_object_separately() {
    each_release(|mut f| {
        let first = f.wall(vec![]);
        let second = f.wall(vec![]);
        let options = AssignTypeOptions::new().map_representations(true);
        let op = f.assign(f.base.wall_type, &[first, second], options);
        f.apply(vec![op]);
        let a = assert_mapped(&f, first, f.base.wall_map);
        let b = assert_mapped(&f, second, f.base.wall_map);
        assert_ne!(a, b, "{}: each object owns its representation", f.name());
    });
}

#[test]
fn a_given_mapping_target_is_used_by_handle() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let mut b = Batch::default();
        let point = b.create(
            "IfcCartesianPoint",
            vec![("Coordinates", reals(&[0.5, 0.0, 0.0]))],
        );
        let target = b.create(
            "IfcCartesianTransformationOperator3D",
            vec![("LocalOrigin", Value::Ref(point))],
        );
        let options = AssignTypeOptions::new()
            .map_representations(true)
            .mapping_target(Some(target));
        b.push(f.assign(f.base.wall_type, &[wall], options));
        let ids = f.apply(b.ops);
        let shape = f.reference(wall, "Representation");
        let representation = f.list(shape, "Representations")[0];
        let item = f.list(representation, "Items")[0];
        assert_eq!(Some(f.reference(item, "MappingTarget")), ids[1]);
    });
}

#[test]
fn a_representation_in_the_way_is_refused_and_another_kept() {
    each_release(|mut f| {
        let mut b = Batch::default();
        let start = b.create(
            "IfcCartesianPoint",
            vec![("Coordinates", reals(&[0.0, 0.0]))],
        );
        let end = b.create(
            "IfcCartesianPoint",
            vec![("Coordinates", reals(&[5.0, 0.0]))],
        );
        let line = b.create("IfcPolyline", vec![("Points", refs(&[start, end]))]);
        let make = |b: &mut Batch, identifier: &str, kind: &str| {
            b.create(
                "IfcShapeRepresentation",
                vec![
                    ("ContextOfItems", Value::Ref(f.base.context)),
                    ("RepresentationIdentifier", text(identifier)),
                    ("RepresentationType", text(kind)),
                    ("Items", refs(&[line])),
                ],
            )
        };
        let axis = make(&mut b, "Axis", "Curve2D");
        let body = make(&mut b, "Body", "Curve2D");
        let axis_shape = b.create(
            "IfcProductDefinitionShape",
            vec![("Representations", refs(&[axis]))],
        );
        let body_shape = b.create(
            "IfcProductDefinitionShape",
            vec![("Representations", refs(&[body]))],
        );
        let with_shape = |b: &mut Batch, shape: EntityId| {
            b.push(f.product(
                "IfcWall",
                vec![
                    ("Name", text("Wall")),
                    ("Representation", Value::Ref(shape)),
                ],
            ))
        };
        let with_axis = with_shape(&mut b, axis_shape);
        let with_body = with_shape(&mut b, body_shape);
        let ids = f.apply(b.ops);
        let real =
            |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
        let (with_axis, with_body, axis) = (real(with_axis), real(with_body), real(axis));

        let options = AssignTypeOptions::new().map_representations(true);
        let op = f.assign(f.base.wall_type, &[with_body], options);
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(failure, AuthoringFailure::RepresentationConflict { object, .. } if object == with_body),
            "{}: {failure:?}",
            f.name()
        );

        let op = f.assign(f.base.wall_type, &[with_axis], options);
        f.apply(vec![op]);
        let shape = f.reference(with_axis, "Representation");
        let representations = f.list(shape, "Representations");
        assert_eq!(representations.len(), 2, "{}", f.name());
        assert_eq!(representations[0], axis, "the object's own stays first");
    });
}

#[test]
fn a_map_of_another_representation_kind_is_refused() {
    // IFC2X3 is the release whose `IfcRepresentationMap` has no rule
    // asking for a shape model, so a topology representation is legal
    // there and must be refused rather than mapped.
    let mut f = fixture(SchemaVersion::Ifc4, "IFC4");
    let mut b = Batch::default();
    let point = b.create(
        "IfcCartesianPoint",
        vec![("Coordinates", reals(&[0.0, 0.0, 0.0]))],
    );
    let vertex = b.create(
        "IfcVertexPoint",
        vec![("VertexGeometry", Value::Ref(point))],
    );
    let topology = b.create(
        "IfcTopologyRepresentation",
        vec![
            ("ContextOfItems", Value::Ref(f.base.context)),
            ("RepresentationIdentifier", text("Reference")),
            ("RepresentationType", text("Vertex")),
            ("Items", refs(&[vertex])),
        ],
    );
    let origin = b.create("IfcAxis2Placement3D", vec![("Location", Value::Ref(point))]);
    let map = b.create(
        "IfcRepresentationMap",
        vec![
            ("MappingOrigin", Value::Ref(origin)),
            ("MappedRepresentation", Value::Ref(topology)),
        ],
    );
    let kind = b.push(AuthorOp::TypeObject {
        type_name: "IfcWallType".into(),
        attributes: named(vec![
            ("Name", text("Topology")),
            ("RepresentationMaps", refs(&[map])),
            ("PredefinedType", enumeration("SOLIDWALL")),
        ]),
        owner_history: Some(f.base.owner),
    });
    let ids = f.apply(b.ops);
    let real = |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
    let (kind, map) = (real(kind), real(map));
    let wall = f.wall(vec![]);
    let op = f.assign(
        kind,
        &[wall],
        AssignTypeOptions::new().map_representations(true),
    );
    let failure = f.refuse(vec![op]);
    assert!(
        matches!(failure, AuthoringFailure::UnsupportedRepresentationMap { map: at, .. } if at == map),
        "{failure:?}"
    );
}

// --- material usage ---

fn usage_of(f: &Fixture, object: EntityId) -> EntityId {
    let associations = f.relationships("IFCRELASSOCIATESMATERIAL", "RelatedObjects", object);
    assert_eq!(associations.len(), 1, "{}: one material", f.name());
    f.reference(associations[0], "RelatingMaterial")
}

#[test]
fn a_wall_type_layer_set_gives_each_wall_a_layer_set_usage() {
    each_release(|mut f| {
        let first = f.wall(vec![]);
        let second = f.wall(vec![]);
        let op = f.assign(
            f.base.wall_type,
            &[first, second],
            AssignTypeOptions::new().material_usage(true),
        );
        f.apply(vec![op]);
        let usages: Vec<EntityId> = [first, second].iter().map(|w| usage_of(&f, *w)).collect();
        assert_ne!(usages[0], usages[1], "{}: one usage each", f.name());
        for usage in usages {
            assert_eq!(f.get(usage).type_name.as_ref(), "IFCMATERIALLAYERSETUSAGE");
            assert_eq!(f.reference(usage, "ForLayerSet"), f.base.wall_set);
            assert_eq!(f.attr(usage, "LayerSetDirection"), enumeration("AXIS2"));
            assert_eq!(f.attr(usage, "DirectionSense"), enumeration("POSITIVE"));
            assert_eq!(
                f.attr(usage, "OffsetFromReferenceLine")
                    .unwrap_typed()
                    .as_f64(),
                Some(0.0)
            );
        }
    });
}

#[test]
fn a_slab_takes_axis3_and_an_undocumented_class_needs_a_direction() {
    each_release(|mut f| {
        let mut b = Batch::default();
        let set = layer_set(&mut b, "Screed", 0.05);
        let slab_type = b.push(AuthorOp::TypeObject {
            type_name: "IfcSlabType".into(),
            attributes: named(vec![
                ("Name", text("S")),
                ("PredefinedType", enumeration("FLOOR")),
            ]),
            owner_history: Some(f.base.owner),
        });
        associate(&mut b, f.base.owner, &[slab_type], set);
        let covering_type = b.push(AuthorOp::TypeObject {
            type_name: "IfcCoveringType".into(),
            attributes: named(vec![
                ("Name", text("C")),
                ("PredefinedType", enumeration("FLOORING")),
            ]),
            owner_history: Some(f.base.owner),
        });
        associate(&mut b, f.base.owner, &[covering_type], set);
        let slab = b.push(f.product("IfcSlab", vec![("Name", text("Slab"))]));
        let covering = b.push(f.product("IfcCovering", vec![("Name", text("Covering"))]));
        let ids = f.apply(b.ops);
        let real =
            |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
        let (slab_type, covering_type, slab, covering) = (
            real(slab_type),
            real(covering_type),
            real(slab),
            real(covering),
        );

        let usage = AssignTypeOptions::new().material_usage(true);
        let op = f.assign(slab_type, &[slab], usage);
        f.apply(vec![op]);
        assert_eq!(
            f.attr(usage_of(&f, slab), "LayerSetDirection"),
            enumeration("AXIS3")
        );

        let op = f.assign(covering_type, &[covering], usage);
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(failure, AuthoringFailure::MaterialUsageUndetermined { object, .. } if object == covering),
            "{}: {failure:?}",
            f.name()
        );
        let op = f.assign(
            covering_type,
            &[covering],
            usage.layer_set_direction(Some(LayerAxis::Axis3)),
        );
        f.apply(vec![op]);
        assert_eq!(
            f.attr(usage_of(&f, covering), "LayerSetDirection"),
            enumeration("AXIS3")
        );
    });
}

#[test]
fn an_object_with_its_own_material_is_refused() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let mut b = Batch::default();
        let own = b.create("IfcMaterial", vec![("Name", text("Brick"))]);
        associate(&mut b, f.base.owner, &[wall], own);
        f.apply(b.ops);
        let op = f.assign(
            f.base.wall_type,
            &[wall],
            AssignTypeOptions::new().material_usage(true),
        );
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(failure, AuthoringFailure::AlreadyRelated { id, relationship: "IFCRELASSOCIATESMATERIAL", .. } if id == wall),
            "{}: {failure:?}",
            f.name()
        );
    });
}

#[test]
fn a_type_with_several_materials_or_a_usage_is_refused() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let mut b = Batch::default();
        let extra = b.create("IfcMaterial", vec![("Name", text("Extra"))]);
        associate(&mut b, f.base.owner, &[f.base.wall_type], extra);
        let usage_on_type = b.create(
            "IfcMaterialLayerSetUsage",
            vec![
                ("ForLayerSet", Value::Ref(f.base.other_wall_set)),
                ("LayerSetDirection", enumeration("AXIS2")),
                ("DirectionSense", enumeration("POSITIVE")),
                ("OffsetFromReferenceLine", Value::Real(0.0)),
            ],
        );
        let odd_type = wall_type(
            &mut b,
            f.base.owner,
            "Odd",
            f.base.other_wall_map,
            f.kinds.0,
        );
        associate(&mut b, f.base.owner, &[odd_type], usage_on_type);
        let ids = f.apply(b.ops);
        let odd_type = ids[usize::try_from(odd_type.0 - HANDLE_BASE).unwrap()].unwrap();
        for type_object in [f.base.wall_type, odd_type] {
            let op = f.assign(
                type_object,
                &[wall],
                AssignTypeOptions::new().material_usage(true),
            );
            let failure = f.refuse(vec![op]);
            assert!(
                matches!(failure, AuthoringFailure::MaterialUsageUndetermined { object, .. } if object == type_object),
                "{}: {failure:?}",
                f.name()
            );
        }
    });
}

#[test]
fn a_profile_set_gives_a_profile_set_usage() {
    for (release, token) in RELEASES.into_iter().skip(1) {
        let mut f = fixture(release, token);
        let mut b = Batch::default();
        let steel = b.create("IfcMaterial", vec![("Name", text("Steel"))]);
        let centre = b.create(
            "IfcCartesianPoint",
            vec![("Coordinates", reals(&[0.0, 0.0]))],
        );
        let position = b.create(
            "IfcAxis2Placement2D",
            vec![("Location", Value::Ref(centre))],
        );
        let profile = b.create(
            "IfcRectangleProfileDef",
            vec![
                ("ProfileType", enumeration("AREA")),
                ("Position", Value::Ref(position)),
                ("XDim", Value::Real(0.2)),
                ("YDim", Value::Real(0.4)),
            ],
        );
        let material_profile = b.create(
            "IfcMaterialProfile",
            vec![
                ("Material", Value::Ref(steel)),
                ("Profile", Value::Ref(profile)),
            ],
        );
        let set = b.create(
            "IfcMaterialProfileSet",
            vec![
                ("Name", text("B-200x400")),
                ("MaterialProfiles", refs(&[material_profile])),
            ],
        );
        let beam_type = b.push(AuthorOp::TypeObject {
            type_name: "IfcBeamType".into(),
            attributes: named(vec![
                ("Name", text("B")),
                ("PredefinedType", enumeration("BEAM")),
            ]),
            owner_history: Some(f.base.owner),
        });
        associate(&mut b, f.base.owner, &[beam_type], set);
        let beam = b.push(f.product("IfcBeam", vec![("Name", text("Beam"))]));
        let ids = f.apply(b.ops);
        let real =
            |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
        let (beam_type, beam, set) = (real(beam_type), real(beam), real(set));
        let op = f.assign(
            beam_type,
            &[beam],
            AssignTypeOptions::new().material_usage(true),
        );
        f.apply(vec![op]);
        let usage = usage_of(&f, beam);
        assert_eq!(
            f.get(usage).type_name.as_ref(),
            "IFCMATERIALPROFILESETUSAGE"
        );
        assert_eq!(f.reference(usage, "ForProfileSet"), set);
        assert_eq!(f.attr(usage, "CardinalPoint"), Value::Null);
    }
}

#[test]
fn a_single_material_applies_through_the_type() {
    each_release(|mut f| {
        let mut b = Batch::default();
        let wood = b.create("IfcMaterial", vec![("Name", text("Wood"))]);
        associate(&mut b, f.base.owner, &[f.base.door_type], wood);
        let door = b.push(f.product("IfcDoor", vec![("Name", text("Door"))]));
        let ids = f.apply(b.ops);
        let door = ids[usize::try_from(door.0 - HANDLE_BASE).unwrap()].unwrap();
        let op = f.assign(f.base.door_type, &[door], AssignTypeOptions::propagate());
        f.apply(vec![op]);
        assert!(f
            .relationships("IFCRELASSOCIATESMATERIAL", "RelatedObjects", door)
            .is_empty());
        assert_mapped(&f, door, f.base.door_map);
    });
}

// --- predefined type ---

#[test]
fn sync_unsets_the_wall_predefined_type_the_type_provides() {
    each_release(|mut f| {
        if !f.has_predefined("IfcWall") {
            // IFC2X3: `IfcWall` has no `PredefinedType`; sync does
            // nothing and the assignment still succeeds.
            let wall = f.wall(vec![]);
            let op = f.assign(
                f.base.wall_type,
                &[wall],
                AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Sync),
            );
            f.apply(vec![op]);
            return;
        }
        let wall = f.wall(vec![("PredefinedType", enumeration("PARTITIONING"))]);
        let sync = AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Sync);
        let op = f.assign(f.base.wall_type, &[wall], sync);
        f.apply(vec![op]);
        assert_eq!(f.attr(wall, "PredefinedType"), Value::Null, "{}", f.name());
    });
}

#[test]
fn validate_refuses_a_conflicting_predefined_type_and_accepts_an_equal_one() {
    for (release, token) in RELEASES.into_iter().skip(1) {
        let mut f = fixture(release, token);
        let validate = AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Validate);
        let conflicting = f.wall(vec![("PredefinedType", enumeration("PARTITIONING"))]);
        let op = f.assign(f.base.wall_type, &[conflicting], validate);
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(&failure, AuthoringFailure::PredefinedTypeConflict { object, object_value, type_value }
                if *object == conflicting && object_value == "PARTITIONING" && type_value == "SOLIDWALL"),
            "{failure:?}"
        );
        let equal = f.wall(vec![("PredefinedType", enumeration("SOLIDWALL"))]);
        let undefined = f.wall(vec![("PredefinedType", enumeration("NOTDEFINED"))]);
        let op = f.assign(f.base.wall_type, &[equal, undefined], validate);
        f.apply(vec![op]);
        assert_eq!(f.attr(equal, "PredefinedType"), enumeration("SOLIDWALL"));
    }
}

#[test]
fn userdefined_compares_object_type_with_element_type() {
    for (release, token) in RELEASES.into_iter().skip(1) {
        let mut f = fixture(release, token);
        let mut b = Batch::default();
        let custom = b.push(AuthorOp::TypeObject {
            type_name: "IfcDoorType".into(),
            attributes: named(vec![
                ("Name", text("Custom")),
                ("ElementType", text("Revolving")),
                ("PredefinedType", enumeration("USERDEFINED")),
                ("OperationType", enumeration("REVOLVING")),
            ]),
            owner_history: Some(f.base.owner),
        });
        let door = |b: &mut Batch, name: &str| {
            b.push(f.product(
                "IfcDoor",
                vec![
                    ("Name", text("Door")),
                    ("ObjectType", text(name)),
                    ("PredefinedType", enumeration("USERDEFINED")),
                ],
            ))
        };
        let same = door(&mut b, "Revolving");
        let other = door(&mut b, "Sliding");
        let ids = f.apply(b.ops);
        let real =
            |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
        let (custom, same, other) = (real(custom), real(same), real(other));

        let validate = AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Validate);
        let op = f.assign(custom, &[other], validate);
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(&failure, AuthoringFailure::PredefinedTypeConflict { type_value, .. } if type_value == "USERDEFINED ('Revolving')"),
            "{failure:?}"
        );
        let op = f.assign(custom, &[same], validate);
        f.apply(vec![op]);

        let sync = AssignTypeOptions::new()
            .predefined_type(PredefinedTypeMode::Sync)
            .replace(true);
        let op = f.assign(custom, &[other], sync);
        f.apply(vec![op]);
        assert_eq!(f.attr(other, "PredefinedType"), Value::Null);
        assert_eq!(f.attr(other, "ObjectType"), Value::Null);
    }
}

#[test]
fn a_notdefined_type_leaves_the_object_predefined_type() {
    for (release, token) in RELEASES.into_iter().skip(1) {
        let mut f = fixture(release, token);
        let mut b = Batch::default();
        let open = wall_type(
            &mut b,
            f.base.owner,
            "Open",
            f.base.other_wall_map,
            "NOTDEFINED",
        );
        let ids = f.apply(b.ops);
        let open = ids[usize::try_from(open.0 - HANDLE_BASE).unwrap()].unwrap();
        let wall = f.wall(vec![("PredefinedType", enumeration("PARTITIONING"))]);
        let op = f.assign(
            open,
            &[wall],
            AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Sync),
        );
        f.apply(vec![op]);
        assert_eq!(f.attr(wall, "PredefinedType"), enumeration("PARTITIONING"));
    }
}

#[test]
fn ifc2x3_slab_predefined_type_is_validated() {
    let mut f = fixture(SchemaVersion::Ifc2x3, "IFC2X3");
    let mut b = Batch::default();
    let slab_type = b.push(AuthorOp::TypeObject {
        type_name: "IfcSlabType".into(),
        attributes: named(vec![
            ("Name", text("S")),
            ("PredefinedType", enumeration("ROOF")),
        ]),
        owner_history: Some(f.base.owner),
    });
    let slab = b.push(f.product(
        "IfcSlab",
        vec![
            ("Name", text("Slab")),
            ("PredefinedType", enumeration("FLOOR")),
        ],
    ));
    let ids = f.apply(b.ops);
    let real = |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
    let (slab_type, slab) = (real(slab_type), real(slab));
    let op = f.assign(
        slab_type,
        &[slab],
        AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Validate),
    );
    assert!(matches!(
        f.refuse(vec![op]),
        AuthoringFailure::PredefinedTypeConflict { .. }
    ));
    let op = f.assign(
        slab_type,
        &[slab],
        AssignTypeOptions::new().predefined_type(PredefinedTypeMode::Sync),
    );
    f.apply(vec![op]);
    assert_eq!(f.attr(slab, "PredefinedType"), Value::Null);
}

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
