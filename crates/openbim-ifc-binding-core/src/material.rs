//! Materials (feature `material`, #123).
//!
//! The facade's material view resolves the one material association that
//! applies to an object: its own `IfcRelAssociatesMaterial`, or else its
//! type object's. The association crosses as one record: what it points at
//! (a material, a list, a layer, profile or constituent set, or a set
//! usage), the set's layers, profiles or constituents with their materials,
//! and a usage's placement parameters. Records are read against the release
//! the header declares, IFC2X3, IFC4 or IFC4X3; any other is refused with
//! `unsupported-schema`.
//!
//! Thicknesses, fractions and offsets are attributes the schema types as
//! `REAL` measures, in the file's length unit, so they cross as host
//! numbers; nothing is converted. A descriptive attribute the bound release
//! does not declare (`Name` of an IFC2X3 `IfcMaterialLayer`, `Category` of
//! an IFC2X3 `IfcMaterial`) is `null`, as is one the file leaves unset.

use crate::record::{Field, Record, ToRecord};
use crate::value::Tagged;
use crate::{BindingError, IfcModel};

/// The material association that applies to an object.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialAssignment {
    /// The `IfcRelAssociatesMaterial`.
    pub relationship: u64,
    /// The relationship's `GlobalId`.
    pub global_id: Option<String>,
    /// `occurrence`: associated with the queried object; `type`: inherited
    /// from its type object.
    pub source: String,
    /// For `type`, the type object's id.
    pub type_object: Option<u64>,
    /// The `RelatingMaterial`.
    pub target: u64,
    /// The target's entity type, upper-case.
    pub type_name: String,
    /// `material`, `list`, `layer`, `layer-set`, `layer-set-usage`,
    /// `profile`, `profile-set`, `profile-set-usage`, `constituent` or
    /// `constituent-set`.
    pub kind: String,
    /// For a set or a usage, the set's id (a usage's `ForLayerSet` or
    /// `ForProfileSet`).
    pub set: Option<u64>,
    /// The set's `LayerSetName` or `Name`.
    pub name: Option<String>,
    /// `material`: the material; `list`: its materials, in file order.
    pub materials: Vec<MaterialRef>,
    /// `layer`, `layer-set`, `layer-set-usage`: the layers, in file order.
    pub layers: Vec<MaterialLayer>,
    /// `profile`, `profile-set`, `profile-set-usage`: the profiles.
    pub profiles: Vec<MaterialProfile>,
    /// `constituent`, `constituent-set`: the constituents.
    pub constituents: Vec<MaterialConstituent>,
    /// `layer-set-usage` and `profile-set-usage`: how the set is placed.
    pub usage: Option<MaterialUsage>,
}

/// An `IfcMaterial`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialRef {
    /// Its entity id.
    pub id: u64,
    /// `Name`.
    pub name: String,
    /// `Category` (IFC4, IFC4X3).
    pub category: Option<String>,
}

/// An `IfcMaterialLayer` (or `IfcMaterialLayerWithOffsets`).
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialLayer {
    /// Its entity id.
    pub id: u64,
    /// `Material`, when stated (an air gap may have none).
    pub material: Option<MaterialRef>,
    /// `LayerThickness`, in the file's length unit.
    pub thickness: f64,
    /// `IsVentilated`: `bool`, `unknown`, or `null` when unset.
    pub is_ventilated: Tagged,
    /// `Name` (IFC4, IFC4X3).
    pub name: Option<String>,
    /// `Category` (IFC4, IFC4X3).
    pub category: Option<String>,
    /// `Priority` (IFC4, IFC4X3).
    pub priority: Option<i64>,
}

/// An `IfcMaterialProfile` (or `IfcMaterialProfileWithOffsets`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialProfile {
    /// Its entity id.
    pub id: u64,
    /// `Material`, when stated.
    pub material: Option<MaterialRef>,
    /// `Profile`: the `IfcProfileDef`.
    pub profile: u64,
    /// `Name`.
    pub name: Option<String>,
    /// `Category`.
    pub category: Option<String>,
    /// `Priority`.
    pub priority: Option<i64>,
}

/// An `IfcMaterialConstituent`.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialConstituent {
    /// Its entity id.
    pub id: u64,
    /// `Material`.
    pub material: MaterialRef,
    /// `Name`.
    pub name: Option<String>,
    /// `Category`.
    pub category: Option<String>,
    /// `Fraction`, a ratio.
    pub fraction: Option<f64>,
}

