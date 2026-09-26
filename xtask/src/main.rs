//! Repository automation for `openbimrs/ifc`, run as `cargo run -p xtask -- <command>`.
//!
//! Everything the documentation states that can be derived from the source
//! is generated here, and every generator has a `--check` mode that the gate
//! runs so a page cannot silently drift from the code it describes.
//!
//! There is deliberately no committed `cargo xtask` alias: `.cargo/` is
//! gitignored because developers keep local `[patch]` redirects there.
//!
//! ```text
//! cargo run -p xtask -- docs           regenerate every generated docs region
//! cargo run -p xtask -- docs --check   fail if any generated region is out of date
//! ```

mod docs;
mod rust_source;
mod text;
mod workspace;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, rest) = match args.split_first() {
        Some((command, rest)) => (command.as_str(), rest),
        None => return usage(),
    };
    let check = match rest {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => return usage(),
    };
    let result = match command {
        "docs" => docs::run(check),
        _ => return usage(),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask {command}: {error}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: cargo run -p xtask -- docs [--check]");
    ExitCode::from(2)
}
