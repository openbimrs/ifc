//! Property and quantity set names using a reserved prefix (#511).
//!
//! buildingSMART reserves the `Pset_` and `Qto_` prefixes for the sets its
//! PSD/QTO catalog defines. A custom `IfcPropertySet` named
//! `Pset_FRS_BASt` is legal EXPRESS, so no schema rule catches it, but the
//! bSI validation service rejects it (rule PSE001) and every tool reading
//! `Pset_` names as catalog sets misreads it.
//!
//! The check is a [`Severity::Warning`](crate::Severity::Warning): the file
//! is legal, and very likely wrong. It needs the release's catalog, which
//! this crate does not link (the catalog belongs to `ifc-template-catalog`,
//! and the facade composes the two); a caller supplies it as a
//! [`PropertySetCatalog`]. Without one, a file using reserved names gets a
//! single unsupported finding saying they were not checked, never silence.
//!
//! For a set the catalog defines, the members inside it are checked too
//! (#537): properties and quantities the template does not
//! declare, and members whose entity form or value types differ from it.
//! The catalog supplies the template through
//! [`PropertySetCatalog::template`]; without one, those sets' members are
//! reported once per rule as not checked.
//!
//! The prefix is matched without regard to case (`PSET_x` is reserved
//! too); the catalog lookup is exact.

mod members;
mod template;

pub use template::{
    CatalogMember, CatalogMemberForm, CatalogQuantityKind, CatalogSet, CatalogSetKind,
};

use ifc_model::{EntityId, Model};
use ifc_schema::{Schema, SchemaVersion};

use crate::report::{Finding, FindingDetail, Path, Report, ReservedSetName};

/// The rule id of this check.
const RULE: &str = "property.reserved_set_name";

/// The prefixes buildingSMART reserves.
const RESERVED: [&str; 2] = ["Pset_", "Qto_"];

/// The set names a release's PSD/QTO catalog defines.
///
/// Implemented by the facade over `ifc-template-catalog`; any other source
/// of the catalog may implement it too.
pub trait PropertySetCatalog {
    /// Whether the catalog of `version` defines a property or quantity set
    /// named exactly `name`; `None` when no catalog of that release is
    /// available here.
    fn defines(&self, version: SchemaVersion, name: &str) -> Option<bool>;

    /// The members the catalog of `version` declares for the set named
    /// exactly `name`; `None` when it gives no member templates for it.
    ///
    /// The default gives none, so a catalog that only knows names still
    /// works: its sets' members are reported once as not checked (#537).
    fn template(&self, version: SchemaVersion, name: &str) -> Option<CatalogSet> {
        let _ = (version, name);
        None
    }
}

/// Warns on every set whose reserved-prefix name the catalog lacks.
pub(crate) fn check(
    model: &Model,
    schema: &Schema,
    catalog: Option<&dyn PropertySetCatalog>,
    report: &mut Report,
) {
    let sets = reserved_sets(model, schema);
    if sets.is_empty() {
        return;
    }
    let release = schema
        .version()
        .map_or_else(|| schema.name().to_owned(), |v| v.release_id().to_owned());
    let lookup = |name: &str| {
        let version = schema.version()?;
        catalog?.defines(version, name)
    };
    // Whether any lookup is possible at all, asked once.
    if lookup(&sets[0].2).is_none() {
        let why = if catalog.is_none() {
            "no PSD/QTO catalog was supplied".to_owned()
        } else {
            format!("the supplied catalog has no {release} edition")
        };
        report.push(Finding::unsupported(
            RULE,
            Path::File,
            format!(
                "{} property or quantity set names use a reserved prefix (Pset_, Qto_) and were not \
                 checked: {why}",
                sets.len()
            ),
        ));
        return;
    }
    let mut defined = Vec::new();
    for (id, slot, name, prefix) in sets {
        let known = lookup(&name);
        if known == Some(true) {
            defined.push((id, name.clone()));
        }
        if known == Some(false) {
            report.push(
                Finding::warning(
                    RULE,
                    Path::Attribute {
                        entity: id,
                        index: slot,
                        name: Some("Name".into()),
                    },
                    format!(
                        "'{name}' uses the reserved prefix {prefix}, but the {release} catalog \
                         defines no set of that name; a custom set must not use Pset_ or Qto_ \
                         (bSI PSE001)"
                    ),
                )
                .with_detail(FindingDetail::ReservedSetName(ReservedSetName {
                    name,
                    prefix: prefix.to_owned(),
                    release: release.clone(),
                })),
            );
        }
    }
    if let Some(catalog) = catalog {
        members::check(model, schema, catalog, &release, &defined, report);
    }
}