/// How a layer or profile set is placed relative to the element.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialUsage {
    /// Layer sets: `AXIS1`, `AXIS2` or `AXIS3`.
    pub layer_set_direction: Option<String>,
    /// Layer sets: `POSITIVE` or `NEGATIVE`.
    pub direction_sense: Option<String>,
    /// Layer sets: `OffsetFromReferenceLine`, in the file's length unit.
    pub offset_from_reference_line: Option<f64>,
    /// `ReferenceExtent`, when stated.
    pub reference_extent: Option<f64>,
    /// Profile sets: `CardinalPoint`.
    pub cardinal_point: Option<i64>,
    /// Tapering profile sets: `ForProfileEndSet`.
    pub end_set: Option<u64>,
    /// Tapering profile sets: `CardinalEndPoint`.
    pub cardinal_end_point: Option<i64>,
}

impl IfcModel {
    /// The material association that applies to `object`, or `None` when
    /// neither it nor its type object has one.
    ///
    /// Refused with `missing-entity` for an id not in the model,
    /// `unsupported-schema` for a release other than IFC2X3, IFC4 or
    /// IFC4X3, `invalid-model` when two associations (or two types) compete,
    /// `missing-reference` for a dangling one, and `feature-disabled`
    /// without the `material` feature.
    pub fn material(&self, object: u64) -> Result<Option<MaterialAssignment>, BindingError> {
        #[cfg(feature = "material")]
        {
            read::material(self, object)
        }
        #[cfg(not(feature = "material"))]
        {
            let _ = object;
            Err(BindingError::FeatureDisabled("material"))
        }
    }
}

#[cfg(feature = "material")]
mod read {
    use ifc::material::{
        material_schema, AssignmentSource, LogicalValue, MaterialDefinition, MaterialError,
        MaterialResult, MaterialUsageDefinition, MaterialView, ResolvedMaterialSelect,
    };
    use ifc::EntityId;

    use super::{
        MaterialAssignment, MaterialConstituent, MaterialLayer, MaterialProfile, MaterialRef,
        MaterialUsage,
    };
    use crate::value::Tagged;
    use crate::{BindingError, IfcModel};

    pub(super) fn material(
        model: &IfcModel,
        object: u64,
    ) -> Result<Option<MaterialAssignment>, BindingError> {
        if !model.inner.contains(EntityId(object)) {
            return Err(BindingError::MissingEntity(object));
        }
        material_schema(&model.inner).map_err(error)?;
        let view = MaterialView::new(&model.inner);
        let Some(resolved) = view.assigned_material(EntityId(object)).map_err(error)? else {
            return Ok(None);
        };
        let (source, type_object) = match resolved.source {
            AssignmentSource::Occurrence => ("occurrence", None),
            AssignmentSource::Type(id) => ("type", Some(id.0)),
        };
        let target = resolved.assignment.relating_material_id().map_err(error)?;
        let mut out = MaterialAssignment {
            relationship: resolved.assignment.id().0,
            global_id: Some(resolved.assignment.global_id().map_err(error)?.to_owned()),
            source: source.to_owned(),
            type_object,
            target: target.0,
            type_name: model.type_of(target.0)?.to_owned(),
            kind: String::new(),
            set: None,
            name: None,
            materials: Vec::new(),
            layers: Vec::new(),
            profiles: Vec::new(),
            constituents: Vec::new(),
            usage: None,
        };
        fill(view, resolved.material, &mut out)?;
        Ok(Some(out))
    }

