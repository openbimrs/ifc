//! The resolved cut fill of a material, as hatch line families.

use ifc_model::EntityId;

/// A material's cut fill: one `IfcFillAreaStyle` resolved into hatch lines
/// in metres and radians (`StyleIndex::material_fill`).
///
/// When `model_or_draughting` is `Some(false)` the lengths are draughting
/// (paper-space) measures, per `IfcFillAreaStyle.ModelorDraughting`; they
/// are still scaled by the project length unit.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MaterialFill {
    /// The `IfcFillAreaStyle`.
    pub style: EntityId,
    /// The item-less `IfcStyledItem` it was read from; `None` for a bare
    /// style resolved by `StyleIndex::fill_area_style`.
    pub styled_item: Option<EntityId>,
    /// The style's `Name`, when authored.
    pub name: Option<String>,
    /// `ModelOrDraughting`; `None` when unauthored or in IFC2X3.
    pub model_or_draughting: Option<bool>,
    /// The background colour; alone it is a solid fill (`lines` empty).
    pub background: Option<[f64; 3]>,
    /// One family per hatching, in `FillStyles` order as stored.
    pub lines: Vec<HatchLine>,
}

/// One `IfcFillAreaStyleHatching` as a family of parallel lines.
///
/// The lengths are model measures, or draughting (paper-space) measures when
/// the owning [`MaterialFill::model_or_draughting`] is `Some(false)`; either
/// way they are scaled by the project length unit.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct HatchLine {
    /// Angle from the hatching x axis, counter-clockwise, in radians.
    pub angle_rad: f64,
    /// Perpendicular distance between neighbouring lines, in metres; positive.
    pub spacing: f64,
    /// Offset of each next line along the line direction, in metres; 0 for a length distance.
    pub shift: f64,
    /// `PointOfReferenceHatchLine` in metres; `[0, 0]` when absent.
    pub origin: [f64; 2],
    /// `PatternStart` in metres, when authored.
    pub pattern_start: Option<[f64; 2]>,
    /// `[visible, invisible]` lengths in metres; `Some(vec![])` is the
    /// `'continuous'` font and `None` states no font.
    pub dashes: Option<Vec<[f64; 2]>>,
    /// The line colour, when stated.
    pub colour: Option<[f64; 3]>,
    /// The line width in metres, when stated.
    pub width: Option<f64>,
}

/// A fill a material states in one representation context
/// (`StyleIndex::material_fills`).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ContextFill {
    /// The `IfcStyledRepresentation` the fill was read from (the lowest one
    /// of the context when several agree).
    pub representation: EntityId,
    /// Its `ContextOfItems`.
    pub context: EntityId,
    /// The context's `ContextIdentifier`, when authored.
    pub identifier: Option<String>,
    /// The context's `ContextType`, when authored.
    pub context_type: Option<String>,
    /// `TargetView` of an `IfcGeometricRepresentationSubContext` without the
    /// dots (`"SECTION_VIEW"`); `None` for any other context.
    pub target_view: Option<String>,
    /// `UserDefinedTargetView` of a sub-context, when authored.
    pub user_defined_target_view: Option<String>,
    /// `ParentContext` of a sub-context; `None` for any other context.
    pub parent_context: Option<EntityId>,
    /// The fill.
    pub fill: MaterialFill,
}
