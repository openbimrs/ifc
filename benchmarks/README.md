# Cross-implementation parse benchmark

Parse cost against two other IFC readers on identical files, so a claim about
this parser can be checked rather than believed.

## What is compared

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

## Results

Median of 3 runs, release build, one process per measurement, on one machine
(x86-64, glibc malloc). Memory is resident growth over the pre-parse baseline.

| file | entities | openbim/ifc | ifc-lite scan | ifc-lite decode | ifcopenshell 0.8.5 |
|---|---|---|---|---|---|
| 1 MB | 18,008 | 0.02 s / 5 MB | 0.00 s / 0 MB | 0.02 s / 6 MB | 0.08 s / 16 MB |
| 10 MB | 180,008 | 0.21 s / 49 MB | 0.01 s / 0 MB | 0.19 s / 55 MB | 0.88 s / 134 MB |
| 100 MB | 1,800,008 | 3.02 s / 488 MB | 0.20 s / 0 MB | 1.92 s / 548 MB | 10.60 s / 1303 MB |
| 513 MB | 9,000,008 | 18.16 s / 2366 MB | 2.05 s / 0 MB | 17.14 s / 2750 MB | 54.34 s / 6444 MB |

## Reading the numbers

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

## Caveats

The fixture is synthetic and uniform: walls with placements, shape
representations and property sets. A real export has a wider type mix, deeper
aggregates and more text. Treat these as same-shape-different-size
measurements, not as a claim about any particular authoring tool output.

One machine, one allocator, no CPU pinning. Ratios between implementations
are the durable part; absolute seconds are not.

## Running it

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
