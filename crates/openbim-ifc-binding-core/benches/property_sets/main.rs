//! Property sets of every object through the bindings (#358), in the
//! harness of the codec and property baselines (#14, #119, #352). See
//! `benchmarks/README.md` for the method and `benchmarks/baseline.md` for
//! the measured comparison.
//!
//! ```text
//! cargo bench -p openbim-ifc-binding-core --bench property_sets -- [--scale props-1k,props-10k,props-100k]
//!     [--bench-only NAME,...] [--samples N] [--warmup N] [--json out.json] [--data-dir DIR]
//! cargo test -p openbim-ifc-binding-core --bench property_sets     # the smoke run
//! ```
//!
//! | Benchmark | Measures |
//! | --- | --- |
//! | `bindings.property_sets_many.every_object` | `IfcModel::property_sets_many` of every wall: one call, one property index |
//! | `bindings.property_sets.every_object` | `IfcModel::property_sets` of every wall, one call each: what a host loop did before #358 |
//! | `bindings.property_sets.per_call.100` | `IfcModel::property_sets` of 100 walls spread over the model, each call on its own |
//!
//! The workload is the #352 property bench's generator, compiled in by
//! path, as are the sampling, statistics, allocator and report modules of
//! the `ifc-step` baseline harness; `benchmarks/baseline.py` pools and
//! compares the JSON unchanged. Before any timing, the batch's answers for
//! the 100 sampled walls are compared with the per-object call's, so the
//! batch can only be fast by being right. Each output is checksummed
//! outside the clock over the `Debug` form of every answer.
//!
//! `property_sets.every_object` is quadratic: at `props-10k` one pass takes
//! minutes and at `props-100k` hours, so it runs only when named with
//! `--bench-only`; `per_call.100` keeps the per-object cost measurable at
//! every scale.

#[path = "../../../ifc-step/benches/baseline/alloc.rs"]
mod alloc;
#[path = "../../../ifc-properties/benches/properties/generate.rs"]
mod generate;
#[path = "../../../ifc-step/benches/baseline/report.rs"]
mod report;
#[path = "../../../ifc-step/benches/baseline/stats.rs"]
mod stats;

use std::hint::black_box;
use std::path::PathBuf;

use openbim_ifc_binding_core::properties::{ObjectPropertySets, PropertySet};
use openbim_ifc_binding_core::{BindingError, IfcModel};

use report::{Record, Recorder};

#[global_allocator]
static GLOBAL: alloc::Counting = alloc::Counting;

/// 64-bit FNV-1a, as the codec harness's `checksum` module computes it.
struct Fnv(u64);

impl Fnv {
    const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    const fn finish(&self) -> u64 {
        self.0
    }
}

const MANY: &str = "bindings.property_sets_many.every_object";
const EACH: &str = "bindings.property_sets.every_object";
const PER_CALL: &str = "bindings.property_sets.per_call.100";

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
        "usage: property_sets [--bench] [--scale props-smoke,props-1k,props-10k,props-100k] \
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

/// Whether `bench` runs: the quadratic loop only when named, or in the
/// smoke run, where it is small.
fn wanted(options: &Options, bench: &str) -> bool {
    if options.benches.is_empty() {
        bench != EACH || !options.measure
    } else {
        options.benches.iter().any(|name| name == bench)
    }
}

/// Folds the `Debug` form of every answer into one checksum.
fn checksum<T: std::fmt::Debug>(answers: &[T]) -> u64 {
    let mut hash = Fnv::new();
    let mut text = String::new();
    for answer in answers {
        use std::fmt::Write;
        text.clear();
        let _ = write!(text, "{answer:?}");
        hash.bytes(text.as_bytes());
    }
    hash.finish()
}

fn many(model: &IfcModel, walls: &[u64]) -> Vec<ObjectPropertySets> {
    model
        .property_sets_many(Some(black_box(walls)))
        .expect("the workload resolves")
}

