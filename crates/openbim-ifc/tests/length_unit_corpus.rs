//! `convert_length_unit` over the fixture corpus (#469): a millimetre and
//! metre round trip restores every value, geometry compiled before and
//! after a conversion agrees in metres (volumes in project units scale by
//! the cube of the length factor), and validation findings are unchanged.

#![cfg(all(feature = "step", feature = "properties", feature = "schema"))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ifc::properties::ConversionUnit;
use ifc::{
    convert_length_unit, Codec, LengthUnitError, LengthUnitOptions, Model, StepCodec, Value,
};

fn fixtures() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("fixture directory").flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "ifc") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures"),
        &mut out,
    );
    out.sort();
    out
}

fn read(path: &Path) -> Option<Model> {
    let model = StepCodec.read_bytes(&std::fs::read(path).ok()?).ok()?;
    model.diagnostics().is_empty().then_some(model)
}

fn name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

/// The models the conversion applies to: a release it binds and a project
/// length unit it resolves. Refusals are reported, never hidden.
fn convertible() -> Vec<(PathBuf, Model)> {
    fixtures()
        .into_iter()
        .filter_map(|path| read(&path).map(|model| (path, model)))
        .filter(|(_, model)| ifc::properties::exact_project_unit(model, "LENGTHUNIT").is_ok())
        .collect()
}

const UNIT_RESOURCES: [&str; 5] = [
    "IFCSIUNIT",
    "IFCCONVERSIONBASEDUNIT",
    "IFCMEASUREWITHUNIT",
    "IFCDIMENSIONALEXPONENTS",
    "IFCUNITASSIGNMENT",
];

fn close(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Real(a), Value::Real(b)) => (a - b).abs() <= 1e-12 * a.abs().max(b.abs()),
        (Value::Integer(a), Value::Real(b)) | (Value::Real(b), Value::Integer(a)) => {
            #[allow(clippy::cast_precision_loss)]
            let a = *a as f64;
            (a - b).abs() <= 1e-12 * a.abs().max(b.abs())
        }
        (Value::List(a), Value::List(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| close(a, b))
        }
        (
            Value::Typed {
                type_name: ta,
                value: va,
            },
            Value::Typed {
                type_name: tb,
                value: vb,
            },
        ) => ta == tb && close(va, vb),
        _ => a == b,
    }
}

#[test]
fn millimetres_to_metres_and_back_restores_every_value() {
    let options = LengthUnitOptions::new().area_and_volume(false);
    let mut converted: BTreeMap<String, usize> = BTreeMap::new();
    let mut refused = Vec::new();
    for (path, original) in convertible() {
        let schema = original.header().schema.join(",");
        let from = ifc::properties::resolve_unit(
            &original,
            ifc::properties::exact_project_unit(&original, "LENGTHUNIT").unwrap(),
        )
        .unwrap();
        let other = if from.decimal_exponent == -3 {
            "METRE"
        } else {
            "MILLIMETRE"
        };
        let mut model = original.clone();
        let there =
            match convert_length_unit(&mut model, &ConversionUnit::parse(other).unwrap(), &options)
            {
                Ok(report) => report,
                Err(LengthUnitError::Refused(refusals)) => {
                    assert_eq!(
                        model.revision(),
                        original.revision(),
                        "{}: a refusal changes nothing",
                        name(&path)
                    );
                    refused.push((name(&path), refusals.len(), refusals[0].reason.clone()));
                    continue;
                }
                Err(error) => panic!("{}: {error}", name(&path)),
            };
        assert!(there.applied, "{}", name(&path));
        let back_to = ConversionUnit::parse(if other == "METRE" {
            "MILLIMETRE"
        } else {
            "METRE"
        })
        .unwrap();
        convert_length_unit(&mut model, &back_to, &options)
            .unwrap_or_else(|error| panic!("{}: back: {error}", name(&path)));
        for (id, before) in original.iter() {
            if UNIT_RESOURCES.contains(&before.type_name.as_ref()) {
                continue;
            }
            let after = model
                .get(id)
                .unwrap_or_else(|| panic!("{}: #{} kept", name(&path), id.0));
            assert_eq!(after.type_name, before.type_name);
            for (slot, (a, b)) in before.attributes.iter().zip(&after.attributes).enumerate() {
                assert!(
                    close(a, b),
                    "{}: #{} slot {slot}: {a:?} became {b:?}",
                    name(&path),
                    id.0
                );
            }
        }
        *converted.entry(schema).or_default() += 1;
        // A metre value went through the other unit and back.
        assert!(
            there.values_converted > 0 || there.length.is_identity(),
            "{}",
            name(&path)
        );
    }
    eprintln!("round trips per release: {converted:?}");
    eprintln!("refused: {refused:#?}");
    for release in ["IFC2X3", "IFC4", "IFC4X3_ADD2"] {
        assert!(
            converted.get(release).copied().unwrap_or(0) >= 5,
            "{release}: {converted:?}"
        );
    }
}

