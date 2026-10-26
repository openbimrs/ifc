//! Propagating a type to the objects it is assigned to (#480): mapped
//! representations, the material set usage and `PredefinedType`, and,
//! when a type is replaced, taking back what the previous type gave.
//!
//! # Representations
//!
//! Each `IfcRepresentationMap` of the type's `RepresentationMaps` becomes
//! one `IfcShapeRepresentation` on the object: the map representation's
//! `ContextOfItems` and `RepresentationIdentifier`, the
//! `RepresentationType` `MappedRepresentation` (whose items IFC requires
//! to be `IfcMappedItem`s), and one `IfcMappedItem` of the map. Its
//! `MappingTarget` is the caller's operator or a fresh
//! `IfcCartesianTransformationOperator3D` (`2D` for a map whose
//! `MappingOrigin` is an `IfcAxis2Placement2D`) at the origin with no
//! axes or scale: the identity, so the mapped geometry sits at the
//! object's `ObjectPlacement` as the type defines it. A map whose
//! representation is not an `IfcShapeRepresentation` is refused: an
//! `IfcProductDefinitionShape` holds shape models only, and no other
//! representation type has a mapped form.
//!
//! # Material usage
//!
//! IFC assigns a material set to the type and a set usage to each
//! occurrence. The usage's defaults are documented, not guessed: the
//! layer direction is the one the IFC4 ADD2 TC1 documentation fixes per
//! class (`IfcWall`: "LayerSetDirection is always AXIS2"; `IfcSlab` and
//! `IfcPlate`: AXIS3); `DirectionSense` `POSITIVE` and an
//! `OffsetFromReferenceLine` of 0 put the first layer on the reference
//! line, the reading every usage takes until edited. A profile set usage
//! leaves `CardinalPoint` unset, as the schema allows. Another class
//! needs the caller's direction.
//!
//! # Replacing a type
//!
//! What the previous type gave is recognised by its source, never by
//! name: a representation of the object every item of which is an
//! `IfcMappedItem` of one of the previous type's maps, and a set usage
//! whose `ForLayerSet` / `ForProfileSet` is the previous type's set. Each
//! is detached from the object and removed with what only it uses, by
//! the cascade's ownership rules; something another entity still uses
//! stays.

use ifc_model::{Entity, EntityId, Value};

use super::plan::{with_owner, Planner};
use super::{AssignTypeOptions, AuthoringFailure as F, LayerAxis, PredefinedTypeMode};

const MATERIAL_RELATIONSHIP: &str = "IFCRELASSOCIATESMATERIAL";
const NOTDEFINED: &str = "NOTDEFINED";
const USERDEFINED: &str = "USERDEFINED";

/// The type's material, as far as usage derivation is concerned.
enum TypeMaterial {
    /// No material, or one that applies through the type.
    None,
    /// An `IfcMaterialLayerSet`.
    LayerSet(EntityId),
    /// An `IfcMaterialProfileSet`.
    ProfileSet(EntityId),
}

/// One representation to map: the map, its context and identifier, and
/// whether its origin is 3D.
struct Planned {
    map: EntityId,
    context: Value,
    identifier: Value,
    three_d: bool,
}

impl Planner<'_, '_> {
    /// Propagate `type_object` to `objects` as `options` asks.
    pub(super) fn propagate(
        &mut self,
        type_object: EntityId,
        objects: &[EntityId],
        owner: Option<EntityId>,
        options: &AssignTypeOptions,
    ) -> Result<(), F> {
        let material = if options.material_usage {
            self.type_material(type_object)?
        } else {
            TypeMaterial::None
        };
        for object in objects {
            if options.map_representations {
                self.map_representations(type_object, *object, options.mapping_target)?;
            }
            if options.material_usage {
                self.derive_usage(&material, *object, owner, options.layer_set_direction)?;
            }
            match options.predefined_type {
                PredefinedTypeMode::Ignore => {}
                mode => self.reconcile_predefined(type_object, *object, mode)?,
            }
        }
        Ok(())
    }

