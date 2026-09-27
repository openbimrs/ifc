//! `IfcComplexProperty` members: nested properties under a usage name.
//!
//! ```text
//! IfcComplexProperty  2 = UsageName  3 = HasProperties (SET [1:?] OF IfcProperty)
//! ```
//!
//! Members are read through [`Nesting`], which tracks the path from the root
//! property so a cycle of any length is cut and reported, not just the
//! direct self-member `WR21` forbids.

use ifc_model::{EntityId, Model, Value};

use crate::nesting::Nesting;
use crate::pset::scalar::{read_property, Property};

/// Resolve the `HasProperties` of complex property `id`, in file order.
///
/// Members the traversal refuses (cycle, depth, budget, absent entity,
/// non-reference item, repeated member) are left out and reported through
/// `nesting`.
pub(super) fn complex_members(
    model: &Model,
    id: EntityId,
    has_properties: Option<&Value>,
    nesting: &mut Nesting<'_>,
) -> Vec<Property> {
    if !nesting.enter(id) {
        return Vec::new();
    }
    let mut properties = Vec::new();
    for member in nesting.members(id, "HasProperties", has_properties) {
        if !nesting.admit(model, id, member) {
            continue;
        }
        if let Some(property) = read_property(model, member, nesting) {
            properties.push(property);
        }
    }
    nesting.leave();
    properties
}
