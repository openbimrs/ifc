//! The cut fill of a material (#597); see [`StyleIndex::material_fill`].

use std::cmp::Ordering;

use ifc_model::EntityId;

use crate::error::{StyleError, StyleResult};
use crate::fill_style::{resolve_fill, HatchLine, MaterialFill};
use crate::units::StyleUnits;

use super::StyleIndex;

impl StyleIndex<'_, '_> {
    /// The cut fill `material`'s `IfcMaterialDefinitionRepresentation` states,
    /// over every one of its `IfcStyledRepresentation`s (one per
    /// representation context), as hatch line families in metres and
    /// radians (see [`MaterialFill`]). `None` when the material has no
    /// representation or none states an `IfcFillAreaStyle`.
    ///
    /// The traversal is that of [`StyleIndex::material_colour`]. Fills in
    /// different contexts must draw the same: equal flag, background and
    /// hatch lines (as a set, compared exactly); the first one, in
    /// ascending order of definition, representation and item, is returned.
    ///
    /// # Errors
    ///
    /// [`StyleError::AmbiguousFill`] when one styled item, or the material
    /// across its styled items, states fills that differ;
    /// [`StyleError::UnsupportedFill`] for tiles, an externally defined
    /// hatch style, a predefined curve font other than 'continuous' and a
    /// 'by layer' colour or width; [`StyleError::UnitUndeclared`] when a
    /// measure needs a unit `units` does not state (a colour-only fill needs
    /// none); [`StyleError::InvalidValue`] and [`StyleError::OutOfRange`]
    /// for a fill that breaks a rule of the standard (`MaxOneColour`, a
    /// non-positive distance, a vector along the hatch lines, ...);
    /// [`StyleError::UnknownEntity`] for a dangling style;
    /// [`StyleError::ReferenceType`] for a representation that is no
    /// `IfcStyledRepresentation` or an item that is no `IfcStyledItem`; and
    /// the projection errors of a malformed style.
    pub fn material_fill(
        &self,
        material: EntityId,
        units: StyleUnits,
    ) -> StyleResult<Option<MaterialFill>> {
        let view = self.view();
        let mut found = Vec::new();
        for definition in self.definitions_of(material)? {
            let representations = view
                .record(definition, "IfcMaterialDefinitionRepresentation")?
                .required_refs("Representations", "IfcStyledRepresentation", 1, None)?;
            for representation in representations {
                let items = view
                    .record(representation, "IfcStyledRepresentation")?
                    .required_refs("Items", "IfcStyledItem", 1, None)?;
                for styled in items {
                    let styles = view.flatten_assignments(view.styled_item(styled)?.styles()?)?;
                    let mut fills = Vec::new();
                    for style in styles {
                        let entity = view
                            .model
                            .get(style)
                            .ok_or(StyleError::UnknownEntity { id: style })?;
                        if view.schema.is_a(&entity.type_name, "IfcFillAreaStyle") {
                            fills.push(resolve_fill(view, style, styled, units)?);
                        }
                    }
                    if let Some(fill) = agree_fill(styled, fills)? {
                        found.push(fill);
                    }
                }
            }
        }
        agree_fill(material, found)
    }
}

/// The first of `found` when every fill agrees, `None` when empty, else
/// ambiguous.
fn agree_fill(owner: EntityId, found: Vec<MaterialFill>) -> StyleResult<Option<MaterialFill>> {
    let Some(first) = found.first() else {
        return Ok(None);
    };
    if found.iter().all(|fill| same_fill(first, fill)) {
        return Ok(found.into_iter().next());
    }
    let mut styles: Vec<EntityId> = found.iter().map(|fill| fill.style).collect();
    styles.sort_unstable();
    styles.dedup();
    Err(StyleError::AmbiguousFill { owner, styles })
}

/// Whether two fills draw the same: flag, background and the hatch lines as
/// a multiset. Names and ids do not draw.
fn same_fill(a: &MaterialFill, b: &MaterialFill) -> bool {
    a.model_or_draughting == b.model_or_draughting
        && a.background == b.background
        && sorted(&a.lines) == sorted(&b.lines)
}

/// The lines in a total order, with `-0.0` read as `0.0` so that equal
/// values never sort apart.
fn sorted(lines: &[HatchLine]) -> Vec<HatchLine> {
    let mut lines: Vec<HatchLine> = lines.iter().map(normalised).collect();
    lines.sort_by(compare);
    lines
}

fn normalised(line: &HatchLine) -> HatchLine {
    let mut line = line.clone();
    line.angle_rad += 0.0;
    line.spacing += 0.0;
    line.shift += 0.0;
    line.origin = line.origin.map(|x| x + 0.0);
    line.pattern_start = line.pattern_start.map(|p| p.map(|x| x + 0.0));
    line.dashes = line
        .dashes
        .map(|pairs| pairs.into_iter().map(|p| p.map(|x| x + 0.0)).collect());
    line.colour = line.colour.map(|c| c.map(|x| x + 0.0));
    line.width = line.width.map(|x| x + 0.0);
    line
}

fn compare(a: &HatchLine, b: &HatchLine) -> Ordering {
    let dashes = |line: &HatchLine| -> Option<Vec<f64>> {
        line.dashes
            .as_ref()
            .map(|pairs| pairs.iter().flatten().copied().collect())
    };
    a.angle_rad
        .total_cmp(&b.angle_rad)
        .then(a.spacing.total_cmp(&b.spacing))
        .then(a.shift.total_cmp(&b.shift))
        .then_with(|| slices(&a.origin, &b.origin))
        .then_with(|| options(a.pattern_start, b.pattern_start, |x, y| slices(&x, &y)))
        .then_with(|| options(dashes(a), dashes(b), |x, y| slices(&x, &y)))
        .then_with(|| options(a.colour, b.colour, |x, y| slices(&x, &y)))
        .then_with(|| options(a.width, b.width, |x, y| x.total_cmp(&y)))
}

fn slices(a: &[f64], b: &[f64]) -> Ordering {
    a.iter()
        .zip(b)
        .map(|(x, y)| x.total_cmp(y))
        .find(|order| order.is_ne())
        .unwrap_or_else(|| a.len().cmp(&b.len()))
}

/// `None` before `Some`.
fn options<T>(a: Option<T>, b: Option<T>, compare: impl Fn(T, T) -> Ordering) -> Ordering {
    match (a, b) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a), Some(b)) => compare(a, b),
    }
}