    /// Take back from `object` what `old_type` propagated to it.
    pub(super) fn strip_type(&mut self, old_type: EntityId, object: EntityId) -> Result<(), F> {
        self.strip_representations(old_type, object)?;
        let set = match self.type_material(old_type) {
            Ok(TypeMaterial::LayerSet(set) | TypeMaterial::ProfileSet(set)) => set,
            // A previous type without a set, or with a material that
            // gives no usage, propagated none.
            _ => return Ok(()),
        };
        for (relationship, usage) in self.associations(object) {
            let derived = self.view.get(usage).is_some_and(|entity| {
                ["ForLayerSet", "ForProfileSet"]
                    .iter()
                    .any(|name| self.reference(entity, name) == Some(set))
            });
            if derived {
                self.leave(
                    relationship,
                    "IFCRELASSOCIATESMATERIAL",
                    "RelatedObjects",
                    object,
                )?;
                self.remove_owned(&[usage])?;
            }
        }
        Ok(())
    }

    // --- representations ---

    fn map_representations(
        &mut self,
        type_object: EntityId,
        object: EntityId,
        target: Option<EntityId>,
    ) -> Result<(), F> {
        let maps = self.type_maps(type_object);
        if maps.is_empty() {
            return Ok(());
        }
        let entity = self.view.get(object).ok_or(F::MissingEntity(object))?;
        if !self.schema.is_a(&entity.type_name, "IfcProduct") {
            return Err(F::NotA {
                type_name: entity.type_name.to_string(),
                expected: "IfcProduct, to map its type's representations onto".to_owned(),
            });
        }
        let shape = self.reference(entity, "Representation");
        let planned = maps
            .into_iter()
            .map(|map| self.plan_map(map))
            .collect::<Result<Vec<_>, _>>()?;
        let existing = shape.map_or_else(Vec::new, |shape| {
            self.view.get(shape).map_or_else(Vec::new, |entity| {
                self.references(entity, "Representations")
            })
        });
        if let Some(shape) = shape {
            if let Some(other) = self.other_users(shape, object) {
                return Err(F::RepresentationConflict {
                    object,
                    existing: shape,
                    detail: format!("is a product definition shape {other} shares"),
                });
            }
        }
        for plan in &planned {
            for representation in &existing {
                if let Some(detail) = self.clashes(*representation, plan) {
                    return Err(F::RepresentationConflict {
                        object,
                        existing: *representation,
                        detail,
                    });
                }
            }
        }
        let mut operators: [Option<EntityId>; 2] = [None, None];
        let mut created = Vec::new();
        for plan in planned {
            let target = match target {
                Some(target) => target,
                None => {
                    let slot = usize::from(plan.three_d);
                    if let Some(operator) = operators[slot] {
                        operator
                    } else {
                        let operator = self.identity_operator(plan.three_d)?;
                        operators[slot] = Some(operator);
                        operator
                    }
                }
            };
            let item = self.create(
                "IfcMappedItem",
                vec![
                    ("MappingSource".to_owned(), Value::Ref(plan.map)),
                    ("MappingTarget".to_owned(), Value::Ref(target)),
                ],
            )?;
            let mut attributes = vec![
                ("ContextOfItems".to_owned(), plan.context),
                (
                    "RepresentationType".to_owned(),
                    Value::Text("MappedRepresentation".into()),
                ),
                ("Items".to_owned(), Value::List(vec![Value::Ref(item)])),
            ];
            if !matches!(plan.identifier, Value::Null) {
                attributes.push(("RepresentationIdentifier".to_owned(), plan.identifier));
            }
            created.push(Value::Ref(
                self.create("IfcShapeRepresentation", attributes)?,
            ));
        }
        match shape {
            Some(shape) => {
                let mut representations: Vec<Value> =
                    existing.into_iter().map(Value::Ref).collect();
                representations.extend(created);
                self.edit(
                    shape,
                    vec![("Representations".to_owned(), Value::List(representations))],
                )?;
            }
            None => {
                let shape = self.create(
                    "IfcProductDefinitionShape",
                    vec![("Representations".to_owned(), Value::List(created))],
                )?;
                self.edit(
                    object,
                    vec![("Representation".to_owned(), Value::Ref(shape))],
                )?;
            }
        }
        Ok(())
    }