#[cfg(feature = "validate")]
#[test]
fn validation_findings_are_unchanged() {
    let rules = |model: &Model| {
        let mut rules: Vec<String> = ifc::validate::validate_declared(model)
            .map(|report| report.findings().iter().map(|f| f.rule.clone()).collect())
            .unwrap_or_default();
        rules.sort();
        rules
    };
    let mut checked = 0;
    for (path, original) in convertible() {
        let mut model = original.clone();
        let target = ConversionUnit::parse("foot").unwrap();
        if convert_length_unit(&mut model, &target, &LengthUnitOptions::new()).is_err() {
            continue;
        }
        assert_eq!(rules(&model), rules(&original), "{}", name(&path));
        checked += 1;
    }
    assert!(checked >= 30, "{checked} models validated");
}

#[cfg(feature = "mesh")]
mod geometry {
    use super::*;
    use ifc::{product_meshes, ProductMesh};

    /// World vertex positions in metres and the enclosed volume.
    fn world(mesh: &ProductMesh) -> (Vec<[f64; 3]>, f64) {
        let points: Vec<[f64; 3]> = mesh
            .positions
            .chunks(3)
            .map(|p| {
                mesh.world
                    .apply([f64::from(p[0]), f64::from(p[1]), f64::from(p[2])])
            })
            .collect();
        let mut volume = 0.0;
        for triangle in mesh.indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|i| points[triangle[i] as usize]);
            volume += (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                / 6.0;
        }
        (points, volume)
    }

    /// The products whose mesh does not change when every point
    /// coordinate moves by a relative 2e-16, about one rounding. A unit
    /// conversion moves coordinates by that much; a body whose compiled
    /// volume jumps under it cannot tell a correct conversion from a wrong
    /// one, and is left out and counted. In the corpus these are booleans
    /// with coplanar faces (`flush_openings_site_placements.ifc`, the walls
    /// and windows of `issue_098_wall_W.ifc`, the openings of
    /// `binding_geometry.ifc`), whose result the kernel decides by the
    /// last bit; a millimetre-metre round trip meshes them identically.
    fn stable_products(
        model: &Model,
        meshes: &[(ifc::EntityId, ifc::geometry::GeometryResult<ProductMesh>)],
    ) -> std::collections::HashSet<ifc::EntityId> {
        let mut perturbed = model.clone();
        let points: Vec<_> = perturbed.ids_of_type("IFCCARTESIANPOINT").to_vec();
        for id in points {
            let Some(Value::List(coordinates)) = perturbed.get(id).map(|e| e.attributes[0].clone())
            else {
                continue;
            };
            let moved = coordinates
                .iter()
                .map(|value| match value {
                    Value::Real(x) => Value::Real(x * (1.0 + 2e-16)),
                    other => other.clone(),
                })
                .collect();
            perturbed.set_attribute(id, 0, Value::List(moved));
        }
        let again = product_meshes(&perturbed, None);
        meshes
            .iter()
            .zip(again)
            .filter_map(|((product, a), (_, b))| match (a, b) {
                (Ok(a), Ok(b)) => {
                    let (va, vb) = (world(a).1, world(&b).1);
                    ((va - vb).abs() <= 1e-6 * va.abs().max(1e-12)).then_some(*product)
                }
                _ => Some(*product),
            })
            .collect()
    }

    fn bounds(points: &[[f64; 3]]) -> ([f64; 3], [f64; 3]) {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for point in points {
            for axis in 0..3 {
                lo[axis] = lo[axis].min(point[axis]);
                hi[axis] = hi[axis].max(point[axis]);
            }
        }
        (lo, hi)
    }

    #[test]
    fn compiled_geometry_agrees_after_converting_to_feet() {
        let mut products = 0;
        let mut models: BTreeMap<String, usize> = BTreeMap::new();
        let mut total = (0.0, 0.0);
        let mut unstable = 0;
        for (path, original) in convertible() {
            let mut model = original.clone();
            let target = ConversionUnit::parse("foot").unwrap();
            let Ok(report) = convert_length_unit(&mut model, &target, &LengthUnitOptions::new())
            else {
                continue;
            };
            let k = report.length.factor();
            let before = product_meshes(&original, None);
            let after = product_meshes(&model, None);
            let stable = stable_products(&original, &before);
            assert_eq!(before.len(), after.len(), "{}", name(&path));
            for ((product, a), (_, b)) in before.iter().zip(&after) {
                let (a, b) = match (a, b) {
                    (Ok(a), Ok(b)) => (a, b),
                    (Err(a), Err(b)) => {
                        assert_eq!(
                            std::mem::discriminant(a),
                            std::mem::discriminant(b),
                            "{}: #{} refused differently",
                            name(&path),
                            product.0
                        );
                        continue;
                    }
                    (a, b) => panic!("{}: #{}: {a:?} became {b:?}", name(&path), product.0),
                };
                let (pa, va) = world(a);
                let (pb, vb) = world(b);
                if pa.is_empty() || pb.is_empty() {
                    assert_eq!(pa.len(), pb.len(), "{}: #{}", name(&path), product.0);
                    continue;
                }
                if !stable.contains(product) {
                    unstable += 1;
                    continue;
                }
                // Tessellation may order or place arc vertices differently;
                // the extent and the volume may not differ.
                let (lo_a, hi_a) = bounds(&pa);
                let (lo_b, hi_b) = bounds(&pb);
                for axis in 0..3 {
                    for (x, y) in [(lo_a[axis], lo_b[axis]), (hi_a[axis], hi_b[axis])] {
                        let tolerance = 1e-5 + 1e-6 * x.abs();
                        assert!(
                            (x - y).abs() <= tolerance,
                            "{}: #{}: extent {lo_a:?}..{hi_a:?} became {lo_b:?}..{hi_b:?}",
                            name(&path),
                            product.0
                        );
                    }
                }
                assert!(
                    (va - vb).abs() <= 1e-6 * va.abs().max(1e-9),
                    "{}: #{}: volume {va} m3 became {vb} m3",
                    name(&path),
                    product.0
                );
                // In project units the volume scales by the cube of the
                // length factor: the same metres, a unit k times smaller.
                let before_units = va / report.from.scale().powi(3);
                let after_units = vb / report.to.scale().powi(3);
                assert!(
                    (after_units - before_units * k.powi(3)).abs()
                        <= 1e-6 * after_units.abs().max(1e-9),
                    "{}: #{}",
                    name(&path),
                    product.0
                );
                total.0 += va;
                total.1 += vb;
                products += 1;
            }
            *models
                .entry(original.header().schema.join(","))
                .or_default() += 1;
        }
        eprintln!(
            "meshed products: {products}, models per release: {models:?}, total volume \
             {total:?} m3, {unstable} ill-conditioned products skipped"
        );
        assert!(products >= 50, "{products} products compared");
        for release in ["IFC2X3", "IFC4", "IFC4X3_ADD2"] {
            assert!(models.get(release).copied().unwrap_or(0) >= 3, "{models:?}");
        }
    }
}
