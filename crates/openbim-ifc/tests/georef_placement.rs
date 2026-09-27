//! Facade composition: a product placement from `ifc-geometry` carried into
//! map coordinates by `ifc-georef`.
//!
//! The two crates split "where is this" at the project frame on purpose
//! (see `../../ifc-georef/AGENTS.md`): `ifc-geometry` resolves the
//! `IfcLocalPlacement` chain in metres, and `ifc-georef` resolves
//! `IfcMapConversion` from project metres to map metres. Neither may call
//! the other, so the only place the two halves meet is this layer. That
//! seam is where a unit slip lands -- a raw millimetre frame fed to a
//! metre-to-metre operation places the product a thousand times too far
//! out -- so it is checked here against coordinates computed by hand.

#![cfg(all(feature = "geometry", feature = "georef", feature = "step"))]

use ifc::georef::{compose_project_frame, resolve_project_to_map};
use ifc::{from_step_bytes, product_world_transform, EntityId};

/// An IFC4 model in MILLIMETRES whose map is in metres.
///
/// - The site sits at (10 000, 20 000, 5 000) mm with default axes.
/// - The wall sits at (3 000, 4 000, 0) mm relative to the site, turned a
///   quarter turn: its `RefDirection` (0, 1, 0) sends local X to project Y.
/// - `IfcMapConversion`: origin at E 500 000, N 5 700 000, H 100 m;
///   `XAxisAbscissa`/`XAxisOrdinate` (0, 1), so project X points to map
///   north; `Scale` 0.001, the IFC4 conversion from project millimetres to
///   map metres, which leaves a unit metre-to-metre scale.
const FIXTURE: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [ReferenceView]'),'2;1');
FILE_NAME('georef_placement.ifc','2026-09-26T00:00:00',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#3=IFCUNITASSIGNMENT((#1,#2));
#4=IFCCARTESIANPOINT((0.,0.,0.));
#5=IFCAXIS2PLACEMENT3D(#4,$,$);
#6=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-5,#5,$);
#7=IFCPROJECT('2O2Fr$t4X7Zf8NOew3FLOH',$,'georef placement',$,$,$,$,(#6),#3);
#10=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#11=IFCPROJECTEDCRS('EPSG:25832','ETRS89 / UTM zone 32N','ETRS89','DHHN2016','UTM','32N',#10);
#12=IFCMAPCONVERSION(#6,#11,500000.,5700000.,100.,0.,1.,0.001);
#20=IFCCARTESIANPOINT((10000.,20000.,5000.));
#21=IFCAXIS2PLACEMENT3D(#20,$,$);
#22=IFCLOCALPLACEMENT($,#21);
#23=IFCSITE('1cwlDi_hLEvPsClAelBNnz',$,'Site',$,$,#22,$,$,.ELEMENT.,$,$,$,$,$);
#30=IFCCARTESIANPOINT((3000.,4000.,0.));
#31=IFCDIRECTION((0.,0.,1.));
#32=IFCDIRECTION((0.,1.,0.));
#33=IFCAXIS2PLACEMENT3D(#30,#31,#32);
#34=IFCLOCALPLACEMENT(#22,#33);
#35=IFCWALL('3vB2YO$MX4xv5uCqZZG05x',$,'Wall',$,$,#34,$,$,.STANDARD.);
ENDSEC;
END-ISO-10303-21;
";

const WALL: EntityId = EntityId(35);
const MAP_CONVERSION: EntityId = EntityId(12);

/// Read without naming a geometry type: `ifc-geometry`'s
/// `no_backend_dependency` gate forbids `axiolid-*` in this manifest, even
/// as a dev-dependency.
fn assert_close(actual: [f64; 3], expected: [f64; 3], what: &str) {
    let delta = actual
        .iter()
        .zip(expected)
        .map(|(a, e)| (a - e).abs())
        .fold(0.0, f64::max);
    assert!(
        delta < 1e-9,
        "{what}: got {actual:?}, expected {expected:?}"
    );
}

/// Hand computation, in metres:
///
/// - Project frame of the wall: origin (10 + 3, 20 + 4, 5 + 0) =
///   (13, 24, 5); local X maps to project (0, 1, 0).
/// - Map conversion with (a, b) = (0, 1) and unit scale:
///   E = 500 000 + (a x - b y) = 500 000 - y,
///   N = 5 700 000 + (b x + a y) = 5 700 000 + x,
///   H = 100 + z.
/// - Wall origin (13, 24, 5) -> (499 976, 5 700 013, 105).
/// - Wall-local point (1, 0, 0), one metre along the wall, is project
///   (13, 25, 5) -> (499 975, 5 700 013, 105): the wall runs due west.
#[test]
fn a_products_resolved_placement_composes_into_known_map_coordinates() {
    let model = from_step_bytes(FIXTURE.as_bytes()).expect("fixture parses");

    // Both halves take their unit from the same model, which is exactly the
    // agreement this layer is responsible for.
    let units = ifc::geometry::units::resolve(&model);
    assert_eq!(units.length_to_metres, 0.001);

    let project_frame = product_world_transform(&model, &units, WALL)
        .expect("wall placement resolves")
        .to_geom();
    let operation = resolve_project_to_map(&model, MAP_CONVERSION, units.length_to_metres)
        .expect("map conversion resolves");
    assert_eq!(operation.target_crs.name, "EPSG:25832");

    let map_frame = compose_project_frame(&operation, project_frame).expect("composes");

    // An affine frame maps local p to `matrix3 * p + translation`: the
    // translation is the image of the local origin, and `x_axis` the image
    // of the local unit X vector.
    let origin = map_frame.translation.to_array();
    let along = map_frame.matrix3.x_axis.to_array();
    assert_close(origin, [499_976.0, 5_700_013.0, 105.0], "wall origin");
    assert_close(
        [
            origin[0] + along[0],
            origin[1] + along[1],
            origin[2] + along[2],
        ],
        [499_975.0, 5_700_013.0, 105.0],
        "one metre along the wall",
    );
    assert_close(along, [-1.0, 0.0, 0.0], "the wall must run due west");
}

/// The composition is only this layer's to make if neither bridge reaches
/// into the other. `ifc-model`'s architecture test allows bridge-to-bridge
/// edges in general, so the specific pair is pinned here.
#[test]
fn geometry_and_georef_do_not_depend_on_each_other() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for (krate, forbidden) in [
        ("ifc-geometry", "ifc-georef"),
        ("ifc-georef", "ifc-geometry"),
    ] {
        let manifest = std::fs::read_to_string(root.join(krate).join("Cargo.toml"))
            .unwrap_or_else(|error| panic!("{krate}/Cargo.toml: {error}"));
        let mut section = String::new();
        let mut checked = 0;
        for line in manifest.lines().map(str::trim) {
            if let Some(header) = line.strip_prefix('[') {
                section = header.trim_end_matches(']').to_owned();
                continue;
            }
            let production = section == "dependencies"
                || section == "build-dependencies"
                || (section.starts_with("target.") && !section.ends_with("dev-dependencies"));
            if production && !line.is_empty() && !line.starts_with('#') {
                checked += 1;
                let name = line.split(['=', '.', ' ']).next().unwrap_or_default();
                assert_ne!(name, forbidden, "{krate} depends on {forbidden}");
            }
        }
        // A manifest restructure that hid every dependency from this scan
        // would pass vacuously; both crates have several.
        assert!(checked >= 3, "{krate}: scanned only {checked} dependencies");
    }
}
