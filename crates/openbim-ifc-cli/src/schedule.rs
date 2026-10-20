//! `schedule`: every work schedule and work plan with its tree of tasks.
//!
//! Through the facade's `ifc::schedule` readers: a schedule's tasks are
//! those assigned to it (`IfcRelAssignsToControl`) that none of its other
//! tasks nests, each with its scheduled start, finish and duration from
//! its `IfcTaskTime` (IFC4 and later) and its sub-tasks (`IfcRelNests`).
//! `--depth N` expands N levels of sub-tasks and counts the rest.

use std::io::Write;

use ifc::schedule::{
    subtasks_of, tasks, tasks_of_schedule, work_plans, work_schedules, ScheduleReadError, Task,
    WorkControl,
};
use ifc::{EntityId, Model};

use crate::cli::{CostArgs, TextFormat};
use crate::error::{CliError, CliResult, Outcome};
use crate::input;
use crate::json::Json;

struct Node {
    id: u64,
    identification: Option<String>,
    name: Option<String>,
    milestone: Option<bool>,
    start: Option<String>,
    finish: Option<String>,
    duration: Option<String>,
    children: Children,
}

enum Children {
    Expanded(Vec<Node>),
    Truncated(usize),
}

/// Run `schedule`.
pub(crate) fn run(args: &CostArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let loaded = input::read(&args.file, args.input.input_layout)?;
    let model = &loaded.model;
    input::declared_schema(model)?;
    let all = tasks(model).map_err(error)?;
    let mut controls: Vec<WorkControl<'_>> = work_schedules(model).map_err(error)?;
    controls.extend(work_plans(model).map_err(error)?);
    controls.sort_by_key(WorkControl::id);
    let mut schedules = Vec::new();
    for control in &controls {
        let assigned = tasks_of_schedule(model, control.id()).map_err(error)?;
        let mut nested = Vec::new();
        for id in &assigned {
            nested.extend(subtasks_of(model, *id).map_err(error)?);
        }
        let mut path = Vec::new();
        let mut tree = Vec::new();
        for id in assigned.into_iter().filter(|id| !nested.contains(id)) {
            if let Some(node) = node(model, &all, id, args.depth, &mut path)? {
                tree.push(node);
            }
        }
        schedules.push((control, tree));
    }
    let text = match args.format {
        TextFormat::Human => {
            if schedules.is_empty() {
                "no work schedule or work plan\n".to_owned()
            } else {
                let mut text = String::new();
                for (control, tree) in &schedules {
                    let class = if model
                        .get(control.id())
                        .is_some_and(|entity| entity.type_name.eq_ignore_ascii_case("IFCWORKPLAN"))
                    {
                        "IfcWorkPlan"
                    } else {
                        "IfcWorkSchedule"
                    };
                    text.push_str(&format!(
                        "#{} {class}{}{}\n",
                        control.id().0,
                        control
                            .name()
                            .map_or_else(String::new, |n| format!(" {n:?}")),
                        control
                            .predefined_type()
                            .map_or_else(String::new, |k| format!(" ({k})"))
                    ));
                    for node in tree {
                        human(node, 1, &mut text);
                    }
                }
                text
            }
        }
        TextFormat::Json => Json::Array(
            schedules
                .iter()
                .map(|(control, tree)| {
                    Json::object([
                        ("id", Json::uint(control.id().0)),
                        ("name", Json::opt_str(control.name())),
                        ("predefined_type", Json::opt_str(control.predefined_type())),
                        ("tasks", Json::Array(tree.iter().map(node_json).collect())),
                    ])
                })
                .collect(),
        )
        .pretty(),
    };
    out.write_all(text.as_bytes())
        .map_err(|error| CliError::stdout(&error))?;
    Ok(Outcome::Clean)
}

fn node(
    model: &Model,
    all: &[Task<'_>],
    id: EntityId,
    depth: Option<usize>,
    path: &mut Vec<EntityId>,
) -> CliResult<Option<Node>> {
    let Some(task) = all.iter().find(|task| task.id() == id) else {
        return Ok(None);
    };
    if path.contains(&id) {
        return Err(CliError::InvalidModel(format!(
            "task #{} nests itself",
            id.0
        )));
    }
    let (time, _) = task.time(model);
    let owned = |text: Option<&str>| text.map(str::to_owned);
    let nested = subtasks_of(model, id).map_err(error)?;
    let children = match depth {
        Some(0) if !nested.is_empty() => Children::Truncated(nested.len()),
        _ => {
            path.push(id);
            let mut expanded = Vec::new();
            for child in nested {
                if let Some(child) =
                    node(model, all, child, depth.map(|d| d.saturating_sub(1)), path)?
                {
                    expanded.push(child);
                }
            }
            path.pop();
            Children::Expanded(expanded)
        }
    };
    Ok(Some(Node {
        id: id.0,
        identification: owned(task.identification()),
        name: owned(task.name()),
        milestone: task.is_milestone(),
        start: time.as_ref().and_then(|time| owned(time.schedule_start())),
        finish: time.as_ref().and_then(|time| owned(time.schedule_finish())),
        duration: time
            .as_ref()
            .and_then(|time| owned(time.schedule_duration())),
        children,
    }))
}

fn human(node: &Node, depth: usize, text: &mut String) {
    let indent = "  ".repeat(depth);
    let mut line = format!("{indent}#{}", node.id);
    if let Some(identification) = &node.identification {
        line.push_str(&format!(" {identification}"));
    }
    if let Some(name) = &node.name {
        line.push_str(&format!(" {name:?}"));
    }
    if node.milestone == Some(true) {
        line.push_str(" milestone");
    }
    match (&node.start, &node.finish) {
        (Some(start), Some(finish)) => line.push_str(&format!(" {start} .. {finish}")),
        (Some(start), None) => line.push_str(&format!(" from {start}")),
        (None, Some(finish)) => line.push_str(&format!(" until {finish}")),
        (None, None) => {}
    }
    if let Some(duration) = &node.duration {
        line.push_str(&format!(" ({duration})"));
    }
    text.push_str(&line);
    text.push('\n');
    match &node.children {
        Children::Expanded(children) => {
            for child in children {
                human(child, depth + 1, text);
            }
        }
        Children::Truncated(count) => {
            text.push_str(&format!("{indent}  ... {count} sub-task(s) not expanded\n"));
        }
    }
}

fn node_json(node: &Node) -> Json {
    let children = match &node.children {
        Children::Expanded(children) => Json::Array(children.iter().map(node_json).collect()),
        Children::Truncated(count) => Json::object([
            ("truncated", Json::Bool(true)),
            ("count", Json::uint(*count as u64)),
        ]),
    };
    Json::object([
        ("id", Json::uint(node.id)),
        ("identification", Json::opt_str(node.identification.clone())),
        ("name", Json::opt_str(node.name.clone())),
        (
            "is_milestone",
            node.milestone.map_or(Json::Null, Json::Bool),
        ),
        ("start", Json::opt_str(node.start.clone())),
        ("finish", Json::opt_str(node.finish.clone())),
        ("duration", Json::opt_str(node.duration.clone())),
        ("subtasks", children),
    ])
}

fn error(error: ScheduleReadError) -> CliError {
    match error {
        ScheduleReadError::UnsupportedSchema { .. } | ScheduleReadError::MultipleSchemas { .. } => {
            CliError::UnsupportedSchema(error.to_string())
        }
        ScheduleReadError::Cycle(_) | ScheduleReadError::SequenceDepthExceeded { .. } => {
            CliError::BudgetExceeded(error.to_string())
        }
        other => CliError::InvalidModel(other.to_string()),
    }
}
