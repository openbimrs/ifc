//! `cargo run -p xtask -- plans`: every open task left in a PLAN.md.
//!
//! The nested PLAN.md files predate GitHub issues as the backlog. This
//! report is the migration's inventory: each unchecked task with its file and
//! the evidence its own text names, so a stale task (its module exists, its
//! type is exported) can be told from open work before either is moved.
//!
//! `--check` fails while any PLAN.md is tracked, so one cannot return.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use crate::workspace::Workspace;

struct Task {
    id: String,
    text: String,
    /// Backticked paths the task names, and whether each exists.
    paths: Vec<(String, bool)>,
    /// Backticked identifiers the task names, and whether Rust source uses them.
    symbols: Vec<(String, bool)>,
}

pub(crate) fn run(check: bool) -> Result<(), String> {
    let workspace = Workspace::load()?;
    let plans = tracked_plans(&workspace.root)?;
    if check {
        return if plans.is_empty() {
            println!("no PLAN.md tracked");
            Ok(())
        } else {
            Err(format!(
                "{} PLAN.md files are tracked; open work belongs in GitHub issues \
                 and `TODO(#N)` markers: {}",
                plans.len(),
                plans.join(", ")
            ))
        };
    }
    let source = rust_source(&workspace.root);
    let mut total = 0;
    for plan in &plans {
        let text = std::fs::read_to_string(workspace.root.join(plan))
            .map_err(|error| format!("{plan}: {error}"))?;
        let base = Path::new(plan).parent().unwrap_or(Path::new(""));
        let tasks = open_tasks(&text, &workspace.root, base, &source);
        if tasks.is_empty() {
            continue;
        }
        total += tasks.len();
        println!("## {plan}\n");
        for task in tasks {
            let evidence: Vec<String> =
                task.paths
                    .iter()
                    .map(|(p, found)| format!("{p} {}", if *found { "exists" } else { "missing" }))
                    .chain(task.symbols.iter().map(|(s, found)| {
                        format!("{s} {}", if *found { "used" } else { "unused" })
                    }))
                    .collect();
            println!("- `{}` {}", task.id, task.text);
            if !evidence.is_empty() {
                println!("  - evidence: {}", evidence.join("; "));
            }
        }
        println!();
    }
    println!("{total} open tasks in {} PLAN.md files", plans.len());
    Ok(())
}

fn tracked_plans(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["ls-files", "*PLAN.md", "PLAN.md"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("git ls-files: {error}"))?;
    let mut plans: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect();
    plans.sort();
    plans.dedup();
    Ok(plans)
}

/// Every tracked Rust file's text, concatenated per crate directory.
fn rust_source(root: &Path) -> BTreeMap<String, String> {
    let output = Command::new("git")
        .args(["ls-files", "*.rs"])
        .current_dir(root)
        .output();
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let Ok(output) = output else { return out };
    for file in String::from_utf8_lossy(&output.stdout).lines() {
        let krate = file.split('/').next().unwrap_or("").to_owned();
        if let Ok(text) = std::fs::read_to_string(root.join(file)) {
            out.entry(krate).or_default().push_str(&text);
        }
    }
    out
}

fn open_tasks(
    text: &str,
    root: &Path,
    base: &Path,
    source: &BTreeMap<String, String>,
) -> Vec<Task> {
    let krate = base
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut tasks = Vec::new();
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let Some(rest) = line.trim_start().strip_prefix("- [ ]") else {
            continue;
        };
        let indent = line.len() - line.trim_start().len();
        let mut body = rest.trim().to_owned();
        while let Some(next) = lines.peek() {
            let next_indent = next.len() - next.trim_start().len();
            if next.trim().is_empty() || next_indent <= indent {
                break;
            }
            body.push(' ');
            body.push_str(next.trim());
            lines.next();
        }
        let ticked = backticked(&body);
        let id = ticked
            .iter()
            .find(|t| is_task_id(t))
            .cloned()
            .unwrap_or_else(|| "-".to_owned());
        let mut paths = Vec::new();
        let mut symbols = Vec::new();
        for token in ticked.iter().filter(|t| **t != id) {
            if token.contains('/') || token.ends_with(".rs") || token.ends_with(".md") {
                let found = root.join(base).join(token).exists() || root.join(token).exists();
                paths.push((token.clone(), found));
            } else if token
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
                && token.len() > 3
            {
                let name = token.rsplit("::").next().unwrap_or(token);
                let found = source.get(&krate).is_some_and(|text| text.contains(name));
                symbols.push((token.clone(), found));
            }
        }
        tasks.push(Task {
            id,
            text: body,
            paths,
            symbols,
        });
    }
    tasks
}

fn backticked(text: &str) -> Vec<String> {
    text.split('`')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, t)| t.to_owned())
        .collect()
}

fn is_task_id(token: &str) -> bool {
    token.len() > 2
        && token.contains('-')
        && token
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_tasks_carry_their_continuation_and_evidence() {
        let plan = "- [x] `DONE-1` - finished\n\
                    - [ ] `QTY-SET` - views in `src/quantity.rs` via `QuantitySet`\n  \
                    - Proof: tests.\n\
                    - [ ] `QTY-EDIT` - edits\n";
        let mut source = BTreeMap::new();
        source.insert("krate".to_owned(), "pub struct QuantitySet;".to_owned());
        let tasks = open_tasks(plan, Path::new("/nonexistent"), Path::new("krate"), &source);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "QTY-SET");
        assert!(tasks[0].text.ends_with("Proof: tests."));
        assert_eq!(tasks[0].paths, [("src/quantity.rs".to_owned(), false)]);
        assert_eq!(tasks[0].symbols, [("QuantitySet".to_owned(), true)]);
        assert_eq!(tasks[1].id, "QTY-EDIT");
    }
}
