//! The public `IfcAlignment` hierarchy.
//!
//! IFC4X3 ADD2 relates an alignment to its parts through three
//! relationships, and this module is the one place that walks them:
//!
//! - `IfcRelNests` (ordered) from the alignment to its horizontal, vertical
//!   and cant layouts ("Alignment Layouts") and to its `IfcReferent`s
//!   ("Object Nesting": the first nested referent is the start station);
//! - `IfcRelAggregates` from a parent alignment to child alignments that
//!   reuse its horizontal layout ("Alignment Layout - Reusing Horizontal
//!   Layout");
//! - `IfcRelPositions` from the alignment, as an `IfcPositioningElement`,
//!   to the products it positions.
//!
//! The view only reports what the file states. Choosing one layout out of
//! several is a separate, explicit step ([`AlignmentHierarchy::sole_vertical`]
//! and siblings), because silently picking one would describe a road other
//! than the one the caller meant.

use std::collections::HashSet;

use ifc_model::{EntityId, Model};

use super::{AlignmentView, RelationSlots};
use crate::error::{AlignmentError, AlignmentResult};

/// What one `IfcAlignment` is made of, as stated by its relationships.
///
/// Every list keeps the order the file states: nesting order for layouts
/// and referents, relationship order for children and positioned products.
/// A list may be empty; how many layouts of a kind an alignment may have is
/// a question the `sole_*` methods answer with a typed refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AlignmentHierarchy {
    /// The `IfcAlignment` described.
    pub alignment: EntityId,
    /// The `IfcAlignment` that aggregates this one, when it is a child.
    /// `None` for an alignment aggregated directly under the project (or
    /// not aggregated at all).
    pub parent: Option<EntityId>,
    /// Nested `IfcAlignmentHorizontal` layouts.
    pub horizontal: Vec<EntityId>,
    /// Nested `IfcAlignmentVertical` layouts.
    pub vertical: Vec<EntityId>,
    /// Nested `IfcAlignmentCant` layouts.
    pub cant: Vec<EntityId>,
    /// Nested `IfcReferent`s, in nesting order.
    pub referents: Vec<EntityId>,
    /// Child `IfcAlignment`s aggregated under this one.
    pub children: Vec<EntityId>,
    /// Products this alignment positions through `IfcRelPositions`.
    pub positioned: Vec<EntityId>,
}

impl AlignmentHierarchy {
    /// The alignment's own horizontal layout: `None` when it nests none
    /// (a child reusing its parent's, see
    /// [`AlignmentView::governing_horizontal`]).
    ///
    /// # Errors
    ///
    /// [`AlignmentError::SemanticViolation`] when it nests several.
    pub fn sole_horizontal(&self) -> AlignmentResult<Option<EntityId>> {
        sole(
            self.alignment,
            &self.horizontal,
            "an alignment nesting several horizontal layouts is ambiguous",
        )
    }

    /// The alignment's vertical layout, `None` when it nests none.
    ///
    /// # Errors
    ///
    /// [`AlignmentError::SemanticViolation`] when it nests several.
    pub fn sole_vertical(&self) -> AlignmentResult<Option<EntityId>> {
        sole(
            self.alignment,
            &self.vertical,
            "an alignment nesting several vertical layouts is ambiguous",
        )
    }

    /// The alignment's cant layout, `None` when it nests none.
    ///
    /// # Errors
    ///
    /// [`AlignmentError::SemanticViolation`] when it nests several.
    pub fn sole_cant(&self) -> AlignmentResult<Option<EntityId>> {
        sole(
            self.alignment,
            &self.cant,
            "an alignment nesting several cant layouts is ambiguous",
        )
    }
}

fn sole(
    alignment: EntityId,
    layouts: &[EntityId],
    rule: &'static str,
) -> AlignmentResult<Option<EntityId>> {
    match layouts {
        [] => Ok(None),
        [only] => Ok(Some(*only)),
        _ => Err(AlignmentError::SemanticViolation {
            entity: Some(alignment),
            rule,
        }),
    }
}

/// How many aggregation levels [`AlignmentView::governing_horizontal`] climbs
/// before refusing. Real files nest one level; the bound only exists so a
/// malformed file cannot make the walk unbounded.
const MAX_PARENT_DEPTH: usize = 64;

