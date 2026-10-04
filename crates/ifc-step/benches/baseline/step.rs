//! STEP codec baselines (#119): eager, lazy and mapped reads, `decode_all`
//! and write.
//!
//! Before anything is timed, every read path is run once and the results
//! are compared: the eager model, the lazy model after `decode_all`, the
//! mapped model after `decode_all`, and the eager re-read of the written
//! file must have one content checksum, and writing the re-read model must
//! reproduce the written bytes. Only then are the timings meaningful,
//! because only then do the paths do the same work.

use crate::alloc;
use crate::checksum;
use crate::report::{Record, Recorder};
use crate::stats::time;
use crate::Workload;
use ifc_model::{Codec, Model};
use ifc_step::{ParseOptions, StepCodec, StepReader};
use std::hint::black_box;

/// What the verification pass established, for the per-sample checks.
pub(crate) struct Verified {
    /// The eagerly read model, reused by the model benchmarks.
    pub(crate) model: Model,
    pub(crate) entities: usize,
    pub(crate) checksum: u64,
    written: Vec<u8>,
}

fn eager() -> StepReader {
    StepReader::new(ParseOptions::strict()).eager()
}

fn lazy_read(bytes: &[u8]) -> Model {
    StepCodec.read_bytes(bytes).expect("lazy read")
}

fn mapped_read(workload: &Workload) -> Model {
    // SAFETY: the workload file is a committed fixture or a file this
    // binary generated and verified; nothing modifies it while it runs.
    unsafe { StepReader::new(ParseOptions::strict()).read_path_mapped(&workload.path) }
        .expect("mapped read")
}

fn write(model: &Model, capacity: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(capacity);
    StepCodec.write(model, &mut out).expect("write");
    out
}

/// Runs every read path once and asserts they agree.
///
/// # Panics
///
/// When any two paths disagree, or a lazy read decoded anything up front.
pub(crate) fn verify(workload: &Workload) -> Verified {
    let model = eager().read_bytes(&workload.bytes).expect("eager read");
    let entities = model.len();
    let checksum = checksum::model(&model);

    let lazy = lazy_read(&workload.bytes);
    assert_eq!(
        lazy.decoded_len(),
        0,
        "{}: the lazy read decoded",
        workload.name
    );
    lazy.decode_all(8);
    assert_eq!(
        checksum::model(&lazy),
        checksum,
        "{}: lazy != eager",
        workload.name
    );

    let mapped = mapped_read(workload);
    assert_eq!(
        mapped.decoded_len(),
        0,
        "{}: the mapped read decoded",
        workload.name
    );
    mapped.decode_all(1);
    assert_eq!(
        checksum::model(&mapped),
        checksum,
        "{}: mapped != eager",
        workload.name
    );

    let written = write(&model, workload.bytes.len());
    let reread = eager().read_bytes(&written).expect("re-read");
    assert_eq!(
        checksum::model(&reread),
        checksum,
        "{}: write lost content",
        workload.name
    );
    assert_eq!(
        write(&reread, written.len()),
        written,
        "{}: write unstable",
        workload.name
    );

    Verified {
        model,
        entities,
        checksum,
        written,
    }
}

/// Times and measures every codec operation on `workload`.
pub(crate) fn run(workload: &Workload, verified: &Verified, out: &mut Recorder) {
    let bytes = workload.bytes.as_slice();
    let n = verified.entities;
    let plan = out.plan;
    let record = |bench, samples, memory, checksum| Record {
        workload: workload.name.clone(),
        bench,
        entities: n,
        bytes: bytes.len(),
        samples,
        memory,
        checksum,
    };

    // Reads. A lazy model must still be undecoded when the read returns.
    let samples = time(
        plan,
        || (),
        |()| eager().read_bytes(black_box(bytes)).expect("eager read"),
        |model| assert_eq!(model.len(), n),
    );
    let (memory, model) = alloc::measure(bytes, |b| eager().read_bytes(b).expect("eager read"));
    drop(model);
    out.push(record(
        "step.read.eager",
        samples,
        Some(memory),
        verified.checksum,
    ));

    let lazy_check = |model: &Model| {
        assert_eq!(model.len(), n);
        assert_eq!(model.decoded_len(), 0);
    };
    let samples = time(plan, || (), |()| lazy_read(black_box(bytes)), lazy_check);
    let (memory, model) = alloc::measure(bytes, lazy_read);
    drop(model);
    out.push(record(
        "step.read.lazy",
        samples,
        Some(memory),
        verified.checksum,
    ));

    let samples = time(plan, || (), |()| mapped_read(workload), lazy_check);
    let (memory, model) = alloc::measure(workload, mapped_read);
    drop(model);
    out.push(record(
        "step.read.mapped",
        samples,
        Some(memory),
        verified.checksum,
    ));

    // decode_all on a fresh lazy model; the read is setup, not measured.
    for (bench, threads) in [("step.decode_all.1", 1), ("step.decode_all.8", 8)] {
        let samples = time(
            plan,
            || lazy_read(bytes),
            |model| {
                model.decode_all(threads);
                model
            },
            |model| assert_eq!(model.decoded_len(), n),
        );
        let (memory, model) = alloc::measure(lazy_read(bytes), |model| {
            model.decode_all(threads);
            model
        });
        drop(model);
        out.push(record(bench, samples, Some(memory), verified.checksum));
    }

    // Write of the decoded model into a buffer reserved up front, so the
    // measurement is serialisation rather than buffer regrowth.
    let len = verified.written.len();
    let samples = time(
        plan,
        || (),
        |()| write(&verified.model, len),
        |written| assert_eq!(written.len(), len),
    );
    let (memory, written) = alloc::measure(&verified.model, |model| write(model, len));
    assert_eq!(written, verified.written);
    drop(written);
    let sum = checksum::bytes(&verified.written);
    out.push(record("step.write", samples, Some(memory), sum));
}