    /// Read one representation map, refusing one that cannot be mapped.
    fn plan_map(&self, map: EntityId) -> Result<Planned, F> {
        let unsupported = |detail: String| F::UnsupportedRepresentationMap { map, detail };
        let entity = self.view.get(map).ok_or(F::MissingEntity(map))?;
        if !self.schema.is_a(&entity.type_name, "IfcRepresentationMap") {
            return Err(unsupported(format!("it is an {}", entity.type_name)));
        }
        let origin = self
            .reference(entity, "MappingOrigin")
            .and_then(|origin| self.view.get(origin))
            .ok_or_else(|| unsupported("its MappingOrigin is missing".to_owned()))?;
        let three_d = if self.schema.is_a(&origin.type_name, "IfcAxis2Placement3D") {
            true
        } else if self.schema.is_a(&origin.type_name, "IfcAxis2Placement2D") {
            false
        } else {
            return Err(unsupported(format!(
                "its MappingOrigin is an {}",
                origin.type_name
            )));
        };
        let representation = self
            .reference(entity, "MappedRepresentation")
            .and_then(|representation| self.view.get(representation))
            .ok_or_else(|| unsupported("its MappedRepresentation is missing".to_owned()))?;
        if !self
            .schema
            .is_a(&representation.type_name, "IfcShapeRepresentation")
        {
            return Err(unsupported(format!(
                "its MappedRepresentation is an {}, not an IfcShapeRepresentation",
                representation.type_name
            )));
        }
        Ok(Planned {
            map,
            context: self
                .value(representation, "ContextOfItems")
                .cloned()
                .unwrap_or(Value::Null),
            identifier: self
                .value(representation, "RepresentationIdentifier")
                .cloned()
                .unwrap_or(Value::Null),
            three_d,
        })
    }

    /// Why mapping `plan` would clash with the object's `representation`.
    fn clashes(&self, representation: EntityId, plan: &Planned) -> Option<String> {
        let entity = self.view.get(representation)?;
        let identifier = self.value(entity, "RepresentationIdentifier");
        match (text(identifier), text(Some(&plan.identifier))) {
            (Some(have), Some(want)) if have.eq_ignore_ascii_case(want) => {
                Some(format!("is already its '{have}' representation"))
            }
            (None, None) if self.value(entity, "ContextOfItems") == Some(&plan.context) => {
                Some("is already its unnamed representation in the same context".to_owned())
            }
            _ => None,
        }
    }

    /// An identity `IfcCartesianTransformationOperator3D` (or `2D`).
    fn identity_operator(&mut self, three_d: bool) -> Result<EntityId, F> {
        let coordinates = if three_d { 3 } else { 2 };
        let origin = self.create(
            "IfcCartesianPoint",
            vec![(
                "Coordinates".to_owned(),
                Value::List(vec![Value::Real(0.0); coordinates]),
            )],
        )?;
        let operator = if three_d {
            "IfcCartesianTransformationOperator3D"
        } else {
            "IfcCartesianTransformationOperator2D"
        };
        self.create(
            operator,
            vec![("LocalOrigin".to_owned(), Value::Ref(origin))],
        )
    }

    /// Remove from `object` the representations mapped from `old_type`'s
    /// maps, with what only they use.
    fn strip_representations(&mut self, old_type: EntityId, object: EntityId) -> Result<(), F> {
        let maps = self.type_maps(old_type);
        let Some(shape) = self
            .view
            .get(object)
            .and_then(|entity| self.reference(entity, "Representation"))
        else {
            return Ok(());
        };
        if maps.is_empty() {
            return Ok(());
        }
        let representations = self.view.get(shape).map_or_else(Vec::new, |entity| {
            self.references(entity, "Representations")
        });
        let (mapped, kept): (Vec<EntityId>, Vec<EntityId>) = representations
            .into_iter()
            .partition(|representation| self.maps_only(*representation, &maps));
        if mapped.is_empty() {
            return Ok(());
        }
        if kept.is_empty() {
            self.edit(object, vec![("Representation".to_owned(), Value::Null)])?;
            return self.remove_owned(&[shape]);
        }
        if let Some(by) = self.other_users(shape, object) {
            return Err(F::StillReferenced { id: shape, by });
        }
        self.edit(
            shape,
            vec![(
                "Representations".to_owned(),
                Value::List(kept.into_iter().map(Value::Ref).collect()),
            )],
        )?;
        self.remove_owned(&mapped)
    }

    /// Whether every item of `representation` is an `IfcMappedItem` of
    /// one of `maps`, and it has one at least.
    fn maps_only(&self, representation: EntityId, maps: &[EntityId]) -> bool {
        let Some(entity) = self.view.get(representation) else {
            return false;
        };
        let items = self.references(entity, "Items");
        !items.is_empty()
            && items.iter().all(|item| {
                self.view.get(*item).is_some_and(|item| {
                    self.schema.is_a(&item.type_name, "IfcMappedItem")
                        && self
                            .reference(item, "MappingSource")
                            .is_some_and(|source| maps.contains(&source))
                })
            })
    }

