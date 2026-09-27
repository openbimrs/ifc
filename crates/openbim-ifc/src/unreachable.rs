//! Why an authored product will not appear in a viewer.
//!
//! # The failure this answers
//!
//! A file can be schema-valid, pass `IfcOpenShell.validate` with zero errors,
//! contain every entity the author intended, and still open blank. Validation
//! asks "is this legal IFC"; this module asks the different question "will a
//! viewer draw it". The three causes below were all hit exporting a single
//! plan, and none produced any diagnostic.
//!
//! # Why this lives in the facade
//!
//! Answering needs spatial containment *and* representation contexts, which
//! are sibling domain crates. [ADR 0003](https://openbimrs.github.io/ifc/adr/0003-domain-crates-as-borrowed-views)
//! forbids siblings depending on each other and puts cross-domain workflows in
//! an orchestration layer, so the lint belongs here rather than in
//! `ifc-author`, which cannot see either crate.
//!
//! # What it deliberately does not flag
//!
//! Being unreachable through the spatial tree is normal for several entity
//! kinds, and a lint that cries wolf gets switched off:
//!
//! - **Openings, voids and fills** reach the model through
//!   `IfcRelVoidsElement` / `IfcRelFillsElement`, never containment.
//! - **Assembly parts** reach it through `IfcRelAggregates` or `IfcRelNests`.
//! - **Spatial containers themselves** are aggregated, not contained.
//! - **Products with no representation** have nothing to draw; a grid axis or
//!   a space boundary marker is not a defect.
//!
//! Measured on `AC20-FZK-Haus.ifc`: 127 products, 20 of them outside the
//! containment tree, and **zero** findings — 17 openings, 3 representationless
//! virtual elements. A lint reporting 20 problems on a good reference file
//! would be worthless.

use std::collections::HashSet;

use ifc_geometry::{context_of, geometric_products, TargetView};
use ifc_model::{EntityId, Model, Value};
use ifc_spatial::{SpatialKind, SpatialTree};

/// Why one product will not be drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unreachable {
    /// No `IfcRelContainedInSpatialStructure` names it, and no aggregation,
    /// nesting or voiding relationship explains the absence.
    ///
    /// Viewers reach geometry by walking the spatial tree, so an product
    /// outside it is never visited however well-formed it is.
    NotContainedInSpatialStructure,

    /// Every representation sits in a context no model viewer renders.
    ///
    /// A body authored only into `PlanView` is the usual cause: a 3D viewer
    /// draws `ModelView` and skips it, so the product is invisible in the
    /// tool the author is most likely to check in.
    NoRepresentationInModelContext {
        /// The target views that were actually found, for the message.
        found: Vec<String>,
    },

    /// The product has a representation whose context could not be resolved.
    ///
    /// Distinct from the above: the geometry may be fine, but a dangling or
    /// absent context reference means no viewer can decide when to draw it.
    RepresentationWithoutContext,
}

impl Unreachable {
    /// A one-line explanation suitable for a CLI or a UI warning.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::NotContainedInSpatialStructure => {
                "not contained in the spatial structure: no viewer will traverse to it \
                 (add an IfcRelContainedInSpatialStructure to a storey)"
                    .to_string()
            }
            Self::NoRepresentationInModelContext { found } => format!(
                "geometry exists only in {} context(s): a model viewer renders Model \
                 and will skip it",
                if found.is_empty() {
                    "non-model".to_string()
                } else {
                    found.join(", ")
                }
            ),
            Self::RepresentationWithoutContext => {
                "representation has no resolvable context: a viewer cannot decide \
                 when to draw it"
                    .to_string()
            }
        }
    }
}

