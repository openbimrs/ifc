//! The printed report: methodology and environment first, then the tables.
//!
//! Numbers are never printed without the method beside them.

use std::fmt::Write as _;
use std::time::Duration;

use axiolid_contracts::BackendId;
use ifc_model::{EntityId, Model};

use crate::harness::{compare, Agreement, Area, Contestant, Measure, Outcome, Run};

/// Longest refusal reason printed in full; longer ones are cut, marked.
const REASON_CHARS: usize = 160;

struct Row {
    fixture: String,
    backend: BackendId,
    products: usize,
    meshed: usize,
    refused: usize,
    no_body: usize,
    vertices: usize,
    triangles: usize,
    first: Duration,
    median: Option<Duration>,
    range: Option<(Duration, Duration)>,
}

pub struct Report {
    backends: Vec<(BackendId, Area)>,
    iterations: usize,
    agreement: Agreement,
    load_before: String,
    rows: Vec<Row>,
    unreadable: Vec<(String, String)>,
    coverage: Vec<String>,
    shared_refusals: Vec<String>,
    divergences: Vec<String>,
    compared: usize,
    agreed: usize,
}

impl Report {
    pub fn new(
        contestants: &[Box<dyn Contestant>],
        iterations: usize,
        agreement: Agreement,
    ) -> Self {
        Self {
            backends: contestants
                .iter()
                .map(|c| (c.backend(), c.area()))
                .collect(),
            iterations,
            agreement,
            load_before: load(),
            rows: Vec::new(),
            unreadable: Vec::new(),
            coverage: Vec::new(),
            shared_refusals: Vec::new(),
            divergences: Vec::new(),
            compared: 0,
            agreed: 0,
        }
    }

    pub fn unreadable(&mut self, fixture: &str, error: &str) {
        self.unreadable.push((fixture.to_owned(), error.to_owned()));
    }

    /// Record one fixture's runs, one per contestant, in contestant order.
    pub fn fixture(&mut self, fixture: &str, model: &Model, products: &[EntityId], runs: &[Run]) {
        for ((backend, _), run) in self.backends.iter().zip(runs) {
            let mut row = Row {
                fixture: fixture.to_owned(),
                backend: *backend,
                products: products.len(),
                meshed: 0,
                refused: 0,
                no_body: 0,
                vertices: 0,
                triangles: 0,
                first: run.first,
                median: run.median(),
                range: run.range(),
            };
            for outcome in &run.outcomes {
                match outcome {
                    Outcome::Produced(Measure::Mesh(mesh)) => {
                        row.meshed += 1;
                        row.vertices += mesh.vertices;
                        row.triangles += mesh.triangles;
                    }
                    Outcome::NoBody => row.no_body += 1,
                    Outcome::Refused(_) => row.refused += 1,
                }
            }
            self.rows.push(row);
        }

        let type_name = |product: EntityId| model.get(product).map_or("?", |e| &*e.type_name);
        for (i, (left_id, left_area)) in self.backends.iter().enumerate() {
            for (j, (right_id, right_area)) in self.backends.iter().enumerate().skip(i + 1) {
                if left_area != right_area {
                    continue;
                }
                for (index, product) in products.iter().enumerate() {
                    let (left, right) = (&runs[i].outcomes[index], &runs[j].outcomes[index]);
                    let at = format!("{fixture} | #{} | {}", product.0, type_name(*product));
                    match (left, right) {
                        (Outcome::Produced(a), Outcome::Produced(b)) => {
                            self.compared += 1;
                            let found = compare(a, b, self.agreement);
                            if found.is_empty() {
                                self.agreed += 1;
                            }
                            for d in found {
                                self.divergences.push(format!(
                                    "| {at} | {} | `{left_id}` {} | `{right_id}` {} | {:.3e} | {:.3e} |",
                                    d.metric, d.left, d.right, d.difference, d.allowed
                                ));
                            }
                        }
                        (Outcome::NoBody, Outcome::NoBody) => {}
                        (a @ Outcome::Refused(_), b @ Outcome::Refused(_)) => {
                            self.shared_refusals.push(format!(
                                "| {at} | `{left_id}`: {} | `{right_id}`: {} |",
                                describe(a),
                                describe(b)
                            ));
                        }
                        (a, b) => self.coverage.push(format!(
                            "| {at} | `{left_id}`: {} | `{right_id}`: {} |",
                            describe(a),
                            describe(b)
                        )),
                    }
                }
            }
        }
    }

    /// Print everything; returns the number of divergences.
    pub fn print(&self) -> usize {
        let mut out = String::new();
        self.methodology(&mut out);
        self.table(&mut out);
        self.findings(&mut out);
        print!("{out}");
        self.divergences.len()
    }

