//! `IfcCoordinateReferenceSystem.Name` is mandatory in IFC4 and optional in
//! IFC4X3 (#142).
//!
//! `IFC4.exp` declares `Name : IfcLabel`. `IFC4X3_ADD2.exp` declares
//! `Name : OPTIONAL IfcLabel`, the inverse
//! `WellKnownText : SET [0:1] OF IfcWellKnownText FOR CoordinateReferenceSystem`
//! and `WHERE NameOrWKT : (HIINDEX(WellKnownText) = 1) OR EXISTS(Name)`.
//! So under IFC4X3 an unnamed CRS with exactly one well-known text reads,
//! one with neither is refused under `NameOrWKT`, and IFC4 keeps requiring
//! the name.

use std::sync::Arc;

use ifc_georef::{resolve_project_to_map, resolve_project_to_map_in, GeorefError, GeorefView};
use ifc_model::value::Value;
use ifc_model::{Entity, EntityId, Model};

const WKT: &str = "PROJCRS[\"ETRS89 / UTM zone 32N\",ID[\"EPSG\",25832]]";

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}

/// A context #1, a projected CRS #2 named `name` (or `$`), `wkts`
/// `IfcWellKnownText` records for it, and a map conversion #4.
fn model(schema: &str, name: Option<&str>, wkts: usize) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec![schema.to_owned()];
    model.insert(
        EntityId(1),
        Entity::new(
            "IFCGEOMETRICREPRESENTATIONCONTEXT",
            vec![
                Value::Null,
                Value::Null,
                Value::Integer(3),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ),
    );
    let mut crs = vec![name.map_or(Value::Null, text)];
    crs.resize(7, Value::Null);
    model.insert(EntityId(2), Entity::new("IFCPROJECTEDCRS", crs));
    for n in 0..wkts {
        model.insert(
            EntityId(10 + n as u64),
            Entity::new("IFCWELLKNOWNTEXT", vec![text(WKT), Value::Ref(EntityId(2))]),
        );
    }
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCMAPCONVERSION",
            vec![
                Value::Ref(EntityId(1)),
                Value::Ref(EntityId(2)),
                Value::Real(500_000.0),
                Value::Real(5_800_000.0),
                Value::Real(100.0),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ),
    );
    model
}

fn resolve(model: &Model) -> Result<ifc_georef::ProjectToMap, GeorefError> {
    let pinned = GeorefView::for_model(model)
        .and_then(|view| resolve_project_to_map_in(&view, EntityId(4), 1.0));
    let unpinned = resolve_project_to_map(model, EntityId(4), 1.0);
    assert_eq!(pinned, unpinned, "both entry points apply the same rule");
    pinned
}

#[test]
fn ifc4x3_reads_an_unnamed_crs_defined_by_well_known_text() {
    let operation = resolve(&model("IFC4X3_ADD2", None, 1)).expect("unnamed CRS with WKT reads");
    assert_eq!(operation.target_crs.name, None);
    assert_eq!(operation.target_crs.well_known_text.as_deref(), Some(WKT));
}

#[test]
fn ifc4x3_refuses_a_crs_with_neither_name_nor_well_known_text() {
    assert_eq!(
        resolve(&model("IFC4X3_ADD2", None, 0)),
        Err(GeorefError::RuleViolation {
            entity: EntityId(2),
            rule: "NameOrWKT",
        })
    );
}

#[test]
fn ifc4x3_reads_a_named_crs_with_or_without_well_known_text() {
    let named = resolve(&model("IFC4X3", Some("EPSG:25832"), 0)).expect("named reads");
    assert_eq!(named.target_crs.name.as_deref(), Some("EPSG:25832"));
    assert_eq!(named.target_crs.well_known_text, None);
    let both = resolve(&model("IFC4X3", Some("EPSG:25832"), 1)).expect("both read");
    assert_eq!(both.target_crs.name.as_deref(), Some("EPSG:25832"));
    assert_eq!(both.target_crs.well_known_text.as_deref(), Some(WKT));
}

/// `SET [0:1]`: two definitions of one CRS are refused, not one picked.
#[test]
fn ifc4x3_refuses_two_well_known_texts_for_one_crs() {
    assert!(matches!(
        resolve(&model("IFC4X3", None, 2)),
        Err(GeorefError::RuleViolation {
            entity: EntityId(2),
            ..
        })
    ));
}

#[test]
fn ifc4_keeps_requiring_the_name() {
    // IFC4 declares no IfcWellKnownText, so a stray one cannot stand in.
    assert_eq!(
        resolve(&model("IFC4", None, 1)),
        Err(GeorefError::MissingAttribute {
            entity: EntityId(2),
            index: 0,
            name: "Name",
        })
    );
    let named = resolve(&model("IFC4", Some("EPSG:25832"), 0)).expect("named IFC4 reads");
    assert_eq!(named.target_crs.name.as_deref(), Some("EPSG:25832"));
    assert_eq!(named.target_crs.well_known_text, None);
}
