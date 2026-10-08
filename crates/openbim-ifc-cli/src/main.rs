//! `openbim-ifc`: validate, convert and inspect IFC files from a shell or CI.
//!
//! A thin command layer over the `openbim-ifc` facade: every answer comes
//! from a facade call, and this crate only parses arguments, picks a codec
//! and formats the result. Nothing here interprets IFC on its own.
//!
//! # Exit codes
//!
//! | Code | Meaning |
//! | --- | --- |
//! | 0 | Success; for `validate` and `lint`, no findings that fail the run |
//! | 1 | `validate` or `lint` reported findings that fail the run |
//! | 2 | Usage error, unreadable file, parse error, or a typed refusal |
//!
//! A refusal (an undeclared or unbundled schema, a missing entity, an
//! entity that carries no property sets, a model a layout cannot carry) is
//! never turned into an empty or guessed answer: it is reported on stderr
//! with its kind and the run exits 2.
//!
//! # Module map
//!
//! | Module | Role |
//! | --- | --- |
//! | `cli` | The argument grammar (clap) |
//! | `error` | `CliError`: the refusal kinds and their exit code |
//! | `input` | Reading STEP or ifcXML, the declared release, entity lookup |
//! | `json` | A small JSON writer for the machine-readable outputs |
//! | `sarif` | SARIF 2.1.0 for `validate` and `lint` |
//! | `validate`, `convert`, `info`, `psets`, `tree`, `lint` | One command each |

mod cli;
mod convert;
mod error;
mod info;
mod input;
mod json;
mod lint;
mod psets;
mod sarif;
mod tree;
mod validate;

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let cli = cli::Cli::parse();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let result = match cli.command {
        cli::Command::Validate(args) => validate::run(&args, &mut out),
        cli::Command::Convert(args) => convert::run(&args, &mut out),
        cli::Command::Info(args) => info::run(&args, &mut out),
        cli::Command::Psets(args) => psets::run(&args, &mut out),
        cli::Command::Tree(args) => tree::run(&args, &mut out),
        cli::Command::Lint(args) => lint::run(&args, &mut out),
    };
    let _ = out.flush();
    match result {
        Ok(outcome) => ExitCode::from(outcome.code()),
        Err(error) => {
            eprintln!("openbim-ifc: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}
