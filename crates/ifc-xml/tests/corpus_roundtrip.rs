//! Differential round trip over the committed fixture corpus.
//!
//! Every `.ifc` under `test/fixtures` goes STEP -> ifcXML -> Model -> STEP,
//! and each stage must reproduce the entity graph the STEP reader produced:
//! same ids, same type names, same attributes, reals compared bit for bit.
//! Hand-built models only exercise what their author thought of; the corpus
//! carries real exporter output (units, nested lists, typed selects, unknown
//! entities, hostile strings), which is where a codec's value-kind inference
//! actually gets tested.
//!
//! Three codec configurations are exercised, because they serialize the same
//! slot differently: positional `a<i>` names (the default), schema-backed
//! names from the file's declared schema, and the strict IFC4 release profile
//! for IFC4 files. Schema-backed names are the configuration that exposed a
//! slot-order bug: scalars are XML attributes and structured values child
//! elements, so document order is not slot order.
//!
//! A missing or shrunken corpus fails the test rather than skipping it.

use ifc_model::{Codec, Model, Value};
use ifc_step::StepCodec;
use ifc_xml::XmlCodec;
use std::path::{Path, PathBuf};

/// The corpus root, relative to this crate.
const FIXTURE_ROOT: &str = "../../test/fixtures";

/// Fewer files than this means the corpus moved or was pruned, and a
/// differential test over a handful of files would pass while proving little.
const MIN_FIXTURES: usize = 40;

/// Fixtures excluded from the round trip, each with the reason it cannot
/// take part, as `(path relative to FIXTURE_ROOT, justification)`.
///
/// Currently empty: the STEP reader accepts every committed fixture,
/// including the `fail-*` validation cases (their defects are schema-level,
/// not syntactic) and `hostile/`. An entry here must still fail the STEP read,
/// so an exclusion cannot outlive its reason.
const EXCLUDED: &[(&str, &str)] = &[];

#[test]
fn every_committed_fixture_round_trips_through_ifcxml() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_ROOT);
    assert!(
        root.is_dir(),
        "fixture corpus missing at {}; this test fails rather than skips",
        root.display()
    );
    let fixtures = collect_fixtures(&root);
    assert!(
        fixtures.len() >= MIN_FIXTURES,
        "found {} fixtures under {}, expected at least {MIN_FIXTURES}",
        fixtures.len(),
        root.display()
    );

    let mut failures = Vec::new();
    let mut round_tripped = 0;
    for path in &fixtures {
        let relative = relative_name(&root, path);
        let bytes = std::fs::read(path).expect("read fixture");
        let step = StepCodec.read_bytes(&bytes);

        if let Some((_, reason)) = EXCLUDED.iter().find(|(name, _)| *name == relative) {
            assert!(
                step.is_err(),
                "{relative} is excluded ({reason}) but now reads; remove the exclusion"
            );
            continue;
        }
        let source = match step {
            Ok(model) => model,
            Err(error) => {
                failures.push(format!("{relative}: STEP read failed: {error}"));
                continue;
            }
        };
        for (label, codec) in configurations(&source) {
            if let Err(detail) = round_trip(&source, &codec) {
                failures.push(format!("{relative} [{label}]: {detail}"));
            }
        }
        round_tripped += 1;
    }

    assert!(
        failures.is_empty(),
        "{} round-trip failures across {} fixtures:\n{}",
        failures.len(),
        fixtures.len(),
        failures.join("\n")
    );
    assert_eq!(round_tripped + EXCLUDED.len(), fixtures.len());
}

/// Excluded entries must name real files, or they silently exclude nothing.
#[test]
fn every_exclusion_names_a_committed_fixture() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_ROOT);
    for (name, reason) in EXCLUDED {
        assert!(
            !reason.trim().is_empty(),
            "{name}: exclusion without reason"
        );
        assert!(
            root.join(name).is_file(),
            "{name}: excluded but not present"
        );
    }
}

/// The codec configurations a fixture is round-tripped through.
///
/// Without the `schema` feature only positional names exist.
#[cfg(not(feature = "schema"))]
fn configurations(_: &Model) -> Vec<(&'static str, XmlCodec)> {
    vec![("positional", XmlCodec::default())]
}

