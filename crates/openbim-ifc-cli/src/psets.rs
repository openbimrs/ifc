//! `psets`: the property and quantity sets of one object or type object.
//!
//! Resolved exactly by the facade (`ifc::properties::exact_properties`),
//! against the release the header declares: the object's own sets first,
//! then those its type object holds, an occurrence property overriding an
//! inherited one of the same set and name. A malformed or ambiguous set is
//! refused with the resolver's reason, never skipped.

mod value;

use std::io::Write;

use ifc::properties::{exact_properties, ExactPropertyError, ExactSource, ExactValue};
use ifc::{EntityId, Model};

use crate::cli::{PsetsArgs, TableFormat};
use crate::error::{CliError, CliResult, Outcome};
use crate::input;
use crate::json::Json;

/// One property, flattened; a complex property's members follow it as
/// rows of their own, named `Parent.Member`.
struct Row {
    set: String,
    set_id: u64,
    set_type: String,
    source: &'static str,
    source_id: Option<u64>,
    name: String,
    id: u64,
    kind: &'static str,
    value: String,
    value_type: Option<String>,
    unit: Option<u64>,
    json: Json,
    /// A member of the complex property before it.
    member: bool,
}

/// Run `psets`.
pub(crate) fn run(args: &PsetsArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let loaded = input::read(&args.file, args.input.input_layout)?;
    let model = &loaded.model;
    let schema = input::declared_schema(model)?;
    let object = input::find_entity(model, schema, &args.entity)?;
    let entries = exact_properties(model, object).map_err(property_error)?;

    let mut rows = Vec::new();
    for entry in &entries {
        let property = &entry.property;
        let (source, source_id) = match property.source {
            ExactSource::Occurrence => ("occurrence", None),
            ExactSource::Type(id) => ("type", Some(id.0)),
            ExactSource::Material(id) => ("material", Some(id.0)),
            _ => ("other", None),
        };
        let set_type = type_name(model, property.set_id);
        push_rows(
            &mut rows,
            &RowContext {
                set: &property.property_set,
                set_id: property.set_id.0,
                set_type: &set_type,
                source,
                source_id,
            },
            &entry.name,
            property.property_id,
            property.value_type.as_deref(),
            property.unit_id,
            &property.value,
        );
    }

    let identity = ifc::root_identity(model, schema, object);
    let global_id = identity.and_then(|identity| identity.global_id);
    let name = identity.and_then(|identity| identity.name);
    let entity_type = type_name(model, object);
    let written = match args.format {
        TableFormat::Table => {
            let mut text = format!("#{} {entity_type}", object.0);
            if let Some(name) = name {
                text.push_str(&format!(" '{name}'"));
            }
            if let Some(global_id) = global_id {
                text.push_str(&format!(" {global_id}"));
            }
            text.push('\n');
            if rows.is_empty() {
                text.push_str("no property or quantity sets\n");
            } else {
                text.push('\n');
                text.push_str(&table(&rows));
            }
            out.write_all(text.as_bytes())
        }
        TableFormat::Csv => out.write_all(csv(&rows).as_bytes()),
        TableFormat::Json => {
            let document = Json::object([
                ("entity", Json::uint(object.0)),
                ("type", Json::str(entity_type)),
                ("global_id", Json::opt_str(global_id)),
                ("name", Json::opt_str(name)),
                ("sets", sets_json(&rows)),
            ]);
            out.write_all(document.pretty().as_bytes())
        }
    };
    written.map_err(|error| CliError::stdout(&error))?;
    Ok(Outcome::Clean)
}

/// The set a row belongs to.
struct RowContext<'a> {
    set: &'a str,
    set_id: u64,
    set_type: &'a str,
    source: &'static str,
    source_id: Option<u64>,
}

fn push_rows(
    rows: &mut Vec<Row>,
    context: &RowContext<'_>,
    name: &str,
    id: EntityId,
    value_type: Option<&str>,
    unit: Option<EntityId>,
    value: &ExactValue,
) {
    rows.push(Row {
        set: context.set.to_owned(),
        set_id: context.set_id,
        set_type: context.set_type.to_owned(),
        source: context.source,
        source_id: context.source_id,
        name: name.to_owned(),
        id: id.0,
        kind: value::kind(value),
        value: value::text(value),
        value_type: value_type.map(str::to_owned),
        unit: unit.map(|unit| unit.0),
        json: value::json(value),
        member: false,
    });
    if let ExactValue::Complex(complex) = value {
        let first = rows.len();
        for member in &complex.members {
            push_rows(
                rows,
                context,
                &format!("{name}.{}", member.name),
                member.id,
                member.value_type.as_deref(),
                member.unit_id,
                &member.value,
            );
        }
        for row in &mut rows[first..] {
            row.member = true;
        }
    }
}

fn type_name(model: &Model, id: EntityId) -> String {
    model
        .get(id)
        .map(|entity| entity.type_name.to_string())
        .unwrap_or_default()
}

