//! Authoring cost against model size (#587), in the harness of the codec
//! and property baselines (#14, #119, #352). See `benchmarks/README.md`
//! for the method and `benchmarks/baseline.md` for the measured
//! comparison.
//!
//! ```text
//! cargo bench -p openbim-ifc --features authoring,ifc4x3 --bench authoring -- \
//!     [--scale author-1k,author-10k,author-100k] [--bench-only NAME,...]
//!     [--samples N] [--warmup N] [--json out.json]
//! cargo test -p openbim-ifc --features authoring,ifc4x3 --bench authoring   # the smoke run
//! ```
//!
//! | Benchmark | Measures |
//! | --- | --- |
//! | `authoring.batch.walls` | `apply_authoring` of N walls (ten operations, thirteen entities each) in one batch on the scene |
//! | `authoring.batch.typed_walls` | `apply_authoring` of N bare walls, each contained in the storey and typed by one wall type, in one batch: N joins of one relationship |
//! | `authoring.one_wall.into_n` | `Inverse::record` of `apply_authoring` of one wall into a model of N walls |
//! | `authoring.undo.batch` | `Inverse::apply` of the inverse of the N-wall batch |
//! | `authoring.preview.one_wall` | `Model::preview` of `apply_authoring` of one wall into a model of N walls: the change is looked at and rolled back, the revision included (#595) |
//! | `authoring.preview.clone` | `Model::clone` of the model of N walls and `apply_authoring` of one wall into the clone: what a preview cost before `Model::preview` |
//! | `authoring.index.build` | the first `reverse_index` and `global_id_candidates` of a model of N walls: the one-off cost of the indexes a model keeps, with the heap they retain |
//!
//! The model of N walls is replicated from one authored wall (see
//! `walls.rs`); on the smallest scales it is checked against walls authored
//! through `apply_authoring`. Before any timing, the entity count and a
//! checksum of the content are asserted, and the sampling, statistics,
//! allocator and report modules of the `ifc-step` harness are compiled in
//! by path, so `benchmarks/baseline.py` pools and compares the JSON
//! unchanged.

#[path = "../../../ifc-step/benches/baseline/alloc.rs"]
mod alloc;
#[path = "../../../ifc-step/benches/baseline/report.rs"]
mod report;
#[path = "../../../ifc-step/benches/baseline/stats.rs"]
mod stats;
mod walls;

use std::cell::RefCell;
use std::fmt::Write;
use std::path::PathBuf;

use ifc::{apply_authoring, Inverse, Model};

use report::{Record, Recorder};

#[global_allocator]
static GLOBAL: alloc::Counting = alloc::Counting;

const BATCH: &str = "authoring.batch.walls";
const ONE_WALL: &str = "authoring.one_wall.into_n";
const TYPED: &str = "authoring.batch.typed_walls";
const UNDO: &str = "authoring.undo.batch";
const INDEX: &str = "authoring.index.build";
const PREVIEW: &str = "authoring.preview.one_wall";
const CLONE: &str = "authoring.preview.clone";

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
}

/// The model's type names and attributes in file order, hashed.
fn checksum(model: &Model) -> u64 {
    let mut hash = Fnv::new();
    let mut text = String::new();
    for (id, entity) in model.iter() {
        text.clear();
        let _ = write!(text, "{id} {} {:?}", entity.type_name, entity.attributes);
        hash.bytes(text.as_bytes());
    }
    hash.0
}

struct Scale {
    name: &'static str,
    walls: usize,
}

const SCALES: [Scale; 4] = [
    Scale {
        name: "author-smoke",
        walls: 50,
    },
    Scale {
        name: "author-1k",
        walls: 1_000,
    },
    Scale {
        name: "author-10k",
        walls: 10_000,
    },
    Scale {
        name: "author-100k",
        walls: 100_000,
    },
];

struct Options {
    measure: bool,
    scales: Vec<String>,
    benches: Vec<String>,
    plan: stats::Plan,
    json: Option<PathBuf>,
}

