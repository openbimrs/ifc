//! Authoring `IfcWellKnownText`.
//!
//! The literal is written verbatim: WKT is its own grammar, and
//! reformatting it here would change a definition the exporting tool
//! produced deliberately.

use ifc_georef::{create_projected_crs, create_well_known_text, ProjectedCrsDraft};
use ifc_model::{Entity, Model, Transaction, Value};

const WKT: &str = "PROJCS[\"ETRS89 / UTM zone 32N\",GEOGCS[\"ETRS89\"]]";

/// A well-known text stages against its CRS.
#[test]
fn a_well_known_text_stages() {
    let mut model = Model::default();
    let mut tx = Transaction::new(&model);
    let crs = create_projected_crs(
        &mut tx,
        ProjectedCrsDraft {
            name: "EPSG:25832",
            ..ProjectedCrsDraft::default()
        },
    )
    .expect("crs");

    let id = create_well_known_text(&mut tx, &model, WKT, crs).expect("wkt");
    tx.commit(&mut model).expect("commit");

    let staged = model.get(id).expect("staged");
    assert_eq!(staged.type_name.as_ref(), "IFCWELLKNOWNTEXT");
    assert_eq!(staged.attributes.len(), 2);
    assert_eq!(
        staged.attributes[0],
        Value::Text(WKT.into()),
        "the literal is written verbatim",
    );
    assert_eq!(staged.attributes[1], Value::Ref(crs));
}

/// A blank literal and a non-CRS reference are refused.
#[test]
fn a_degenerate_well_known_text_is_refused() {
    let model = Model::default();
    let mut tx = Transaction::new(&model);
    let crs = create_projected_crs(
        &mut tx,
        ProjectedCrsDraft {
            name: "EPSG:25832",
            ..ProjectedCrsDraft::default()
        },
    )
    .expect("crs");
    let stray = tx.create(Entity::new("IFCSIUNIT", vec![Value::Null; 4]));

    assert!(
        create_well_known_text(&mut tx, &model, "   ", crs).is_err(),
        "a blank literal was accepted",
    );
    assert!(
        create_well_known_text(&mut tx, &model, WKT, stray).is_err(),
        "an IfcSIUnit was accepted as a coordinate reference system",
    );
}
