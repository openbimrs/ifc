//! The XSD writer over the committed fixture corpus: write, read back with
//! the XSD reader, compare with the source model.
//!
//! Every IFC4 and IFC4X3 ADD2 fixture under `test/fixtures` is read with the
//! STEP codec, written with [`XmlCodec::xsd`] for its release and read back
//! with the same codec. The read-back model must be the source model:
//!
//! - the same entities, numbered in the source's model order (the reader
//!   numbers in document order, and the writer writes in model order);
//! - the same type names and attribute values, reals bit for bit;
//! - `SET` and `BAG` values as multisets, because the configuration writes
//!   some sets (`IfcRelDefinesByType.RelatedObjects`) through the inverse
//!   elements of their members, which read back in document order;
//! - the same header, except the STEP implementation level, which ifcXML
//!   has no field for.
//!
//! A fixture the writer refuses must be in [`REFUSED`] with the refusal it
//! gives, so a refusal cannot appear, change or disappear unnoticed. Every
//! refusal is a typed [`XmlError`]. A missing or shrunken corpus fails.
#![cfg(feature = "schema")]

#[path = "support/xsd_models.rs"]
#[allow(dead_code)]
mod support;

use ifc_model::{Codec, Model};
use ifc_schema::{Schema, SchemaVersion};
use ifc_step::StepCodec;
use ifc_xml::{XmlCodec, XmlError, XmlProfile};
use std::path::Path;
use std::sync::Arc;
use support::same_model;

const FIXTURE_ROOT: &str = "../../test/fixtures";

/// IFC4 and IFC4X3 ADD2 fixtures expected at least.
const MIN_DOCUMENTS: usize = 34;

/// Fixtures the writer refuses, as `(fixture, text the refusal contains,
/// why the configuration cannot carry the model)`.
const REFUSED: &[(&str, &str, &str)] = &[
    (
        "costing/costing_schedule.ifc",
        "`IFCFUTURESUSTAINABILITYMETRIC` is not an entity the schema declares",
        "an entity IFC4 does not declare has no element in the XSD",
    ),
    (
        "ifclite-geometry/nested_mapped_item.ifc",
        "0 items in a SET OF IfcRepresentationItem declared with [1:?] items",
        "an empty IfcShapeRepresentation.Items, which the XSD requires one item in",
    ),
    (
        "ifclite-geometry/nested_mapped_item_cycle.ifc",
        "an unset `MappingTarget`",
        "an unset mandatory IfcMappedItem.MappingTarget, whose element the XSD requires",
    ),
    (
        "ifcopenshell-validate/fail-header-wrong-type-list.ifc",
        "2 header `organization` entries",
        "two header organizations; the XSD header holds one",
    ),
    (
        "ifcopenshell-validate/fail-invalid-selected-simple-type.ifc",
        "declared IfcPositiveLengthMeasure (REAL), found a string",
        "a string typed as a length measure",
    ),
    (
        "ifcopenshell-validate/fail-nil-as-derived.ifc",
        "a derived (*) value for the explicit attribute `Prefix`",
        "`*` in an explicit attribute, which only a derived one can hold",
    ),
    (
        "nurbs/invalid_abstract_base_splines.ifc",
        "`IfcBSplineCurve` is abstract",
        "instances of an abstract entity, which the XSD declares abstract",
    ),
];

#[test]
fn the_fixture_corpus_round_trips_through_the_xsd_configuration() {
    let mut failures = Vec::new();
    let mut written = 0;
    let mut refused = Vec::new();
    for (name, source, profile) in corpus() {
        let codec = XmlCodec::xsd(schema(profile), profile);
        let bytes = match ifc_xml::writer::write(&codec, &source) {
            Ok(bytes) => bytes,
            Err(error) => {
                refused.push(name.clone());
                match REFUSED.iter().find(|(fixture, _, _)| *fixture == name) {
                    Some((_, expected, _)) if error.to_string().contains(expected) => {}
                    _ => failures.push(format!("{name}: refused: {error}")),
                }
                continue;
            }
        };
        written += 1;
        if REFUSED.iter().any(|(fixture, _, _)| *fixture == name) {
            failures.push(format!("{name}: listed in REFUSED but now written"));
        }
        let read = match ifc_xml::reader::read(&codec, &bytes) {
            Ok(model) => model,
            Err(error) => {
                failures.push(format!(
                    "{name}: the XSD reader refuses the output: {error}"
                ));
                continue;
            }
        };
        if let Err(difference) = same_model(codec.schema().expect("schema"), &source, &read) {
            failures.push(format!("{name}: {difference}"));
        }
    }
    println!(
        "{written} documents written and read back equal, {} refused",
        refused.len()
    );
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        written + refused.len() >= MIN_DOCUMENTS,
        "only {} IFC4/IFC4X3 fixtures",
        written + refused.len()
    );
    assert!(
        written * 4 >= (written + refused.len()) * 3,
        "the writer refuses {} of {} fixtures",
        refused.len(),
        written + refused.len()
    );
}

#[test]
fn every_refusal_names_a_committed_fixture_with_a_reason() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_ROOT);
    for (fixture, expected, reason) in REFUSED {
        assert!(root.join(fixture).is_file(), "{fixture}: not present");
        assert!(
            !expected.is_empty() && !reason.trim().is_empty(),
            "{fixture}"
        );
    }
}

/// Every refusal is typed: an [`XmlError`] the caller can match.
#[test]
fn refusals_are_typed() {
    for (name, source, profile) in corpus() {
        let codec = XmlCodec::xsd(schema(profile), profile);
        if let Err(error) = ifc_xml::writer::write(&codec, &source) {
            assert!(
                matches!(
                    error.root_cause(),
                    XmlError::Unrepresentable { .. }
                        | XmlError::TypeMismatch { .. }
                        | XmlError::InvalidScalar { .. }
                        | XmlError::UnknownEntity { .. }
                        | XmlError::AbstractEntity { .. }
                        | XmlError::UnresolvedReference { .. }
                ),
                "{name}: {error:?}"
            );
            assert!(error.path().is_some(), "{name}: refusal without a path");
        }
    }
}

fn schema(profile: XmlProfile) -> Arc<Schema> {
    Arc::new(
        ifc_schema::for_version(profile.version())
            .unwrap_or_else(|refused| panic!("{refused}"))
            .clone(),
    )
}

/// Every IFC4 and IFC4X3 ADD2 fixture, with its profile.
fn corpus() -> Vec<(String, Model, XmlProfile)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_ROOT);
    assert!(
        root.is_dir(),
        "fixture corpus missing at {}",
        root.display()
    );
    let mut paths = Vec::new();
    let mut pending = vec![root.clone()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("read fixture directory") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ifc"))
            {
                paths.push(path);
            }
        }
    }
    paths.sort();
    let mut out = Vec::new();
    for path in paths {
        let name = path
            .strip_prefix(&root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = std::fs::read(&path).expect("read fixture");
        let model = StepCodec
            .read_bytes(&bytes)
            .unwrap_or_else(|error| panic!("{name}: STEP read failed: {error}"));
        let profile = match model
            .header()
            .schema_token()
            .and_then(SchemaVersion::from_header_token)
        {
            Some(SchemaVersion::Ifc4) if model.header().schema_token() == Some("IFC4") => {
                XmlProfile::Ifc4Add2Tc1
            }
            Some(SchemaVersion::Ifc4x3) if model.header().schema_token() == Some("IFC4X3_ADD2") => {
                XmlProfile::Ifc4x3Add2
            }
            _ => continue,
        };
        out.push((name, model, profile));
    }
    out
}
