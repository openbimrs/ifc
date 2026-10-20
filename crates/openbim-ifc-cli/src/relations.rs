//! `relations`: what one entity is related to, by kind, and every
//! relationship that names it.
//!
//! The kinds are the facade's: the selector's relation paths (`container`,
//! `parent`, `type`, `occurrences`, `material`, `materials`,
//! `classification`, `group`), its property sets, and the openings,
//! fillings and voided element of `ifc::openings_of` and its siblings.
//! Under them every objectified relationship that references the entity is
//! listed with the attribute it names it in, so nothing the kinds leave out
//! is hidden. `--traverse-up` lists the chain of parents instead, from the
//! entity up to the project. A relationship the schema allows once and the
//! file states twice is refused as `invalid-model`, never answered with
//! one of them.

use std::io::Write;

use ifc::selector::{SelectedValue, SelectorError, SelectorIndex, ValuePath};
use ifc::{EntityId, Model, ObjectRelationError, ReverseIndex, Schema};

use crate::cli::{RelationsArgs, TextFormat};
use crate::error::{CliError, CliResult, Outcome};
use crate::input;
use crate::json::Json;
use crate::select::selector_error;

/// An entity as a report names it.
pub(crate) struct Label {
    pub(crate) id: u64,
    pub(crate) class: String,
    pub(crate) name: Option<String>,
}

impl Label {
    pub(crate) fn of(index: &SelectorIndex<'_>, id: EntityId) -> Self {
        let read = |path: &str| {
            ValuePath::parse(path)
                .ok()
                .and_then(|path| index.value(id, &path).ok())
        };
        let class = match read("class") {
            Some(SelectedValue::Text(class)) => class,
            _ => String::new(),
        };
        // `Name` reads a property set named `Name` on an entity that
        // declares no such attribute; only an attribute's text is a name.
        // A layer set's `LayerSetName` reads as `Name`, as the selector
        // reads it.
        let named = index.model().get(id).is_some_and(|entity| {
            ifc::attribute_slot(index.schema(), &entity.type_name, "Name").is_ok()
                || index
                    .schema()
                    .is_a(&entity.type_name, "IfcMaterialLayerSet")
        });
        let name = named.then_some(()).and_then(|()| match read("Name") {
            Some(SelectedValue::Text(name)) => Some(name),
            _ => None,
        });
        Self {
            id: id.0,
            class,
            name,
        }
    }

    pub(crate) fn text(&self) -> String {
        match &self.name {
            Some(name) => format!("#{} {} {name:?}", self.id, self.class),
            None => format!("#{} {}", self.id, self.class),
        }
    }

    pub(crate) fn json(&self) -> Json {
        Json::object([
            ("id", Json::uint(self.id)),
            ("type", Json::str(self.class.clone())),
            ("name", Json::opt_str(self.name.clone())),
        ])
    }
}

/// One relationship that references the entity.
struct Mention {
    relationship: Label,
    attribute: String,
}

/// Run `relations`.
pub(crate) fn run(args: &RelationsArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let loaded = input::read(&args.file, args.input.input_layout)?;
    let model = &loaded.model;
    let schema = input::declared_schema(model)?;
    let id = input::find_entity(model, schema, &args.entity)?;
    let index = SelectorIndex::new(model, schema);
    let me = Label::of(&index, id);
    let text = match (args.traverse_up, args.format) {
        (true, format) => {
            let chain = ancestors(&index, id)?;
            match format {
                TextFormat::Human => chain
                    .iter()
                    .enumerate()
                    .map(|(depth, label)| format!("{}{}\n", "  ".repeat(depth), label.text()))
                    .collect(),
                TextFormat::Json => Json::Array(chain.iter().map(Label::json).collect()).pretty(),
            }
        }
        (false, format) => {
            let kinds = kinds(model, schema, &index, id)?;
            let mentions = mentions(model, schema, &index, id);
            match format {
                TextFormat::Human => human(&me, &kinds, &mentions),
                TextFormat::Json => json(&me, &kinds, &mentions).pretty(),
            }
        }
    };
    out.write_all(text.as_bytes())
        .map_err(|error| CliError::stdout(&error))?;
    Ok(Outcome::Clean)
}

/// The entity and its parents, up to the root.
fn ancestors(index: &SelectorIndex<'_>, id: EntityId) -> CliResult<Vec<Label>> {
    let parent = ValuePath::parse("parent").map_err(|error| CliError::Usage(error.to_string()))?;
    let mut chain = vec![Label::of(index, id)];
    let mut seen = vec![id];
    let mut current = id;
    loop {
        match index.value(current, &parent).map_err(error)? {
            SelectedValue::Entity(next) if !seen.contains(&next) => {
                chain.push(Label::of(index, next));
                seen.push(next);
                current = next;
            }
            SelectedValue::Entity(next) => {
                return Err(CliError::InvalidModel(format!(
                    "#{} is its own ancestor through #{}",
                    id.0, next.0
                )))
            }
            _ => return Ok(chain),
        }
    }
}

