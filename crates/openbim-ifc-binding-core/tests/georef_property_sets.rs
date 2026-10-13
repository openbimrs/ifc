//! IFC2X3 georeferencing through `ePSet_MapConversion` and
//! `ePSet_ProjectedCRS` (#451), as every host binds it: the same record as
//! the IFC4 twin's `IfcMapConversion`, with source kind `property-set`.
//! The fixtures are `test/fixtures/synthetic-georef/`.

#![cfg(all(feature = "georef", feature = "ifc2x3"))]

use openbim_ifc_binding_core::IfcModel;

fn fixture(path: &str) -> IfcModel {
    let path = format!("{}/../../test/fixtures/{path}", env!("CARGO_MANIFEST_DIR"));
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

/// #451: IFC2X3 `ePSet_MapConversion` and `ePSet_ProjectedCRS` cross
/// as the same record as the IFC4 twin's `IfcMapConversion`.
#[cfg(all(feature = "ifc2x3", feature = "ifc4"))]
#[test]
fn ifc2x3_property_sets_cross_like_the_ifc4_entities() {
    let [pset] = fixture("synthetic-georef/epset_project_ifc2x3.ifc")
        .georeferencing()
        .unwrap()
        .try_into()
        .expect("one operation");
    let [entity] = fixture("synthetic-georef/map_conversion_ifc4.ifc")
        .georeferencing()
        .unwrap()
        .try_into()
        .expect("one operation");
    assert_eq!(
        (
            pset.operation,
            pset.kind.as_str(),
            pset.source,
            pset.source_kind.as_str()
        ),
        (20, "map-conversion", 13, "property-set")
    );
    assert_eq!(pset.target_crs.id, 29);
    assert_eq!(pset.target_crs.name, entity.target_crs.name);
    assert_eq!(pset.target_crs.map_zone, entity.target_crs.map_zone);
    assert_eq!(
        (pset.eastings, pset.northings, pset.orthogonal_height),
        (entity.eastings, entity.northings, entity.orthogonal_height)
    );
    assert_eq!(pset.x_axis, entity.x_axis);
    assert_eq!(pset.scale, entity.scale);
    assert_eq!(pset.project_unit, entity.project_unit);
    assert_eq!(pset.map_unit, entity.map_unit);
    assert!(pset.map_unit_declared && entity.map_unit_declared);
    assert_eq!(pset.linear, entity.linear);
    assert_eq!(pset.translation, entity.translation);
}

#[cfg(feature = "ifc2x3")]
#[test]
fn an_incomplete_ifc2x3_property_set_pair_is_an_invalid_model() {
    let refused = fixture("synthetic-georef/epset_split_pair_ifc2x3.ifc")
        .georeferencing()
        .unwrap_err();
    assert_eq!(refused.code(), "invalid-model", "{refused}");
}
