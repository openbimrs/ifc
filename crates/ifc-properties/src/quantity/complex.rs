//! `IfcPhysicalComplexQuantity` members: nested physical quantities.
//!
//! ```text
//! IfcPhysicalComplexQuantity  2 = HasQuantities (SET [1:?] OF IfcPhysicalQuantity)
//!                             3 = Discrimination
//! ```
//!
//! Members are read through [`Nesting`], which tracks the path from the
//! top-level quantity so a cycle of any length is cut and reported, not just
//! the direct self-member `NoSelfReference` forbids.

use ifc_model::{Entity, EntityId, Model};

use crate::nesting::Nesting;
use crate::quantity::set::{read_quantity, Quantity};

/// `IfcPhysicalComplexQuantity.HasQuantities`.
const HAS_QUANTITIES: usize = 2;

/// Resolve the `HasQuantities` of complex quantity `id`, in file order.
///
/// Members the traversal refuses (cycle, depth, budget, absent entity,
/// non-reference item, repeated member) or that cannot be represented are
/// left out and reported through `nesting`.
pub(super) fn complex_quantities(
    model: &Model,
    id: EntityId,
    entity: &Entity,
    nesting: &mut Nesting<'_>,
) -> Vec<Quantity> {
    if !nesting.enter(id) {
        return Vec::new();
    }
    let mut quantities = Vec::new();
    for member in nesting.members(id, "HasQuantities", entity.attributes.get(HAS_QUANTITIES)) {
        if !nesting.admit(model, id, member) {
            continue;
        }
        if let Some(quantity) = read_quantity(model, member, nesting) {
            quantities.push(quantity);
        }
    }
    nesting.leave();
    quantities
}