    /// The type's `RepresentationMaps`; none for a type object that is
    /// not an `IfcTypeProduct`.
    fn type_maps(&self, type_object: EntityId) -> Vec<EntityId> {
        self.view.get(type_object).map_or_else(Vec::new, |entity| {
            self.references(entity, "RepresentationMaps")
        })
    }

    /// A product other than `object` whose shape `shape` is, if any.
    fn other_users(&self, shape: EntityId, object: EntityId) -> Option<EntityId> {
        self.view.referrers(shape).into_iter().find(|by| {
            *by != object
                && self
                    .view
                    .get(*by)
                    .is_some_and(|entity| self.schema.is_a(&entity.type_name, "IfcProduct"))
        })
    }

    // --- material usage ---

    /// The material associated with `type_object`, read for usage
    /// derivation.
    fn type_material(&self, type_object: EntityId) -> Result<TypeMaterial, F> {
        let associations = self.associations(type_object);
        let material = match associations.as_slice() {
            [] => return Ok(TypeMaterial::None),
            [(_, material)] => *material,
            several => {
                return Err(F::MaterialUsageUndetermined {
                    object: type_object,
                    detail: format!(
                        "the type has {} material associations; IFC expects one",
                        several.len()
                    ),
                })
            }
        };
        let Some(entity) = self.view.get(material) else {
            return Err(F::MissingEntity(material));
        };
        let t = &entity.type_name;
        if self.schema.is_a(t, "IfcMaterialLayerSet") {
            Ok(TypeMaterial::LayerSet(material))
        } else if self.schema.is_a(t, "IfcMaterialProfileSet") {
            Ok(TypeMaterial::ProfileSet(material))
        } else if self.schema.is_a(t, "IfcMaterialLayerSetUsage")
            || self.schema.is_a(t, "IfcMaterialProfileSetUsage")
        {
            Err(F::MaterialUsageUndetermined {
                object: type_object,
                detail: format!(
                    "the type carries an {t}, which belongs to an occurrence; \
                     assign the type its set"
                ),
            })
        } else {
            Ok(TypeMaterial::None)
        }
    }

    /// Derive `object`'s set usage from the type's set.
    fn derive_usage(
        &mut self,
        material: &TypeMaterial,
        object: EntityId,
        owner: Option<EntityId>,
        direction: Option<LayerAxis>,
    ) -> Result<(), F> {
        let usage = match material {
            TypeMaterial::None => return Ok(()),
            TypeMaterial::LayerSet(set) => {
                let direction = match direction {
                    Some(direction) => direction,
                    None => self.default_direction(object)?,
                };
                self.refuse_material(object)?;
                self.create(
                    "IfcMaterialLayerSetUsage",
                    vec![
                        ("ForLayerSet".to_owned(), Value::Ref(*set)),
                        (
                            "LayerSetDirection".to_owned(),
                            Value::Enum(direction.as_str().into()),
                        ),
                        ("DirectionSense".to_owned(), Value::Enum("POSITIVE".into())),
                        ("OffsetFromReferenceLine".to_owned(), Value::Real(0.0)),
                    ],
                )?
            }
            TypeMaterial::ProfileSet(set) => {
                self.refuse_material(object)?;
                self.create(
                    "IfcMaterialProfileSetUsage",
                    vec![("ForProfileSet".to_owned(), Value::Ref(*set))],
                )?
            }
        };
        self.create(
            "IfcRelAssociatesMaterial",
            with_owner(
                vec![
                    (
                        "RelatedObjects".to_owned(),
                        Value::List(vec![Value::Ref(object)]),
                    ),
                    ("RelatingMaterial".to_owned(), Value::Ref(usage)),
                ],
                owner,
            ),
        )?;
        Ok(())
    }

    /// Refuse an object that already has a material.
    fn refuse_material(&self, object: EntityId) -> Result<(), F> {
        match self.associations(object).first() {
            Some((existing, _)) => Err(F::AlreadyRelated {
                id: object,
                relationship: MATERIAL_RELATIONSHIP,
                existing: *existing,
            }),
            None => Ok(()),
        }
    }

    /// The `LayerSetDirection` the IFC documentation fixes for the
    /// object's class.
    fn default_direction(&self, object: EntityId) -> Result<LayerAxis, F> {
        let entity = self.view.get(object).ok_or(F::MissingEntity(object))?;
        let t = &entity.type_name;
        if self.schema.is_a(t, "IfcWall") {
            Ok(LayerAxis::Axis2)
        } else if self.schema.is_a(t, "IfcSlab") || self.schema.is_a(t, "IfcPlate") {
            Ok(LayerAxis::Axis3)
        } else {
            Err(F::MaterialUsageUndetermined {
                object,
                detail: format!(
                    "IFC documents no LayerSetDirection for an {t}; \
                     pass layer_set_direction"
                ),
            })
        }
    }