/// Products that no viewer will draw, with the reason, in stable id order.
///
/// Run this before export and the "valid but blank" class of bug disappears.
///
/// # Example
///
/// ```no_run
/// # use ifc::{Codec, StepCodec};
/// let model = StepCodec.read_bytes(&std::fs::read("plan.ifc").unwrap()).unwrap();
/// for (id, why) in ifc::unreachable_products(&model) {
///     eprintln!("#{id:?}: {}", why.message());
/// }
/// ```
///
/// # Cost
///
/// One spatial tree build plus one pass over the model to index the
/// relationships that excuse absence. Both are linear; the tree is built once
/// and shared, not rebuilt per product.
#[must_use]
pub fn unreachable_products(model: &Model) -> Vec<(EntityId, Unreachable)> {
    let tree = SpatialTree::build(model);
    let excused = ExcusedByRelationship::index(model);

    let mut findings = Vec::new();

    // Only products carrying a shape can be "invisible" -- something with no
    // representation was never going to be drawn and is not a defect.
    for id in geometric_products(model) {
        let Some(entity) = model.get(id) else {
            continue;
        };

        // Spatial containers hang off IfcRelAggregates, not containment.
        if SpatialKind::classify(&entity.type_name).is_container() {
            continue;
        }

        if tree.container_of(id).is_none() && !excused.contains(id) {
            findings.push((id, Unreachable::NotContainedInSpatialStructure));
            // Containment is the dominant defect; reporting a context problem
            // on top of it would bury the thing to fix first.
            continue;
        }

        if let Some(finding) = context_finding(model, id) {
            findings.push((id, finding));
        }
    }

    findings
}

/// Whether the product's geometry is authored somewhere a model viewer looks.
fn context_finding(model: &Model, product: EntityId) -> Option<Unreachable> {
    let representations = representations_of(model, product);
    if representations.is_empty() {
        return None;
    }

    let mut found = Vec::new();
    let mut any_context = false;

    for representation in representations {
        let Some(context) = context_of(model, representation) else {
            continue;
        };
        any_context = true;
        match context.target_view() {
            // A model viewer draws Model. NotDefined is the pre-IFC4 default
            // and is drawn by every viewer, so it counts as reachable.
            Some(TargetView::ModelView | TargetView::NotDefined) | None => return None,
            Some(other) => {
                let label = format!("{other:?}");
                if !found.contains(&label) {
                    found.push(label);
                }
            }
        }
    }

    if !any_context {
        return Some(Unreachable::RepresentationWithoutContext);
    }
    Some(Unreachable::NoRepresentationInModelContext { found })
}

/// The `IfcShapeRepresentation` ids hanging off a product's shape.
fn representations_of(model: &Model, product: EntityId) -> Vec<EntityId> {
    /// `IfcProduct.Representation`.
    const REPRESENTATION: usize = 6;
    /// `IfcProductRepresentation.Representations`.
    const REPRESENTATIONS: usize = 2;

    let Some(entity) = model.get(product) else {
        return Vec::new();
    };
    let Some(Value::Ref(shape)) = entity.attributes.get(REPRESENTATION) else {
        return Vec::new();
    };
    let Some(shape_entity) = model.get(*shape) else {
        return Vec::new();
    };
    match shape_entity.attributes.get(REPRESENTATIONS) {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::Ref(r) => Some(*r),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Products whose absence from the containment tree is legitimate.
///
/// Built in one pass: the alternative is re-scanning the model per product,
/// which turns a linear check quadratic on files with thousands of openings.
struct ExcusedByRelationship {
    ids: HashSet<EntityId>,
}

impl ExcusedByRelationship {
    /// Relationships that attach a product to the model without containment.
    /// Slot 5 is `RelatedObjects`/`RelatedOpeningElement`/`RelatedElements`
    /// on each of these, per the fixed-slot convention of ADR 0008.
    const RELATED_SLOT: usize = 5;
    const KINDS: [&'static str; 5] = [
        "IFCRELAGGREGATES",
        "IFCRELNESTS",
        "IFCRELVOIDSELEMENT",
        "IFCRELFILLSELEMENT",
        "IFCRELPROJECTSELEMENT",
    ];

    fn index(model: &Model) -> Self {
        let mut ids = HashSet::new();
        for (_, entity) in model.iter() {
            let upper = entity.type_name.to_ascii_uppercase();
            if !Self::KINDS.contains(&upper.as_str()) {
                continue;
            }
            match entity.attributes.get(Self::RELATED_SLOT) {
                Some(Value::Ref(r)) => {
                    ids.insert(*r);
                }
                Some(Value::List(items)) => {
                    ids.extend(items.iter().filter_map(|v| match v {
                        Value::Ref(r) => Some(*r),
                        _ => None,
                    }));
                }
                _ => {}
            }
        }
        Self { ids }
    }

    fn contains(&self, id: EntityId) -> bool {
        self.ids.contains(&id)
    }
}

#[cfg(test)]
#[path = "unreachable/tests.rs"]
mod tests;