fn usage(message: &str) -> ! {
    eprintln!("error: {message}");
    eprintln!(
        "usage: authoring [--bench] [--scale author-smoke,author-1k,author-10k,author-100k] \
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
            // The shared run script passes a data directory; nothing is
            // written here.
            "--data-dir" => drop(args.next()),
            other => usage(&format!("unexpected argument {other}")),
        }
    }
    if options.scales.is_empty() {
        options.scales = if options.measure {
            vec!["author-1k".into(), "author-10k".into()]
        } else {
            vec!["author-smoke".into()]
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

fn wanted(options: &Options, bench: &str) -> bool {
    options.benches.is_empty() || options.benches.iter().any(|name| name == bench)
}

fn main() {
    let options = options();
    let before = report::load();
    let env = report::environment(options.plan, false);
    let mut out = Recorder {
        plan: options.plan,
        records: Vec::new(),
    };
    let schema = ifc::schema::ifc4x3();
    for name in &options.scales {
        let scale = SCALES
            .iter()
            .find(|scale| scale.name == name)
            .unwrap_or_else(|| usage(&format!("unknown scale {name}")));
        let walls = scale.walls;
        let workload = format!("{} ({walls} walls)", scale.name);
        if walls <= 1_000 {
            walls::check_replication(walls);
        }

        // The scene alone, and the same scene with the batch applied.
        let mut scene_model = Model::new();
        let scene = walls::scene(&mut scene_model);
        let scene_len = scene_model.len();
        let scene_sum = checksum(&scene_model);
        let batch = walls::ops(&scene, 0..walls, "w");
        let expected_len = scene_len + walls::ENTITIES_PER_WALL * walls;

        let record = |bench, entities, samples, memory, checksum| Record {
            workload: workload.clone(),
            bench,
            entities,
            bytes: 0,
            samples,
            memory,
            checksum,
        };

        // Batch and undo share one run of the batch.
        let needs_batch = wanted(&options, BATCH) || wanted(&options, UNDO);
        let mut after_batch = None;
        if needs_batch {
            eprintln!("{workload}: batch");
            let mut model = scene_model.clone();
            let (applied, inverse) =
                Inverse::record(&mut model, |m| apply_authoring(m, schema, &batch, 2));
            applied.expect("the batch applies");
            assert_eq!(model.len(), expected_len, "scene + 13 entities per wall");
            let sum = checksum(&model);
            if wanted(&options, BATCH) {
                let samples = stats::time(
                    out.plan,
                    || scene_model.clone(),
                    |mut m| {
                        let applied = apply_authoring(&mut m, schema, &batch, 2);
                        (m, applied)
                    },
                    |(m, applied)| {
                        assert!(applied.is_ok());
                        assert_eq!(m.len(), expected_len);
                    },
                );
                // What the batch leaves behind: the model's growth.
                let (memory, grown) = alloc::measure(scene_model.clone(), |mut m| {
                    apply_authoring(&mut m, schema, &batch, 2).expect("the batch applies");
                    m
                });
                drop(grown);
                out.push(record(BATCH, expected_len, samples, Some(memory), sum));
            }
            after_batch = Some((model, inverse));
        }
        if wanted(&options, UNDO) {
            eprintln!("{workload}: undo");
            let (model, inverse) = after_batch.as_ref().expect("the batch ran");
            let samples = stats::time(
                out.plan,
                || (model.clone(), inverse.clone()),
                |(mut m, inverse)| {
                    let undone = inverse.apply(&mut m);
                    (m, undone)
                },
                |(m, undone)| {
                    assert!(undone.is_ok());
                    assert_eq!(m.len(), scene_len);
                },
            );
            // The undo leaves the scene as it was.
            let mut check = model.clone();
            inverse.clone().apply(&mut check).expect("the batch undoes");
            assert_eq!(checksum(&check), scene_sum, "undo restores the scene");
            out.push(record(UNDO, expected_len, samples, None, scene_sum));
        }
        drop(after_batch);

        if wanted(&options, TYPED) {
            eprintln!("{workload}: typed walls");
            let typed = walls::typed_ops(&scene, walls);
            // The type, its relationship, and a wall with its containment
            // relationship each.
            let expected = scene_len + 2 + 2 * walls;
            let mut model = scene_model.clone();
            apply_authoring(&mut model, schema, &typed, 2).expect("the typed batch applies");
            assert_eq!(
                model.len(),
                expected,
                "scene + type + relationship + 2 per wall"
            );
            let samples = stats::time(
                out.plan,
                || scene_model.clone(),
                |mut m| {
                    let applied = apply_authoring(&mut m, schema, &typed, 2);
                    (m, applied)
                },
                |(m, applied)| {
                    assert!(applied.is_ok());
                    assert_eq!(m.len(), expected);
                },
            );
            out.push(record(TYPED, expected, samples, None, checksum(&model)));
        }
        if wanted(&options, INDEX) {
            eprintln!("{workload}: index build");
            let (model, _) = walls::model(walls);
            let first_wall = model.ids_of_type("IfcWall")[0];
            let probe = match model
                .get(first_wall)
                .and_then(|wall| wall.attributes.first())
            {
                Some(ifc::Value::Text(text)) => ifc::Guid::parse(text),
                _ => None,
            }
            .expect("a wall holds a GlobalId");
            let build = |mut m: Model| {
                m.drop_derived_indexes();
                let referrers = m.reverse_index().len();
                let holders = m.global_id_candidates(&probe).len();
                (m, referrers, holders)
            };
            let samples = stats::time(
                out.plan,
                || model.clone(),
                build,
                |(_, referrers, holders)| {
                    assert!(*referrers > 0);
                    assert_eq!(*holders, 1, "the probe wall holds its GlobalId");
                },
            );
            let (memory, built) = alloc::measure(model.clone(), |mut m| {
                m.drop_derived_indexes();
                m.reverse_index();
                m.global_id_candidates(&probe);
                m
            });
            drop(built);
            out.push(record(
                INDEX,
                expected_len,
                samples,
                Some(memory),
                checksum(&model),
            ));
        }

        if wanted(&options, ONE_WALL) {
            eprintln!("{workload}: one wall");
            let (model, wall_scene) = walls::model(walls);
            assert_eq!(model.len(), expected_len, "scene + 13 entities per wall");
            let sum = checksum(&model);
            let one = walls::ops(&wall_scene, 0..1, "x");
            let model = RefCell::new(model);
            let apply = |m: &mut Model| apply_authoring(m, schema, &one, 5);
            // Warm: one recorded apply and its undo.
            let (applied, inverse) = Inverse::record(&mut model.borrow_mut(), apply);
            applied.expect("the wall applies");
            inverse.apply(&mut model.borrow_mut()).expect("it undoes");
            let samples = stats::time(
                out.plan,
                || (),
                |()| Inverse::record(&mut model.borrow_mut(), apply),
                |(applied, inverse)| {
                    assert!(applied.is_ok());
                    // Free the seed and the GlobalIds again.
                    inverse
                        .clone()
                        .apply(&mut model.borrow_mut())
                        .expect("it undoes");
                },
            );
            assert_eq!(checksum(&model.borrow()), sum, "the samples left no trace");
            out.push(record(ONE_WALL, expected_len, samples, None, sum));
        }

        if wanted(&options, PREVIEW) || wanted(&options, CLONE) {
            let (model, wall_scene) = walls::model(walls);
            assert_eq!(model.len(), expected_len, "scene + 13 entities per wall");
            let sum = checksum(&model);
            let one = walls::ops(&wall_scene, 0..1, "x");
            let revision = model.revision();
            let model = RefCell::new(model);
            let apply = |m: &mut Model| apply_authoring(m, schema, &one, 5);
            // Both benches start from a model that keeps its indexes.
            {
                let model = model.borrow();
                model.reverse_index();
                let probe = ifc::Guid::parse("0000000000000000000001").expect("a GlobalId");
                model.global_id_candidates(&probe);
            }
            if wanted(&options, PREVIEW) {
                eprintln!("{workload}: preview one wall");
                // Warm: one preview of the apply.
                model.borrow_mut().preview(apply).expect("the wall applies");
                let samples = stats::time(
                    out.plan,
                    || (),
                    |()| model.borrow_mut().preview(apply),
                    |applied| assert!(applied.is_ok()),
                );
                assert_eq!(checksum(&model.borrow()), sum, "the samples left no trace");
                assert_eq!(model.borrow().revision(), revision, "nor a revision");
                out.push(record(PREVIEW, expected_len, samples, None, sum));
            }
            if wanted(&options, CLONE) {
                eprintln!("{workload}: clone one wall");
                let samples = stats::time(
                    out.plan,
                    || (),
                    |()| {
                        let mut copy = model.borrow().clone();
                        let applied = apply(&mut copy);
                        (copy, applied)
                    },
                    |(copy, applied)| {
                        assert!(applied.is_ok());
                        assert_eq!(copy.len(), expected_len + walls::ENTITIES_PER_WALL);
                    },
                );
                assert_eq!(checksum(&model.borrow()), sum, "the original is untouched");
                out.push(record(CLONE, expected_len, samples, None, sum));
            }
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
