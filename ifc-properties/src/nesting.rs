//! Bounded, cycle-aware traversal of nested complex properties and quantities.
//!
//! `IfcComplexProperty.HasProperties` and `IfcPhysicalComplexQuantity.
//! HasQuantities` may nest. The schema forbids only DIRECT self-reference
//! (`WR21` / `NoSelfReference`), so a malformed file can still close a
//! longer cycle, and a member may legally be shared by several complex
//! properties (`IfcProperty.PartOfComplex` is `SET [0:?]`), so the resolved
//! tree can be exponentially larger than the file. Every read therefore
//! carries the path from its root (for cycles), a depth bound and a member
//! budget, and reports each cut through [`PropertyAnomaly`] instead of
//! returning a silently shortened value.

use std::collections::BTreeSet;

use ifc_model::{EntityId, Model};

use crate::error::PropertyAnomaly;

/// Maximum complex nesting followed below one root read.
///
/// Deep enough for any real authoring tool. A complex property or quantity
/// that many levels down is reported as
/// [`PropertyAnomaly::ComplexTooDeep`] and its members are not read.
pub(crate) const MAX_COMPLEX_DEPTH: usize = 16;

/// Maximum nested member references followed during one read.
///
/// Bounds the work a legal but heavily shared nesting (or a malformed dense
/// cycle) can cause. Exceeding it is reported once as
/// [`PropertyAnomaly::ComplexBudgetExceeded`] once for every complex entity
/// whose remaining members are left unread.
pub(crate) const MAX_COMPLEX_MEMBERS: usize = 10_000;

/// Traversal state for one read of a property, set or quantity set.
pub(crate) struct Nesting<'a> {
    /// Complex entities from the root down to the one being read.
    path: Vec<EntityId>,
    /// Member references followed so far.
    members: usize,
    /// Complex entities whose members the exhausted budget cut, each
    /// reported once.
    truncated: BTreeSet<EntityId>,
    /// `(complex, member)` cycles already reported, so a doubled member
    /// reference does not report the same cycle twice.
    cycles: BTreeSet<(EntityId, EntityId)>,
    anomalies: &'a mut Vec<PropertyAnomaly>,
}

impl<'a> Nesting<'a> {
    pub(crate) fn new(anomalies: &'a mut Vec<PropertyAnomaly>) -> Self {
        Self {
            path: Vec::new(),
            members: 0,
            truncated: BTreeSet::new(),
            cycles: BTreeSet::new(),
            anomalies,
        }
    }

    /// Start reading the members of `complex`.
    ///
    /// Returns `false`, after reporting it, when `complex` is already
    /// [`MAX_COMPLEX_DEPTH`] levels deep; the caller must then leave its
    /// members unread and must not call [`Self::leave`].
    pub(crate) fn enter(&mut self, complex: EntityId) -> bool {
        if self.path.len() >= MAX_COMPLEX_DEPTH {
            self.anomalies.push(PropertyAnomaly::ComplexTooDeep {
                complex,
                limit: MAX_COMPLEX_DEPTH,
            });
            return false;
        }
        self.path.push(complex);
        true
    }

    /// Finish the members of the complex most recently entered.
    pub(crate) fn leave(&mut self) {
        self.path.pop();
    }

    /// Whether `member`, listed by `container`, may be read.
    ///
    /// `container` is the complex being read, or a property/quantity set
    /// when reading its top-level members. A member that re-enters the
    /// current path, names no entity, or arrives after the budget ran out is
    /// reported and must be skipped. Top-level set members never cost
    /// budget, so the budget cuts only nested members.
    pub(crate) fn admit(&mut self, model: &Model, container: EntityId, member: EntityId) -> bool {
        // Only nested members cost budget: a set's own member list is read
        // once, so its work is linear in the file and is never cut.
        if !self.path.is_empty() {
            self.members += 1;
            if self.members > MAX_COMPLEX_MEMBERS {
                if self.truncated.insert(container) {
                    self.anomalies.push(PropertyAnomaly::ComplexBudgetExceeded {
                        complex: container,
                        limit: MAX_COMPLEX_MEMBERS,
                    });
                }
                return false;
            }
        }
        if self.path.contains(&member) {
            if self.cycles.insert((container, member)) {
                self.anomalies.push(PropertyAnomaly::ComplexCycle {
                    complex: container,
                    member,
                });
            }
            return false;
        }
        if model.get(member).is_none() {
            self.anomalies
                .push(PropertyAnomaly::MissingMember { container, member });
            return false;
        }
        true
    }

    /// Report a malformed file fact found while reading.
    pub(crate) fn report(&mut self, anomaly: PropertyAnomaly) {
        self.anomalies.push(anomaly);
    }
}