/// The related entities by kind, in a fixed order; empty kinds left out.
fn kinds(
    model: &Model,
    schema: &Schema,
    index: &SelectorIndex<'_>,
    id: EntityId,
) -> CliResult<Vec<(&'static str, Vec<Label>)>> {
    let mut kinds = Vec::new();
    for (kind, path) in [
        ("container", "container"),
        ("parent", "parent"),
        ("type", "type"),
        ("occurrences", "occurrences"),
        ("material", "material"),
        ("materials", "materials"),
        ("classification", "classification"),
        ("groups", "group"),
    ] {
        let path = ValuePath::parse(path).map_err(|error| CliError::Usage(error.to_string()))?;
        let found = match index.value(id, &path).map_err(error)? {
            SelectedValue::Entity(other) if other != id => vec![other],
            SelectedValue::List(items) => items
                .into_iter()
                .filter_map(|item| match item {
                    SelectedValue::Entity(other) => Some(other),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        // An element's parent is its container: said once.
        let repeated = kind == "parent"
            && kinds.iter().any(|(other, labels): &(&str, Vec<Label>)| {
                *other == "container" && labels.iter().map(|l| l.id).eq(found.iter().map(|f| f.0))
            });
        if !found.is_empty() && !repeated {
            kinds.push((
                kind,
                found
                    .into_iter()
                    .map(|other| Label::of(index, other))
                    .collect(),
            ));
        }
    }
    let element = |found: Result<Vec<EntityId>, ObjectRelationError>| match found {
        Ok(ids) => Ok(ids),
        Err(ObjectRelationError::WrongEntityType { .. }) => Ok(Vec::new()),
        Err(other) => Err(relation_error(other)),
    };
    let openings = element(ifc::openings_of(model, schema, id))?;
    let fillings = element(ifc::fillings_of(model, schema, id))?;
    let voided = element(
        ifc::voided_element_of(model, schema, id).map(|found| found.into_iter().collect()),
    )?;
    for (kind, ids) in [
        ("openings", openings),
        ("fillings", fillings),
        ("voids", voided),
    ] {
        if !ids.is_empty() {
            kinds.push((
                kind,
                ids.into_iter()
                    .map(|other| Label::of(index, other))
                    .collect(),
            ));
        }
    }
    Ok(kinds)
}

/// Every relationship that references `id`, with the attribute naming it.
fn mentions(
    model: &Model,
    schema: &Schema,
    index: &SelectorIndex<'_>,
    id: EntityId,
) -> Vec<Mention> {
    let reverse = ReverseIndex::build(model);
    let mut out = Vec::new();
    for referrer in reverse.referrers(id) {
        let Some(entity) = model.get(referrer.from) else {
            continue;
        };
        if !schema.is_a(&entity.type_name, "IfcRelationship") {
            continue;
        }
        let attribute = schema
            .attribute_at(&entity.type_name, referrer.slot)
            .map_or_else(
                || format!("#{}", referrer.slot),
                |attribute| attribute.name.clone(),
            );
        out.push(Mention {
            relationship: Label::of(index, referrer.from),
            attribute,
        });
    }
    out
}

fn human(me: &Label, kinds: &[(&str, Vec<Label>)], mentions: &[Mention]) -> String {
    let mut text = format!("{}\n", me.text());
    for (kind, labels) in kinds {
        let labels: Vec<String> = labels.iter().map(Label::text).collect();
        text.push_str(&format!("  {kind:<15}{}\n", labels.join(", ")));
    }
    if !mentions.is_empty() {
        text.push_str("  relationships\n");
        for mention in mentions {
            text.push_str(&format!(
                "    #{} {}.{}\n",
                mention.relationship.id, mention.relationship.class, mention.attribute
            ));
        }
    }
    text
}

fn json(me: &Label, kinds: &[(&str, Vec<Label>)], mentions: &[Mention]) -> Json {
    let mut fields = vec![
        ("id".to_owned(), Json::uint(me.id)),
        ("type".to_owned(), Json::str(me.class.clone())),
        ("name".to_owned(), Json::opt_str(me.name.clone())),
    ];
    for (kind, labels) in kinds {
        fields.push((
            (*kind).to_owned(),
            Json::Array(labels.iter().map(Label::json).collect()),
        ));
    }
    let mentions = mentions
        .iter()
        .map(|mention| {
            Json::object([
                ("id", Json::uint(mention.relationship.id)),
                ("type", Json::str(mention.relationship.class.clone())),
                ("attribute", Json::str(mention.attribute.clone())),
            ])
        })
        .collect();
    fields.push(("relationships".to_owned(), Json::Array(mentions)));
    Json::Object(fields)
}

fn error(error: SelectorError) -> CliError {
    selector_error("", error)
}

fn relation_error(error: ObjectRelationError) -> CliError {
    let detail = error.to_string();
    match error {
        ObjectRelationError::MissingEntity(_) => CliError::MissingEntity(detail),
        ObjectRelationError::MissingReference { .. } => CliError::MissingReference(detail),
        ObjectRelationError::WrongEntityType { .. } => CliError::WrongEntityType(detail),
        _ => CliError::InvalidModel(detail),
    }
}
