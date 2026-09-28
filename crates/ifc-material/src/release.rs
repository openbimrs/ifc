//! The IFC release a model's material records are read and written against.
//!
//! Every slot position comes from the bundled table of one release, so an
//! IFC2X3 record is never decoded or authored with IFC4 positions.
//!
//! Binding, from `FILE_SCHEMA`:
//! - one recognised declaration (`IFC2X3`, `IFC4`, `IFC4X3`/`IFC4X3_ADD2`)
//!   binds that release's own table;
//! - one unrecognised declaration fails closed with
//!   [`MaterialError::UnsupportedSchema`];
//! - several declarations fail closed with [`MaterialError::MultipleSchemas`];
//! - no declaration at all (an in-memory [`Model::new`]) binds IFC4, the
//!   0.2.0 behaviour, as `ifc-classification` does since #51.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{for_version, Attribute, Schema, SchemaVersion};

use crate::{MaterialError, MaterialResult};

/// Every attribute this crate reads or writes, per entity, under its IFC4
/// ADD2 TC1 name and in IFC4 positional order (inherited attributes first).
///
/// This is the IFC4 layout the crate was written against. Accessors never
/// index it: they ask the bound release for the attribute's position by
/// name. `release_layout` tests pin it against the three bundled tables.
pub(crate) const IFC4_LAYOUT: &[(&str, &[&str])] = &[
    ("IFCMATERIAL", &["Name", "Description", "Category"]),
    (
        "IFCMATERIALCLASSIFICATIONRELATIONSHIP",
        &["MaterialClassifications", "ClassifiedMaterial"],
    ),
    (
        "IFCMATERIALCONSTITUENT",
        &["Name", "Description", "Material", "Fraction", "Category"],
    ),
    (
        "IFCMATERIALCONSTITUENTSET",
        &["Name", "Description", "MaterialConstituents"],
    ),
    (
        "IFCMATERIALDEFINITIONREPRESENTATION",
        &[
            "Name",
            "Description",
            "Representations",
            "RepresentedMaterial",
        ],
    ),
    ("IFCMATERIALLAYER", LAYER),
    (
        "IFCMATERIALLAYERSET",
        &["MaterialLayers", "LayerSetName", "Description"],
    ),
    (
        "IFCMATERIALLAYERSETUSAGE",
        &[
            "ForLayerSet",
            "LayerSetDirection",
            "DirectionSense",
            "OffsetFromReferenceLine",
            "ReferenceExtent",
        ],
    ),
    (
        "IFCMATERIALLAYERWITHOFFSETS",
        &[
            "Material",
            "LayerThickness",
            "IsVentilated",
            "Name",
            "Description",
            "Category",
            "Priority",
            "OffsetDirection",
            "OffsetValues",
        ],
    ),
    ("IFCMATERIALLIST", &["Materials"]),
    ("IFCMATERIALPROFILE", PROFILE),
    (
        "IFCMATERIALPROFILESET",
        &[
            "Name",
            "Description",
            "MaterialProfiles",
            "CompositeProfile",
        ],
    ),
    ("IFCMATERIALPROFILESETUSAGE", PROFILE_USAGE),
    (
        "IFCMATERIALPROFILESETUSAGETAPERING",
        &[
            "ForProfileSet",
            "CardinalPoint",
            "ReferenceExtent",
            "ForProfileEndSet",
            "CardinalEndPoint",
        ],
    ),
    (
        "IFCMATERIALPROFILEWITHOFFSETS",
        &[
            "Name",
            "Description",
            "Material",
            "Profile",
            "Priority",
            "Category",
            "OffsetValues",
        ],
    ),
    (
        "IFCMATERIALPROPERTIES",
        &["Name", "Description", "Properties", "Material"],
    ),
    (
        "IFCMATERIALRELATIONSHIP",
        &[
            "Name",
            "Description",
            "RelatingMaterial",
            "RelatedMaterials",
            "Expression",
        ],
    ),
    ("IFCRELASSOCIATESMATERIAL", ROOT_RELATION_MATERIAL),
    ("IFCRELDEFINESBYTYPE", ROOT_RELATION_TYPE),
];

const LAYER: &[&str] = &[
    "Material",
    "LayerThickness",
    "IsVentilated",
    "Name",
    "Description",
    "Category",
    "Priority",
];
const PROFILE: &[&str] = &[
    "Name",
    "Description",
    "Material",
    "Profile",
    "Priority",
    "Category",
];
const PROFILE_USAGE: &[&str] = &["ForProfileSet", "CardinalPoint", "ReferenceExtent"];
const ROOT_RELATION_MATERIAL: &[&str] = &[
    "GlobalId",
    "OwnerHistory",
    "Name",
    "Description",
    "RelatedObjects",
    "RelatingMaterial",
];
const ROOT_RELATION_TYPE: &[&str] = &[
    "GlobalId",
    "OwnerHistory",
    "Name",
    "Description",
    "RelatedObjects",
    "RelatingType",
];

