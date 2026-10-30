//! `IfcDraughtingPreDefinedColour` and the `IfcColour` select as RGB.

use ifc_model::EntityId;

use crate::error::{StyleError, StyleResult};
use crate::StyleView;

/// The RGB of an `IfcDraughtingPreDefinedColour` name.
///
/// IFC4 ADD2 TC1, `IfcDraughtingPreDefinedColour`: black (0, 0, 0), red
/// (1, 0, 0), green (0, 1, 0), blue (0, 0, 1), yellow (1, 1, 0), magenta
/// (1, 0, 1), cyan (0, 1, 1) and white (1, 1, 1). The names are
/// case-sensitive (`PreDefinedColourNames`); `'by layer'` has no RGB.
pub(crate) fn draughting_rgb(name: &str) -> Option<[f64; 3]> {
    Some(match name {
        "black" => [0.0, 0.0, 0.0],
        "red" => [1.0, 0.0, 0.0],
        "green" => [0.0, 1.0, 0.0],
        "blue" => [0.0, 0.0, 1.0],
        "yellow" => [1.0, 1.0, 0.0],
        "magenta" => [1.0, 0.0, 1.0],
        "cyan" => [0.0, 1.0, 1.0],
        "white" => [1.0, 1.0, 1.0],
        _ => return None,
    })
}

/// The RGB of `id`, an `IfcColour` member read while resolving the fill
/// area style `style`.
///
/// # Errors
///
/// [`StyleError::UnsupportedFill`] for `'by layer'`, which names the colour
/// of a layer the fill does not know; [`StyleError::InvalidValue`] for a
/// name outside the allowed list; [`StyleError::ReferenceType`] for a
/// colour that is neither an `IfcColourRgb` nor an
/// `IfcDraughtingPreDefinedColour`; and the projection errors of a malformed
/// colour.
pub(crate) fn colour_rgb_of(
    view: StyleView<'_, '_>,
    style: EntityId,
    id: EntityId,
) -> StyleResult<[f64; 3]> {
    let entity = view.model.get(id).ok_or(StyleError::UnknownEntity { id })?;
    if view.schema.is_a(&entity.type_name, "IfcColourRgb") {
        return view.colour_rgb(id)?.channels();
    }
    if view
        .schema
        .is_a(&entity.type_name, "IfcDraughtingPreDefinedColour")
    {
        let name = view
            .record(id, "IfcDraughtingPreDefinedColour")?
            .required_text("Name")?;
        if name == "by layer" {
            return Err(StyleError::UnsupportedFill {
                style,
                entity: id,
                construct: "IfcDraughtingPreDefinedColour 'by layer'".to_owned(),
            });
        }
        return draughting_rgb(name).ok_or_else(|| StyleError::InvalidValue {
            entity: entity.type_name.to_string(),
            id,
            attribute: "Name",
            value: format!("PreDefinedColourNames: {name:?}"),
        });
    }
    Err(StyleError::ReferenceType {
        target: id,
        expected: "IfcColour",
        actual: entity.type_name.to_string(),
    })
}