    /// Fill `out` from the resolved select branch.
    fn fill(
        view: MaterialView<'_>,
        select: ResolvedMaterialSelect<'_>,
        out: &mut MaterialAssignment,
    ) -> Result<(), BindingError> {
        use MaterialDefinition as D;
        use MaterialUsageDefinition as U;
        use ResolvedMaterialSelect as S;
        match select {
            S::Definition(D::Material(material)) => {
                out.kind = "material".to_owned();
                out.materials.push(material_ref(view, material.id())?);
            }
            S::List(list) => {
                out.kind = "list".to_owned();
                for id in list.material_ids().map_err(error)? {
                    out.materials.push(material_ref(view, id)?);
                }
            }
            S::Definition(D::Layer(_) | D::LayerWithOffsets(_)) => {
                out.kind = "layer".to_owned();
                out.layers.push(layer(view, EntityId(out.target))?);
            }
            S::Definition(D::LayerSet(set)) => {
                out.kind = "layer-set".to_owned();
                layer_set(view, set.id(), out)?;
            }
            S::Usage(U::LayerSet(usage)) => {
                out.kind = "layer-set-usage".to_owned();
                layer_set(view, usage.layer_set_id().map_err(error)?, out)?;
                out.usage = Some(MaterialUsage {
                    layer_set_direction: Some(
                        usage
                            .layer_set_direction()
                            .map_err(error)?
                            .as_token()
                            .to_owned(),
                    ),
                    direction_sense: Some(
                        usage
                            .direction_sense()
                            .map_err(error)?
                            .as_token()
                            .to_owned(),
                    ),
                    offset_from_reference_line: Some(
                        usage.offset_from_reference_line().map_err(error)?,
                    ),
                    reference_extent: declared(usage.reference_extent())?.flatten(),
                    cardinal_point: None,
                    end_set: None,
                    cardinal_end_point: None,
                });
            }
            S::Definition(D::Profile(_) | D::ProfileWithOffsets(_)) => {
                out.kind = "profile".to_owned();
                out.profiles.push(profile(view, EntityId(out.target))?);
            }
            S::Definition(D::ProfileSet(set)) => {
                out.kind = "profile-set".to_owned();
                profile_set(view, set.id(), out)?;
            }
            S::Usage(U::ProfileSet(usage)) => {
                out.kind = "profile-set-usage".to_owned();
                profile_set(view, usage.profile_set_id().map_err(error)?, out)?;
                let mut placed = MaterialUsage {
                    layer_set_direction: None,
                    direction_sense: None,
                    offset_from_reference_line: None,
                    reference_extent: usage.reference_extent().map_err(error)?,
                    cardinal_point: cardinal(usage.cardinal_point().map_err(error)?),
                    end_set: None,
                    cardinal_end_point: None,
                };
                if let Some(tapering) = usage.tapering() {
                    placed.end_set = Some(tapering.end_profile_set_id().map_err(error)?.0);
                    placed.cardinal_end_point =
                        cardinal(tapering.cardinal_end_point().map_err(error)?);
                }
                out.usage = Some(placed);
            }
            S::Usage(U::ProfileSetTapering(usage)) => {
                out.kind = "profile-set-usage".to_owned();
                profile_set(view, usage.profile_set_id().map_err(error)?, out)?;
                out.usage = Some(MaterialUsage {
                    layer_set_direction: None,
                    direction_sense: None,
                    offset_from_reference_line: None,
                    reference_extent: usage.reference_extent().map_err(error)?,
                    cardinal_point: cardinal(usage.cardinal_point().map_err(error)?),
                    end_set: Some(usage.end_profile_set_id().map_err(error)?.0),
                    cardinal_end_point: cardinal(usage.cardinal_end_point().map_err(error)?),
                });
            }
            S::Definition(D::Constituent(_)) => {
                out.kind = "constituent".to_owned();
                out.constituents
                    .push(constituent(view, EntityId(out.target))?);
            }
            S::Definition(D::ConstituentSet(set)) => {
                out.kind = "constituent-set".to_owned();
                out.set = Some(set.id().0);
                out.name = declared(set.name())?.flatten().map(str::to_owned);
                for id in set.constituent_ids().map_err(error)?.unwrap_or_default() {
                    out.constituents.push(constituent(view, id)?);
                }
            }
        }
        Ok(())
    }

    fn layer_set(
        view: MaterialView<'_>,
        id: EntityId,
        out: &mut MaterialAssignment,
    ) -> Result<(), BindingError> {
        let set = ifc::material::MaterialLayerSet::try_from_view(view, id).map_err(error)?;
        out.set = Some(id.0);
        out.name = declared(set.name())?.flatten().map(str::to_owned);
        for layer_id in set.layer_ids().map_err(error)? {
            out.layers.push(layer(view, layer_id)?);
        }
        Ok(())
    }

    fn profile_set(
        view: MaterialView<'_>,
        id: EntityId,
        out: &mut MaterialAssignment,
    ) -> Result<(), BindingError> {
        let set = ifc::material::MaterialProfileSet::try_from_view(view, id).map_err(error)?;
        out.set = Some(id.0);
        out.name = declared(set.name())?.flatten().map(str::to_owned);
        for profile_id in set.profile_ids().map_err(error)? {
            out.profiles.push(profile(view, profile_id)?);
        }
        Ok(())
    }

