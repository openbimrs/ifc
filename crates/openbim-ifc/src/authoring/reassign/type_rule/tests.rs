use std::path::PathBuf;

use ifc_schema::{for_version, Schema, SchemaVersion};

use super::{occurrence_class, type_rule, NOT_TYPE_RULES, TYPE_RULES};

const RELEASES: [(SchemaVersion, &str); 5] = [
    (SchemaVersion::Ifc2x3, "ifc2x3-tc1/IFC2X3_TC1.exp"),
    (SchemaVersion::Ifc4, "ifc4-add2-tc1/IFC4.exp"),
    (SchemaVersion::Ifc4x1, "ifc4x1-final/IFC4x1.exp"),
    (SchemaVersion::Ifc4x2, "ifc4x2-final/IFC4x2.exp"),
    (SchemaVersion::Ifc4x3, "ifc4x3-add2/IFC4X3_ADD2.exp"),
];

/// Where the normative rule names `IFCTRANFORMERTYPE` for `IfcTransformer`.
const TRANSFORMER_TYPO: [SchemaVersion; 3] = [
    SchemaVersion::Ifc4,
    SchemaVersion::Ifc4x1,
    SchemaVersion::Ifc4x2,
];

fn spec_root() -> Option<PathBuf> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    [
        "../../../../references/ifc-spec",
        "../../references/ifc-spec",
    ]
    .into_iter()
    .map(|rel| crate_dir.join(rel))
    .find(|path| path.is_dir())
}

/// The normative schema of each release, with its bundled table.
fn releases() -> Option<Vec<(SchemaVersion, Schema, &'static Schema)>> {
    let Some(root) = spec_root() else {
        assert!(
            std::env::var_os("IFC_SPEC_REQUIRED").is_none(),
            "IFC_SPEC_REQUIRED is set but references/ifc-spec was not found; \
             run scripts/fetch-ifc-schemas.sh"
        );
        eprintln!("skipped: references/ifc-spec not present");
        return None;
    };
    Some(
        RELEASES
            .iter()
            .filter_map(|(version, file)| {
                let bytes = std::fs::read(root.join(file)).expect("read the reference schema");
                Some((
                    *version,
                    Schema::from_express_bytes(&bytes),
                    for_version(*version).ok()?,
                ))
            })
            .collect(),
    )
}

fn over_type_relationship(expression: &str) -> bool {
    expression.contains("IsTypedBy") || expression.contains("RelatingType")
}

/// The batch reads type rules by label and pairs classes by name; a rule
/// the labels or names miss would pass unchecked.
#[test]
fn the_labels_and_the_owner_type_convention_match_the_normative_schema() {
    let Some(releases) = releases() else { return };
    assert_eq!(releases.len(), RELEASES.len(), "every release is bundled");
    for (version, normative, _) in &releases {
        let name = normative.name().to_uppercase();
        let mut seen_not_type_rules = Vec::new();
        for entity in normative.entities() {
            for rule in &entity.where_rules {
                let labelled = TYPE_RULES.contains(&rule.label.as_str());
                let typed = over_type_relationship(&rule.expression);
                let at = format!("{version:?} {}.{}", entity.name, rule.label);
                // IFC2X3 `IfcObject.WR1` is the one rule over the type
                // relationship there; it is not a type-assignment rule.
                if entity.name == "IfcObject" && rule.label == "WR1" {
                    continue;
                }
                if typed {
                    assert!(labelled, "{at}: a rule over the type relationship");
                    let expected = format!("'{name}.{}TYPE'", entity.name.to_uppercase());
                    let transformer =
                        entity.name == "IfcTransformer" && TRANSFORMER_TYPO.contains(version);
                    if transformer {
                        assert!(rule.expression.contains("IFCTRANFORMERTYPE"), "{at}");
                    } else {
                        assert!(
                            rule.expression.to_uppercase().contains(&expected),
                            "{at}: names {expected}: {}",
                            rule.expression
                        );
                    }
                } else if labelled {
                    let key = (entity.name.as_str(), rule.label.as_str());
                    assert!(NOT_TYPE_RULES.contains(&key), "{at}: not over the type");
                    seen_not_type_rules.push(key);
                }
            }
        }
        let expected: Vec<_> = if *version == SchemaVersion::Ifc2x3 {
            vec![]
        } else {
            NOT_TYPE_RULES.to_vec()
        };
        assert_eq!(seen_not_type_rules, expected, "{version:?}: exceptions");
    }
}

