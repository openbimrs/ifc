//! `PropertyIndex` answers exactly what the per-query functions answer
//! (#352), results and refusals alike.
//!
//! Two corpora: every committed fixture under `test/fixtures`, and models
//! built here for IFC2X3, IFC4 and IFC4X3 that carry occurrence property
//! sets, quantity sets, type-inherited sets and occurrence overrides, each
//! also perturbed with the malformed or ambiguous assignments the resolver
//! refuses, placed both before and after the well-formed relationships so
//! the order in which refusals are met is exercised. Every entity of every
//! model is queried, plus an id the model does not contain.

use std::path::{Path, PathBuf};

use ifc_model::{Codec, EntityId, Model};
use ifc_properties::{
    exact_predefined_sets, exact_properties, exact_properties_where, exact_property,
    exact_property_sets_where, exact_schema, PropertyIndex,
};
use ifc_schema::{for_version, Schema, SchemaVersion};
use ifc_step::StepCodec;

/// The property lookups asked of every entity.
const LOOKUPS: &[(Option<&str>, &str)] = &[
    (Some("Pset_WallCommon"), "IsExternal"),
    (Some("Pset_WallCommon"), "FireRating"),
    (None, "FireRating"),
    (None, "LoadBearing"),
    (Some("Qto_WallBaseQuantities"), "Length"),
    (None, "NoSuchProperty"),
];

/// Every query of `model`, through the index and through the functions.
fn assert_equivalent(label: &str, model: &Model) -> usize {
    let index = PropertyIndex::build(model);
    assert!(std::ptr::eq(index.model(), model));
    assert_eq!(index.schema(), exact_schema(model), "{label}");
    let mut ids: Vec<EntityId> = model.ids().collect();
    ids.push(EntityId(u64::from(u32::MAX)));
    for &id in &ids {
        let at = || format!("{label} #{}", id.0);
        assert_eq!(
            index.exact_properties(id),
            exact_properties(model, id),
            "{}",
            at()
        );
        for (set, name) in LOOKUPS {
            assert_eq!(
                index.exact_property(id, *set, name),
                exact_property(model, id, *set, name),
                "{} {set:?}.{name}",
                at()
            );
        }
        assert_eq!(
            index.exact_properties_where(id, |s| s.starts_with("Pset_"), |p| p != "Reference"),
            exact_properties_where(model, id, |s| s.starts_with("Pset_"), |p| p != "Reference"),
            "{}",
            at()
        );
        assert_eq!(
            index.exact_property_sets_where(id, |_| true),
            exact_property_sets_where(model, id, |_| true),
            "{}",
            at()
        );
        assert_eq!(
            index.exact_property_sets_where(id, |s| s.starts_with("Qto_")),
            exact_property_sets_where(model, id, |s| s.starts_with("Qto_")),
            "{}",
            at()
        );
        for entity in ["IfcDoorLiningProperties", "IfcPropertySet"] {
            assert_eq!(
                index.exact_predefined_sets(id, entity),
                exact_predefined_sets(model, id, entity),
                "{} {entity}",
                at()
            );
        }
    }
    ids.len()
}

