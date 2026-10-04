//! Reproducible baselines for the STEP codec (#119) and the entity graph
//! (#14). See `benchmarks/README.md` for the method, the committed
//! baseline and how to compare a change against it.
//!
//! ```text
//! cargo bench -p ifc-step --bench baseline -- [--scale fixtures,small,crossover,large]
//!     [--samples N] [--warmup N] [--probe] [--json out.json] [--data-dir DIR]
//! cargo test -p ifc-step --bench baseline     # the smoke run
//! ```
//!
//! `cargo bench` passes `--bench`, which selects a measurement. Without it
//! -- under `cargo test`, as CI runs it -- the binary makes a smoke run:
//! the tiny synthetic scale and one fixture, three samples each, every
//! equivalence assertion and the probe's, no timing judged. It proves the
//! harness builds, runs and agrees with itself; it measures nothing.

mod alloc;
mod checksum;
mod model;
mod report;
mod stats;
mod step;
mod synthetic;

use std::path::{Path, PathBuf};

#[global_allocator]
static GLOBAL: alloc::Counting = alloc::Counting;

/// One input file: a committed fixture or a generated synthetic scale.
pub(crate) struct Workload {
    pub(crate) name: String,
    /// On disk, for the mapped read.
    pub(crate) path: PathBuf,
    pub(crate) bytes: Vec<u8>,
}

/// Redistributable fixtures (test/fixtures/PROVENANCE.tsv), the three
/// the wasm opt-level benchmark used (#303): a generated product gallery
/// and two MPL-2.0 ifc-lite geometry files, 64-236 KB.
const FIXTURES: &[&str] = &[
    "synthetic-coverage/meshing_coverage.ifc",
    "ifclite-geometry/issue_098_wall_W.ifc",
    "ifclite-geometry/shared_point_faceted_brep.ifc",
];

fn fixture(relative: &str) -> Workload {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures")
        .join(relative);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let name = Path::new(relative)
        .file_stem()
        .map_or_else(|| relative.to_owned(), |s| s.to_string_lossy().into_owned());
    Workload { name, path, bytes }
}

/// Generates `scale` into `dir`, rewriting the file only when it differs.
fn synthetic(scale: synthetic::Scale, dir: &Path) -> Workload {
    let bytes = synthetic::generate(scale);
    let path = dir.join(format!("synthetic-{}-walls.ifc", scale.walls));
    if std::fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
        std::fs::create_dir_all(dir).expect("create the data directory");
        std::fs::write(&path, &bytes).expect("write the synthetic workload");
    }
    Workload {
        name: format!("{} ({} walls)", scale.name, scale.walls),
        path,
        bytes,
    }
}

struct Options {
    measure: bool,
    scales: Vec<String>,
    plan: stats::Plan,
    probe: bool,
    json: Option<PathBuf>,
    data_dir: PathBuf,
}

fn usage(message: &str) -> ! {
    eprintln!("error: {message}");
    eprintln!(
        "usage: baseline [--bench] [--scale fixtures,tiny,small,crossover,large] \
         [--samples N] [--warmup N] [--probe] [--json PATH] [--data-dir DIR]"
    );
    std::process::exit(2);
}

fn options() -> Options {
    let mut args = std::env::args().skip(1);
    let mut options = Options {
        measure: false,
        scales: Vec::new(),
        plan: stats::Plan {
            warmup: 3,
            samples: 20,
        },
        probe: false,
        json: None,
        data_dir: PathBuf::from(env!("CARGO_TARGET_TMPDIR")),
    };
    let mut plan_set = false;
    let number = |value: Option<String>| -> usize {
        value
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| usage("expected a number"))
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bench" => options.measure = true,
            "--scale" => {
                let list = args.next().unwrap_or_else(|| usage("--scale needs a list"));
                options.scales = list.split(',').map(str::to_owned).collect();
            }
            "--samples" => {
                options.plan.samples = number(args.next()).max(1);
                plan_set = true;
            }
            "--warmup" => {
                options.plan.warmup = number(args.next());
                plan_set = true;
            }
            "--probe" => options.probe = true,
            "--json" => options.json = Some(args.next().unwrap_or_else(|| usage("--json")).into()),
            "--data-dir" => {
                options.data_dir = args.next().unwrap_or_else(|| usage("--data-dir")).into();
            }
            other => usage(&format!("unexpected argument {other}")),
        }
    }
    if options.scales.is_empty() {
        options.scales = if options.measure {
            vec!["fixtures".into(), "small".into(), "crossover".into()]
        } else {
            vec!["tiny".into(), "fixture-smoke".into()]
        };
    }
    if !options.measure && !plan_set {
        options.plan = stats::Plan {
            warmup: 1,
            samples: 3,
        };
    }
    options
}

fn workloads(options: &Options) -> Vec<Workload> {
    let mut out = Vec::new();
    for name in &options.scales {
        match name.as_str() {
            "fixtures" => out.extend(FIXTURES.iter().map(|f| fixture(f))),
            "fixture-smoke" => out.push(fixture(FIXTURES[1])),
            other => match synthetic::scale(other) {
                Some(scale) => out.push(synthetic(scale, &options.data_dir)),
                None => usage(&format!("unknown scale {other}")),
            },
        }
    }
    out
}

fn main() {
    let options = options();
    let before = report::load();
    let env = report::environment(options.plan, options.probe);
    let mut recorder = report::Recorder {
        plan: options.plan,
        records: Vec::new(),
    };
    for workload in workloads(&options) {
        eprintln!("{}: verifying", workload.name);
        let verified = step::verify(&workload);
        if !options.measure {
            model::probe_doubles(&verified.model);
        }
        eprintln!(
            "{}: {} entities, measuring",
            workload.name, verified.entities
        );
        step::run(&workload, &verified, &mut recorder);
        model::run(&workload, &verified.model, options.probe, &mut recorder);
    }
    let after = report::load();
    let load = (before.as_str(), after.as_str());
    print!("{}", report::markdown(&env, load, &recorder.records));
    if let Some(path) = &options.json {
        report::write_json(path, &env, load, &recorder.records)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}