    /// The shared accessors of a layer and a layer with offsets.
    macro_rules! read_layer {
        ($view:expr, $layer:expr) => {{
            let layer = $layer;
            Ok(MaterialLayer {
                id: layer.id().0,
                material: optional_material($view, layer.material_id().map_err(error)?)?,
                thickness: layer.thickness().map_err(error)?,
                is_ventilated: match layer.is_ventilated().map_err(error)? {
                    None => Tagged::Null,
                    Some(LogicalValue::True) => Tagged::Bool(true),
                    Some(LogicalValue::False) => Tagged::Bool(false),
                    Some(LogicalValue::Unknown) => Tagged::Unknown,
                },
                name: declared(layer.name())?.flatten().map(str::to_owned),
                category: declared(layer.category())?.flatten().map(str::to_owned),
                priority: declared(layer.priority())?.flatten(),
            })
        }};
    }

    fn layer(view: MaterialView<'_>, id: EntityId) -> Result<MaterialLayer, BindingError> {
        match view.resolve_material_select(id).map_err(error)? {
            ResolvedMaterialSelect::Definition(MaterialDefinition::Layer(layer)) => {
                read_layer!(view, layer)
            }
            ResolvedMaterialSelect::Definition(MaterialDefinition::LayerWithOffsets(layer)) => {
                read_layer!(view, layer)
            }
            _ => Err(wrong(view, id, "IfcMaterialLayer")),
        }
    }

    /// The shared accessors of a profile and a profile with offsets.
    macro_rules! read_profile {
        ($view:expr, $profile:expr) => {{
            let profile = $profile;
            Ok(MaterialProfile {
                id: profile.id().0,
                material: optional_material($view, profile.material_id().map_err(error)?)?,
                profile: profile.profile_id().map_err(error)?.0,
                name: declared(profile.name())?.flatten().map(str::to_owned),
                category: declared(profile.category())?.flatten().map(str::to_owned),
                priority: declared(profile.priority())?.flatten(),
            })
        }};
    }

    fn profile(view: MaterialView<'_>, id: EntityId) -> Result<MaterialProfile, BindingError> {
        match view.resolve_material_select(id).map_err(error)? {
            ResolvedMaterialSelect::Definition(MaterialDefinition::Profile(profile)) => {
                read_profile!(view, profile)
            }
            ResolvedMaterialSelect::Definition(MaterialDefinition::ProfileWithOffsets(profile)) => {
                read_profile!(view, profile)
            }
            _ => Err(wrong(view, id, "IfcMaterialProfile")),
        }
    }

    fn constituent(
        view: MaterialView<'_>,
        id: EntityId,
    ) -> Result<MaterialConstituent, BindingError> {
        let constituent =
            ifc::material::MaterialConstituent::try_from_view(view, id).map_err(error)?;
        Ok(MaterialConstituent {
            id: id.0,
            material: material_ref(view, constituent.material_id().map_err(error)?)?,
            name: declared(constituent.name())?.flatten().map(str::to_owned),
            category: declared(constituent.category())?
                .flatten()
                .map(str::to_owned),
            fraction: declared(constituent.fraction())?.flatten(),
        })
    }

    fn optional_material(
        view: MaterialView<'_>,
        id: Option<EntityId>,
    ) -> Result<Option<MaterialRef>, BindingError> {
        id.map(|id| material_ref(view, id)).transpose()
    }

    fn material_ref(view: MaterialView<'_>, id: EntityId) -> Result<MaterialRef, BindingError> {
        let material = ifc::material::Material::try_from_view(view, id).map_err(error)?;
        Ok(MaterialRef {
            id: id.0,
            name: material.name().map_err(error)?.to_owned(),
            category: declared(material.category())?.flatten().map(str::to_owned),
        })
    }

    fn cardinal(point: Option<ifc::material::CardinalPointReference>) -> Option<i64> {
        point.map(|point| i64::try_from(point.get()).unwrap_or(i64::MAX))
    }

    fn wrong(view: MaterialView<'_>, id: EntityId, expected: &str) -> BindingError {
        let actual = view
            .model()
            .get(id)
            .map_or("nothing", |entity| &*entity.type_name);
        BindingError::WrongEntityType(format!("#{} is {actual}, not {expected}", id.0))
    }