/// The codec configurations a fixture is round-tripped through.
#[cfg(feature = "schema")]
fn configurations(source: &Model) -> Vec<(&'static str, XmlCodec)> {
    use ifc_schema::SchemaVersion;
    use ifc_xml::XmlProfile;
    use std::sync::Arc;

    let mut codecs = vec![("positional", XmlCodec::default())];
    let token = source.header().schema_token().unwrap_or_default();
    let version = SchemaVersion::from_header_token(token)
        .unwrap_or_else(|| panic!("fixture declares unrecognised schema {token:?}"));
    let schema = ifc_schema::for_version(version).unwrap_or_else(|refused| panic!("{refused}"));
    codecs.push(("schema", XmlCodec::with_schema(Arc::new(schema.clone()))));
    if token == XmlProfile::Ifc4Add2Tc1.schema_token() {
        codecs.push((
            "strict",
            XmlCodec::with_schema_and_profile(Arc::new(schema.clone()), XmlProfile::Ifc4Add2Tc1),
        ));
    }
    codecs
}

/// STEP model -> XML -> model -> STEP -> model, comparing at each stage.
fn round_trip(source: &Model, codec: &XmlCodec) -> Result<(), String> {
    let xml = codec
        .write_bytes(source)
        .map_err(|error| format!("XML write: {error}"))?;
    let from_xml = codec
        .read_bytes(&xml)
        .map_err(|error| format!("XML read: {error}"))?;
    compare(source, &from_xml).map_err(|detail| format!("STEP -> XML: {detail}"))?;
    if from_xml.header().schema != source.header().schema {
        return Err(format!(
            "schema header {:?} became {:?}",
            source.header().schema,
            from_xml.header().schema
        ));
    }

    // The writer is deterministic: re-writing what was read is byte-identical.
    let rewritten = codec
        .write_bytes(&from_xml)
        .map_err(|error| format!("XML rewrite: {error}"))?;
    if rewritten != xml {
        return Err("re-writing the read model changed the XML".into());
    }

    let step = StepCodec
        .write_bytes(&from_xml)
        .map_err(|error| format!("STEP write: {error}"))?;
    let back = StepCodec
        .read_bytes(&step)
        .map_err(|error| format!("STEP re-read: {error}"))?;
    compare(source, &back).map_err(|detail| format!("STEP -> XML -> STEP: {detail}"))
}

/// Identical entity graphs: ids, type names and attributes.
fn compare(expected: &Model, actual: &Model) -> Result<(), String> {
    if expected.len() != actual.len() {
        return Err(format!(
            "{} entities became {}",
            expected.len(),
            actual.len()
        ));
    }
    for (id, entity) in expected.iter() {
        let other = actual.get(id).ok_or_else(|| format!("{id} missing"))?;
        if entity.type_name != other.type_name {
            return Err(format!(
                "{id} type {} became {}",
                entity.type_name, other.type_name
            ));
        }
        let same = entity.attributes.len() == other.attributes.len()
            && entity
                .attributes
                .iter()
                .zip(&other.attributes)
                .all(|(left, right)| identical(left, right));
        if !same {
            return Err(format!(
                "{id} {}\n  expected {:?}\n  found    {:?}",
                entity.type_name, entity.attributes, other.attributes
            ));
        }
    }
    Ok(())
}

/// Structural equality with reals compared bit for bit, so a sign-of-zero or
/// last-digit drift is a failure rather than an `f64 ==` coincidence.
fn identical(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Real(left), Value::Real(right)) => left.to_bits() == right.to_bits(),
        (Value::List(left), Value::List(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(l, r)| identical(l, r))
        }
        (
            Value::Typed {
                type_name: left_type,
                value: left,
            },
            Value::Typed {
                type_name: right_type,
                value: right,
            },
        ) => left_type == right_type && identical(left, right),
        (left, right) => left == right,
    }
}

fn collect_fixtures(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("read fixture directory") {
            let path = entry.expect("fixture directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ifc"))
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

fn relative_name(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
