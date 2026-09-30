//! `IfcSite` reference point and its elevation against the map conversion
//! (#242), per release.

use std::sync::Arc;

use ifc_georef::{
    relate_site_elevation, resolve_project_to_map, site_reference, GeorefError, SiteElevationCheck,
    SITE_ELEVATION_TOLERANCE_M,
};
use ifc_model::value::Value;
use ifc_model::{Entity, EntityId, Model};
use ifc_schema::SchemaVersion;

const SITE: EntityId = EntityId(20);
const MAP: EntityId = EntityId(4);

fn id(value: u64) -> EntityId {
    EntityId(value)
}

fn text(value: &str) -> Value {
    Value::Text(Arc::from(value))
}

fn angle(parts: &[i64]) -> Value {
    Value::List(parts.iter().map(|p| Value::Integer(*p)).collect())
}

fn elevation(metres: f64) -> Value {
    Value::Typed {
        type_name: Arc::from("IFCLENGTHMEASURE"),
        value: Box::new(Value::Real(metres)),
    }
}

/// An `IfcSite` with the given reference attributes at slots 9..11.
fn site(lat: Value, lon: Value, elev: Value) -> Entity {
    let mut attributes = vec![Value::Null; 14];
    attributes[0] = text("2hQBAVPOr5VxhS3Jl0O47h");
    attributes[8] = Value::Enum(Arc::from("ELEMENT"));
    attributes[9] = lat;
    attributes[10] = lon;
    attributes[11] = elev;
    Entity::new("IFCSITE", attributes)
}

fn model(token: &str, site_entity: Entity) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec![token.to_owned()];
    model.insert(SITE, site_entity);
    model
}

/// Adds an `IfcProjectedCRS` (vertical datum `DHHN2016`, metres) and an
/// `IfcMapConversion` with the given `OrthogonalHeight` and `Scale`.
fn with_map_conversion(mut model: Model, height: f64, scale: Value) -> Model {
    model.insert(
        id(1),
        Entity::new("IFCGEOMETRICREPRESENTATIONCONTEXT", vec![]),
    );
    model.insert(
        id(2),
        Entity::new(
            "IFCPROJECTEDCRS",
            vec![
                text("EPSG:25832"),
                Value::Null,
                text("ETRS89"),
                text("DHHN2016"),
                Value::Null,
                Value::Null,
                Value::Null,
            ],
        ),
    );
    model.insert(
        MAP,
        Entity::new(
            "IFCMAPCONVERSION",
            vec![
                Value::Ref(id(1)),
                Value::Ref(id(2)),
                Value::Real(1000.0),
                Value::Real(2000.0),
                Value::Real(height),
                Value::Null,
                Value::Null,
                scale,
            ],
        ),
    );
    model
}

fn chicago_site(elev: Value) -> Entity {
    site(
        angle(&[41, 53, 30]),
        angle(&[-87, -35, -40, -250_000]),
        elev,
    )
}

#[test]
fn reads_the_reference_point_in_every_supported_release() {
    for (token, version) in [
        ("IFC2X3", SchemaVersion::Ifc2x3),
        ("IFC4", SchemaVersion::Ifc4),
        ("IFC4X3_ADD2", SchemaVersion::Ifc4x3),
    ] {
        // Project unit millimetres: RefElevation 176500 mm is 176.5 m.
        let model = model(token, chicago_site(elevation(176_500.0)));
        let site = site_reference(&model, SITE, 0.001).expect(token);
        assert_eq!(site.version, version);
        assert_eq!(site.ref_elevation, Some(176.5));
        let lat = site.latitude_degrees().expect("latitude");
        assert!((lat - (41.0 + 53.0 / 60.0 + 30.0 / 3600.0)).abs() < 1e-12);
        let lon = site.longitude_degrees().expect("longitude");
        let expected = -(87.0 + 35.0 / 60.0 + 40.25 / 3600.0);
        assert!((lon - expected).abs() < 1e-12, "{token}: {lon}");
    }
}