#[test]
fn every_rule_owner_pairs_with_a_type_class_and_back() {
    let Some(releases) = releases() else { return };
    for (version, _, schema) in &releases {
        let mut owners = 0;
        for entity in schema.entities() {
            let own = entity.where_rules.iter().any(|rule| {
                TYPE_RULES.contains(&rule.label.as_str())
                    && !NOT_TYPE_RULES.contains(&(entity.name.as_str(), rule.label.as_str()))
            });
            if !own {
                continue;
            }
            owners += 1;
            let at = format!("{version:?} {}", entity.name);
            assert!(!entity.abstract_, "{at}: an abstract rule owner");
            let rule = type_rule(schema, &entity.name)
                .unwrap_or_else(|gap| panic!("{at}: {gap}"))
                .unwrap_or_else(|| panic!("{at}: no rule"));
            assert_eq!(rule.owner, entity.name, "{at}");
            assert_eq!(rule.type_class, format!("{}Type", entity.name), "{at}");
            let type_def = schema.entity(rule.type_class).expect("declared");
            assert!(!type_def.abstract_, "{at}: abstract {}", rule.type_class);
            assert!(schema.is_a(rule.type_class, "IfcTypeObject"), "{at}");
            assert_eq!(
                occurrence_class(schema, rule.type_class),
                Some(entity.name.as_str()),
                "{at}: round trip"
            );
        }
        assert_eq!(owners > 0, *version != SchemaVersion::Ifc2x3, "{version:?}");
    }
}

#[test]
fn a_nested_owner_has_a_type_class_below_its_ancestors() {
    let Some(releases) = releases() else { return };
    let schema = releases
        .iter()
        .find(|release| release.0 == SchemaVersion::Ifc4x3)
        .expect("IFC4X3")
        .2;
    for (nested, ancestor) in [
        ("IfcPile", "IfcDeepFoundation"),
        ("IfcCaissonFoundation", "IfcDeepFoundation"),
    ] {
        let nested_type = type_rule(schema, nested).unwrap().unwrap().type_class;
        let ancestor_type = type_rule(schema, ancestor).unwrap().unwrap().type_class;
        assert!(schema.is_a(nested_type, ancestor_type), "{nested}");
        assert_eq!(occurrence_class(schema, nested_type), Some(nested));
    }
}

#[test]
fn an_event_has_no_type_rule_and_styles_pair_with_nothing() {
    let ifc4 = for_version(SchemaVersion::Ifc4).unwrap();
    assert_eq!(type_rule(ifc4, "IfcEvent"), Ok(None));
    assert_eq!(type_rule(ifc4, "IfcTask"), Ok(None));
    let wall = type_rule(ifc4, "IfcWallStandardCase")
        .unwrap()
        .expect("a wall rule");
    assert_eq!(wall.name(), "IfcWall.CorrectTypeAssigned");
    assert_eq!(wall.type_class, "IfcWallType");
    assert_eq!(occurrence_class(ifc4, "IfcDoorStyle"), None);
    assert_eq!(occurrence_class(ifc4, "IfcWallType"), Some("IfcWall"));
    let ifc2x3 = for_version(SchemaVersion::Ifc2x3).unwrap();
    assert_eq!(type_rule(ifc2x3, "IfcWall"), Ok(None));
}