impl<'m> AlignmentView<'m> {
    /// The model this view reads.
    #[must_use]
    pub fn model(&self) -> &'m Model {
        self.model
    }

    /// Every `IfcAlignment` in the model, in file order.
    #[must_use]
    pub fn alignments(&self) -> Vec<EntityId> {
        self.ids_of_ancestor("IfcAlignment")
    }

    /// The hierarchy of one `IfcAlignment`.
    ///
    /// # Errors
    ///
    /// Refuses an id that is missing or not an `IfcAlignment`
    /// ([`AlignmentError::WrongType`]); a malformed or dangling relationship;
    /// the same object nested twice; and an alignment aggregated by more
    /// than one relationship (`Decomposes` is `SET [0:1]`).
    pub fn hierarchy(&self, alignment: EntityId) -> AlignmentResult<AlignmentHierarchy> {
        self.require(alignment, "IfcAlignment")?;
        let nested = self.related_by("IfcRelNests", RelationSlots::DECOMPOSES, alignment)?;
        let mut seen = HashSet::with_capacity(nested.len());
        if let Some(twice) = nested.iter().find(|id| !seen.insert(**id)) {
            return Err(AlignmentError::SemanticViolation {
                entity: Some(*twice),
                rule: "IfcRelNests must not list the same object twice",
            });
        }
        let aggregated =
            self.related_by("IfcRelAggregates", RelationSlots::DECOMPOSES, alignment)?;
        let positioned = self.related_by("IfcRelPositions", RelationSlots::POSITIONS, alignment)?;
        Ok(AlignmentHierarchy {
            alignment,
            parent: self.parent_alignment(alignment)?,
            horizontal: self.filter_is_a(nested.clone(), "IfcAlignmentHorizontal"),
            vertical: self.filter_is_a(nested.clone(), "IfcAlignmentVertical"),
            cant: self.filter_is_a(nested.clone(), "IfcAlignmentCant"),
            referents: self.filter_is_a(nested, "IfcReferent"),
            children: self.filter_is_a(aggregated, "IfcAlignment"),
            positioned,
        })
    }

    /// The `IfcAlignment` aggregating `alignment`, if any.
    ///
    /// # Errors
    ///
    /// As [`Self::hierarchy`].
    pub fn parent_alignment(&self, alignment: EntityId) -> AlignmentResult<Option<EntityId>> {
        self.require(alignment, "IfcAlignment")?;
        let parents = self.relating_of("IfcRelAggregates", RelationSlots::DECOMPOSES, alignment)?;
        match parents.as_slice() {
            [] => Ok(None),
            [only] => Ok(self
                .model
                .get(*only)
                .filter(|entity| self.schema.is_a(&entity.type_name, "IfcAlignment"))
                .map(|_| *only)),
            _ => Err(AlignmentError::SemanticViolation {
                entity: Some(alignment),
                rule:
                    "an object is aggregated by at most one IfcRelAggregates (Decomposes SET [0:1])",
            }),
        }
    }

    /// The horizontal layout that governs `alignment`: its own, or, for a
    /// child alignment that nests none, the nearest ancestor's ("Alignment
    /// Layout - Reusing Horizontal Layout"). `None` when no alignment on the
    /// way up nests one.
    ///
    /// # Errors
    ///
    /// As [`Self::hierarchy`], plus several horizontal layouts on the
    /// governing alignment and an aggregation cycle.
    pub fn governing_horizontal(&self, alignment: EntityId) -> AlignmentResult<Option<EntityId>> {
        let mut current = alignment;
        let mut visited = HashSet::new();
        for _ in 0..MAX_PARENT_DEPTH {
            if !visited.insert(current) {
                return Err(AlignmentError::SemanticViolation {
                    entity: Some(current),
                    rule: "IfcAlignment aggregation must not form a cycle",
                });
            }
            let hierarchy = self.hierarchy(current)?;
            if let Some(horizontal) = hierarchy.sole_horizontal()? {
                return Ok(Some(horizontal));
            }
            match hierarchy.parent {
                Some(parent) => current = parent,
                None => return Ok(None),
            }
        }
        Err(AlignmentError::BudgetExceeded {
            max_depth: MAX_PARENT_DEPTH,
            max_nodes: MAX_PARENT_DEPTH,
        })
    }

    /// The parameter segments of one layout (`IfcAlignmentHorizontal`,
    /// `IfcAlignmentVertical` or `IfcAlignmentCant`), in nesting order.
    ///
    /// Each id is the `IfcAlignmentSegment.DesignParameters` target, ready
    /// for `read_horizontal_segment`, `read_vertical_segment` or
    /// `read_cant_segment`.
    ///
    /// # Errors
    ///
    /// Refuses an id that is not one of the three layouts, a segment listed
    /// twice, and a `DesignParameters` of the wrong family.
    pub fn layout_segments(&self, layout: EntityId) -> AlignmentResult<Vec<EntityId>> {
        let entity = self
            .model
            .get(layout)
            .ok_or(AlignmentError::MissingEntity { entity: layout })?;
        let family = [
            ("IfcAlignmentHorizontal", "IfcAlignmentHorizontalSegment"),
            ("IfcAlignmentVertical", "IfcAlignmentVerticalSegment"),
            ("IfcAlignmentCant", "IfcAlignmentCantSegment"),
        ]
        .into_iter()
        .find(|(kind, _)| self.schema.is_a(&entity.type_name, kind))
        .map(|(_, parameters)| parameters)
        .ok_or_else(|| AlignmentError::WrongType {
            entity: layout,
            expected: "IfcAlignmentHorizontal, IfcAlignmentVertical or IfcAlignmentCant",
            actual: entity.type_name.to_string(),
        })?;
        self.segment_chain(layout, family)
    }

    /// Refuse `id` unless it exists and is an `expected` (or subtype).
    pub(crate) fn require(&self, id: EntityId, expected: &'static str) -> AlignmentResult<()> {
        let entity = self
            .model
            .get(id)
            .ok_or(AlignmentError::MissingEntity { entity: id })?;
        if self.schema.is_a(&entity.type_name, expected) {
            Ok(())
        } else {
            Err(AlignmentError::WrongType {
                entity: id,
                expected,
                actual: entity.type_name.to_string(),
            })
        }
    }
}