/// Every `IfcPropertySet` and `IfcElementQuantity` whose name starts with
/// a reserved prefix: id, `Name` slot, name, prefix.
fn reserved_sets(model: &Model, schema: &Schema) -> Vec<(EntityId, usize, String, &'static str)> {
    let mut found = Vec::new();
    for (type_name, _) in model.type_histogram() {
        if !["IfcPropertySet", "IfcElementQuantity"]
            .iter()
            .any(|set| schema.is_a(type_name, set))
        {
            continue;
        }
        let Some(slot) = schema
            .attribute_names(type_name)
            .iter()
            .position(|name| *name == "Name")
        else {
            continue;
        };
        for (id, entity) in model.of_type(type_name) {
            let Some(name) = entity
                .attribute(slot)
                .and_then(|v| v.unwrap_typed().as_text())
            else {
                continue;
            };
            let prefix = RESERVED.iter().find(|prefix| {
                name.get(..prefix.len())
                    .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
            });
            if let Some(prefix) = prefix {
                found.push((id, slot, name.to_owned(), *prefix));
            }
        }
    }
    found.sort_by_key(|(id, ..)| *id);
    found
}

#[cfg(all(test, feature = "ifc4", feature = "ifc4x1"))]
mod tests {
    use super::*;
    use ifc_model::{Entity, Value};

    struct Fixed(&'static [&'static str]);

    impl PropertySetCatalog for Fixed {
        fn defines(&self, version: SchemaVersion, name: &str) -> Option<bool> {
            (version == SchemaVersion::Ifc4).then(|| self.0.contains(&name))
        }
    }

    fn model(names: &[&str]) -> Model {
        let mut model = Model::new();
        for name in names {
            model.push(Entity::new(
                "IFCPROPERTYSET",
                vec![
                    Value::Text("0abcdefghijklmnopqrstu".into()),
                    Value::Null,
                    Value::Text((*name).into()),
                    Value::Null,
                    Value::List(Vec::new()),
                ],
            ));
        }
        model
    }

    fn run(model: &Model, catalog: Option<&dyn PropertySetCatalog>) -> Report {
        let mut report = Report::new();
        check(model, ifc_schema::ifc4(), catalog, &mut report);
        report
    }

    #[test]
    fn a_reserved_name_outside_the_catalog_is_a_warning() {
        let catalog = Fixed(&["Pset_WallCommon"]);
        let report = run(
            &model(&["Pset_WallCommon", "Pset_FRS_BASt", "MyCompany_Set", "qto_x"]),
            Some(&catalog),
        );
        let warned: Vec<_> = report
            .findings()
            .iter()
            .filter(|f| f.rule == RULE)
            .map(|f| f.message.clone())
            .collect();
        assert_eq!(warned.len(), 2, "{warned:?}");
        assert!(warned[0].contains("Pset_FRS_BASt"));
        assert!(warned[1].contains("qto_x"));
        assert!(report.is_conformant(), "a warning is not a violation");
    }

    #[test]
    fn without_a_catalog_the_names_are_admitted_unchecked() {
        let report = run(&model(&["Pset_FRS_BASt", "Pset_WallCommon"]), None);
        assert_eq!(report.findings().len(), 1);
        assert_eq!(report.findings()[0].severity, crate::Severity::Unsupported);
        assert!(run(&model(&["MySet"]), None).findings().is_empty());
    }

    #[test]
    fn a_release_the_catalog_lacks_is_admitted_unchecked() {
        let catalog = Fixed(&[]);
        let mut report = Report::new();
        check(
            &model(&["Pset_FRS_BASt"]),
            ifc_schema::ifc4x1(),
            Some(&catalog),
            &mut report,
        );
        assert_eq!(report.findings().len(), 1);
        assert!(report.findings()[0].message.contains("IFC4X1"));
    }
}
