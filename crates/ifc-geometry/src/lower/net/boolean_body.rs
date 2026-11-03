//! Does a product's Body hold a boolean or a clip (#655)?

use ifc_model::{EntityId, Model};

use crate::input::body::BodyKind;
use crate::input::representation::{
    select_product_representation, Representation, RepresentationPurpose,
};

/// Whether an item of `product`'s selected Body is an `IfcBooleanResult`,
/// an `IfcBooleanClippingResult` or an `IfcCsgSolid`. Any read failure
/// answers `false`.
pub(super) fn body_holds_boolean(model: &Model, product: EntityId) -> bool {
    let Ok(Some(representation)) =
        select_product_representation(model, product, RepresentationPurpose::Body)
    else {
        return false;
    };
    let Some(entity) = model.get(representation) else {
        return false;
    };
    let Ok(items) = Representation::new(representation, entity).items() else {
        return false;
    };
    items.into_iter().any(|item| {
        model
            .get(item)
            .is_some_and(|entity| BodyKind::classify(&entity.type_name) == Some(BodyKind::Csg))
    })
}
