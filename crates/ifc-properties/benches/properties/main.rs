//! Property-resolution benchmarks (#352), in the harness of the codec and
//! entity-graph baselines (#14, #119). See `benchmarks/README.md` for the
//! method and `benchmarks/baseline.md` for the measured comparison.
//!
//! ```text
//! cargo bench -p ifc-properties --bench properties -- [--scale props-1k,props-10k,props-100k]
//!     [--bench-only NAME,...] [--samples N] [--warmup N] [--json out.json] [--data-dir DIR]
//! cargo test -p ifc-properties --bench properties     # the smoke run
//! ```
//!
//! Sampling, statistics, the counting allocator and the report are the
//! `ifc-step` baseline harness's own modules, compiled in from there, so
//! both benches report the same way and `benchmarks/baseline.py` pools and
//! compares either. Without `--bench` (under `cargo test`, as the gate runs
//! it) the binary makes a smoke run on `props-smoke`: every equivalence
//! assertion, no timing judged.

#[path = "../../../ifc-step/benches/baseline/alloc.rs"]
mod alloc;
#[path = "../../../ifc-step/benches/baseline/checksum.rs"]
#[allow(dead_code)] // The model checksum is the codec bench's.
mod checksum;
mod generate;
#[path = "../../../ifc-step/benches/baseline/report.rs"]
mod report;
mod resolve;
#[path = "../../../ifc-step/benches/baseline/stats.rs"]
mod stats;

use std::path::{Path, PathBuf};

use ifc_model::Codec;

#[global_allocator]
static GLOBAL: alloc::Counting = alloc::Counting;

/// One generated input file.
pub(crate) struct Workload {
    pub(crate) name: String,
    pub(crate) walls: usize,
    pub(crate) bytes: Vec<u8>,
}

struct Options {
    measure: bool,
    scales: Vec<String>,
    benches: Vec<String>,
    plan: stats::Plan,
    json: Option<PathBuf>,
    data_dir: PathBuf,
}

fn usage(message: &str) -> ! {
    eprintln!("error: {message}");
    eprintln!(
        "usage: properties [--bench] [--scale props-smoke,props-1k,props-10k,props-100k] \
         [--bench-only NAME,...] [--samples N] [--warmup N] [--json PATH] [--data-dir DIR]"
    );
    std::process::exit(2);
}

fn options() -> Options {
    let mut args = std::env::args().skip(1);
    let mut options = Options {
        measure: false,
        scales: Vec::new(),
        benches: Vec::new(),
        plan: stats::Plan {
            warmup: 3,
            samples: 20,
        },
        json: None,
        data_dir: PathBuf::from(env!("CARGO_TARGET_TMPDIR")),
    };
    let mut plan_set = false;
    let number = |value: Option<String>| -> usize {
        value
            .and_then(|v| v.parse().ok())
            .unwrap_or_else(|| usage("expected a number"))
    };
    let list = |value: Option<String>| -> Vec<String> {
        value
            .unwrap_or_else(|| usage("expected a list"))
            .split(',')
            .map(str::to_owned)
            .collect()
    };
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bench" => options.measure = true,
            "--scale" => options.scales = list(args.next()),
            "--bench-only" => options.benches = list(args.next()),
            "--samples" => {
                options.plan.samples = number(args.next()).max(1);
                plan_set = true;
            }
            "--warmup" => {
                options.plan.warmup = number(args.next());
                plan_set = true;
            }
            "--json" => options.json = Some(args.next().unwrap_or_else(|| usage("--json")).into()),
            "--data-dir" => {
                options.data_dir = args.next().unwrap_or_else(|| usage("--data-dir")).into();
            }
            other => usage(&format!("unexpected argument {other}")),
        }
    }
    if options.scales.is_empty() {
        options.scales = if options.measure {
            vec!["props-1k".into(), "props-10k".into(), "props-100k".into()]
        } else {
            vec!["props-smoke".into()]
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

/// Generates `scale` into `dir`, rewriting the file only when it differs.
fn workload(scale: generate::Scale, dir: &Path) -> (Workload, PathBuf) {
    let bytes = generate::generate(scale);
    let path = dir.join(format!("properties-{}-walls.ifc", scale.walls));
    if std::fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
        std::fs::create_dir_all(dir).expect("create the data directory");
        std::fs::write(&path, &bytes).expect("write the generated workload");
    }
    let workload = Workload {
        name: format!("{} ({} walls)", scale.name, scale.walls),
        walls: scale.walls,
        bytes,
    };
    (workload, path)
}

fn main() {
    let options = options();
    let before = report::load();
    let env = report::environment(options.plan, false);
    let mut recorder = report::Recorder {
        plan: options.plan,
        records: Vec::new(),
    };
    for name in &options.scales {
        let scale =
            generate::scale(name).unwrap_or_else(|| usage(&format!("unknown scale {name}")));
        let (workload, path) = workload(scale, &options.data_dir);
        let mut hash = checksum::Fnv::new();
        hash.bytes(&workload.bytes);
        // Eagerly, so the benchmarks measure resolution, never decoding.
        let model = ifc_step::StepReader::new(ifc_step::ParseOptions::strict())
            .eager()
            .read_bytes(&workload.bytes)
            .unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        assert!(model.diagnostics().is_empty(), "the workload reads cleanly");
        eprintln!(
            "{}: {} bytes (FNV-1a {:016x}), {} entities, measuring",
            workload.name,
            workload.bytes.len(),
            hash.finish(),
            model.len()
        );
        resolve::run(&workload, &model, &options.benches, &mut recorder);
    }
    let after = report::load();
    let load = (before.as_str(), after.as_str());
    print!("{}", report::markdown(&env, load, &recorder.records));
    if let Some(path) = &options.json {
        report::write_json(path, &env, load, &recorder.records)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}
