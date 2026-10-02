//! Borrowed read views of the four controls this crate authors (#100).
//!
//! # Read by name in the declared release
//!
//! A view binds the model's declared release exactly as the
//! `*_with_owner_history` writers do (`release.rs`): one recognised
//! `FILE_SCHEMA` binds its table, IFC4X1, IFC4X2 and unknown tokens fail
//! with [`ControlError::UnsupportedSchema`], several declarations with
//! [`ControlError::MultipleSchemas`], and a header with none reads as IFC4.
//! Every accessor then finds its slot by attribute name in that table, so
//! an IFC2X3 `IfcPermit` (`PermitID` after `ObjectType`) is never read
//! through IFC4's positions, where slot 6 is the permit's `PredefinedType`.
//!
//! From the EXPRESS sources, after the four `IfcRoot` slots and
//! `ObjectType`:
//!
//! ```text
//! IFC2X3_TC1     IfcPermit              PermitID
//! IFC2X3_TC1     IfcActionRequest       RequestID
//! IFC2X3_TC1     IfcProjectOrder        ID, PredefinedType, Status
//! IFC2X3_TC1     IfcPerformanceHistory  LifeCyclePhase
//! IFC4, IFC4X3   IfcPermit              Identification, PredefinedType, Status, LongDescription
//! IFC4, IFC4X3   IfcActionRequest       Identification, PredefinedType, Status, LongDescription
//! IFC4, IFC4X3   IfcProjectOrder        Identification, PredefinedType, Status, LongDescription
//! IFC4, IFC4X3   IfcPerformanceHistory  Identification, LifeCyclePhase, PredefinedType
//! ```
//!
//! [`Control::identification`] answers IFC2X3's `PermitID`, `RequestID`
//! and `ID`, which IFC4 renamed to `Identification`. An attribute the
//! release does not declare reads as `None`; [`Control::declares`] tells
//! that apart from an unset value.
//!
//! # Checked once, then infallible
//!
//! A view is built only from a record that fits its release: no more
//! attributes than declared, every required one set
//! ([`ControlError::MissingAttribute`]), and every set one a value its
//! declaration can hold ([`ControlError::InvalidAttribute`]), an enum
//! token included. A record that does not fit is refused, never read as
//! absent, so the accessors need no error of their own.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{Schema, SchemaVersion};

use crate::authoring::ControlKind;
use crate::error::{ControlError, ControlResult};
use crate::release::{bind, conforms, release_name};

const RELATION: &str = "IFCRELASSIGNSTOCONTROL";

/// A borrowed, release-bound view of one `IfcPermit`, `IfcProjectOrder`,
/// `IfcActionRequest` or `IfcPerformanceHistory`.
#[derive(Debug, Clone, Copy)]
pub struct Control<'m> {
    model: &'m Model,
    id: EntityId,
    entity: &'m Entity,
    kind: ControlKind,
    release: Bound,
    global_id: &'m str,
}

/// The table a view reads against and the release it describes.
#[derive(Debug, Clone, Copy)]
struct Bound {
    schema: &'static Schema,
    version: SchemaVersion,
}

impl Bound {
    fn of(model: &Model) -> ControlResult<Self> {
        let schema = bind(model)?.schema();
        let version = schema
            .version()
            .ok_or_else(|| ControlError::UnsupportedSchema {
                schema: schema.name().to_owned(),
            })?;
        Ok(Self { schema, version })
    }

    fn slot(self, entity: &str, attribute: &'static str) -> Option<usize> {
        let name = release_name(Some(self.version), entity, attribute);
        self.schema
            .attribute_names(entity)
            .iter()
            .position(|declared| declared.eq_ignore_ascii_case(name))
    }