fn source_text(row: &Row) -> String {
    match row.source_id {
        Some(id) => format!("{} #{id}", row.source),
        None => row.source.to_owned(),
    }
}

/// An aligned text table.
fn table(rows: &[Row]) -> String {
    let header = ["Set", "Source", "Property", "Value", "Type", "Unit"];
    let cells: Vec<[String; 6]> = rows
        .iter()
        .map(|row| {
            [
                row.set.clone(),
                source_text(row),
                row.name.clone(),
                row.value.clone(),
                row.value_type.clone().unwrap_or_default(),
                row.unit.map(|unit| format!("#{unit}")).unwrap_or_default(),
            ]
        })
        .collect();
    let mut widths = header.map(|title| title.chars().count());
    for row in &cells {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }
    let line = |cells: &[String]| {
        let mut text = cells
            .iter()
            .zip(widths)
            .map(|(cell, width)| {
                let pad = width - cell.chars().count();
                format!("{cell}{}", " ".repeat(pad))
            })
            .collect::<Vec<_>>()
            .join("  ");
        text.truncate(text.trim_end().len());
        text.push('\n');
        text
    };
    let mut text = line(&header.map(str::to_owned));
    text.push_str(&line(&widths.map(|width| "-".repeat(width))));
    for row in &cells {
        text.push_str(&line(row));
    }
    text
}

/// RFC 4180 CSV with a header row.
fn csv(rows: &[Row]) -> String {
    let field = |text: &str| {
        if text.contains([',', '"', '\n', '\r']) {
            format!("\"{}\"", text.replace('"', "\"\""))
        } else {
            text.to_owned()
        }
    };
    let id = |id: Option<u64>| id.map(|id| id.to_string()).unwrap_or_default();
    let mut text = String::from(
        "set,set_id,source,source_id,property,property_id,kind,value,value_type,unit\r\n",
    );
    for row in rows {
        let cells = [
            field(&row.set),
            row.set_id.to_string(),
            row.source.to_owned(),
            id(row.source_id),
            field(&row.name),
            row.id.to_string(),
            row.kind.to_owned(),
            field(&row.value),
            field(row.value_type.as_deref().unwrap_or_default()),
            id(row.unit),
        ];
        text.push_str(&cells.join(","));
        text.push_str("\r\n");
    }
    text
}

/// The rows regrouped by set, each property with its typed value. Members
/// of a complex property are in its value, not repeated as properties.
fn sets_json(rows: &[Row]) -> Json {
    let mut sets: Vec<(u64, Option<u64>, Vec<&Row>)> = Vec::new();
    for row in rows.iter().filter(|row| !row.member) {
        match sets
            .iter_mut()
            .find(|(set, source, _)| *set == row.set_id && *source == row.source_id)
        {
            Some((_, _, members)) => members.push(row),
            None => sets.push((row.set_id, row.source_id, vec![row])),
        }
    }
    Json::Array(
        sets.into_iter()
            .map(|(_, _, members)| {
                let first = members[0];
                Json::object([
                    ("id", Json::uint(first.set_id)),
                    ("name", Json::str(first.set.clone())),
                    ("type", Json::str(first.set_type.clone())),
                    ("source", Json::str(first.source)),
                    ("source_id", Json::opt_uint(first.source_id)),
                    (
                        "properties",
                        Json::Array(
                            members
                                .into_iter()
                                .map(|row| {
                                    Json::object([
                                        ("id", Json::uint(row.id)),
                                        ("name", Json::str(row.name.clone())),
                                        ("kind", Json::str(row.kind)),
                                        ("value_type", Json::opt_str(row.value_type.clone())),
                                        ("unit", Json::opt_uint(row.unit)),
                                        ("value", row.json.clone()),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                ])
            })
            .collect(),
    )
}

/// The resolver's refusal as the CLI's kind.
fn property_error(error: ExactPropertyError) -> CliError {
    use ExactPropertyError as E;
    let detail = error.to_string();
    match error {
        E::MissingSchema | E::UnsupportedSchema { .. } | E::MultipleSchemas { .. } => {
            CliError::UnsupportedSchema(detail)
        }
        E::MissingReference { .. } => CliError::MissingReference(detail),
        E::InvalidQueryObject { object, type_name } => CliError::WrongEntityType(format!(
            "#{} is an {type_name}, which carries no property sets in this release",
            object.0
        )),
        E::ComplexCycle { .. } | E::ComplexTooDeep { .. } | E::ComplexBudgetExceeded { .. } => {
            CliError::BudgetExceeded(detail)
        }
        E::UnsupportedDefinition { .. }
        | E::UnsupportedProperty { .. }
        | E::UnsupportedRelationship { .. }
        | E::UnsupportedUnit { .. } => CliError::Unsupported(detail),
        _ => CliError::InvalidModel(detail),
    }
}
