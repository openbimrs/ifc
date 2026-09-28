//! Material property sets in exact resolution (#218).
//!
//! A material's property sets are no `IfcPropertySetDefinition` assigned by
//! a relationship. Each points at its material itself:
//!
//! ```text
//! IFC4 ADD2 TC1 / IFC4X3 ADD2
//! IfcMaterialProperties          0 OPT Name  1 OPT Description
//!                                2 Properties  3 Material : IfcMaterialDefinition
//!   (read back through the inverse IfcMaterialDefinition.HasProperties)
//!
//! IFC2X3 TC1
//! IfcMaterialProperties          0 Material : IfcMaterial   (ABSTRACT)
//! IfcExtendedMaterialProperties  1 ExtendedProperties  2 OPT Description  3 Name
//! IfcGeneralMaterialProperties, IfcMechanicalMaterialProperties, ...
//!                                own attributes after Material
//! ```
//!
//! Positions are read by name from the bound release's table. Named
//! properties resolve as those of an `IfcPropertySet` do; the attributes of
//! an IFC2X3 typed subtype resolve as those of a predefined set do, keyed
//! by its entity name (`IfcGeneralMaterialProperties`), since it has no
//! `Name`. An IFC4 set whose optional `Name` is `$` is keyed by its entity
//! name too.
//!
//! Every material property set in the file is validated, not only those of
//! the queried material, as every property relationship is for an object:
//! a malformed one could otherwise hide a set.

use ifc_model::{EntityId, Model};

use super::enumerate::{collect, list_sets, Selector};
use super::refs::{ref_at, require_ref};
use super::release::{validate_model, Release};
use super::set::find_property;
use super::{
    ExactPropertyEntry, ExactPropertyError, ExactPropertySetEntry, ExactResolution, ExactSource,
};

/// Validate `material` as a query target and collect its material property
/// sets in file order.
///
/// # Errors
///
/// A missing, foreign or mis-sized material; a material the release's
/// `IfcMaterialProperties.Material` does not accept
/// ([`ExactPropertyError::InvalidQueryObject`]); or any material property
/// set in the file with the wrong arity or a missing `Material`.
fn material_sets(
    model: &Model,
    release: Release,
    material: EntityId,
) -> Result<Vec<EntityId>, ExactPropertyError> {
    let schema = release.schema;
    let entity = model
        .get(material)
        .ok_or(ExactPropertyError::MissingReference {
            from: material,
            to: material,
        })?;
    if schema.entity(entity.type_name.as_ref()).is_none() {
        return Err(release.not_in_schema(material, entity.type_name.clone()));
    }
    let (material_slot, _) = release
        .attribute("IFCMATERIALPROPERTIES", "Material")
        .expect("IfcMaterialProperties.Material is declared in every bundled release");
    if !release.slot_accepts(
        "IFCMATERIALPROPERTIES",
        material_slot,
        entity.type_name.as_ref(),
    ) {
        return Err(ExactPropertyError::InvalidQueryObject {
            object: material,
            type_name: entity.type_name.clone(),
        });
    }
    release.require_exact_slots(material, entity)?;
    let mut ids: Vec<EntityId> = model
        .type_histogram()
        .into_iter()
        .filter(|(type_name, _)| {
            schema.entity(type_name).is_some() && schema.is_a(type_name, "IFCMATERIALPROPERTIES")
        })
        .flat_map(|(type_name, _)| model.ids_of_type(type_name).iter().copied())
        .collect();
    ids.sort_unstable();
    let mut sets = Vec::new();
    for id in ids {
        let set = model.get(id).expect("type index is current");
        release.require_exact_slots(id, set)?;
        // `Material` is inherited, so it keeps its supertype slot.
        let target = ref_at(id, set.attributes.get(material_slot), "Material")?;
        require_ref(model, id, target)?;
        if target == material {
            sets.push(id);
        }
    }
    Ok(sets)
}

