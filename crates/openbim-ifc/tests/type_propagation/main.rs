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

mod material;
mod predefined;
mod replace;
mod representation;

use material::usage_of;
use representation::assert_mapped;

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
    let spatial = |b: &mut Batch, type_name: &str, name: &str, parent: EntityId| {
        // One placement per object: IFC2X3's `PlacesObject` is `SET [1:1]`.
        let site_placement = b.push(AuthorOp::Placement {
            relative_to: None,
            location: [0.0; 3],
            axes: None,
        });
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
    let outcome = apply_authoring(&mut model, schema, &b.ops, ifc::fresh_seed())
        .expect("the base model builds");
    let real =
        |handle: EntityId| outcome.ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
    let base = Base {
        owner: real(owner),
        context: real(context),
        storey: real(storey),
        other_storey: real(other_storey),
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
        let outcome = apply_authoring(&mut self.model, self.schema, &ops, ifc::fresh_seed())
            .unwrap_or_else(|error| panic!("{}: {error}", self.name()));
        assert_valid(&self.name(), &self.model, self.schema);
        outcome.ids
    }

    /// Apply `ops`, expect a refusal, and check the model is unchanged.
    fn refuse(&mut self, ops: Vec<AuthorOp>) -> AuthoringFailure {
        let before = self.model.revision();
        let count = self.model.ids().count();
        let error = apply_authoring(&mut self.model, self.schema, &ops, ifc::fresh_seed())
            .expect_err("the batch is refused");
        assert_eq!(self.model.revision(), before, "a refusal changes nothing");
        assert_eq!(self.model.ids().count(), count);
        *error.failure
    }

    /// A product of `type_name`, contained in the storey, placed.
    ///
    /// Each product gets its own `IfcLocalPlacement` (IFC2X3's
    /// `PlacesObject` is `SET [1:1]`).
    fn product(&mut self, type_name: &str, attributes: Vec<(&str, Value)>) -> AuthorOp {
        // Applied unvalidated: the placement is referenced once the
        // product it is for lands.
        let placement = apply_authoring(
            &mut self.model,
            self.schema,
            &[AuthorOp::Placement {
                relative_to: None,
                location: [0.0; 3],
                axes: None,
            }],
            ifc::fresh_seed(),
        )
        .unwrap_or_else(|error| panic!("{}: {error}", self.name()))
        .ids[0]
            .unwrap();
        AuthorOp::Product {
            type_name: type_name.into(),
            container: Some(self.base.storey),
            attributes: named(attributes),
            placement: Some(placement),
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
    // With INVERSE cardinalities (#534): a type has one IfcRelDefinesByType.
    let options = ifc::validate::ValidateOptions::new().inverse_cardinality(true);
    let report = ifc::validate::validate_with_options(model, schema, &options);
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
