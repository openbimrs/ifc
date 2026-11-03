//! The cut fill of a material (#597), per context and per view (#624); see
//! [`StyleIndex::material_fill`].

use std::cmp::Ordering;

use ifc_model::EntityId;

use crate::error::{StyleError, StyleResult};
use crate::fill_style::{resolve_fill, ContextFill, ContextFillResult, HatchLine, MaterialFill};
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
    /// ascending order of representation and item, is returned.
    /// A material that states different fills per context (cut and
    /// projection) is ambiguous here: see [`StyleIndex::material_fills`] and
    /// [`StyleIndex::material_fill_for_view`].
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
        let mut found = Vec::new();
        for representation in self.representations_of(material)? {
            found.extend(self.representation_fills(representation, units)?);
        }
        agree_fill(material, found)
    }

    /// The fills `material` states per representation context, one
    /// [`ContextFill`] for each context that carries a fill area style, in
    /// ascending order of the lowest `IfcStyledRepresentation` in it.
    ///
    /// This is how one material gives several fills, e.g. a cut fill in a
    /// `SECTION_VIEW` sub-context and a projection fill in an
    /// `ELEVATION_VIEW` one. [`StyleIndex::material_fill`] over the same
    /// material refuses that as [`StyleError::AmbiguousFill`]: use this
    /// method for per-view fills, or [`StyleIndex::material_fill_for_view`]
    /// for one view.
    ///
    /// One refused context fails the whole call;
    /// [`StyleIndex::material_fills_by_context`] keeps refusals apart.
    ///
    /// Fills agree as in `material_fill`, but only within a context: the
    /// styled items of one representation must agree, then the
    /// representations of one context. Of agreeing representations the
    /// lowest is reported.
    ///
    /// # Errors
    ///
    /// [`StyleError::AmbiguousFill`] with the representation as owner when
    /// its styled items differ, and with the context as owner when two
    /// representations in it differ; every other error of
    /// [`StyleIndex::material_fill`]; and the projection errors of a missing
    /// or malformed representation context.
    pub fn material_fills(
        &self,
        material: EntityId,
        units: StyleUnits,
    ) -> StyleResult<Vec<ContextFill>> {
        // (context, its representations' fills), in order of first sight.
        let mut contexts: Vec<(EntityId, Vec<(EntityId, MaterialFill)>)> = Vec::new();
        for representation in self.representations_of(material)? {
            let Some(fill) = agree_fill(
                representation,
                self.representation_fills(representation, units)?,
            )?
            else {
                continue;
            };
            let context = self.context_of(representation)?;
            match contexts.iter_mut().find(|(id, _)| *id == context) {
                Some((_, found)) => found.push((representation, fill)),
                None => contexts.push((context, vec![(representation, fill)])),
            }
        }
        let mut out = Vec::new();
        for (context, found) in contexts {
            let representation = found[0].0;
            let fills = found.into_iter().map(|(_, fill)| fill).collect();
            let Some(fill) = agree_fill(context, fills)? else {
                continue;
            };
            let described = self.describe_context(context)?;
            out.push(ContextFill {
                representation,
                context,
                identifier: described.identifier,
                context_type: described.context_type,
                target_view: described.target_view,
                user_defined_target_view: described.user_defined_target_view,
                parent_context: described.parent_context,
                fill,
            });
        }
        Ok(out)
    }

    /// What `material` states per representation context, like
    /// [`StyleIndex::material_fills`], but a refusal that belongs to one
    /// context stays in that context's entry: one [`ContextFillResult`] for
    /// each context that carries a fill area style or was refused, in order
    /// of the lowest `IfcStyledRepresentation` in it that states a fill or
    /// was refused.
    ///
    /// Within a context the first refusal, in ascending order of
    /// representation, is the entry's; a later representation cannot
    /// replace it. Fills agree as in `material_fills`.
    ///
    /// When `material_fills` returns `Ok`, this returns the same contexts
    /// in the same order, each `Ok` with equal fields. When it returns an
    /// error, this returns an error, or at least one refused entry.
    ///
    /// # Errors
    ///
    /// Only refusals no context owns fail the call: the definitions and
    /// their `Representations` (a member that is no `IfcStyledRepresentation`
    /// included), the `ContextOfItems` of a representation that states a fill
    /// or was refused, and the record of a context that has an entry
    /// (`ContextIdentifier`, `ContextType`, a missing or invalid `TargetView`,
    /// `UserDefinedTargetView`, `ParentContext`). Everything else lands in
    /// [`ContextFillResult::fill`]: the errors of a representation's items
    /// and styles, [`StyleError::AmbiguousFill`] owned by a styled item, a
    /// representation, or a context whose representations differ.
    pub fn material_fills_by_context(
        &self,
        material: EntityId,
        units: StyleUnits,
    ) -> StyleResult<Vec<ContextFillResult>> {
        struct Pending {
            context: EntityId,
            representation: EntityId,
            state: StyleResult<Vec<MaterialFill>>,
        }
        let mut contexts: Vec<Pending> = Vec::new();
        for representation in self.representations_of(material)? {
            let resolved = self
                .representation_fills(representation, units)
                .and_then(|fills| agree_fill(representation, fills));
            if matches!(resolved, Ok(None)) {
                continue;
            }
            let context = self.context_of(representation)?;
            let index = match contexts.iter().position(|p| p.context == context) {
                Some(index) => index,
                None => {
                    contexts.push(Pending {
                        context,
                        representation,
                        state: Ok(Vec::new()),
                    });
                    contexts.len() - 1
                }
            };
            let pending = &mut contexts[index];
            match resolved {
                Ok(Some(fill)) => {
                    if let Ok(fills) = &mut pending.state {
                        fills.push(fill);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    if pending.state.is_ok() {
                        pending.state = Err(error);
                    }
                }
            }
        }
        let mut out = Vec::new();
        for Pending {
            context,
            representation,
            state,
        } in contexts
        {
            let Some(fill) = state
                .and_then(|fills| agree_fill(context, fills))
                .transpose()
            else {
                continue;
            };
            let described = self.describe_context(context)?;
            out.push(ContextFillResult {
                representation,
                context,
                identifier: described.identifier,
                context_type: described.context_type,
                target_view: described.target_view,
                user_defined_target_view: described.user_defined_target_view,
                parent_context: described.parent_context,
                fill,
            });
        }
        Ok(out)
    }

    /// The fill `material` states in the representation contexts whose
    /// `TargetView` is `target_view` (`"SECTION_VIEW"`, without dots): the
    /// fill they agree on, `None` when no such context carries one.
    ///
    /// A plain `IfcGeometricRepresentationContext` matches no view, not even
    /// `MODEL_VIEW`; `USERDEFINED` matches every user-defined sub-context
    /// without comparing `UserDefinedTargetView`. Fills agree as in
    /// [`StyleIndex::material_fill`], so contexts that differ give
    /// [`StyleError::AmbiguousFill`] owned by the material.
    ///
    /// # Errors
    ///
    /// [`StyleError::UnknownTargetView`] (before anything is read) when
    /// `target_view` is no `IfcGeometricProjectionEnum` literal: the match is
    /// exact and case-sensitive. The refusal of the first matching context
    /// that was refused, and the whole-call errors of
    /// [`StyleIndex::material_fills_by_context`]. A refusal in a context of
    /// another view does not matter here.
    pub fn material_fill_for_view(
        &self,
        material: EntityId,
        target_view: &str,
        units: StyleUnits,
    ) -> StyleResult<Option<MaterialFill>> {
        if !PROJECTIONS.contains(&target_view) {
            return Err(StyleError::UnknownTargetView {
                value: target_view.to_owned(),
            });
        }
        let mut fills = Vec::new();
        for entry in self.material_fills_by_context(material, units)? {
            if entry.target_view.as_deref() == Some(target_view) {
                fills.push(entry.fill?);
            }
        }
        agree_fill(material, fills)
    }

    /// The fill a bare `IfcFillAreaStyle` states, as [`StyleIndex::material_fill`]
    /// resolves it, for a style nothing may reference (a pattern library).
    /// [`MaterialFill::styled_item`] is `None`.
    ///
    /// # Errors
    ///
    /// [`StyleError::UnknownEntity`] for a missing entity;
    /// [`StyleError::WrongEntityType`] when `style` is no `IfcFillAreaStyle`;
    /// and the errors of [`StyleIndex::material_fill`] for the style itself.
    pub fn fill_area_style(&self, style: EntityId, units: StyleUnits) -> StyleResult<MaterialFill> {
        let view = self.view();
        resolve_fill(view, style, None, units)
    }

    /// The `IfcStyledRepresentation`s of `material`, ascending.
    fn representations_of(&self, material: EntityId) -> StyleResult<Vec<EntityId>> {
        let view = self.view();
        let mut found = Vec::new();
        for definition in self.definitions_of(material)? {
            found.extend(
                view.record(definition, "IfcMaterialDefinitionRepresentation")?
                    .required_refs("Representations", "IfcStyledRepresentation", 1, None)?,
            );
        }
        found.sort_unstable();
        found.dedup();
        Ok(found)
    }

    /// The `ContextOfItems` of `representation`.
    fn context_of(&self, representation: EntityId) -> StyleResult<EntityId> {
        self.view()
            .record(representation, "IfcStyledRepresentation")?
            .required_ref("ContextOfItems", "IfcRepresentationContext")
    }

    /// The identifying attributes of `context`, and for a sub-context its
    /// projection and parent.
    fn describe_context(&self, context: EntityId) -> StyleResult<Described> {
        let view = self.view();
        let record = view.record(context, "IfcRepresentationContext")?;
        let sub = view
            .schema
            .is_a(record.type_name(), "IfcGeometricRepresentationSubContext");
        let (target_view, user_defined_target_view, parent_context) = if sub {
            let target = record.required_enum("TargetView")?.trim_matches('.');
            if !PROJECTIONS.contains(&target) {
                return Err(StyleError::InvalidValue {
                    entity: record.type_name().to_owned(),
                    id: context,
                    attribute: "TargetView",
                    value: target.to_owned(),
                });
            }
            (
                Some(target.to_owned()),
                record
                    .optional_text("UserDefinedTargetView")?
                    .map(str::to_owned),
                Some(record.required_ref("ParentContext", "IfcGeometricRepresentationContext")?),
            )
        } else {
            (None, None, None)
        };
        Ok(Described {
            identifier: record
                .optional_text("ContextIdentifier")?
                .map(str::to_owned),
            context_type: record.optional_text("ContextType")?.map(str::to_owned),
            target_view,
            user_defined_target_view,
            parent_context,
        })
    }

    /// The fill of each styled item of `representation` that states one,
    /// in ascending item order.
    fn representation_fills(
        &self,
        representation: EntityId,
        units: StyleUnits,
    ) -> StyleResult<Vec<MaterialFill>> {
        let view = self.view();
        let mut items = view
            .record(representation, "IfcStyledRepresentation")?
            .required_refs("Items", "IfcStyledItem", 1, None)?;
        items.sort_unstable();
        items.dedup();
        let mut found = Vec::new();
        for styled in items {
            let styles = view.flatten_assignments(view.styled_item(styled)?.styles()?)?;
            let mut fills = Vec::new();
            for style in styles {
                let entity = view
                    .model
                    .get(style)
                    .ok_or(StyleError::UnknownEntity { id: style })?;
                if view.schema.is_a(&entity.type_name, "IfcFillAreaStyle") {
                    fills.push(resolve_fill(view, style, Some(styled), units)?);
                }
            }
            if let Some(fill) = agree_fill(styled, fills)? {
                found.push(fill);
            }
        }
        Ok(found)
    }
}

/// `IfcGeometricProjectionEnum`, the same in every release.
const PROJECTIONS: [&str; 9] = [
    "GRAPH_VIEW",
    "SKETCH_VIEW",
    "MODEL_VIEW",
    "PLAN_VIEW",
    "REFLECTED_PLAN_VIEW",
    "SECTION_VIEW",
    "ELEVATION_VIEW",
    "USERDEFINED",
    "NOTDEFINED",
];

/// The attributes of a representation context a [`ContextFill`] reports.
struct Described {
    identifier: Option<String>,
    context_type: Option<String>,
    target_view: Option<String>,
    user_defined_target_view: Option<String>,
    parent_context: Option<EntityId>,
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
