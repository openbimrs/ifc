//! The members of catalog sets against their templates (#537).
//!
//! For every `IfcPropertySet` and `IfcElementQuantity` whose name the
//! catalog defines, the properties or quantities in it are compared with the
//! template ([`CatalogSet`]) the catalog gives for that name: the set's own
//! kind (bSI PSE001 scenario 2), each member's name (scenario 4), its entity
//! form and its value types (scenario 5). Every verdict is a
//! [`Severity::Warning`](crate::Severity::Warning).
//!
//! Names match exactly; a name differing only in case is still undeclared,
//! and the message says which spelling the template uses. A value type
//! passes when it is the declared one or a defined type whose alias chain
//! reaches it (`IfcPositiveLengthMeasure` for `IfcLengthMeasure`).
//!
//! Out of scope: the template's applicability and enumeration membership.
//! A set the catalog gives no template for is admitted once per rule id as
//! unsupported, never passed silently.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::{Schema, TypeKind};

use super::{CatalogMember, CatalogMemberForm, CatalogSet, CatalogSetKind, PropertySetCatalog};
use crate::report::{CatalogMemberDetail, CatalogMismatch, Finding, FindingDetail, Path, Report};

/// An undeclared member.
const UNDECLARED: &str = "property.undeclared_member";

/// A set kind, form or value type differing from the template.
const MEMBER_TYPE: &str = "property.member_type";

/// Complex properties nested deeper than this are not followed.
const MAX_DEPTH: usize = 32;

/// Alias steps followed from a value's type to the declared one.
const MAX_ALIAS_STEPS: usize = 16;

/// Checks the members of every set in `defined` (id and name, in id order)
/// against the catalog's templates.
pub(super) fn check(
    model: &Model,
    schema: &Schema,
    catalog: &dyn PropertySetCatalog,
    release: &str,
    defined: &[(EntityId, String)],
    report: &mut Report,
) {
    let Some(version) = schema.version() else {
        return;
    };
    let mut run = Run {
        model,
        schema,
        release,
        report,
        seen: HashSet::new(),
        unknown_types: BTreeSet::new(),
    };
    let mut templates: BTreeMap<&str, Option<CatalogSet>> = BTreeMap::new();
    let mut untemplated = 0usize;
    for (id, name) in defined {
        let template = templates
            .entry(name.as_str())
            .or_insert_with(|| catalog.template(version, name));
        match template {
            Some(template) => run.check_set(*id, name, template),
            None => untemplated += 1,
        }
    }
    if untemplated > 0 {
        for rule in [UNDECLARED, MEMBER_TYPE] {
            run.report.push(Finding::unsupported(
                rule,
                Path::File,
                format!(
                    "the members of {untemplated} catalog sets were not checked: the supplied \
                     catalog gives no member templates"
                ),
            ));
        }
    }
    if !run.unknown_types.is_empty() {
        let listed = run
            .unknown_types
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        run.report.push(Finding::unsupported(
            MEMBER_TYPE,
            Path::File,
            format!(
                "some declared value types are not in the {release} tables and were not checked: \
                 {listed}"
            ),
        ));
    }
}

/// The state of one run.
struct Run<'a> {
    model: &'a Model,
    schema: &'a Schema,
    release: &'a str,
    report: &'a mut Report,
    /// Findings already pushed, by rule, path and message.
    seen: HashSet<(String, Path, String)>,
    /// `{set}.{member}: {type}` for declared types the release lacks.
    unknown_types: BTreeSet<String>,
}

/// Where in which set a member sits.
struct Site<'a> {
    set: &'a str,
    /// What the set's members are called in messages.
    noun: &'static str,
}

