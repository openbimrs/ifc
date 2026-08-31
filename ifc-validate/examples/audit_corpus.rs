//! Validate every committed fixture and print a triage table.
//!
//! Run: `cargo run -p ifc-validate --example audit_corpus`
//!
//! Prints one block per fixture with findings grouped by rule, so a real
//! defect is distinguishable from a systematic validator gap at a glance.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ifc_model::Codec;
use ifc_step::StepCodec;
use ifc_validate::Severity;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures");
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();

    let mut clean = 0usize;
    let mut with_errors = 0usize;
    let mut unparsable = 0usize;
    let mut skipped = 0usize;
    // rule -> (files affected, total occurrences)
    let mut by_rule: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for file in &files {
        let shown = file
            .strip_prefix(&root)
            .unwrap_or(file)
            .display()
            .to_string();
        let model = match StepCodec.read_path(file) {
            Ok(model) => model,
            Err(error) => {
                unparsable += 1;
                println!("\n### {shown}\n  UNPARSABLE: {error}");
                continue;
            }
        };
        // Honour the file's own FILE_SCHEMA. Validating an IFC2X3 file
        // against IFC4 tables reports slot-count errors that are artefacts of
        // the wrong tables -- `IfcWallStandardCase` has 8 attributes in
        // IFC2X3 and 9 in IFC4.
        let report = match ifc_validate::validate_declared(&model) {
            Ok(report) => report,
            Err(error) => {
                skipped += 1;
                println!("\n### {shown}\n  SKIPPED: {error}");
                continue;
            }
        };

        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for finding in report.findings() {
            if finding.severity == Severity::Unsupported {
                continue;
            }
            *counts.entry(finding.rule.as_str()).or_default() += 1;
        }
        let errors = report
            .findings()
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();

        for (rule, count) in &counts {
            let entry = by_rule.entry((*rule).to_string()).or_default();
            entry.0 += 1;
            entry.1 += count;
        }

        if errors == 0 {
            clean += 1;
            println!("\n### {shown}\n  clean ({} entities)", model.len());
            continue;
        }
        with_errors += 1;
        println!("\n### {shown}\n  {errors} errors, {} entities", model.len());
        for (rule, count) in &counts {
            println!("    {count:>4}x {rule}");
        }
        // First three concrete examples make triage possible.
        for finding in report
            .sorted()
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .take(3)
        {
            println!("      e.g. {} -- {}", finding.path, finding.message);
        }
    }

    println!("\n================ SUMMARY ================");
    println!("fixtures:   {}", files.len());
    println!("clean:      {clean}");
    println!("errors:     {with_errors}");
    println!("unparsable: {unparsable}");
    println!("skipped:    {skipped} (schema this build has no tables for)");
    println!("\nrule                                          files  hits");
    for (rule, (files_hit, hits)) in &by_rule {
        println!("{rule:<45} {files_hit:>5} {hits:>5}");
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "ifc") {
            out.push(path);
        }
    }
}