    /// The value at `attribute`'s slot, `None` when undeclared or unset.
    fn value<'m>(
        self,
        entity: &str,
        record: &'m Entity,
        attribute: &'static str,
    ) -> Option<&'m Value> {
        match record.attribute(self.slot(entity, attribute)?)? {
            Value::Null => None,
            value => Some(value),
        }
    }

    fn text<'m>(
        self,
        entity: &str,
        record: &'m Entity,
        attribute: &'static str,
    ) -> Option<&'m str> {
        self.value(entity, record, attribute)?
            .unwrap_typed()
            .as_text()
    }

    /// Refuse a record that does not fit `entity`'s declaration here.
    fn check(self, entity: &'static str, id: EntityId, record: &Entity) -> ControlResult<()> {
        let declared = self.schema.attributes(entity);
        if record.attributes.len() > declared.len() {
            return Err(ControlError::ExtraAttributes {
                entity,
                id,
                declared: declared.len(),
                found: record.attributes.len(),
                schema: self.schema.name().to_owned(),
            });
        }
        for (slot, attribute) in declared.into_iter().enumerate() {
            match record.attribute(slot).unwrap_or(&Value::Null) {
                Value::Null if !attribute.optional => {
                    return Err(ControlError::MissingAttribute {
                        entity,
                        id,
                        attribute: attribute.name.as_str(),
                    })
                }
                Value::Null => {}
                value if !conforms(self.schema, attribute, value) => {
                    return Err(ControlError::InvalidAttribute {
                        entity,
                        id,
                        attribute: attribute.name.as_str(),
                        declared: attribute.type_name.clone(),
                        schema: self.schema.name().to_owned(),
                    })
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Read the control `id` against `model`'s declared release.
///
/// # Errors
///
/// [`ControlError::UnsupportedSchema`] or [`ControlError::MultipleSchemas`]
/// if the model binds no single release this crate is verified against;
/// [`ControlError::UnknownEntity`] if `id` is absent;
/// [`ControlError::ForeignControl`] if it is not one of the four
/// [`ControlKind`]s; and the record refusals of the module documentation
/// ([`ControlError::ExtraAttributes`], [`ControlError::MissingAttribute`],
/// [`ControlError::InvalidAttribute`]).
pub fn read_control(model: &Model, id: EntityId) -> ControlResult<Control<'_>> {
    let release = Bound::of(model)?;
    let entity = model.get(id).ok_or(ControlError::UnknownEntity { id })?;
    let kind = ControlKind::from_type_name(&entity.type_name).ok_or_else(|| {
        ControlError::ForeignControl {
            id,
            actual: entity.type_name.to_string(),
        }
    })?;
    Control::bound(model, id, entity, kind, release)
}

/// Every control of `kind` in `model`, in file order, read against its
/// declared release.
///
/// # Errors
///
/// Those of [`read_control`] except the identity refusals: the first
/// record of `kind` that does not fit its release fails the whole call
/// rather than being skipped.
pub fn read_controls(model: &Model, kind: ControlKind) -> ControlResult<Vec<Control<'_>>> {
    let release = Bound::of(model)?;
    model
        .of_type(kind.type_name())
        .map(|(id, entity)| Control::bound(model, id, entity, kind, release))
        .collect()
}

impl<'m> Control<'m> {
    fn bound(
        model: &'m Model,
        id: EntityId,
        entity: &'m Entity,
        kind: ControlKind,
        release: Bound,
    ) -> ControlResult<Self> {
        let name = kind.type_name();
        release.check(name, id, entity)?;
        // Required by every release and checked above to be text.
        let global_id =
            release
                .text(name, entity, "GlobalId")
                .ok_or(ControlError::MissingAttribute {
                    entity: name,
                    id,
                    attribute: "GlobalId",
                })?;
        Ok(Self {
            model,
            id,
            entity,
            kind,
            release,
            global_id,
        })
    }

    fn text(&self, attribute: &'static str) -> Option<&'m str> {
        self.release
            .text(self.kind.type_name(), self.entity, attribute)
    }

    /// The entity id in the file.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// Which of the four controls this is.
    #[must_use]
    pub fn kind(&self) -> ControlKind {
        self.kind
    }

    /// The release the record is read against.
    #[must_use]
    pub fn release(&self) -> SchemaVersion {
        self.release.version
    }

    /// Whether the release declares `attribute`, named by its IFC4 name as
    /// the accessors are (`Identification` covers IFC2X3's `PermitID`,
    /// `RequestID` and `ID`). An undeclared attribute reads as `None`; this
    /// tells it apart from one left unset.
    #[must_use]
    pub fn declares(&self, attribute: &'static str) -> bool {
        self.release
            .slot(self.kind.type_name(), attribute)
            .is_some()
    }

    /// `GlobalId`.
    #[must_use]
    pub fn global_id(&self) -> &'m str {
        self.global_id
    }

    /// `OwnerHistory`: optional from IFC4 on, required in IFC2X3.
    #[must_use]
    pub fn owner_history(&self) -> Option<EntityId> {
        match self
            .release
            .value(self.kind.type_name(), self.entity, "OwnerHistory")?
        {
            Value::Ref(id) => Some(*id),
            _ => None,
        }
    }

    /// `Name`.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.text("Name")
    }

    /// `Description`.
    #[must_use]
    pub fn description(&self) -> Option<&'m str> {
        self.text("Description")
    }

    /// `ObjectType`: the name a `USERDEFINED` predefined type stands for.
    #[must_use]
    pub fn object_type(&self) -> Option<&'m str> {
        self.text("ObjectType")
    }

    /// `Identification`, or the IFC2X3 identifier IFC4 renamed to it
    /// (`PermitID`, `RequestID`, `ID`). IFC2X3 `IfcPerformanceHistory`
    /// declares none.
    #[must_use]
    pub fn identification(&self) -> Option<&'m str> {
        self.text("Identification")
    }

    /// The `PredefinedType` token without its dots, e.g. `WORKORDER`.
    /// IFC2X3 declares one only for `IfcProjectOrder`, where it is required.
    #[must_use]
    pub fn predefined_type(&self) -> Option<&'m str> {
        match self
            .release
            .value(self.kind.type_name(), self.entity, "PredefinedType")?
        {
            Value::Enum(token) => Some(token),
            _ => None,
        }
    }

    /// `Status`. Declared by neither performance history, nor in IFC2X3 by
    /// a permit or an action request.
    #[must_use]
    pub fn status(&self) -> Option<&'m str> {
        self.text("Status")
    }

    /// `LongDescription`. Not declared by IFC2X3 or by a performance
    /// history.
    #[must_use]
    pub fn long_description(&self) -> Option<&'m str> {
        self.text("LongDescription")
    }

    /// `LifeCyclePhase`. Declared, and required, only by a performance
    /// history.
    #[must_use]
    pub fn life_cycle_phase(&self) -> Option<&'m str> {
        self.text("LifeCyclePhase")
    }

    /// Every `IfcRelAssignsToControl` whose `RelatingControl` is this
    /// control, in file order.
    ///
    /// # Errors
    ///
    /// The record refusals of [`read_control`] for a relationship that
    /// names this control but does not fit its release. Relationships
    /// naming other controls are not inspected.
    pub fn assignments(&self) -> ControlResult<Vec<ControlAssignment<'m>>> {
        let release = self.release;
        let Some(relating) = release.slot(RELATION, "RelatingControl") else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for (id, entity) in self.model.of_type(RELATION) {
            if entity.attribute(relating) != Some(&Value::Ref(self.id)) {
                continue;
            }
            release.check(RELATION, id, entity)?;
            let global_id = release.text(RELATION, entity, "GlobalId").ok_or(
                ControlError::MissingAttribute {
                    entity: RELATION,
                    id,
                    attribute: "GlobalId",
                },
            )?;
            out.push(ControlAssignment {
                id,
                entity,
                release,
                global_id,
            });
        }
        Ok(out)
    }
}