impl Run<'_> {
    fn warn(
        &mut self,
        rule: &str,
        path: Path,
        message: String,
        set: &str,
        member: Option<&str>,
        mismatch: CatalogMismatch,
    ) {
        if !self
            .seen
            .insert((rule.to_owned(), path.clone(), message.clone()))
        {
            return;
        }
        self.report
            .push(
                Finding::warning(rule, path, message).with_detail(FindingDetail::CatalogMember(
                    CatalogMemberDetail {
                        set: set.to_owned(),
                        member: member.map(str::to_owned),
                        release: self.release.to_owned(),
                        mismatch,
                    },
                )),
            );
    }

    /// The schema's spelling of an entity type token.
    fn entity_name(&self, token: &str) -> String {
        self.schema
            .entity(token)
            .map_or_else(|| token.to_owned(), |entity| entity.name.clone())
    }

    /// The schema's spelling of a defined type token.
    fn type_name(&self, token: &str) -> String {
        self.schema
            .type_def(token)
            .map_or_else(|| token.to_owned(), |def| def.name.clone())
    }

    fn slot(&self, type_name: &str, attribute: &str) -> Option<usize> {
        self.schema
            .attribute_names(type_name)
            .iter()
            .position(|name| name.eq_ignore_ascii_case(attribute))
    }

    fn check_set(&mut self, id: EntityId, name: &str, template: &CatalogSet) {
        let Some(entity) = self.model.get(id) else {
            return;
        };
        let type_name: &str = &entity.type_name;
        let found_kind = if self.schema.is_a(type_name, "IfcElementQuantity") {
            CatalogSetKind::Quantity
        } else {
            CatalogSetKind::Property
        };
        let kind_entity = |kind: CatalogSetKind| match kind {
            CatalogSetKind::Quantity => "IfcElementQuantity",
            _ => "IfcPropertySet",
        };
        if found_kind != template.kind {
            let declared = kind_entity(template.kind).to_owned();
            let found = self.entity_name(type_name);
            self.warn(
                MEMBER_TYPE,
                Path::Entity(id),
                format!(
                    "'{name}' is an {found}, but the {} catalog declares it as an {declared} \
                     (bSI PSE001)",
                    self.release
                ),
                name,
                None,
                CatalogMismatch::SetKind { declared, found },
            );
            return;
        }
        let (attribute, noun) = match found_kind {
            CatalogSetKind::Quantity => ("Quantities", "quantity"),
            _ => ("HasProperties", "property"),
        };
        let Some(slot) = self.slot(type_name, attribute) else {
            return;
        };
        let site = Site { set: name, noun };
        let mut visited = HashSet::new();
        self.walk(
            entity.attribute(slot),
            &template.members,
            "",
            &site,
            0,
            &mut visited,
        );
    }

    /// Checks the members referenced by `list` against `declared`;
    /// `prefix` is the dotted path of the enclosing complex property.
    fn walk(
        &mut self,
        list: Option<&Value>,
        declared: &[CatalogMember],
        prefix: &str,
        site: &Site<'_>,
        depth: usize,
        visited: &mut HashSet<EntityId>,
    ) {
        if depth >= MAX_DEPTH {
            return;
        }
        let Some(Value::List(items)) = list.map(Value::unwrap_typed) else {
            return;
        };
        for item in items {
            let Some(member_id) = item.unwrap_typed().as_ref_id() else {
                continue;
            };
            if !visited.insert(member_id) {
                continue;
            }
            let Some(member) = self.model.get(member_id) else {
                continue;
            };
            self.check_member(member_id, member, declared, prefix, site, depth, visited);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_member(
        &mut self,
        id: EntityId,
        member: &Entity,
        declared: &[CatalogMember],
        prefix: &str,
        site: &Site<'_>,
        depth: usize,
        visited: &mut HashSet<EntityId>,
    ) {
        let type_name: &str = &member.type_name;
        let Some(name_slot) = self.slot(type_name, "Name") else {
            return;
        };
        let Some(name) = member
            .attribute(name_slot)
            .and_then(|value| value.unwrap_typed().as_text())
        else {
            return;
        };
        let dotted = format!("{prefix}{name}");
        let Some(template) = declared.iter().find(|candidate| candidate.name == name) else {
            let mut message = format!(
                "'{dotted}' is not a {} of {} in the {} catalog (bSI PSE001)",
                site.noun, site.set, self.release
            );
            if let Some(near) = declared
                .iter()
                .find(|candidate| candidate.name.eq_ignore_ascii_case(name))
            {
                message.push_str(&format!("; the template declares '{}'", near.name));
            }
            self.warn(
                UNDECLARED,
                Path::Attribute {
                    entity: id,
                    index: name_slot,
                    name: Some("Name".into()),
                },
                message,
                site.set,
                Some(&dotted),
                CatalogMismatch::UndeclaredMember,
            );
            return;
        };
        let want = template.form.entity_name();
        if !self.schema.is_a(type_name, want) {
            let found = self.entity_name(type_name);
            self.warn(
                MEMBER_TYPE,
                Path::Entity(id),
                format!(
                    "'{dotted}' of {} is an {found}, but the {} catalog declares an {want} \
                     (bSI PSE001)",
                    site.set, self.release
                ),
                site.set,
                Some(&dotted),
                CatalogMismatch::Form {
                    declared: want.to_owned(),
                    found,
                },
            );
            return;
        }
        let at = Member {
            id,
            entity: member,
            dotted: &dotted,
        };
        match &template.form {
            CatalogMemberForm::SingleValue { data_type } => {
                self.values(&at, site, "NominalValue", data_type.as_deref());
            }
            CatalogMemberForm::EnumeratedValue { data_type } => {
                self.values(&at, site, "EnumerationValues", data_type.as_deref());
            }
            CatalogMemberForm::BoundedValue { data_type } => {
                for slot in ["UpperBoundValue", "LowerBoundValue", "SetPointValue"] {
                    self.values(&at, site, slot, data_type.as_deref());
                }
            }
            CatalogMemberForm::ListValue { data_type } => {
                self.values(&at, site, "ListValues", data_type.as_deref());
            }
            CatalogMemberForm::TableValue {
                defining_type,
                defined_type,
            } => {
                self.values(&at, site, "DefiningValues", defining_type.as_deref());
                self.values(&at, site, "DefinedValues", defined_type.as_deref());
            }
            CatalogMemberForm::ReferenceValue { reference_type } => {
                self.reference(&at, site, reference_type);
            }
            CatalogMemberForm::Complex { members } => {
                if let Some(slot) = self.slot(type_name, "HasProperties") {
                    self.walk(
                        member.attribute(slot),
                        members,
                        &format!("{dotted}."),
                        site,
                        depth + 1,
                        visited,
                    );
                }
            }
            _ => {}
        }
    }

    /// Checks the typed values in slot `attribute` (one value or a list of
    /// them) against the declared type; `None` leaves them unchecked.
    fn values(
        &mut self,
        at: &Member<'_>,
        site: &Site<'_>,
        attribute: &str,
        declared: Option<&str>,
    ) {
        let Some(declared) = declared else {
            return;
        };
        let Some(slot) = self.slot(&at.entity.type_name, attribute) else {
            return;
        };
        let Some(value) = at.entity.attribute(slot) else {
            return;
        };
        let one = std::slice::from_ref(value);
        let values = match value {
            Value::List(items) => items.as_slice(),
            _ => one,
        };
        for value in values {
            let Value::Typed { type_name, .. } = value else {
                continue;
            };
            if self.schema.type_def(declared).is_none() {
                self.unknown_types
                    .insert(format!("{}.{}: {declared}", site.set, at.dotted));
                continue;
            }
            if self.reaches(type_name, declared) {
                continue;
            }
            let found = self.type_name(type_name);
            let declared_name = self.type_name(declared);
            self.warn(
                MEMBER_TYPE,
                Path::Attribute {
                    entity: at.id,
                    index: slot,
                    name: Some(attribute.to_owned()),
                },
                format!(
                    "'{}' of {} holds a {found}, but the {} catalog declares {declared_name} \
                     (bSI PSE001)",
                    at.dotted, site.set, self.release
                ),
                site.set,
                Some(at.dotted),
                CatalogMismatch::DataType {
                    declared: declared_name,
                    found,
                },
            );
        }
    }

    /// Checks that the `PropertyReference` resolves to a `reference_type`.
    fn reference(&mut self, at: &Member<'_>, site: &Site<'_>, reference_type: &str) {
        let Some(slot) = self.slot(&at.entity.type_name, "PropertyReference") else {
            return;
        };
        let Some(target) = at
            .entity
            .attribute(slot)
            .and_then(|value| value.unwrap_typed().as_ref_id())
            .and_then(|target| self.model.get(target))
        else {
            return;
        };
        if self.schema.entity(reference_type).is_none() {
            self.unknown_types
                .insert(format!("{}.{}: {reference_type}", site.set, at.dotted));
            return;
        }
        let found_token: &str = &target.type_name;
        if self.schema.is_a(found_token, reference_type) {
            return;
        }
        let found = self.entity_name(found_token);
        let declared = self.entity_name(reference_type);
        self.warn(
            MEMBER_TYPE,
            Path::Attribute {
                entity: at.id,
                index: slot,
                name: Some("PropertyReference".into()),
            },
            format!(
                "'{}' of {} references an {found}, but the {} catalog declares an {declared} \
                 (bSI PSE001)",
                at.dotted, site.set, self.release
            ),
            site.set,
            Some(at.dotted),
            CatalogMismatch::DataType { declared, found },
        );
    }

    /// Whether `candidate` is `declared`, or a defined type whose alias
    /// chain reaches it, ignoring case.
    fn reaches(&self, candidate: &str, declared: &str) -> bool {
        let mut current = candidate.trim().to_owned();
        for _ in 0..MAX_ALIAS_STEPS {
            if current.eq_ignore_ascii_case(declared) {
                return true;
            }
            let Some(TypeKind::Defined(target)) =
                self.schema.type_def(&current).map(|def| &def.kind)
            else {
                return false;
            };
            current = target.trim().to_owned();
        }
        false
    }
}

/// One member entity under check.
struct Member<'a> {
    id: EntityId,
    entity: &'a Entity,
    /// Its name, dotted with its complex ancestors'.
    dotted: &'a str,
}

#[cfg(all(test, feature = "ifc4", feature = "ifc4x3"))]
mod tests;