/// The release a projection or authoring call binds to, or why none could be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Release<'m> {
    /// Reads and writes resolve against this release's bundled table.
    Bound(SchemaVersion),
    /// The header declares this many schemas.
    Multiple(usize),
    /// The header declares one schema this crate has no table for.
    Unsupported(&'m str),
}

/// Releases this crate's layouts are proven against.
///
/// `ifc-schema` also bundles IFC4X1 and IFC4X2, but nothing here is verified
/// against their tables, so a header declaring either is refused with the
/// unsupported-schema error rather than read through a neighbour's layout.
const fn proven(version: SchemaVersion) -> bool {
    matches!(
        version,
        SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3
    )
}

impl<'m> Release<'m> {
    /// The binding of projections built without a model (`try_new`) and of
    /// a model whose header declares no schema.
    pub(crate) const LEGACY: Release<'static> = Release::Bound(SchemaVersion::Ifc4);

    /// The release `model`'s header binds.
    pub(crate) fn of(model: &'m Model) -> Self {
        match model.header().schema.as_slice() {
            [] => Release::LEGACY,
            [token] => SchemaVersion::from_header_token(token)
                .filter(|version| proven(*version))
                .map_or(Self::Unsupported(token.as_str()), Self::Bound),
            tokens => Self::Multiple(tokens.len()),
        }
    }

    /// The bound version and its bundled table.
    pub(crate) fn bound(self) -> MaterialResult<(SchemaVersion, &'static Schema)> {
        match self {
            Self::Bound(version) => Ok((
                version,
                for_version(version).expect("every SchemaVersion has a bundled table"),
            )),
            Self::Multiple(schemas) => Err(MaterialError::MultipleSchemas { schemas }),
            Self::Unsupported(schema) => Err(MaterialError::UnsupportedSchema {
                schema: schema.to_owned(),
            }),
        }
    }

    /// Whether the bound release declares `entity` as an instantiable
    /// (non-abstract) entity. `false` when no release is bound.
    pub(crate) fn instantiates(self, entity: &str) -> bool {
        self.bound()
            .is_ok_and(|(_, schema)| instantiable(schema, entity))
    }

    /// Fail with [`MaterialError::EntityNotInSchema`] unless the bound
    /// release can instantiate `entity`.
    pub(crate) fn require_entity(
        self,
        entity: &'static str,
        id: Option<EntityId>,
    ) -> MaterialResult<(SchemaVersion, &'static Schema)> {
        let (version, schema) = self.bound()?;
        if instantiable(schema, entity) {
            Ok((version, schema))
        } else {
            Err(MaterialError::EntityNotInSchema {
                entity,
                id,
                schema: version,
            })
        }
    }

    /// Position and declaration of `attribute` (IFC4 name) on `entity` in
    /// the bound release.
    ///
    /// A record type the release cannot instantiate is `EntityNotInSchema`;
    /// an attribute it does not declare is `NotInSchema`. Neither is ever a
    /// silent `None` read from a slot the release gives another meaning.
    pub(crate) fn attribute(
        self,
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
    ) -> MaterialResult<(usize, &'static Attribute)> {
        debug_assert!(
            in_ifc4_layout(entity, attribute),
            "{entity}.{attribute} is read but missing from IFC4_LAYOUT"
        );
        let (version, schema) = self.require_entity(entity, Some(id))?;
        position(schema, version, entity, attribute).ok_or(MaterialError::NotInSchema {
            entity,
            id,
            attribute,
            schema: version,
        })
    }

    /// Position of `attribute` (IFC4 name) on `entity` in the bound release.
    pub(crate) fn slot(
        self,
        entity: &'static str,
        id: EntityId,
        attribute: &'static str,
    ) -> MaterialResult<usize> {
        self.attribute(entity, id, attribute).map(|(slot, _)| slot)
    }

    /// Whether `candidate` is a legal value type of `attribute` on `entity`
    /// in the bound release (entity inheritance and SELECTs included), and
    /// instantiable there.
    pub(crate) fn accepts(
        self,
        entity: &'static str,
        attribute: &'static str,
        candidate: &str,
    ) -> MaterialResult<(bool, &'static str)> {
        let (_, schema) = self.bound()?;
        let declared = self.declared(entity, attribute)?.type_name.as_str();
        Ok((
            instantiable(schema, candidate) && schema.accepts_type(declared, candidate),
            declared,
        ))
    }

