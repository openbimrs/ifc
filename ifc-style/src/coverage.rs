//! Auditable IFC4 ADD2 presentation-appearance declaration census.
//!
//! `StrictView` means this crate exposes a dedicated typed projection. Schema
//! declarations and structural-only entities remain available through lower layers,
//! but are deliberately not advertised as typed style contracts.

/// The syntactic kind of an IFC4 ADD2 presentation-appearance schema
/// declaration: an entity, a defined/select type, or a global rule function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceKind {
    /// An `ENTITY` declaration.
    Entity,
    /// A `TYPE` declaration (defined type or select).
    Type,
    /// A global `FUNCTION` rule (e.g. `IfcCorrectFillAreaStyle`).
    Function,
}

/// How thoroughly this crate exposes a given IFC4 ADD2 presentation-appearance
/// schema declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceSupport {
    /// Exposed as a dedicated typed `*View`/projection struct in this crate.
    StrictView,
    /// Read as a raw schema-level value (e.g. an enum token) without a
    /// dedicated view type.
    SchemaValue,
    /// A schema-defined global rule; not something this crate evaluates.
    SchemaRule,
    /// Present in the schema but consumed only structurally (e.g. as an
    /// intermediate select member), with no standalone typed contract.
    StructuralOnly,
}

/// One census entry: an IFC4 ADD2 presentation-appearance schema declaration
/// paired with how this crate supports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppearanceDeclaration {
    /// The schema declaration's name, e.g. `"IfcSurfaceStyle"`.
    pub name: &'static str,
    /// Whether the declaration is an entity, type, or function.
    pub kind: AppearanceKind,
    /// How thoroughly this crate supports the declaration.
    pub support: AppearanceSupport,
}

/// The full, test-verified census of IFC4 ADD2 presentation-appearance
/// schema declarations and how this crate supports each one.
pub const APPEARANCE_DECLARATIONS: &[AppearanceDeclaration] = &[
    AppearanceDeclaration {
        name: "IfcBlobTexture",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcColour",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcColourOrFactor",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcColourRgb",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcColourRgbList",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcColourSpecification",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcCorrectFillAreaStyle",
        kind: AppearanceKind::Function,
        support: AppearanceSupport::SchemaRule,
    },
    AppearanceDeclaration {
        name: "IfcCurveFontOrScaledCurveFontSelect",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcCurveStyle",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcCurveStyleFont",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcCurveStyleFontAndScaling",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcCurveStyleFontPattern",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcCurveStyleFontSelect",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcDraughtingPreDefinedColour",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcDraughtingPreDefinedCurveFont",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcExternallyDefinedHatchStyle",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcExternallyDefinedSurfaceStyle",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcExternallyDefinedTextFont",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcFillAreaStyle",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcFillAreaStyleHatching",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcFillAreaStyleTiles",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcFillStyleSelect",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcFontStyle",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcFontVariant",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcFontWeight",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcHatchLineDistanceSelect",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcImageTexture",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcIndexedColourMap",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcIndexedTextureMap",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcIndexedTriangleTextureMap",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcNullStyle",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcPixelTexture",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcPreDefinedColour",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcPreDefinedCurveFont",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcPreDefinedItem",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcPreDefinedTextFont",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcPresentableText",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcPresentationStyle",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcPresentationStyleAssignment",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcPresentationStyleSelect",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcReflectanceMethodEnum",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcSizeSelect",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcSpecularExponent",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcSpecularHighlightSelect",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcSpecularRoughness",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcStyleAssignmentSelect",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcStyledItem",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcSurfaceSide",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcSurfaceStyle",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcSurfaceStyleElementSelect",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcSurfaceStyleLighting",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcSurfaceStyleRefraction",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcSurfaceStyleRendering",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcSurfaceStyleShading",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcSurfaceStyleWithTextures",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcSurfaceTexture",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcTextAlignment",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcTextDecoration",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcTextFontName",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcTextFontSelect",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcTextStyle",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcTextStyleFontModel",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcTextStyleForDefinedFont",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcTextStyleTextModel",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcTextTransformation",
        kind: AppearanceKind::Type,
        support: AppearanceSupport::SchemaValue,
    },
    AppearanceDeclaration {
        name: "IfcTextureCoordinate",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcTextureCoordinateGenerator",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcTextureMap",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StructuralOnly,
    },
    AppearanceDeclaration {
        name: "IfcTextureVertex",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
    AppearanceDeclaration {
        name: "IfcTextureVertexList",
        kind: AppearanceKind::Entity,
        support: AppearanceSupport::StrictView,
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn census_has_all_seventy_unique_ifc4_appearance_declarations() {
        assert_eq!(APPEARANCE_DECLARATIONS.len(), 70);
        let unique = APPEARANCE_DECLARATIONS
            .iter()
            .map(|declaration| declaration.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), APPEARANCE_DECLARATIONS.len());
        let schema = ifc_schema::ifc4();
        for declaration in APPEARANCE_DECLARATIONS {
            match declaration.kind {
                AppearanceKind::Entity => assert!(schema.entity(declaration.name).is_some()),
                AppearanceKind::Type => assert!(schema.type_def(declaration.name).is_some()),
                AppearanceKind::Function => {}
            }
        }
    }
}