#[test]
fn absent_attributes_are_none_and_compare_as_absent() {
    for token in ["IFC2X3", "IFC4", "IFC4X3"] {
        let model = model(token, site(Value::Null, Value::Null, Value::Null));
        let site = site_reference(&model, SITE, 1.0).expect(token);
        assert_eq!(site.ref_latitude, None);
        assert_eq!(site.ref_longitude, None);
        assert_eq!(site.ref_elevation, None);
        assert_eq!(
            relate_site_elevation(&site, None, 0.0, SITE_ELEVATION_TOLERANCE_M),
            Ok(SiteElevationCheck::RefElevationAbsent)
        );
    }
    let model = with_map_conversion(
        model("IFC4", site(Value::Null, Value::Null, Value::Null)),
        50.0,
        Value::Null,
    );
    let site = site_reference(&model, SITE, 1.0).unwrap();
    let map = resolve_project_to_map(&model, MAP, 1.0).unwrap();
    assert_eq!(
        relate_site_elevation(&site, Some(&map), 0.0, SITE_ELEVATION_TOLERANCE_M),
        Ok(SiteElevationCheck::RefElevationAbsent)
    );
}

#[test]
fn ifc2x3_has_no_map_conversion_to_compare_with() {
    let model = model("IFC2X3", chicago_site(elevation(176.5)));
    let site = site_reference(&model, SITE, 1.0).unwrap();
    assert_eq!(
        relate_site_elevation(&site, None, 0.0, SITE_ELEVATION_TOLERANCE_M),
        Ok(SiteElevationCheck::NoMapConversion)
    );
    // A map conversion cannot belong to an IFC2X3 model.
    let other = with_map_conversion(model.clone(), 176.5, Value::Null);
    let map = resolve_project_to_map(&other, MAP, 1.0).unwrap();
    assert!(matches!(
        relate_site_elevation(&site, Some(&map), 0.0, SITE_ELEVATION_TOLERANCE_M),
        Err(GeorefError::UnsupportedSchema { token }) if token == "IFC2X3"
    ));
}

#[test]
fn agreement_within_tolerance_is_consistent() {
    for token in ["IFC4", "IFC4X3"] {
        let model = with_map_conversion(
            model(token, chicago_site(elevation(50.004))),
            50.0,
            Value::Null,
        );
        let site = site_reference(&model, SITE, 1.0).unwrap();
        let map = resolve_project_to_map(&model, MAP, 1.0).unwrap();
        let check = relate_site_elevation(&site, Some(&map), 0.0, SITE_ELEVATION_TOLERANCE_M);
        let Ok(SiteElevationCheck::Consistent(c)) = check else {
            panic!("{token}: expected consistency, got {check:?}");
        };
        assert!((c.difference - 0.004).abs() < 1e-9);
        assert_eq!(c.orthogonal_height, 50.0);
        assert_eq!(c.vertical_datum.as_deref(), Some("DHHN2016"));
        assert_eq!(c.target_crs, id(2));
    }
}

#[test]
fn disagreement_beyond_tolerance_is_reported_with_both_values() {
    for token in ["IFC4", "IFC4X3"] {
        let model = with_map_conversion(
            model(token, chicago_site(elevation(51.0))),
            50.0,
            Value::Null,
        );
        let site = site_reference(&model, SITE, 1.0).unwrap();
        let map = resolve_project_to_map(&model, MAP, 1.0).unwrap();
        let check = relate_site_elevation(&site, Some(&map), 0.0, SITE_ELEVATION_TOLERANCE_M);
        let Ok(SiteElevationCheck::Disagreement(c)) = check else {
            panic!("{token}: expected a disagreement, got {check:?}");
        };
        assert_eq!(c.ref_elevation, 51.0);
        assert_eq!(c.site_origin_map_height, 50.0);
        assert!((c.difference - 1.0).abs() < 1e-12);
        assert_eq!(c.tolerance, SITE_ELEVATION_TOLERANCE_M);
        assert_eq!(c.site, SITE);
    }
}

