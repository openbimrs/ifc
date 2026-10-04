# Benchmarks

Two sets of measurements, each with the machine and method it was taken on:

- [Codec and entity-graph baselines](#codec-and-entity-graph-baselines):
  this repository's own harness, the reference a storage, index or codec
  change is compared against (#14, #119). Baseline: [`baseline.md`](baseline.md).
- [Cross-implementation parse benchmark](#cross-implementation-parse-benchmark):
  this parser against ifc-lite and IfcOpenShell on identical files.

## Codec and entity-graph baselines

A custom harness, `crates/ifc-step/benches/baseline/`, run by `cargo bench`.
Not criterion: no workspace crate uses it, its dependency tree is large,
and the harness needs a counting global allocator and run-to-run pooling
that criterion does not give. It lives in `ifc-step` because that is the
lowest crate that can both read a file and see the `Model`.

### What is measured

| Benchmark | Measures | Issue |
| --- | --- | --- |
| `step.read.eager` | `StepReader::eager().read_bytes`: parse and decode every entity | #119 |
| `step.read.lazy` | `StepCodec.read_bytes`: validate every record, decode none (ADR 0015); includes the one copy of the input | #119 |
| `step.read.mapped` | `read_path_mapped` of the same file, page cache warm | #119 |
| `step.decode_all.1`, `.8` | `Model::decode_all` on 1 and 8 threads, on a fresh lazy model | #119 |
| `step.write` | `Codec::write` of the decoded model into a buffer reserved up front | #119 |
| `model.construct` | `Model::reserve` plus `Model::insert` of every entity, already decoded | #14 |
| `model.lookup` | `Model::get` of every id, in a fixed pseudo-random order | #14 |
| `model.iterate` | `Model::iter` over every entity in file order | #14 |
| `model.by_type` | `ids_of_type` for every type present, then `of_type` of the most common one | #14 |
| `model.traverse` | `breadth_first` along forward references from every unreferenced entity, each walk with its own visited set | #14 |
| `model.reverse_index` | `ReverseIndex::build` | #14 |

The model benchmarks run on the eagerly read model, so they measure the
graph and its indices, never decoding.

### Workloads

The three redistributable fixtures the wasm opt-level benchmark used (#303),
from `test/fixtures` (see `PROVENANCE.tsv`): `meshing_coverage.ifc` (230
entities, generated here), `issue_098_wall_W.ifc` (1,031) and
`shared_point_faceted_brep.ifc` (6,393, both MPL-2.0 from ifc-lite).

Synthetic models from [`generate-fixture.awk`](generate-fixture.awk), ported
to Rust in the bench (`synthetic.rs`) and checked on every run against the
SHA-256 of the awk output below, so the bytes measured are the bytes named.
Nothing generated is committed; the bench writes the files to `--data-dir`
(default: `CARGO_TARGET_TMPDIR`) for the mapped read.

| Scale | Walls (`n`) | Entities | Bytes | Why this size | SHA-256 of `awk -v n=<n> -f generate-fixture.awk` |
| --- | ---: | ---: | ---: | --- | --- |
| `tiny` | 4 | 48 | 2,358 | CI smoke run | `4ae4ff726f9f83b6982e5be48a125ea549a345ff57bb4f910f1cca9a6098f206` |
| `small` | 1,800 | 18,008 | 964,052 | decoded model (~6 MB) fits the 16 MiB last-level cache | `45057a95785845c418dd6d5fc89f261248977e7c24a2cbcfec1493f0b1dd8f0a` |
| `crossover` | 8,000 | 80,008 | 4,381,718 | just past the 4 MiB input at which the lazy read validates on several threads, and a decoded model (~28 MB) past the cache | `0de2d9d8d80768c864b55ac2802026acd349b888d1265e8388cfdf505331fb86` |
| `large` | 180,000 | 1,800,008 | 105,352,418 | far beyond every cache | `09409504da7bffec10b5ec490c19704459fa7ba8d9832267d6b6aa5c0ff3c9c2` |

The synthetic model is uniform (see the caveats below): ten entities per
wall, a shallow reference graph. The fixtures supply the type mix and deeper
geometry graphs it lacks; neither is a real authoring-tool export.

### Method

**Sampling.** Each sample times one call. Its input (a fresh lazy model for
`decode_all`, cloned entities for `construct`) is prepared before the clock
starts and its output dropped after it stops. Warm-up samples (default 3)
run the same code and are discarded; then 20 samples are taken (10 after 2
on `large`). Reported per benchmark: median, interquartile range, median
absolute deviation, min..max, and, across processes, the spread of the
per-run medians.

**Nothing is optimised away, and every path does the same work.** Inputs
and outputs pass through `std::hint::black_box`, and every sample's output
is asserted: entity count and decode state for reads, a content checksum
for the model operations, the exact byte length for the write. Before any
timing, each workload is read every way and compared: the eager model, the
lazy and the mapped model after `decode_all`, and the re-read written file
must have one content checksum (header schema, ids in order, type names and
every attribute value), writing the re-read model must reproduce the
written bytes, and a model rebuilt by `model.construct` must match too.
The checksum of each row is printed, so two runs that did different work
cannot be pooled or compared (`baseline.py` refuses).

**The extra-work probe.** `--probe` walks every traversal twice. The
expected checksum is computed independently of the flag, so the probe
must change both the asserted result (exactly 2x, which the smoke run
checks) and the time (about 2x). [`baseline.md`](baseline.md) records the
measured effect.

**Memory.** The bench binary installs a counting global allocator over the
system allocator. It is off during timing (one relaxed load per
allocation, no shared counter between threads) and on for one extra,
untimed call per benchmark, which reports:

- *heap retained*: live heap bytes after the call while its result is
  still held, net of anything the call freed: the size of what it returns;
- *heap peak*: the high-water mark during the call above the start.

Limits: it counts bytes requested through Rust's global allocator by every
thread of the process, and nothing else. Not counted: allocator overhead
and fragmentation (glibc headers, retained arenas, so RSS is higher), thread
stacks, and the file pages of a memory map, which sit in the page cache: a
mapped read's figures are its index and slots only. A `realloc` that moves
briefly holds both blocks; the peak counts only the difference.
`model.construct` moves entities that were allocated before the call, so
its retained figure is the storage and indices alone, not the entity
payload (`step.read.eager` has the total). No RSS is reported: freed heap
is not returned to the OS promptly, so an RSS delta inside one process
would mostly measure the allocator.

**Machine discipline.** Measure only on a quiet machine. `run-baseline.sh`
builds first and never builds while measuring, starts a run only when the
1-minute load is below 3, discards and repeats a run whose load at the end
is 4 or more (the bench adds about one), pins each run to 8 logical CPUs
with `taskset` (so `available_parallelism`, and with it the lazy read's
validation threads, is 8), runs every scale in 3 separate processes with
the rounds interleaved, and records the machine, OS, rustc, profile, commit,
date and the load before and after every run. Release settings throughout:
`cargo bench` uses the workspace release profile (opt-level 3, thin LTO,
one codegen unit); setup and compilation are never timed.

### Running it

```sh
# CI smoke run (part of scripts/gate.sh test): tiny inputs, all assertions,
# no timing judged. Takes a few seconds.
cargo test -p ifc-step --bench baseline

# One measurement run; prints a Markdown table and writes every sample.
cargo bench -p ifc-step --bench baseline -- \
    --scale fixtures,small,crossover,large --json run.json --data-dir /some/dir

# The extra-work probe: model.traverse should double, in time and checksum.
cargo bench -p ifc-step --bench baseline -- --scale small --probe

# The full baseline: 3 interleaved processes per scale behind the load gate,
# pooled into out-dir/summary.md.
benchmarks/run-baseline.sh out-dir
```

Large generated files belong on a disk, not in a RAM-backed `/tmp`: set
`IFC_BENCH_DATA` (or `--data-dir`) accordingly.

### Comparing a change against the baseline

Run `benchmarks/run-baseline.sh` on the base commit and on the change, on
the same machine, both in a quiet window, then:

```sh
benchmarks/baseline.py compare --base base-out/run*.json --new new-out/run*.json
```

It prints new/base per benchmark and calls a change only when the two
interquartile ranges do not overlap; otherwise the row reads "within
noise". The committed [`baseline.md`](baseline.md) is the reference for
this machine; a number from another machine compares against a fresh base
run there, never against the committed table. A claim of a speedup needs
that comparison in the pull request, not a green gate.

## Cross-implementation parse benchmark

Parse cost against two other IFC readers on identical files, so a claim about
this parser can be checked rather than believed.

### What is compared

The three do different amounts of work, and the difference is the point:

- **openbim/ifc** builds a full `Model`: every attribute decoded into an owned
  `Value`, indexed by id and by type, ready for random access and round-trip.
- **ifc-lite scan** (`build_entity_index`) records only `(id, byte start, byte
  length)` per entity. It never decodes attributes. This is not a slower or
  faster version of the same job -- it is a different job, and the honest
  comparison for it is "how fast can you learn what is in the file".
- **ifc-lite decode** scans and then decodes every entity, which is the closest
  shape to what openbim/ifc does eagerly.
- **ifcopenshell** `open()` plus materialising every instance.

All four agree on the entity count at every size, which is what makes the
timings comparable at all; a run that disagrees is measuring different work.

### Results

Median of 3 runs, release build, one process per measurement, on one machine
(x86-64, glibc malloc). Memory is resident growth over the pre-parse baseline.

| file | entities | openbim/ifc | ifc-lite scan | ifc-lite decode | ifcopenshell 0.8.5 |
|---|---|---|---|---|---|
| 1 MB | 18,008 | 0.02 s / 5 MB | 0.00 s / 0 MB | 0.02 s / 6 MB | 0.08 s / 16 MB |
| 10 MB | 180,008 | 0.21 s / 49 MB | 0.01 s / 0 MB | 0.19 s / 55 MB | 0.88 s / 134 MB |
| 100 MB | 1,800,008 | 3.02 s / 488 MB | 0.20 s / 0 MB | 1.92 s / 548 MB | 10.60 s / 1303 MB |
| 513 MB | 9,000,008 | 18.16 s / 2366 MB | 2.05 s / 0 MB | 17.14 s / 2750 MB | 54.34 s / 6444 MB |

### Reading the numbers

Against the two eager readers, openbim/ifc is consistently ahead: roughly 3x
faster than ifcopenshell with a third of the memory, and level with ifc-lite
decode on time while holding ~14% less memory.

The interesting column is the scan. At 513 MB it answers what is in the file
in 2.05 s holding nothing, against our 18.16 s holding 2.3 GB. For a consumer
that wants a type census, a filtered subset, or the first visible geometry,
eager decoding of all nine million entities is wasted work. That is an
architectural gap, not a constant factor.

Cost per MB is not flat for this parser: 20.0, 21.0, 30.2, 35.4 ms/MB across
the four sizes, while ifc-lite decode holds near 19 ms/MB until the largest
file. Memory per entity stays flat (276-291 B), so the slowdown is not the
model getting fatter. It is unexplained, and worth a profile before any
further optimisation is chosen on guesswork.

### Caveats

The fixture is synthetic and uniform: walls with placements, shape
representations and property sets. A real export has a wider type mix, deeper
aggregates and more text. Treat these as same-shape-different-size
measurements, not as a claim about any particular authoring tool output.

One machine, one allocator, no CPU pinning. Ratios between implementations
are the durable part; absolute seconds are not.

### Running it

```sh
# fixture: n is the wall count; ~570 bytes per wall
awk -v n=180000 -f benchmarks/generate-fixture.awk > /tmp/100mb.ifc

# ifcopenshell (needs: pip install ifcopenshell)
python3 benchmarks/run-ifcopenshell.py /tmp/100mb.ifc
```

The Rust side needs ifc-lite checked out (MPL-2.0,
github.com/LTplus-AG/ifc-lite) and is not wired into this workspace, so it
stays out of the gate: it would make the build depend on a third-party
checkout for no verification benefit.