fn fixtures(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("fixture directory") {
        let path = entry.expect("fixture entry").path();
        if path.is_dir() {
            fixtures(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ifc"))
        {
            out.push(path);
        }
    }
}

#[test]
fn every_committed_fixture_answers_alike_through_the_index() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures");
    let mut paths = Vec::new();
    fixtures(&root, &mut paths);
    paths.sort();
    assert!(paths.len() >= 40, "the fixture corpus: {}", paths.len());
    let mut releases = std::collections::BTreeSet::new();
    let mut queried = 0;
    for path in &paths {
        let bytes = std::fs::read(path).expect("fixture reads");
        // Leniently, so a fixture with skipped records is compared too: the
        // index must refuse it exactly as the functions do.
        let Ok(model) = StepCodec::lenient().read_bytes(&bytes) else {
            continue;
        };
        if let Ok(version) = exact_schema(&model) {
            releases.insert(format!("{version:?}"));
        }
        queried += assert_equivalent(&path.display().to_string(), &model);
    }
    assert!(queried > 1000, "{queried} entities queried");
    for release in ["Ifc2x3", "Ifc4", "Ifc4x3"] {
        assert!(releases.contains(release), "{release} in {releases:?}");
    }
}

/// STEP records built against one release's table, every slot the record
/// does not name written `$`, so arities are always the release's.
struct Records {
    schema: &'static Schema,
    lines: Vec<String>,
}

impl Records {
    fn new(version: SchemaVersion) -> Self {
        Self {
            schema: for_version(version).expect("bundled"),
            lines: Vec::new(),
        }
    }

    fn put(&mut self, id: u64, entity: &str, slots: &[(usize, String)]) {
        let count = self.schema.attribute_count(entity);
        assert!(count > 0, "{entity} is declared");
        let mut values = vec!["$".to_owned(); count];
        values[0] = format!("'{id:0>22}'");
        for (slot, value) in slots {
            values[*slot] = value.clone();
        }
        self.lines.push(format!(
            "#{id}={}({});",
            entity.to_ascii_uppercase(),
            values.join(",")
        ));
    }

    fn raw(&mut self, line: String) {
        self.lines.push(line);
    }

    fn model(&self, token: &str) -> Model {
        let text = format!(
            "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
             FILE_NAME('','',(''),(''),'','','');\nFILE_SCHEMA(('{token}'));\nENDSEC;\n\
             DATA;\n{}\nENDSEC;\nEND-ISO-10303-21;\n",
            self.lines.join("\n")
        );
        StepCodec
            .read_bytes(text.as_bytes())
            .unwrap_or_else(|e| panic!("model parses: {e:?}\n{text}"))
    }
}

fn refs(ids: &[u64]) -> String {
    let refs: Vec<String> = ids.iter().map(|id| format!("#{id}")).collect();
    format!("({})", refs.join(","))
}

fn text(value: &str) -> String {
    format!("'{value}'")
}

/// Six walls and two wall types. Walls 1-3 are typed by type 100, 4-5 by
/// 200, 6 untyped. Each type holds `Pset_WallCommon`; every wall has its
/// own `Pset_WallCommon` (walls 1 and 4 override `IsExternal` and
/// `FireRating`) and walls 1-4 a `Qto_WallBaseQuantities`.
fn building(records: &mut Records, version: SchemaVersion) {
    let length_value = records.schema.attribute_count("IfcQuantityLength") - 1;
    let length_value = if version == SchemaVersion::Ifc2x3 {
        length_value
    } else {
        // IFC4 appends `Formula` after `LengthValue`.
        length_value - 1
    };
    let mut next = 1000;
    let mut id = || {
        next += 1;
        next
    };
    for (type_id, rating) in [(100, "REI60"), (200, "REI90")] {
        let fire = id();
        let external = id();
        let set = id();
        records.put(
            fire,
            "IfcPropertySingleValue",
            &[
                (0, text("FireRating")),
                (2, format!("IFCLABEL('{rating}')")),
            ],
        );
        records.put(
            external,
            "IfcPropertySingleValue",
            &[(0, text("IsExternal")), (2, "IFCBOOLEAN(.F.)".into())],
        );
        records.put(
            set,
            "IfcPropertySet",
            &[(2, text("Pset_WallCommon")), (4, refs(&[fire, external]))],
        );
        records.put(
            type_id,
            "IfcWallType",
            &[
                (2, text(&format!("Type {type_id}"))),
                (5, refs(&[set])),
                (9, ".STANDARD.".into()),
            ],
        );
    }
    for wall in 1..=6u64 {
        records.put(wall, "IfcWall", &[(2, text(&format!("Wall {wall}")))]);
        let bearing = id();
        let mut members = vec![bearing];
        records.put(
            bearing,
            "IfcPropertySingleValue",
            &[(0, text("LoadBearing")), (2, "IFCBOOLEAN(.T.)".into())],
        );
        if wall == 1 || wall == 4 {
            let external = id();
            let fire = id();
            records.put(
                external,
                "IfcPropertySingleValue",
                &[(0, text("IsExternal")), (2, "IFCBOOLEAN(.T.)".into())],
            );
            records.put(
                fire,
                "IfcPropertySingleValue",
                &[(0, text("FireRating")), (2, "IFCLABEL('REI120')".into())],
            );
            members.extend([external, fire]);
        }
        let set = id();
        records.put(
            set,
            "IfcPropertySet",
            &[(2, text("Pset_WallCommon")), (4, refs(&members))],
        );
        let rel = id();
        records.put(
            rel,
            "IfcRelDefinesByProperties",
            &[(4, refs(&[wall])), (5, format!("#{set}"))],
        );
        if wall <= 4 {
            let length = id();
            let quantities = id();
            records.put(
                length,
                "IfcQuantityLength",
                &[(0, text("Length")), (length_value, format!("{wall}.5"))],
            );
            records.put(
                quantities,
                "IfcElementQuantity",
                &[(2, text("Qto_WallBaseQuantities")), (5, refs(&[length]))],
            );
            let rel = id();
            records.put(
                rel,
                "IfcRelDefinesByProperties",
                &[(4, refs(&[wall])), (5, format!("#{quantities}"))],
            );
        }
    }
    records.put(
        300,
        "IfcRelDefinesByType",
        &[(4, refs(&[1, 2, 3])), (5, "#100".into())],
    );
    records.put(
        301,
        "IfcRelDefinesByType",
        &[(4, refs(&[4, 5])), (5, "#200".into())],
    );
}

/// A refusal the resolver must meet, as STEP records.
fn perturbation(records: &mut Records, version: SchemaVersion, kind: &str, base: u64) {
    match kind {
        // Wall 2 typed twice.
        "second type" => records.put(
            base,
            "IfcRelDefinesByType",
            &[(4, refs(&[2])), (5, "#200".into())],
        ),
        // `RelatedObjects` empty, violating `SET [1:?]`.
        "empty related" => records.put(
            base,
            "IfcRelDefinesByType",
            &[(4, "()".into()), (5, "#100".into())],
        ),
        // A type object in `RelatedObjects` of the property relationship.
        "type object related" => records.put(
            base,
            "IfcRelDefinesByProperties",
            &[(4, refs(&[200])), (5, "#1003".into())],
        ),
        // A definition missing from the file.
        "missing definition" => records.put(
            base,
            "IfcRelDefinesByProperties",
            &[(4, refs(&[3])), (5, "#99999".into())],
        ),
        // A relationship whose `RelatingType` is no type object.
        "untyped type" => records.put(
            base,
            "IfcRelDefinesByType",
            &[(4, refs(&[6])), (5, "#1".into())],
        ),
        // A record of the wrong arity on the relationship path.
        "short wall" => {
            records.raw(format!("#{base}=IFCWALL('{base:0>22}',$,'Short',$);"));
            records.put(
                base + 1,
                "IfcRelDefinesByType",
                &[(4, refs(&[base])), (5, "#100".into())],
            );
        }
        // IFC2X3's override relationship, a proper subtype, relating wall 5.
        "overrides" if version == SchemaVersion::Ifc2x3 => records.put(
            base,
            "IfcRelOverridesProperties",
            &[(4, refs(&[5])), (5, "#1003".into()), (6, refs(&[1001]))],
        ),
        // A proper subtype with a malformed `RelatedObjects`.
        "malformed overrides" if version == SchemaVersion::Ifc2x3 => records.put(
            base,
            "IfcRelOverridesProperties",
            &[(4, "$".into()), (5, "#1003".into()), (6, refs(&[1001]))],
        ),
        _ => {}
    }
}

const PERTURBATIONS: &[&str] = &[
    "none",
    "second type",
    "empty related",
    "type object related",
    "missing definition",
    "untyped type",
    "short wall",
    "overrides",
    "malformed overrides",
];

#[test]
fn built_models_answer_alike_for_every_release_and_refusal() {
    for (version, token) in [
        (SchemaVersion::Ifc2x3, "IFC2X3"),
        (SchemaVersion::Ifc4, "IFC4"),
        (SchemaVersion::Ifc4x3, "IFC4X3_ADD2"),
    ] {
        let clean = {
            let mut records = Records::new(version);
            building(&mut records, version);
            records.model(token)
        };
        // The clean model resolves: an inherited value, an override, a
        // quantity, and an untyped wall.
        let index = PropertyIndex::build(&clean);
        for wall in 1..=6 {
            let entries = index.exact_properties(EntityId(wall)).expect("resolves");
            assert!(!entries.is_empty(), "{token} wall {wall}");
        }
        assert_equivalent(token, &clean);
        for kind in PERTURBATIONS {
            for first in [true, false] {
                // A low id is listed before the well-formed relationships, a
                // high one after them.
                let mut records = Records::new(version);
                if first {
                    perturbation(&mut records, version, kind, 10);
                }
                building(&mut records, version);
                if !first {
                    perturbation(&mut records, version, kind, 5000);
                }
                let model = records.model(token);
                assert_equivalent(&format!("{token} {kind} first={first}"), &model);
            }
        }
        // Two refusals in one file, in both orders: the one the per-object
        // traversal meets first must win through the index too, whether it
        // concerns one object (a second type, an override) or the file.
        //
        // The expected refusals pin the order the per-object traversal
        // applies, so a change to it cannot pass by changing both paths:
        // a relationship subtype relating the object, then the first
        // malformed subtype, then the first malformed
        // `IfcRelDefinesByProperties`, then a second type assignment met
        // before the first malformed `IfcRelDefinesByType`, then that.
        let ifc2x3 = version == SchemaVersion::Ifc2x3;
        for (early, late, wall_2, wall_5) in [
            (
                "second type",
                "empty related",
                "MultipleTypeAssignments",
                "MalformedAggregate",
            ),
            (
                "empty related",
                "second type",
                "MalformedAggregate",
                "MalformedAggregate",
            ),
            (
                "second type",
                "missing definition",
                "MissingReference",
                "MissingReference",
            ),
            (
                "overrides",
                "malformed overrides",
                if ifc2x3 { "MalformedAggregate" } else { "Ok" },
                if ifc2x3 {
                    "UnsupportedRelationship"
                } else {
                    "Ok"
                },
            ),
            (
                "malformed overrides",
                "overrides",
                if ifc2x3 { "MalformedAggregate" } else { "Ok" },
                if ifc2x3 { "MalformedAggregate" } else { "Ok" },
            ),
            (
                "overrides",
                "second type",
                "MultipleTypeAssignments",
                if ifc2x3 {
                    "UnsupportedRelationship"
                } else {
                    "Ok"
                },
            ),
        ] {
            let mut records = Records::new(version);
            perturbation(&mut records, version, early, 10);
            building(&mut records, version);
            perturbation(&mut records, version, late, 5000);
            let model = records.model(token);
            let label = format!("{token} {early} then {late}");
            let index = PropertyIndex::build(&model);
            for (wall, expected) in [(2, wall_2), (5, wall_5)] {
                let answer = index.exact_properties(EntityId(wall));
                assert_eq!(
                    outcome(&answer),
                    expected,
                    "{label} wall {wall}: {answer:?}"
                );
            }
            assert_equivalent(&label, &model);
        }
    }
}

/// `Ok`, or the refusal's variant name.
fn outcome<T>(answer: &Result<T, ifc_properties::ExactPropertyError>) -> String {
    match answer {
        Ok(_) => "Ok".into(),
        Err(error) => format!("{error:?}")
            .split([' ', '{', '('])
            .next()
            .unwrap_or_default()
            .to_owned(),
    }
}

#[test]
fn a_model_refused_as_a_whole_is_refused_by_every_query() {
    let mut records = Records::new(SchemaVersion::Ifc4);
    building(&mut records, SchemaVersion::Ifc4);
    for token in ["IFC4X1", "IFC5"] {
        let model = records.model(token);
        assert!(PropertyIndex::build(&model).schema().is_err());
        assert_equivalent(token, &model);
    }
    let empty = Model::new();
    assert_equivalent("no schema", &empty);
}