fn each(model: &IfcModel, walls: &[u64]) -> Vec<Result<Vec<PropertySet>, BindingError>> {
    walls
        .iter()
        .map(|&wall| model.property_sets(black_box(wall)))
        .collect()
}

fn main() {
    let options = options();
    let before = report::load();
    let env = report::environment(options.plan, false);
    let mut out = Recorder {
        plan: options.plan,
        records: Vec::new(),
    };
    for name in &options.scales {
        let scale =
            generate::scale(name).unwrap_or_else(|| usage(&format!("unknown scale {name}")));
        let bytes = generate::generate(scale);
        let path = options
            .data_dir
            .join(format!("properties-{}-walls.ifc", scale.walls));
        if std::fs::read(&path).ok().as_deref() != Some(bytes.as_slice()) {
            std::fs::create_dir_all(&options.data_dir).expect("create the data directory");
            std::fs::write(&path, &bytes).expect("write the generated workload");
        }
        let mut hash = Fnv::new();
        hash.bytes(&bytes);
        let model = IfcModel::parse(&bytes).expect("the workload parses");
        assert!(model.diagnostics().is_empty(), "the workload reads cleanly");
        let walls = model.ids_of_type("IfcWall");
        assert_eq!(walls.len(), scale.walls, "every wall was read");
        let workload = format!("{} ({} walls)", scale.name, scale.walls);
        eprintln!(
            "{workload}: {} bytes (FNV-1a {:016x}), {} entities, measuring",
            bytes.len(),
            hash.finish(),
            model.len()
        );
        let record = |bench, samples, memory, checksum| Record {
            workload: workload.clone(),
            bench,
            entities: model.len(),
            bytes: bytes.len(),
            samples,
            memory,
            checksum,
        };

        // The per-object answers for a hundred walls spread over the
        // model: the batch must give exactly these. This also decodes
        // every entity the answers touch, so no timing pays for decoding.
        let stride = walls.len().div_ceil(100).max(1);
        let sample: Vec<u64> = walls.iter().step_by(stride).copied().collect();
        let reference = each(&model, &sample);
        let batch = many(&model, &sample);
        for ((answer, expected), &wall) in batch.iter().zip(&reference).zip(&sample) {
            assert_eq!(answer.object, wall);
            assert_eq!(answer.refusal, None, "#{wall} resolves");
            assert_eq!(Ok(&answer.sets), expected.as_ref(), "#{wall}");
        }
        drop(many(&model, &walls));

        if wanted(&options, MANY) {
            let expected = checksum(&many(&model, &walls));
            let samples = stats::time(
                out.plan,
                || (),
                |()| many(&model, &walls),
                |answers| assert_eq!(answers.len(), walls.len()),
            );
            let (memory, answers) = alloc::measure((), |()| many(&model, &walls));
            drop(answers);
            out.push(record(MANY, samples, Some(memory), expected));
        }
        if wanted(&options, EACH) {
            let answers = each(&model, &walls);
            // The same sets as the batch, wall for wall.
            let batch = many(&model, &walls);
            assert!(answers
                .iter()
                .zip(&batch)
                .all(|(one, from_batch)| one.as_ref() == Ok(&from_batch.sets)));
            let expected = checksum(&answers);
            let samples = stats::time(
                out.plan,
                || (),
                |()| each(&model, &walls),
                |answers| assert_eq!(answers.len(), walls.len()),
            );
            out.push(record(EACH, samples, None, expected));
        }
        if wanted(&options, PER_CALL) {
            let expected = checksum(&reference);
            let samples = stats::time(
                out.plan,
                || (),
                |()| each(&model, &sample),
                |answers| assert_eq!(answers.len(), sample.len()),
            );
            out.push(record(PER_CALL, samples, None, expected));
        }
    }
    let after = report::load();
    let load = (before.as_str(), after.as_str());
    print!("{}", report::markdown(&env, load, &out.records));
    if let Some(path) = &options.json {
        report::write_json(path, &env, load, &out.records)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    }
}
