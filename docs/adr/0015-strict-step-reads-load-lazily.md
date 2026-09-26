# 0015 — Strict STEP reads validate eagerly and decode lazily

- **Status:** Accepted
- **Date:** 2026-09-26
- **Deciders:** openbim-ifc maintainers
- **Supersedes:** —

## Context

Reading an IFC file built every attribute of every record into an owned
`Value` graph before the caller could look at anything. On real exports the
allocator was 41% of that read and `Model::insert` another 12% (profile of
2026-09-25, #49). Most consumers look at a fraction of a file first: a type
census, one storey, the properties of a selection. ifc-lite's advantage on
such workloads is exactly that it does not do this work up front.

The strict reader's contract is not negotiable, though: a file it accepts
is valid STEP whose every record the IFC record model can represent, and a
file it refuses is refused at read time with the eager reader's error. A
lazy model that discovered a malformed record only when someone touched it
would move errors from the read into arbitrary later calls that return
`Option<&Entity>` and cannot report them.

`openbim-step` 0.8.0 added `scan` (frame every record, no decoding) and
`decode_record` (parse one record from its span), with the invariant that
both succeed for every record exactly when a whole-file parse succeeds.

## Decision

A strict STEP read **validates every record eagerly and decodes every
entity lazily**.

- `ifc-step` frames the file with `scan`, parses each record from its span
  and checks it with `parser::validate`, which shares every fallible step
  with the eager `parser::convert`. Validation runs on up to 8 threads for
  inputs of 4 MB and more. No value is built.
- The model stores a slot per entity: a byte span plus a write-once cell.
  `ifc-model` defines `EntitySource`; the codec implements it. `Model::get`
  decodes on first access (thread-safe, the reference stays stable).
  `ifc-model` still knows nothing about STEP.
- If anything fails, the whole file goes through the eager reader, so the
  caller sees the eager reader's error, and a disagreement could only cost
  time.
- The model owns its source. `read_path` reads into an owned buffer; memory
  mapping is an explicit `unsafe` opt-in (`read_path_mapped`,
  `open_mapped` in the bindings) because a lazy model decodes from its
  source for its whole lifetime and a mapped file changed meanwhile is
  undefined behaviour.
- Recovery and reference-check options, and `StepReader::eager`, keep the
  eager read.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Frame only, validate on access (ifc-lite's model) | Moves read errors into later calls that cannot report them; a strict reader would accept broken files. |
| Keep eager, speed up the allocator (#49) | Worth doing (rusty_alloc, ifc#72) but only trims allocation cost; the work itself remains. |
| A separate lazy model type beside `Model` | Every projection borrows `&Model`; a second type would split the domain crates or force copies. |
| Memory-map by default | Unsound for a model that outlives the read, as above; the win is one copy of the file. |

## Consequences

**Positive**

- Opening a file: 2.1-4.6x faster at 35-76% less peak memory on seven real
  IFC files (18-109 MB). Consumers that touch a subset keep the savings.
- Decoding everything afterwards with `decode_all(8)` is still 1.3-2.5x
  faster than the eager read at the same memory.
- Identical results: lazy and eager models and errors match on the
  2,273-file parse-limits corpus (1,912 models, 361 errors), and every
  model there loaded lazily.

**Negative / costs**

- A model holds its source bytes: a consumer that decodes everything ends
  with the source plus the decoded graph, a few percent more than an eager
  read (use `StepReader::eager` to release the source).
- Decoding everything on one thread afterwards is 5-30% slower than the
  eager read on most files (a parser per record).
- A read of borrowed bytes (`read_bytes`) copies them once.

**Follow-ups / risks to watch**

- The sequential fill of the model (hash-map inserts) now dominates the
  lazy read's critical path; a position-indexed slot table could halve it.
- A validation-only parse in `openbim-step` (no parameter trees) would cut
  the remaining CPU cost of the read.

## Relation to existing code

- `ifc-model/src/lazy.rs`, `Model::{with_source, insert_lazy, decode_all}`.
- `ifc-step/src/lazy.rs` (load), `parser::{validate, convert}` (shared
  checks), `codec.rs` (policy), `index.rs` (framing-only index on `scan`).
- `ifc-step/tests/lazy_read.rs` (agreement, laziness, edits, clones,
  concurrency, entry points).