    fn methodology(&self, out: &mut String) {
        let backends: Vec<String> = self
            .backends
            .iter()
            .map(|(id, area)| format!("`{id}` ({})", area.name()))
            .collect();
        let _ = writeln!(out, "# Backend comparison (ifc-geometry, #31)\n");
        let _ = writeln!(out, "## Method\n");
        let _ = writeln!(out, "- backends: {}", backends.join(", "));
        let _ = writeln!(
            out,
            "- passes: 1 first pass, reported separately, then {} timed passes per \
             fixture and backend (median, min..max); one pass compiles every product once",
            self.iterations
        );
        let _ = writeln!(
            out,
            "- timed: lowering (identical for every backend) plus compilation; \
             parsing and product discovery are not timed"
        );
        let _ = writeln!(
            out,
            "- compile tolerance: 1 mm; agreement: |a - b| <= {:e} * max(|a|, |b|, 1) on \
             signed volume, area and each bounding-box coordinate",
            self.agreement.relative
        );
        let _ = writeln!(
            out,
            "- not a cross-kernel performance claim: backends that mesh different \
             products do different work, so compare times only where `meshed` is equal"
        );
        let _ = writeln!(out, "- machine: {}", machine());
        let _ = writeln!(
            out,
            "- profile: {}",
            if cfg!(debug_assertions) {
                "debug (timings not meaningful)"
            } else {
                "release"
            }
        );
        let _ = writeln!(
            out,
            "- load (1/5/15 min): before {}, after {}\n",
            self.load_before,
            load()
        );
    }

    fn table(&self, out: &mut String) {
        let _ = writeln!(out, "## Per fixture and backend\n");
        let _ = writeln!(
            out,
            "| fixture | backend | products | meshed | refused | no body | vertices | \
             triangles | first pass (ms) | median of {} (ms) | min..max (ms) |",
            self.iterations
        );
        let _ = writeln!(
            out,
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
        );
        for row in &self.rows {
            let (median, range) = match (row.median, row.range) {
                (Some(median), Some((min, max))) => {
                    (ms(median), format!("{}..{}", ms(min), ms(max)))
                }
                _ => ("-".into(), "-".into()),
            };
            let _ = writeln!(
                out,
                "| {} | `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                row.fixture,
                row.backend,
                row.products,
                row.meshed,
                row.refused,
                row.no_body,
                row.vertices,
                row.triangles,
                ms(row.first),
                median,
                range
            );
        }
        for (fixture, error) in &self.unreadable {
            let _ = writeln!(out, "\n{fixture}: not read ({error})");
        }
    }

    fn findings(&self, out: &mut String) {
        let _ = writeln!(out, "\n## Agreement\n");
        if self.backends.len() < 2 {
            let _ = writeln!(
                out,
                "One backend: nothing to compare. Enable `compile-reference-backend` \
                 to compare against the reference."
            );
            return;
        }
        let _ = writeln!(
            out,
            "{} products meshed by both sides of a pair; {} agree within tolerance; \
             {} divergent metrics.",
            self.compared,
            self.agreed,
            self.divergences.len()
        );
        if !self.divergences.is_empty() {
            let _ = writeln!(
                out,
                "\n| fixture | product | type | metric | left | right | abs. difference | allowed |"
            );
            let _ = writeln!(out, "| --- | --- | --- | --- | ---: | ---: | ---: | ---: |");
            for line in &self.divergences {
                let _ = writeln!(out, "{line}");
            }
        }
        outcome_table(
            out,
            "Coverage differences",
            "Products where one backend produced a mesh and another did not. A refusal \
             is data: the run continued.",
            &self.coverage,
        );
        outcome_table(
            out,
            "Refused by both",
            "Products neither backend of a pair compiled.",
            &self.shared_refusals,
        );
    }
}

fn outcome_table(out: &mut String, title: &str, lead: &str, lines: &[String]) {
    let _ = writeln!(out, "\n## {title}\n");
    if lines.is_empty() {
        let _ = writeln!(out, "None.");
        return;
    }
    let _ = writeln!(out, "{lead}\n");
    let _ = writeln!(out, "| fixture | product | type | left | right |");
    let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
    for line in lines {
        let _ = writeln!(out, "{line}");
    }
}

fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Produced(Measure::Mesh(mesh)) => format!("mesh ({} triangles)", mesh.triangles),
        Outcome::NoBody => "no body".into(),
        Outcome::Refused(reason) => {
            let reason = reason.replace('|', "\\|");
            if reason.chars().count() > REASON_CHARS {
                let cut: String = reason.chars().take(REASON_CHARS).collect();
                format!("refused: {cut}…")
            } else {
                format!("refused: {reason}")
            }
        }
    }
}

fn ms(duration: Duration) -> String {
    format!("{:.3}", duration.as_secs_f64() * 1e3)
}

fn load() -> String {
    std::fs::read_to_string("/proc/loadavg").map_or_else(
        |_| "unknown".into(),
        |text| {
            text.split_whitespace()
                .take(3)
                .collect::<Vec<_>>()
                .join(" ")
        },
    )
}

fn machine() -> String {
    let cpu = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|line| line.starts_with("model name"))
                .and_then(|line| line.split(':').nth(1))
                .map(|name| name.trim().to_owned())
        })
        .unwrap_or_else(|| std::env::consts::ARCH.to_owned());
    let threads = std::thread::available_parallelism().map_or(0, |n| n.get());
    let os = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("PRETTY_NAME="))
                .map(|name| name.trim_matches('"').to_owned())
        })
        .unwrap_or_else(|| std::env::consts::OS.to_owned());
    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map_or_else(|_| String::new(), |k| format!(", kernel {}", k.trim()));
    format!("{cpu}, {threads} logical CPUs available, {os}{kernel}; the harness calls each backend from one thread")
}
