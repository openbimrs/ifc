//! `IfcProjectedCRS.MapUnit` as authored next to the resolved map unit
//! (#296).
//!
//! IFC4 and IFC4X3 declare `MapUnit : OPTIONAL IfcNamedUnit`; an omitted
//! unit means the project's default length unit, which the resolved
//! `ProjectToMap::map_unit` and the transform use. A consumer still has to
//! tell "unset" apart from "states the project's unit", so the declared
//! value is kept as authored. Nothing here needs the `transform` feature:
//! the gate runs this file with and without it.

#[path = "support/step.rs"]
mod support;

use ifc_georef::{resolve_project_to_map_in, GeorefView, LengthUnit, ProjectToMap};
use ifc_model::EntityId;
use support::{step, BASE};

/// Beside `support::BASE` (whose `#50` states metres, the project unit):
/// `#11` millimetre, `#51` a CRS with `MapUnit` unset, `#52` one stating
/// millimetres.
const CRSS: &str = "\
#11=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#51=IFCPROJECTEDCRS('EPSG:25832','ETRS89 / UTM zone 32N','ETRS89','DHHN2016','UTM','32N',$);
#52=IFCPROJECTEDCRS('EPSG:25832','ETRS89 / UTM zone 32N','ETRS89','DHHN2016','UTM','32N',#11);
";

/// A map conversion from the context onto `crs`. A context carries at most
/// one operation, so each model holds exactly one.
fn map_conversion(crs: u64) -> String {
    format!("#60=IFCMAPCONVERSION(#7,#{crs},500000.,5800000.,100.,$,$,$);\n")
}

/// IFC4X3 only: a length rigid operation from the context onto `crs`.
fn rigid_operation(crs: u64) -> String {
    format!("#60=IFCRIGIDOPERATION(#7,#{crs},IFCLENGTHMEASURE(10.),IFCLENGTHMEASURE(20.),$);\n")
}

const PROJECT_METRES_PER_UNIT: f64 = 1.0;

fn resolve(schema: &str, operation: &str) -> ProjectToMap {
    let model = step(schema, &format!("{BASE}{CRSS}{operation}"));
    let view = GeorefView::for_model(&model).expect("pinned header");
    resolve_project_to_map_in(&view, EntityId(60), PROJECT_METRES_PER_UNIT)
        .unwrap_or_else(|error| panic!("{schema} {operation} resolves: {error:?}"))
}

fn millimetre() -> LengthUnit {
    LengthUnit {
        name: "MILLIMETRE".into(),
        metres_per_unit: 0.001,
    }
}

/// Unset `MapUnit`: nothing declared, the project length unit resolved and
/// used by the operation.
fn assert_unset(operation: &ProjectToMap, schema: &str) {
    assert_eq!(operation.declared_map_unit(), None, "{schema}");
    assert_eq!(operation.target_crs.map_unit, None, "{schema}");
    assert_eq!(operation.map_unit, operation.project_unit, "{schema}");
    assert_eq!(
        operation.map_unit.metres_per_unit, PROJECT_METRES_PER_UNIT,
        "{schema}"
    );
}

/// Stated `MapUnit`: declared and resolved are that unit, and the
/// translation is converted from it.
fn assert_millimetre(operation: &ProjectToMap, schema: &str) {
    assert_eq!(
        operation.declared_map_unit(),
        Some(&millimetre()),
        "{schema}"
    );
    assert_eq!(
        operation.target_crs.map_unit,
        Some(millimetre()),
        "{schema}"
    );
    assert_eq!(operation.map_unit, millimetre(), "{schema}");
    assert_ne!(operation.map_unit, operation.project_unit, "{schema}");
}

#[test]
fn an_unset_map_unit_is_not_declared_but_resolves_to_the_project_unit() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let operation = resolve(schema, &map_conversion(51));
        assert_unset(&operation, schema);
        assert_eq!(
            operation.translation(),
            [500_000.0, 5_800_000.0, 100.0],
            "{schema}"
        );
    }
}

#[test]
fn a_stated_map_unit_is_both_declared_and_resolved() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let operation = resolve(schema, &map_conversion(52));
        assert_millimetre(&operation, schema);
        // Eastings, Northings and OrthogonalHeight are millimetres here.
        let [e, n, h] = operation.translation();
        assert!(
            (e - 500.0).abs() < 1e-9 && (n - 5_800.0).abs() < 1e-9 && (h - 0.1).abs() < 1e-12,
            "{schema}: {:?}",
            operation.translation()
        );
    }
}

/// A stated unit equal to the project's is still declared: the case the
/// resolved unit alone cannot tell apart from an unset one.
#[test]
fn a_map_unit_stating_the_project_unit_is_still_declared() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let operation = resolve(schema, &map_conversion(50));
        let declared = operation.declared_map_unit().expect("MapUnit is stated");
        assert_eq!(declared.name, "METRE", "{schema}");
        assert_eq!(declared.metres_per_unit, 1.0, "{schema}");
        assert_eq!(operation.map_unit, *declared, "{schema}");
    }
}

#[test]
fn an_ifc4x3_rigid_operation_keeps_the_declared_map_unit_too() {
    let schema = "IFC4X3_ADD2";
    let unset = resolve(schema, &rigid_operation(51));
    assert_unset(&unset, schema);
    assert_eq!(unset.translation(), [10.0, 20.0, 0.0]);

    let stated = resolve(schema, &rigid_operation(52));
    assert_millimetre(&stated, schema);
    let [e, n, h] = stated.translation();
    assert!((e - 0.01).abs() < 1e-15 && (n - 0.02).abs() < 1e-15 && h == 0.0);
}
