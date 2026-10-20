//! `select`: the entities a selector selects, with values read by path.
//!
//! The selector and the paths are the facade's (`ifc::selector`, grammar
//! [`GRAMMAR_VERSION`]), evaluated against the release the header
//! declares through one index, so each property and relationship scan
//! runs once for the whole query and every `--value` column. A query that
//! does not parse, or names a class or attribute the release does not
//! declare, is refused as `invalid-selector` with the byte span it
//! refuses, never answered with an empty list.

use std::io::Write;

use ifc::selector::{
    SelectedValue, Selector, SelectorError, SelectorIndex, ValuePath, GRAMMAR_VERSION,
};
use ifc::{EntityId, Model, Schema};

use crate::cli::{SelectArgs, TableFormat};
use crate::error::{CliError, CliResult, Outcome};
use crate::input;
use crate::json::Json;

/// One selected entity and its `--value` columns.
struct Row {
    id: u64,
    class: String,
    global_id: Option<String>,
    name: Option<String>,
    values: Vec<SelectedValue>,
}

/// Run `select`.
pub(crate) fn run(args: &SelectArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let selector = Selector::parse(&args.query)
        .map_err(|error| CliError::InvalidSelector(format!("{:?}: {error}", args.query)))?;
    let paths = args
        .values
        .iter()
        .map(|text| {
            ValuePath::parse(text)
                .map_err(|error| CliError::InvalidSelector(format!("--value {text:?}: {error}")))
        })
        .collect::<CliResult<Vec<_>>>()?;
    let loaded = input::read(&args.file, args.input.input_layout)?;
    let model = &loaded.model;
    let schema = input::declared_schema(model)?;
    let index = SelectorIndex::new(model, schema);
    let selected = index
        .select(&selector)
        .map_err(|error| selector_error(&args.query, error))?;
    let mut rows = Vec::with_capacity(selected.len());
    for id in selected {
        let values = paths
            .iter()
            .map(|path| index.value(id, path))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| selector_error(&args.query, error))?;
        rows.push(row(model, schema, id, values));
    }

    let written = match args.format {
        TableFormat::Table => out.write_all(table(&args.values, &rows).as_bytes()),
        TableFormat::Csv => out.write_all(csv(&args.values, &rows).as_bytes()),
        TableFormat::Json => {
            let entities = rows
                .iter()
                .map(|row| {
                    let values = args
                        .values
                        .iter()
                        .zip(&row.values)
                        .map(|(path, value)| (path.clone(), json(value)))
                        .collect::<Vec<_>>();
                    Json::object([
                        ("id", Json::uint(row.id)),
                        ("type", Json::str(row.class.clone())),
                        ("global_id", Json::opt_str(row.global_id.clone())),
                        ("name", Json::opt_str(row.name.clone())),
                        ("values", Json::object(values)),
                    ])
                })
                .collect();
            let document = Json::object([
                ("query", Json::str(selector.to_string())),
                ("grammar", Json::str(GRAMMAR_VERSION)),
                ("schema", Json::str(schema.name())),
                ("count", Json::uint(rows.len() as u64)),
                ("entities", Json::Array(entities)),
            ]);
            out.write_all(document.pretty().as_bytes())
        }
    };
    written.map_err(|error| CliError::stdout(&error))?;
    Ok(Outcome::Clean)
}

fn row(model: &Model, schema: &Schema, id: EntityId, values: Vec<SelectedValue>) -> Row {
    let identity = ifc::root_identity(model, schema, id);
    let class = model.get(id).map_or_else(String::new, |entity| {
        schema
            .entity(&entity.type_name)
            .map_or(entity.type_name.to_string(), |definition| {
                definition.name.clone()
            })
    });
    Row {
        id: id.0,
        class,
        global_id: identity.and_then(|identity| identity.global_id.map(str::to_owned)),
        name: identity.and_then(|identity| identity.name.map(str::to_owned)),
        values,
    }
}

/// A value as one table or CSV cell: `#id` for an entity, items joined
/// with `; `, a property set as `name=value` pairs, empty for `NULL`.
fn text(value: &SelectedValue) -> String {
    match value {
        SelectedValue::Null => String::new(),
        SelectedValue::Derived => "*".to_owned(),
        SelectedValue::Bool(b) => b.to_string(),
        SelectedValue::Unknown => "UNKNOWN".to_owned(),
        SelectedValue::Integer(i) => i.to_string(),
        SelectedValue::Real(r) => r.to_string(),
        SelectedValue::Text(text) | SelectedValue::Binary(text) | SelectedValue::Enum(text) => {
            text.clone()
        }
        SelectedValue::Entity(EntityId(id)) => format!("#{id}"),
        SelectedValue::List(items) => items.iter().map(text).collect::<Vec<_>>().join("; "),
        SelectedValue::Properties(members) => members
            .iter()
            .map(|(name, value)| format!("{name}={}", text(value)))
            .collect::<Vec<_>>()
            .join("; "),
        other => format!("{other:?}"),
    }
}