#[test]
fn the_site_origin_height_is_carried_through_the_map_scale() {
    // Site origin 2 m above the world origin; Scale 0.5 halves it on the
    // map, so the site origin sits at 50 + 1 = 51 m.
    let model = with_map_conversion(
        model("IFC4", chicago_site(elevation(51.0))),
        50.0,
        Value::Real(0.5),
    );
    let site = site_reference(&model, SITE, 1.0).unwrap();
    let map = resolve_project_to_map(&model, MAP, 1.0).unwrap();
    let check = relate_site_elevation(&site, Some(&map), 2.0, SITE_ELEVATION_TOLERANCE_M);
    let Ok(SiteElevationCheck::Consistent(c)) = check else {
        panic!("expected consistency, got {check:?}");
    };
    assert_eq!(c.site_origin_map_height, 51.0);
    assert_eq!(c.orthogonal_height, 50.0);
    // Ignoring the offset would be a 1 m disagreement.
    assert!(matches!(
        relate_site_elevation(&site, Some(&map), 0.0, SITE_ELEVATION_TOLERANCE_M),
        Ok(SiteElevationCheck::Disagreement(_))
    ));
}

#[test]
fn invalid_parameters_are_refused() {
    let model = model("IFC4", chicago_site(elevation(1.0)));
    let site = site_reference(&model, SITE, 1.0).unwrap();
    for (z, tolerance, name) in [
        (0.0, -0.1, "tolerance"),
        (0.0, f64::NAN, "tolerance"),
        (f64::INFINITY, 0.01, "site_origin_z"),
    ] {
        assert!(matches!(
            relate_site_elevation(&site, None, z, tolerance),
            Err(GeorefError::InvalidParameter { name: n, .. }) if n == name
        ));
    }
    assert!(matches!(
        site_reference(&model, SITE, 0.0),
        Err(GeorefError::InvalidUnit { .. })
    ));
}

#[test]
fn malformed_compound_angles_are_refused_per_release() {
    // IFC4/IFC4X3 ConsistentSign covers the millionths; IFC2X3 WR4 does not.
    let mixed = || site(angle(&[41, 53, 30, -5]), Value::Null, Value::Null);
    for token in ["IFC4", "IFC4X3"] {
        assert_eq!(
            site_reference(&model(token, mixed()), SITE, 1.0),
            Err(GeorefError::InvalidCompoundAngle {
                entity: SITE,
                index: 9,
                name: "RefLatitude",
                rule: "ConsistentSign",
            })
        );
    }
    assert!(site_reference(&model("IFC2X3", mixed()), SITE, 1.0).is_ok());
    let bad_minutes = site(Value::Null, angle(&[10, 60, 0]), Value::Null);
    assert!(matches!(
        site_reference(&model("IFC2X3", bad_minutes), SITE, 1.0),
        Err(GeorefError::InvalidCompoundAngle {
            index: 10,
            name: "RefLongitude",
            rule: "WR2",
            ..
        })
    ));
    let short = site(angle(&[41, 53]), Value::Null, Value::Null);
    assert!(matches!(
        site_reference(&model("IFC4", short), SITE, 1.0),
        Err(GeorefError::InvalidAttribute {
            index: 9,
            name: "RefLatitude",
            ..
        })
    ));
    let text_elevation = site(Value::Null, Value::Null, text("high"));
    assert!(matches!(
        site_reference(&model("IFC4", text_elevation), SITE, 1.0),
        Err(GeorefError::InvalidAttribute {
            index: 11,
            name: "RefElevation",
            ..
        })
    ));
}

#[test]
fn unsupported_releases_and_wrong_entities_are_refused() {
    for token in ["IFC4X1", "IFC4X2", "IFC5"] {
        assert!(matches!(
            site_reference(&model(token, chicago_site(Value::Null)), SITE, 1.0),
            Err(GeorefError::UnsupportedSchema { token: t }) if t == token
        ));
    }
    let mut missing = model("IFC4", chicago_site(Value::Null));
    missing.header_mut().schema.clear();
    assert_eq!(
        site_reference(&missing, SITE, 1.0),
        Err(GeorefError::MissingSchema)
    );
    let mut building = Model::new();
    building.header_mut().schema = vec!["IFC4".to_owned()];
    building.insert(SITE, Entity::new("IFCBUILDING", vec![]));
    assert!(matches!(
        site_reference(&building, SITE, 1.0),
        Err(GeorefError::WrongType {
            expected: "IFCSITE",
            ..
        })
    ));
}
