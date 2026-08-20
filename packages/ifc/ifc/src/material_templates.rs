//! Application-layer join between authored materials and official PSD templates.

use ifc_material::{Material, MaterialProperties, MaterialResult};
use ifc_template_catalog::catalog::Catalog;
use ifc_template_catalog::definition::SetTemplate;
use ifc_template_catalog::query::{ApplicabilityTarget, ExactEntityHierarchy};

fn query<'c>(catalog: &'c Catalog, qualifier: Option<&str>) -> Vec<&'c SetTemplate> {
    catalog
        .applicable_to(
            &ApplicabilityTarget::new("IfcMaterial", qualifier),
            &ExactEntityHierarchy,
        )
        .into_iter()
        .filter(|template| template.is_property_set())
        .collect()
}

/// Return templates whose published selector applies to `IfcMaterial` without
/// interpreting the authored `IfcMaterial.Category` as a schema predefined type.
///
/// Category-qualified selectors such as `IfcMaterial/Steel` are intentionally
/// excluded. Use [`applicable_to_category`] to opt into that PSD publication
/// convention.
pub fn applicable_to<'c>(
    material: Material<'_>,
    catalog: &'c Catalog,
) -> MaterialResult<Vec<&'c SetTemplate>> {
    material.name()?;
    Ok(query(catalog, None))
}

/// Apply the PSD publication convention that matches `IfcMaterial.Category`
/// against the qualifier in selectors such as `IfcMaterial/Steel`.
///
/// This is an explicit application policy. IFC4 does not declare a
/// `PredefinedType` attribute on `IfcMaterial`, so callers must not treat a
/// category match as schema-normative applicability.
pub fn applicable_to_category<'c>(
    material: Material<'_>,
    catalog: &'c Catalog,
) -> MaterialResult<Vec<&'c SetTemplate>> {
    material.name()?;
    let category = material.category()?;
    Ok(query(catalog, category))
}

/// Resolve an authored `IfcMaterialProperties.Name` to its catalog template.
pub fn template_for<'c>(
    properties: MaterialProperties<'_>,
    catalog: &'c Catalog,
) -> MaterialResult<Option<&'c SetTemplate>> {
    Ok(properties.name()?.and_then(|name| {
        catalog.get(name).filter(|template| {
            template.is_property_set()
                && template
                    .applicability
                    .iter()
                    .any(|selector| selector.entity.eq_ignore_ascii_case("IfcMaterial"))
        })
    }))
}