/// A value in the JSON output: an entity as `{"id": n}`, a property set as
/// an object.
fn json(value: &SelectedValue) -> Json {
    match value {
        SelectedValue::Null => Json::Null,
        SelectedValue::Derived => Json::str("*"),
        SelectedValue::Bool(b) => Json::Bool(*b),
        SelectedValue::Unknown => Json::str("UNKNOWN"),
        SelectedValue::Integer(i) => Json::Int(*i),
        SelectedValue::Real(r) => Json::Real(*r),
        SelectedValue::Text(text) | SelectedValue::Binary(text) | SelectedValue::Enum(text) => {
            Json::str(text.clone())
        }
        SelectedValue::Entity(EntityId(id)) => Json::object([("id", Json::uint(*id))]),
        SelectedValue::List(items) => Json::Array(items.iter().map(json).collect()),
        SelectedValue::Properties(members) => Json::object(
            members
                .iter()
                .map(|(name, value)| (name.clone(), json(value)))
                .collect::<Vec<_>>(),
        ),
        other => Json::str(format!("{other:?}")),
    }
}

fn cells(row: &Row) -> Vec<String> {
    let mut cells = vec![
        format!("#{}", row.id),
        row.class.clone(),
        row.global_id.clone().unwrap_or_default(),
        row.name.clone().unwrap_or_default(),
    ];
    cells.extend(row.values.iter().map(text));
    cells
}

fn table(paths: &[String], rows: &[Row]) -> String {
    if rows.is_empty() {
        return "no entity selected\n".to_owned();
    }
    let mut header: Vec<String> = ["Id", "Class", "GlobalId", "Name"]
        .map(str::to_owned)
        .to_vec();
    header.extend(paths.iter().cloned());
    let body: Vec<Vec<String>> = rows.iter().map(cells).collect();
    let mut widths: Vec<usize> = header.iter().map(|title| title.chars().count()).collect();
    for row in &body {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let line = |cells: &[String]| {
        let text = cells
            .iter()
            .zip(&widths)
            .map(|(cell, width)| format!("{cell:<width$}"))
            .collect::<Vec<_>>()
            .join("  ");
        format!("{}\n", text.trim_end())
    };
    let mut text = line(&header);
    let rule: Vec<String> = widths.iter().map(|width| "-".repeat(*width)).collect();
    text.push_str(&line(&rule));
    for row in &body {
        text.push_str(&line(row));
    }
    text.push_str(&format!("{} selected\n", rows.len()));
    text
}

fn csv(paths: &[String], rows: &[Row]) -> String {
    let field = |text: &str| {
        if text.contains([',', '"', '\n', '\r']) {
            format!("\"{}\"", text.replace('"', "\"\""))
        } else {
            text.to_owned()
        }
    };
    let mut header = vec![
        "id".to_owned(),
        "type".to_owned(),
        "global_id".to_owned(),
        "name".to_owned(),
    ];
    header.extend(paths.iter().map(|path| field(path)));
    let mut text = header.join(",");
    text.push_str("\r\n");
    for row in rows {
        let mut cells = cells(row);
        cells[0] = row.id.to_string();
        let cells: Vec<String> = cells.iter().map(|cell| field(cell)).collect();
        text.push_str(&cells.join(","));
        text.push_str("\r\n");
    }
    text
}

pub(crate) fn selector_error(query: &str, error: SelectorError) -> CliError {
    let detail = error.to_string();
    match error {
        SelectorError::UnknownClass { .. } | SelectorError::UnknownAttribute { .. } => {
            CliError::InvalidSelector(format!("{query:?}: {detail}"))
        }
        SelectorError::MissingReference { .. } => CliError::MissingReference(detail),
        SelectorError::UnsupportedValue { .. } | SelectorError::FeatureDisabled(_) => {
            CliError::Unsupported(detail)
        }
        SelectorError::Property { error, .. } => crate::psets::property_error(error),
        _ => CliError::InvalidModel(detail),
    }
}