/// A borrowed view of one `IfcRelAssignsToControl` relating work to a
/// [`Control`], checked against the same release.
#[derive(Debug, Clone, Copy)]
pub struct ControlAssignment<'m> {
    id: EntityId,
    entity: &'m Entity,
    release: Bound,
    global_id: &'m str,
}

impl<'m> ControlAssignment<'m> {
    /// The relationship's entity id.
    #[must_use]
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// `GlobalId`.
    #[must_use]
    pub fn global_id(&self) -> &'m str {
        self.global_id
    }

    /// `OwnerHistory`.
    #[must_use]
    pub fn owner_history(&self) -> Option<EntityId> {
        match self.release.value(RELATION, self.entity, "OwnerHistory")? {
            Value::Ref(id) => Some(*id),
            _ => None,
        }
    }

    /// `Name`.
    #[must_use]
    pub fn name(&self) -> Option<&'m str> {
        self.release.text(RELATION, self.entity, "Name")
    }

    /// `Description`.
    #[must_use]
    pub fn description(&self) -> Option<&'m str> {
        self.release.text(RELATION, self.entity, "Description")
    }

    /// `RelatedObjects`: the work the control governs, in stored order.
    #[must_use]
    pub fn related_objects(&self) -> Vec<EntityId> {
        self.release
            .value(RELATION, self.entity, "RelatedObjects")
            .and_then(Value::as_list)
            .unwrap_or_default()
            .iter()
            .filter_map(|item| match item {
                Value::Ref(id) => Some(*id),
                _ => None,
            })
            .collect()
    }
}
