//! Meshing a subset against model size (#608), in the harness of the codec
//! and authoring baselines. See `benchmarks/README.md` for the method and
//! `benchmarks/baseline.md` for the measured comparison.
//!
//! ```text
//! cargo bench -p openbim-ifc --features authoring,ifc4x3,mesh --bench mesh_subset -- \
//!     [--scale mesh-1k,mesh-10k,mesh-50k] [--bench-only NAME,...]
//!     [--samples N] [--warmup N] [--json out.json]
//! cargo test -p openbim-ifc --features authoring,ifc4x3,mesh --bench mesh_subset   # the smoke run
//! ```
//!
//! | Benchmark | Measures |
//! | --- | --- |
//! | `mesh.subset.one_wall` | `product_meshes` of the middle wall of a model of N walls, none of them styled |
//! | `mesh.subset.one_wall_material` | the same on a model whose N walls each have an `IfcRelAssociatesMaterial` to one coloured material |
//! | `mesh.preview.one_wall` | `Model::preview` of `apply_authoring` of one wall into a model of N walls, then `product_meshes` of that wall inside the preview; the change is rolled back, the revision included |
//!
//! The model of N walls is replicated from one authored wall (see
//! `../authoring/walls.rs`). Before any timing, the entity count is
//! asserted, the colour resolver is shown to accept the model, and the
//! mesh and its colour are checked; the harness modules of `ifc-step` are
//! compiled in by path, so `benchmarks/baseline.py` pools and compares the
//! JSON unchanged.

// The harness modules are shared; this bench does not measure heap growth.
#[allow(dead_code)]
#[path = "../../../ifc-step/benches/baseline/alloc.rs"]
mod alloc;
mod material;
#[path = "../../../ifc-step/benches/baseline/report.rs"]
mod report;
#[path = "../../../ifc-step/benches/baseline/stats.rs"]
mod stats;
// The authoring bench's workload; this bench uses the scene, the wall and
// the replicated model, not the typed batch.
#[allow(dead_code)]
#[path = "../authoring/walls.rs"]
mod walls;

use std::cell::RefCell;
use std::fmt::Write;
use std::path::PathBuf;

use ifc::{apply_authoring, product_meshes, ColourResolver, EntityId, Model};

use report::{Record, Recorder};

#[global_allocator]
static GLOBAL: alloc::Counting = alloc::Counting;

const SUBSET: &str = "mesh.subset.one_wall";
const MATERIAL: &str = "mesh.subset.one_wall_material";
const PREVIEW: &str = "mesh.preview.one_wall";

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
        name: "mesh-smoke",
        walls: 50,
    },
    Scale {
        name: "mesh-1k",
        walls: 1_000,
    },
    Scale {
        name: "mesh-10k",
        walls: 10_000,
    },
    Scale {
        name: "mesh-50k",
        walls: 50_000,
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
        "usage: mesh_subset [--bench] [--scale mesh-smoke,mesh-1k,mesh-10k,mesh-50k] \
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
            vec!["mesh-1k".into(), "mesh-10k".into()]
        } else {
            vec!["mesh-smoke".into()]
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

/// The model of `walls` walls, declared IFC4X3 ADD2, keeping its reverse
/// index, and the middle wall.
fn model(walls: usize) -> (Model, walls::Scene, EntityId) {
    let (mut model, scene) = walls::model(walls);
    model.header_mut().schema = vec!["IFC4X3_ADD2".into()];
    model.reverse_index();
    let middle = model.ids_of_type("IfcWall")[walls / 2];
    (model, scene, middle)
}

/// The mesh of `wall` alone: non-empty, with `colour` as its colour.
fn assert_wall_mesh(model: &Model, wall: EntityId, grey: bool) {
    let meshes = product_meshes(model, Some(&[wall]));
    assert_eq!(meshes.len(), 1);
    let mesh = meshes[0].1.as_ref().expect("the wall meshes");
    assert!(!mesh.positions.is_empty(), "the wall has a body");
    match mesh.colour.as_ref().expect("the colour resolves") {
        Some(colour) => {
            assert!(grey, "an unstyled wall has no colour");
            assert_eq!(colour.rgb, [0.5; 3]);
        }
        None => assert!(!grey, "the material colours the wall"),
    }
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
        let record = |bench, entities, samples, checksum| Record {
            workload: workload.clone(),
            bench,
            entities,
            bytes: 0,
            samples,
            memory: None,
            checksum,
        };

        if wanted(&options, SUBSET) || wanted(&options, PREVIEW) {
            let (model, scene, wall) = model(walls);
            let expected_len = model.len();
            assert!(
                ColourResolver::new(&model).is_ok(),
                "the resolver accepts it"
            );
            assert_wall_mesh(&model, wall, false);
            let sum = checksum(&model);
            if wanted(&options, SUBSET) {
                eprintln!("{workload}: one wall");
                let samples = stats::time(
                    out.plan,
                    || (),
                    |()| product_meshes(&model, Some(&[wall])),
                    |meshes| assert!(meshes[0].1.is_ok()),
                );
                out.push(record(SUBSET, expected_len, samples, sum));
            }
            if wanted(&options, PREVIEW) {
                eprintln!("{workload}: preview one wall");
                let one = walls::ops(&scene, 0..1, "x");
                let revision = model.revision();
                let model = RefCell::new(model);
                let meshed = |m: &mut Model| {
                    let out = apply_authoring(m, schema, &one, 5).expect("the wall applies");
                    let new = out.ids[5].expect("the wall is created");
                    product_meshes(m, Some(&[new]))
                };
                // Warm, and check what the preview shows.
                let shown = model.borrow_mut().preview(meshed);
                assert!(shown[0].1.as_ref().is_ok_and(|m| !m.positions.is_empty()));
                let samples = stats::time(
                    out.plan,
                    || (),
                    |()| model.borrow_mut().preview(meshed),
                    |shown| assert!(shown[0].1.is_ok()),
                );
                assert_eq!(checksum(&model.borrow()), sum, "the samples left no trace");
                assert_eq!(model.borrow().revision(), revision, "nor a revision");
                out.push(record(PREVIEW, expected_len, samples, sum));
            }
        }

        if wanted(&options, MATERIAL) {
            eprintln!("{workload}: one wall, material");
            let (mut model, scene, wall) = model(walls);
            let plain = model.len();
            material::colour_walls(&mut model, &scene);
            assert_eq!(model.len(), plain + material::added(walls));
            let wall_count = model.ids_of_type("IfcWall").len();
            assert_eq!(wall_count, walls);
            assert!(
                ColourResolver::new(&model).is_ok(),
                "the resolver accepts it"
            );
            assert_wall_mesh(&model, wall, true);
            let samples = stats::time(
                out.plan,
                || (),
                |()| product_meshes(&model, Some(&[wall])),
                |meshes| assert!(meshes[0].1.is_ok()),
            );
            out.push(record(MATERIAL, model.len(), samples, checksum(&model)));
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