    /// The bound release's declaration of `attribute` (IFC4 name) on
    /// `entity`, for authoring: `EntityNotInSchema` or
    /// `AuthoringNotInSchema` when the release lacks either.
    pub(crate) fn declared(
        self,
        entity: &'static str,
        attribute: &'static str,
    ) -> MaterialResult<&'static Attribute> {
        let (version, schema) = self.require_entity(entity, None)?;
        position(schema, version, entity, attribute)
            .map(|(_, declared)| declared)
            .ok_or(MaterialError::AuthoringNotInSchema {
                entity,
                attribute,
                schema: version,
            })
    }

    /// Build `entity`'s record in the bound release's layout from values
    /// named by their IFC4 attribute names.
    ///
    /// A non-null value for an attribute the release does not declare is
    /// refused with `AuthoringNotInSchema` rather than dropped; a null for
    /// an attribute the release requires is refused with
    /// `AuthoringRequired`.
    pub(crate) fn record(
        self,
        entity: &'static str,
        values: Vec<(&'static str, Value)>,
    ) -> MaterialResult<Entity> {
        let (version, schema) = self.require_entity(entity, None)?;
        let declared = schema.attributes(entity);
        let mut slots = vec![None; declared.len()];
        for (attribute, value) in values {
            debug_assert!(
                in_ifc4_layout(entity, attribute),
                "{entity}.{attribute} is written but missing from IFC4_LAYOUT"
            );
            match position(schema, version, entity, attribute) {
                Some((slot, _)) => slots[slot] = Some(value),
                None if value == Value::Null => {}
                None => {
                    return Err(MaterialError::AuthoringNotInSchema {
                        entity,
                        attribute,
                        schema: version,
                    })
                }
            }
        }
        let mut attributes = Vec::with_capacity(slots.len());
        for (slot, value) in slots.into_iter().enumerate() {
            let declaration = declared[slot];
            match value.unwrap_or(Value::Null) {
                Value::Null if !declaration.optional => {
                    return Err(MaterialError::AuthoringRequired {
                        entity,
                        attribute: declaration.name.as_str(),
                        schema: version,
                    })
                }
                value => attributes.push(value),
            }
        }
        Ok(Entity::new(entity, attributes))
    }
}

fn instantiable(schema: &Schema, entity: &str) -> bool {
    schema
        .entity(entity)
        .is_some_and(|declaration| !declaration.abstract_)
}

fn position(
    schema: &'static Schema,
    version: SchemaVersion,
    entity: &str,
    attribute: &'static str,
) -> Option<(usize, &'static Attribute)> {
    let name = release_name(version, entity, attribute);
    schema
        .attributes(entity)
        .into_iter()
        .enumerate()
        .find(|(_, declared)| declared.name.eq_ignore_ascii_case(name))
}

fn in_ifc4_layout(entity: &str, attribute: &str) -> bool {
    IFC4_LAYOUT.iter().any(|(declared, attributes)| {
        declared.eq_ignore_ascii_case(entity) && attributes.contains(&attribute)
    })
}

/// The name `release` gives the attribute this crate knows by its IFC4 name.
///
/// IFC4X3 ADD2 renamed `IfcMaterialRelationship.Expression` to
/// `MaterialExpression`; it keeps its position (5th, after the two inherited
/// `IfcResourceLevelRelationship` attributes) and meaning. No other material
/// attribute is renamed between IFC2X3 TC1, IFC4 ADD2 TC1 and IFC4X3 ADD2.
pub(crate) fn release_name(
    release: SchemaVersion,
    entity: &str,
    attribute: &'static str,
) -> &'static str {
    match (release, attribute) {
        (SchemaVersion::Ifc4x3, "Expression")
            if entity.eq_ignore_ascii_case("IFCMATERIALRELATIONSHIP") =>
        {
            "MaterialExpression"
        }
        _ => attribute,
    }
}

/// Whether `entity` is an instantiable `IfcTypeObject` in the bound
/// release, the only legal `IfcRelDefinesByType.RelatingType`.
///
/// Read from the release's table: IFC4X3 removed `IfcDoorStyle` and
/// `IfcWindowStyle`, and IFC2X3 has no `IfcTypeProcess` branch.
pub(crate) fn is_type_object(release: Release<'_>, entity: &str) -> MaterialResult<bool> {
    let (_, schema) = release.bound()?;
    Ok(instantiable(schema, entity) && schema.is_a(entity, "IFCTYPEOBJECT"))
}

/// The IFC release `model`'s material records are read and written against.
///
/// A consumer binds its own vocabulary with this answer instead of
/// re-parsing the header. A header with no declaration binds IFC4 (an
/// in-memory model; the 0.2.0 behaviour).
///
/// # Errors
///
/// [`MaterialError::MultipleSchemas`] when the header declares several
/// schemas, and [`MaterialError::UnsupportedSchema`] when it declares one
/// this crate is not verified for (including IFC4X1 and IFC4X2).
pub fn material_schema(model: &Model) -> MaterialResult<SchemaVersion> {
    Release::of(model).bound().map(|(version, _)| version)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod intermediate_release_tests {
    use super::*;

    /// IFC4X1 and IFC4X2 have bundled tables but no verified layout here:
    /// refused with the unsupported-schema error, never read as IFC4/IFC4X3.
    #[test]
    fn ifc4x1_and_ifc4x2_are_refused_not_aliased() {
        for token in ["IFC4X1", "IFC4X2"] {
            let mut model = Model::new();
            model.header_mut().schema = vec![token.to_owned()];
            assert!(
                matches!(Release::of(&model).bound(), Err(MaterialError::UnsupportedSchema { schema }) if schema == token),
                "{token} must be refused"
            );
        }
    }
}
