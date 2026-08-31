//! Surface presentation styles.

use ifc_model::EntityId;
use ifc_schema::Schema;

use crate::error::{StyleError, StyleResult};
use crate::view::Record;

mod lighting;
mod refraction;
mod rendering;
mod shading;

pub use lighting::SurfaceStyleLighting;
pub use refraction::SurfaceStyleRefraction;
pub use rendering::SurfaceStyleRendering;
pub use shading::SurfaceStyleShading;

pub(crate) const SURFACE_STYLE_ELEMENT_MEMBERS: &[&str] = &[
    "IfcSurfaceStyleShading",
    "IfcSurfaceStyleLighting",
    "IfcSurfaceStyleRefraction",
    "IfcSurfaceStyleWithTextures",
    "IfcExternallyDefinedSurfaceStyle",
];

pub(crate) fn duplicate_surface_element_category<'a>(
    schema: &Schema,
    actual_types: impl IntoIterator<Item = &'a str>,
) -> Option<&'static str> {
    let mut seen = [false; SURFACE_STYLE_ELEMENT_MEMBERS.len()];
    for actual in actual_types {
        let category = SURFACE_STYLE_ELEMENT_MEMBERS
            .iter()
            .position(|member| schema.is_a(actual, member))?;
        if seen[category] {
            return Some(SURFACE_STYLE_ELEMENT_MEMBERS[category]);
        }
        seen[category] = true;
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceSide {
    Positive,
    Negative,
    Both,
}

impl SurfaceSide {
    pub(crate) fn as_ifc(self) -> &'static str {
        match self {
            Self::Positive => "POSITIVE",
            Self::Negative => "NEGATIVE",
            Self::Both => "BOTH",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SurfaceStyle<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> SurfaceStyle<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn name(&self) -> StyleResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    pub fn side(&self) -> StyleResult<SurfaceSide> {
        match self.record.required_enum("Side")? {
            value if value.eq_ignore_ascii_case("POSITIVE") => Ok(SurfaceSide::Positive),
            value if value.eq_ignore_ascii_case("NEGATIVE") => Ok(SurfaceSide::Negative),
            value if value.eq_ignore_ascii_case("BOTH") => Ok(SurfaceSide::Both),
            value => Err(StyleError::InvalidValue {
                entity: "IFCSURFACESTYLE".to_owned(),
                id: self.record.id,
                attribute: "Side",
                value: value.to_owned(),
            }),
        }
    }

    pub fn elements(&self) -> StyleResult<Vec<EntityId>> {
        let elements = self.record.required_refs_select(
            "Styles",
            "IfcSurfaceStyleElementSelect",
            SURFACE_STYLE_ELEMENT_MEMBERS,
            1,
            Some(5),
        )?;
        let actual_types = elements.iter().filter_map(|id| {
            self.record
                .model
                .get(*id)
                .map(|entity| entity.type_name.as_ref())
        });
        if let Some(category) = duplicate_surface_element_category(self.record.schema, actual_types)
        {
            return Err(StyleError::InvalidValue {
                entity: "IFCSURFACESTYLE".to_owned(),
                id: self.record.id,
                attribute: "Styles",
                value: format!("duplicate {category} category"),
            });
        }
        Ok(elements)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SurfaceStyleWithTextures<'m, 's> {
    record: Record<'m, 's>,
}

impl<'m, 's> SurfaceStyleWithTextures<'m, 's> {
    pub(crate) fn from_record(record: Record<'m, 's>) -> Self {
        Self { record }
    }

    pub fn textures(&self) -> StyleResult<Vec<EntityId>> {
        self.record
            .required_refs("Textures", "IfcSurfaceTexture", 1, None)
    }
}
