//! `materials`: the material definition of each entity a selector selects,
//! with its layers, profiles, constituents or list members.
//!
//! The definition is the selector's `material` path: the object's own
//! association, else its type's, a usage read as the set it uses. Each part
//! is read by path from the release's own attributes, so an attribute a
//! release lacks (a layer's `Name` in IFC2X3) reads as nothing rather than
//! being guessed. Entities without a material are left out.

use std::io::Write;

use ifc::selector::{value_text, SelectedValue, Selector, SelectorIndex, ValuePath};
use ifc::EntityId;

use crate::cli::{MaterialsArgs, TextFormat};
use crate::error::{CliError, CliResult, Outcome};
use crate::input;
use crate::json::Json;
use crate::relations::Label;
use crate::select::selector_error;

/// The paths read from each part, with the names the report gives them.
type Fields = &'static [(&'static str, &'static str)];

/// The parts of each kind of definition: the attribute listing them, and
/// the fields read from each part.
const PARTS: [(&str, &str, Fields); 4] = [
    (
        "IfcMaterialLayerSet",
        "MaterialLayers",
        &[
            ("name", "Name"),
            ("material", "Material.Name"),
            ("thickness", "LayerThickness"),
        ],
    ),
    (
        "IfcMaterialProfileSet",
        "MaterialProfiles",
        &[
            ("name", "Name"),
            ("material", "Material.Name"),
            ("profile", "Profile.ProfileName"),
        ],
    ),
    (
        "IfcMaterialConstituentSet",
        "MaterialConstituents",
        &[
            ("name", "Name"),
            ("material", "Material.Name"),
            ("fraction", "Fraction"),
        ],
    ),
    ("IfcMaterialList", "Materials", &[("material", "Name")]),
];

/// One material definition and its parts.
struct Definition {
    label: Label,
    name: Option<String>,
    parts: Vec<Vec<(&'static str, SelectedValue)>>,
}

/// Run `materials`.
pub(crate) fn run(args: &MaterialsArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let selector = Selector::parse(&args.query)
        .map_err(|error| CliError::InvalidSelector(format!("{:?}: {error}", args.query)))?;
    let loaded = input::read(&args.file, args.input.input_layout)?;
    let model = &loaded.model;
    let schema = input::declared_schema(model)?;
    let index = SelectorIndex::new(model, schema);
    let path =
        |text: &str| ValuePath::parse(text).map_err(|error| CliError::Usage(error.to_string()));
    let material = path("material")?;
    let mut rows = Vec::new();
    for id in index
        .select(&selector)
        .map_err(|error| selector_error(&args.query, error))?
    {
        let found = match index
            .value(id, &material)
            .map_err(|error| selector_error(&args.query, error))?
        {
            SelectedValue::Entity(definition) => vec![definition],
            SelectedValue::List(items) => items
                .into_iter()
                .filter_map(|item| match item {
                    SelectedValue::Entity(definition) => Some(definition),
                    _ => None,
                })
                .collect(),
            _ => continue,
        };
        let definitions = found
            .into_iter()
            .map(|definition| describe(&index, definition, &args.query))
            .collect::<CliResult<Vec<_>>>()?;
        rows.push((Label::of(&index, id), definitions));
    }
    let text = match args.format {
        TextFormat::Human => human(&rows),
        TextFormat::Json => Json::object([(
            "entities",
            Json::Array(
                rows.iter()
                    .map(|(label, definitions)| {
                        let mut fields = match label.json() {
                            Json::Object(fields) => fields,
                            other => vec![("entity".to_owned(), other)],
                        };
                        fields.push((
                            "materials".to_owned(),
                            Json::Array(definitions.iter().map(definition_json).collect()),
                        ));
                        Json::Object(fields)
                    })
                    .collect(),
            ),
        )])
        .pretty(),
    };
    out.write_all(text.as_bytes())
        .map_err(|error| CliError::stdout(&error))?;
    Ok(Outcome::Clean)
}

fn describe(index: &SelectorIndex<'_>, id: EntityId, query: &str) -> CliResult<Definition> {
    let read = |id: EntityId, text: &str| -> CliResult<SelectedValue> {
        let path = ValuePath::parse(text).map_err(|error| CliError::Usage(error.to_string()))?;
        index
            .value(id, &path)
            .map_err(|error| selector_error(query, error))
    };
    let name = match read(id, "Name")? {
        SelectedValue::Text(name) => Some(name),
        _ => None,
    };
    let mut parts = Vec::new();
    let type_name = index.model().get(id).map(|entity| entity.type_name.clone());
    let kind = type_name.and_then(|type_name| {
        PARTS
            .iter()
            .find(|(class, _, _)| index.schema().is_a(&type_name, class))
    });
    if let Some((_, attribute, fields)) = kind {
        if let SelectedValue::List(items) = read(id, attribute)? {
            for item in items {
                let SelectedValue::Entity(part) = item else {
                    continue;
                };
                let mut values = Vec::with_capacity(fields.len());
                for (field, path) in *fields {
                    values.push((*field, read(part, path)?));
                }
                parts.push(values);
            }
        }
    }
    Ok(Definition {
        label: Label::of(index, id),
        name,
        parts,
    })
}

fn human(rows: &[(Label, Vec<Definition>)]) -> String {
    if rows.is_empty() {
        return "no selected entity has a material\n".to_owned();
    }
    let mut text = String::new();
    for (label, definitions) in rows {
        text.push_str(&format!("{}\n", label.text()));
        for definition in definitions {
            let name = definition
                .name
                .as_ref()
                .map_or_else(String::new, |name| format!(" {name:?}"));
            text.push_str(&format!(
                "  #{} {}{name}\n",
                definition.label.id, definition.label.class
            ));
            for (index, part) in definition.parts.iter().enumerate() {
                let cells: Vec<String> = part
                    .iter()
                    .filter(|(_, value)| *value != SelectedValue::Null)
                    .map(|(field, value)| format!("{field} {}", value_text(value, "")))
                    .collect();
                text.push_str(&format!("    {}: {}\n", index + 1, cells.join(", ")));
            }
        }
    }
    text
}

fn definition_json(definition: &Definition) -> Json {
    let parts = definition
        .parts
        .iter()
        .map(|part| {
            Json::object(
                part.iter()
                    .map(|(field, value)| (*field, value_json(value))),
            )
        })
        .collect();
    Json::object([
        ("id", Json::uint(definition.label.id)),
        ("type", Json::str(definition.label.class.clone())),
        ("name", Json::opt_str(definition.name.clone())),
        ("parts", Json::Array(parts)),
    ])
}

fn value_json(value: &SelectedValue) -> Json {
    match value {
        SelectedValue::Null => Json::Null,
        SelectedValue::Integer(i) => Json::Int(*i),
        SelectedValue::Real(r) => Json::Real(*r),
        SelectedValue::Bool(b) => Json::Bool(*b),
        other => Json::str(value_text(other, "")),
    }
}