    /// Every `IfcRelAssociatesMaterial` relating `object`, with its
    /// `RelatingMaterial`.
    fn associations(&self, object: EntityId) -> Vec<(EntityId, EntityId)> {
        self.view
            .ids_of_type(MATERIAL_RELATIONSHIP)
            .into_iter()
            .filter_map(|relationship| {
                let entity = self.view.get(relationship)?;
                if !self.references(entity, "RelatedObjects").contains(&object) {
                    return None;
                }
                Some((relationship, self.reference(entity, "RelatingMaterial")?))
            })
            .collect()
    }

    // --- predefined type ---

    fn reconcile_predefined(
        &mut self,
        type_object: EntityId,
        object: EntityId,
        mode: PredefinedTypeMode,
    ) -> Result<(), F> {
        let type_entity = self
            .view
            .get(type_object)
            .ok_or(F::MissingEntity(type_object))?;
        let Some(provided) = enumeration(self.value(type_entity, "PredefinedType"))
            .filter(|value| !value.eq_ignore_ascii_case(NOTDEFINED))
        else {
            return Ok(());
        };
        let provided_name = ["ElementType", "ProcessType", "ResourceType"]
            .iter()
            .find_map(|name| text(self.value(type_entity, name)))
            .map(str::to_owned);
        let provided = provided.to_owned();
        let entity = self.view.get(object).ok_or(F::MissingEntity(object))?;
        let Some(slot) = self.slot_of(&entity.type_name, "PredefinedType") else {
            return Ok(());
        };
        let Some(own) = enumeration(entity.attributes.get(slot)).map(str::to_owned) else {
            return Ok(());
        };
        let own_name = text(self.value(entity, "ObjectType")).map(str::to_owned);
        let user_defined = own.eq_ignore_ascii_case(USERDEFINED);
        let agrees = own.eq_ignore_ascii_case(NOTDEFINED)
            || (own.eq_ignore_ascii_case(&provided)
                && (!user_defined || own_name == provided_name));
        let optional = self.schema.attributes(&entity.type_name)[slot].optional;
        if mode == PredefinedTypeMode::Validate || !optional {
            if agrees {
                return Ok(());
            }
            return Err(F::PredefinedTypeConflict {
                object,
                object_value: describe(&own, own_name.as_deref()),
                type_value: describe(&provided, provided_name.as_deref()),
            });
        }
        let mut attributes = vec![("PredefinedType".to_owned(), Value::Null)];
        if user_defined && self.slot_of(&entity.type_name, "ObjectType").is_some() {
            attributes.push(("ObjectType".to_owned(), Value::Null));
        }
        self.edit(object, attributes).map(drop)
    }

    // --- reading ---

    fn slot_of(&self, type_name: &str, name: &str) -> Option<usize> {
        self.schema
            .attributes(type_name)
            .iter()
            .position(|attribute| attribute.name.eq_ignore_ascii_case(name))
    }

    fn value<'e>(&self, entity: &'e Entity, name: &str) -> Option<&'e Value> {
        entity
            .attributes
            .get(self.slot_of(&entity.type_name, name)?)
    }

    fn reference(&self, entity: &Entity, name: &str) -> Option<EntityId> {
        self.value(entity, name)?.as_ref_id()
    }

    fn references(&self, entity: &Entity, name: &str) -> Vec<EntityId> {
        let mut found = Vec::new();
        if let Some(value) = self.value(entity, name) {
            value.for_each_ref(&mut |id| found.push(id));
        }
        found
    }
}

/// A set text value.
fn text(value: Option<&Value>) -> Option<&str> {
    match value.map(Value::unwrap_typed) {
        Some(Value::Text(text)) => Some(text),
        _ => None,
    }
}

/// A set enumeration value.
fn enumeration(value: Option<&Value>) -> Option<&str> {
    match value.map(Value::unwrap_typed) {
        Some(Value::Enum(name)) => Some(name),
        _ => None,
    }
}

fn describe(value: &str, name: Option<&str>) -> String {
    match name {
        Some(name) if value.eq_ignore_ascii_case(USERDEFINED) => format!("{value} ('{name}')"),
        _ => value.to_owned(),
    }
}
