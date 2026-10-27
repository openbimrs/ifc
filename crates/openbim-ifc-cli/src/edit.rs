//! `edit`: set values by path on the entities a selector selects.
//!
//! Every `--set PATH VALUE` is written on every selected entity, all of
//! them planned as one transaction through the facade's
//! `SelectorIndex::plan_text_edits`: the value is read as the type of the
//! attribute or property it writes, a value an entity already holds is
//! left alone, and any refusal refuses the run with nothing written
//! (exit 3). The edited model goes to `--output`, or over the input with
//! `--in-place`; `--dry-run` reports the plan and writes nothing.

use std::io::Write;

use ifc::selector::{value_text, Selector, SelectorIndex, TextEdit, ValuePath};

use crate::cli::EditArgs;
use crate::error::{CliError, CliResult, Outcome};
use crate::input;
use crate::output;
use crate::report::{Change, Refusal, Report};
use crate::select::selector_error;

/// Run `edit`.
pub(crate) fn run(args: &EditArgs, out: &mut impl Write) -> CliResult<Outcome> {
    let selector = Selector::parse(&args.query)
        .map_err(|error| CliError::InvalidSelector(format!("{:?}: {error}", args.query)))?;
    let mut sets = Vec::with_capacity(args.set.len() / 2);
    for pair in args.set.chunks(2) {
        let [path, value] = pair else {
            return Err(CliError::Usage("--set takes a PATH and a VALUE".to_owned()));
        };
        let parsed = ValuePath::parse(path)
            .map_err(|error| CliError::InvalidSelector(format!("--set {path:?}: {error}")))?;
        sets.push((parsed, value.clone()));
    }
    let destination = output::destination(&args.output, &args.file)?;
    let mut loaded = input::read(&args.file, args.input.input_layout)?;
    let schema = input::declared_schema(&loaded.model)?;

    let mut report = Report {
        command: "edit",
        file: loaded.path.clone(),
        table: None,
        selected: None,
        dry_run: args.output.dry_run,
        written: None,
        changes: Vec::new(),
        refusals: Vec::new(),
    };
    let mut edits = Vec::new();
    let seed = ifc::fresh_seed().map_err(|error| CliError::Unsupported(error.to_string()))?;
    let planned = {
        let index = SelectorIndex::new(&loaded.model, schema);
        let selected = index
            .select(&selector)
            .map_err(|error| selector_error(&args.query, error))?;
        report.selected = Some(selected.len());
        for id in &selected {
            for (path, value) in &sets {
                edits.push(TextEdit::new(*id, path.clone(), value.clone()));
            }
        }
        index.plan_text_edits(&edits, &args.null, seed)
    };
    let plan = match planned {
        Ok(plan) => plan,
        Err(refused) => {
            report.refusals = refused
                .iter()
                .map(|refusal| {
                    let mut entry = Refusal::of(&refusal.error);
                    if let Some(edit) = refusal.edit.and_then(|edit| edits.get(edit)) {
                        entry.entity = Some(edit.entity);
                        entry.path = Some(edit.path.to_string());
                    }
                    entry
                })
                .collect();
            return report.emit(args.format, out);
        }
    };
    report.changes = plan
        .changes
        .iter()
        .map(|change| Change {
            cell: None,
            entity: change.entity,
            path: edits[change.edit].path.to_string(),
            target: change.target.clone(),
            before: value_text(&change.before, &args.null),
            after: value_text(&change.after, &args.null),
        })
        .collect();
    if let Some(path) = destination {
        if let Err(error) = plan.commit(&mut loaded.model) {
            report.refusals.push(Refusal::of(&error));
            return report.emit(args.format, out);
        }
        output::write_edited(&loaded.model, &loaded, &path, args.output.in_place)?;
        report.written = Some(path.display().to_string());
    }
    report.emit(args.format, out)
}