/// Resolve a property of material definition `material` by exact set and
/// property name (#218).
///
/// The material's property sets are its IFC4 and IFC4X3
/// `IfcMaterialProperties` (the inverse `HasProperties`), or its IFC2X3
/// `IfcExtendedMaterialProperties` and typed `IfcMaterialProperties`
/// subtypes. A named property resolves as in [`exact_property`], with the
/// same values, units and refusals, and [`ExactSource::Material`] of
/// `material`. An attribute of an IFC2X3 typed subtype resolves as a
/// predefined set's attribute does, in the set named by its entity
/// (`IfcMechanicalMaterialProperties.YoungModulus`).
///
/// `material` is an `IfcMaterialDefinition` in IFC4 and IFC4X3 (a material,
/// layer, layer set, profile, profile set, constituent or constituent set)
/// and an `IfcMaterial` in IFC2X3. [`ExactResolution::Absent`] is a proven
/// absence, also when the material has no property sets at all. Material
/// sets do not inherit from anywhere: a layer's properties are the layer's
/// own, not its material's.
///
/// # Errors
///
/// Any [`ExactPropertyError`], as for [`exact_property`]; a query target of
/// another kind is [`ExactPropertyError::InvalidQueryObject`].
///
/// [`exact_property`]: super::exact_property
pub fn exact_material_property(
    model: &Model,
    material: EntityId,
    set_name: Option<&str>,
    property_name: &str,
) -> Result<ExactResolution, ExactPropertyError> {
    let release = validate_model(model)?;
    let sets = material_sets(model, release, material)?;
    Ok(find_property(
        model,
        release,
        &sets,
        ExactSource::Material(material),
        set_name,
        property_name,
    )?
    .map_or(ExactResolution::Absent, ExactResolution::Present))
}

/// The properties of material definition `material` in the sets
/// `select_set` picks, whose names `select_property` picks, resolved
/// exactly (#218).
///
/// It answers what [`exact_material_property`] answers, for every selected
/// property at once, as [`exact_properties_where`] does for an object. The
/// result is in file order of the sets, each set's members in file or
/// schema order. An empty result is a proven absence of every selected
/// property.
///
/// # Errors
///
/// Any [`ExactPropertyError`], as for [`exact_material_property`], and
/// the ambiguity refusals of [`exact_properties_where`].
///
/// [`exact_properties_where`]: super::exact_properties_where
pub fn exact_material_properties_where<S, P>(
    model: &Model,
    material: EntityId,
    mut select_set: S,
    mut select_property: P,
) -> Result<Vec<ExactPropertyEntry>, ExactPropertyError>
where
    S: FnMut(&str) -> bool,
    P: FnMut(&str) -> bool,
{
    let release = validate_model(model)?;
    let sets = material_sets(model, release, material)?;
    let mut selector = Selector {
        set: &mut select_set,
        property: &mut select_property,
    };
    collect(
        model,
        release,
        &sets,
        ExactSource::Material(material),
        &mut selector,
    )
}

/// The property sets of material definition `material` whose names
/// `select_set` picks, empty ones included (#218).
///
/// The material counterpart of [`exact_property_sets_where`]: an empty
/// result proves the material has no selected set, and an empty set is
/// listed with `members == 0`.
///
/// # Errors
///
/// Any [`ExactPropertyError`], as for [`exact_material_property`].
///
/// [`exact_property_sets_where`]: super::exact_property_sets_where
pub fn exact_material_property_sets_where<S>(
    model: &Model,
    material: EntityId,
    mut select_set: S,
) -> Result<Vec<ExactPropertySetEntry>, ExactPropertyError>
where
    S: FnMut(&str) -> bool,
{
    let release = validate_model(model)?;
    let sets = material_sets(model, release, material)?;
    list_sets(
        model,
        release,
        &sets,
        ExactSource::Material(material),
        &mut select_set,
    )
}
