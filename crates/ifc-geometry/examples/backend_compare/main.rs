//! Compile one fixture corpus through several geometry backends and compare
//! them (#31, ADR 0012).
//!
//! ```text
//! # The caller-supplied backend alone: the contracts-only build, no engine.
//! cargo run -p ifc-geometry --features compile --example backend_compare
//!
//! # Against the reference backend, agreement checked.
//! cargo run --release -p ifc-geometry --features compile-reference-backend \
//!     --example backend_compare -- --iterations 20
//! ```
//!
//! Options: `--iterations N` (timed passes after the first, default 10),
//! `--quick` (2 passes, for the gate), `--relative-tolerance R` (agreement,
//! default 1e-6), `--strict` (exit 1 on any divergence), and `--fixture PATH`,
//! repeatable, to replace the corpus. The default corpus is read from the
//! repository's `test/fixtures/`, which the published crate does not carry;
//! a fixture that cannot be read is reported and skipped.
//!
//! # What it does
//!
//! Every backend is a value in one list, and one traversal runs them all:
//! parse each fixture once, find its products, and hand each product to
//! every backend through [`ifc_geometry::compile::compile_product_mesh_with`].
//! The only `cfg` is where the list is built: the reference backend joins
//! it when `compile-reference-backend` is on. The second backend,
//! `polygon-extruder` ([`polygon_kernel`]), is defined here, names only the
//! published Axiolid contracts, and builds under `compile` alone.
//!
//! A refusal by one backend is reported and the run goes on. Where two
//! backends both mesh a product, their signed volume, surface area and
//! bounding box are compared within a stated relative tolerance, and every
//! metric outside it is listed with the fixture, product and both values.
//!
//! # Methodology, and what the numbers are not
//!
//! Each fixture is parsed once, untimed. One pass compiles every product
//! of the fixture once; the first pass is timed and reported on its own
//! (cold caches, first allocations), then `--iterations` further passes are
//! timed and reported as median and min..max. Lowering the IFC into the
//! neutral graph happens inside every pass and is identical for each
//! backend; the difference between backends is compilation. Refused
//! products are timed too: refusing fast is still what that kernel did.
//!
//! The output prints the machine, profile and load beside the numbers. They
//! are **not** a cross-kernel performance claim. The two backends do not do
//! comparable work: `polygon-extruder` refuses most of what the reference
//! compiles, so its pass over a fixture is cheaper because it compiles
//! less. A per-fixture time is only comparable between backends that mesh
//! the same products, which the coverage columns show. Measure in a quiet
//! window (1-minute load below 3) with `--release`; a debug build, or a
//! quick run, proves the harness runs and nothing about speed.

mod harness;
mod polygon_kernel;
mod report;
mod triangulate;

use std::path::PathBuf;
use std::process::ExitCode;

use axiolid_core::Tolerance;
use ifc_geometry::geometric_products;
use ifc_model::Codec;
use ifc_step::StepCodec;

use harness::{Agreement, Contestant, MeshContestant};
use polygon_kernel::PolygonExtruder;

/// The committed corpus: straight extrusions, mapped items, tessellated
/// bodies, and constructs the example kernel refuses (CSG, sweeps along
/// curves, half-spaces, B-reps), so both agreement and refusal are exercised.
const CORPUS: &[&str] = &[
    "ifclite-geometry/issue_098_wall_W.ifc",
    "ifclite-geometry/issue_1985_scaled_kinds.ifc",
    "ifclite-geometry/issue_2019_wall_two_overlapping_openings.ifc",
    "ifclite-geometry/mapped_instances_multi_item.ifc",
    "ifclite-geometry/mapped_instances_nested.ifc",
    "ifclite-geometry/nested_mapped_item.ifc",
    "ifclite-geometry/mapped_instances_indexed_colour.ifc",
    "ifclite-geometry/bath_csg_solid.ifc",
    "ifclite-geometry/issue_1155_halfspace_flyaway.ifc",
    "ifclite-geometry/swept_disk_composite_arc_crankbar.ifc",
    "ifclite-geometry/shared_point_faceted_brep.ifc",
    "synthetic-coverage/meshing_coverage.ifc",
    "synthetic-lowering/indexed_profile_boundaries.ifc",
];

struct Options {
    iterations: usize,
    agreement: Agreement,
    strict: bool,
    fixtures: Vec<PathBuf>,
}

fn options() -> Result<Options, String> {
    let mut options = Options {
        iterations: 10,
        agreement: Agreement { relative: 1e-6 },
        strict: false,
        fixtures: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--quick" => options.iterations = 2,
            "--strict" => options.strict = true,
            "--iterations" => {
                options.iterations = value("--iterations")?
                    .parse()
                    .map_err(|e| format!("--iterations: {e}"))?;
            }
            "--relative-tolerance" => {
                options.agreement.relative = value("--relative-tolerance")?
                    .parse()
                    .map_err(|e| format!("--relative-tolerance: {e}"))?;
            }
            "--fixture" => options.fixtures.push(value("--fixture")?.into()),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    if options.fixtures.is_empty() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test/fixtures");
        options.fixtures = CORPUS.iter().map(|path| root.join(path)).collect();
    }
    Ok(options)
}

/// The backends under comparison. Selection happens here and only here.
fn contestants(tolerance: Tolerance) -> Vec<Box<dyn Contestant>> {
    let mut list = reference(tolerance);
    list.push(Box::new(MeshContestant::new(PolygonExtruder, tolerance)));
    list
}

/// The reference backend, when this build links it.
#[cfg(feature = "compile-reference-backend")]
fn reference(tolerance: Tolerance) -> Vec<Box<dyn Contestant>> {
    let backend = ifc_geometry::compile::default_backend();
    vec![Box::new(MeshContestant::new(backend, tolerance))]
}

/// A `compile`-only build links no reference engine: nothing to add.
#[cfg(not(feature = "compile-reference-backend"))]
fn reference(_tolerance: Tolerance) -> Vec<Box<dyn Contestant>> {
    Vec::new()
}

fn main() -> ExitCode {
    let options = match options() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("backend_compare: {message}");
            return ExitCode::from(2);
        }
    };
    let tolerance = Tolerance::MILLIMETRE;
    let contestants = contestants(tolerance);

    let mut report = report::Report::new(&contestants, options.iterations, options.agreement);
    for path in &options.fixtures {
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        let model = match std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| StepCodec.read_bytes(&bytes).map_err(|e| e.to_string()))
        {
            Ok(model) => model,
            Err(error) => {
                report.unreadable(&name, &error);
                continue;
            }
        };
        let products = geometric_products(&model);
        let runs = contestants
            .iter()
            .map(|contestant| {
                harness::run(contestant.as_ref(), &model, &products, options.iterations)
            })
            .collect::<Vec<_>>();
        report.fixture(&name, &model, &products, &runs);
    }
    let divergences = report.print();
    if options.strict && divergences > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