    /// An attribute the bound release does not declare reads as `None`;
    /// every other failure is the read's.
    fn declared<T>(read: MaterialResult<T>) -> Result<Option<T>, BindingError> {
        match read {
            Ok(value) => Ok(Some(value)),
            Err(MaterialError::NotInSchema { .. }) => Ok(None),
            Err(other) => Err(error(other)),
        }
    }

    fn error(error: MaterialError) -> BindingError {
        use MaterialError as E;
        let detail = error.to_string();
        match error {
            E::UnsupportedSchema { schema } => BindingError::UnsupportedSchema(schema),
            E::MultipleSchemas { .. } => BindingError::UnsupportedSchema(detail),
            E::UnknownEntity { id } => BindingError::MissingEntity(id.0),
            E::DanglingReference { .. } => BindingError::MissingReference(detail),
            E::WrongEntityType { .. } => BindingError::WrongEntityType(detail),
            _ => BindingError::InvalidModel(detail),
        }
    }
}

impl ToRecord for MaterialAssignment {
    fn to_record(&self) -> Record {
        Record::new(
            "MaterialAssignment",
            vec![
                ("relationship", Field::Id(self.relationship)),
                ("global_id", Field::text(self.global_id.clone())),
                ("source", Field::Text(self.source.clone())),
                ("type_object", Field::id(self.type_object)),
                ("target", Field::Id(self.target)),
                ("type_name", Field::Text(self.type_name.clone())),
                ("kind", Field::Text(self.kind.clone())),
                ("set", Field::id(self.set)),
                ("name", Field::text(self.name.clone())),
                ("materials", Field::records(&self.materials)),
                ("layers", Field::records(&self.layers)),
                ("profiles", Field::records(&self.profiles)),
                ("constituents", Field::records(&self.constituents)),
                ("usage", Field::record(self.usage.as_ref())),
            ],
        )
    }
}

impl ToRecord for MaterialRef {
    fn to_record(&self) -> Record {
        Record::new(
            "MaterialRef",
            vec![
                ("id", Field::Id(self.id)),
                ("name", Field::Text(self.name.clone())),
                ("category", Field::text(self.category.clone())),
            ],
        )
    }
}

impl ToRecord for MaterialLayer {
    fn to_record(&self) -> Record {
        Record::new(
            "MaterialLayer",
            vec![
                ("id", Field::Id(self.id)),
                ("material", Field::record(self.material.as_ref())),
                ("thickness", Field::Real(self.thickness)),
                ("is_ventilated", Field::Value(self.is_ventilated.clone())),
                ("name", Field::text(self.name.clone())),
                ("category", Field::text(self.category.clone())),
                ("priority", Field::optional(self.priority, Field::Int)),
            ],
        )
    }
}

impl ToRecord for MaterialProfile {
    fn to_record(&self) -> Record {
        Record::new(
            "MaterialProfile",
            vec![
                ("id", Field::Id(self.id)),
                ("material", Field::record(self.material.as_ref())),
                ("profile", Field::Id(self.profile)),
                ("name", Field::text(self.name.clone())),
                ("category", Field::text(self.category.clone())),
                ("priority", Field::optional(self.priority, Field::Int)),
            ],
        )
    }
}

impl ToRecord for MaterialConstituent {
    fn to_record(&self) -> Record {
        Record::new(
            "MaterialConstituent",
            vec![
                ("id", Field::Id(self.id)),
                ("material", Field::Record(self.material.to_record())),
                ("name", Field::text(self.name.clone())),
                ("category", Field::text(self.category.clone())),
                ("fraction", Field::optional(self.fraction, Field::Real)),
            ],
        )
    }
}

impl ToRecord for MaterialUsage {
    fn to_record(&self) -> Record {
        Record::new(
            "MaterialUsage",
            vec![
                (
                    "layer_set_direction",
                    Field::text(self.layer_set_direction.clone()),
                ),
                ("direction_sense", Field::text(self.direction_sense.clone())),
                (
                    "offset_from_reference_line",
                    Field::optional(self.offset_from_reference_line, Field::Real),
                ),
                (
                    "reference_extent",
                    Field::optional(self.reference_extent, Field::Real),
                ),
                (
                    "cardinal_point",
                    Field::optional(self.cardinal_point, Field::Int),
                ),
                ("end_set", Field::id(self.end_set)),
                (
                    "cardinal_end_point",
                    Field::optional(self.cardinal_end_point, Field::Int),
                ),
            ],
        )
    }
}
