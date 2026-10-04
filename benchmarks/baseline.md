# Codec and entity-graph baseline

The reference measurement for #14 (entity graph) and #119 (STEP codec),
taken with the harness and method in [`README.md`](README.md#codec-and-entity-graph-baselines).
Compare a change against a fresh base run on the same machine with
`benchmarks/baseline.py compare`, not against these absolute numbers from
another machine.

**Quiet-window measurement.** Every run started with the 1-minute load below
3 and ended below 4; the per-run loads are listed below. Three earlier
attempts at the large scale (same build, same day) ended with the load at
4.9, 16.6 and 6.7, were discarded by the load gate and are not in the table.

## Environment

- date: 2026-10-04, 10:16-10:31 UTC
- harness: `crates/ifc-step/benches/baseline` at commit 371cf8cb (the run
  script recorded its parent, 70a1edb9, as the working tree was not yet
  committed; the measured sources are those of 371cf8cb)
- machine: Intel Xeon w7-3565X, 20 cores (1 thread per core), 16 MiB L3,
  63 GiB RAM, shared VM
- pinning: `taskset -c 12-19` (8 logical CPUs; `available_parallelism` = 8)
- OS: Debian GNU/Linux 13 (trixie), kernel 6.12.105+deb13-cloud-amd64,
  glibc malloc
- toolchain: rustc 1.88.0 (6b00bc388 2025-06-23)
- profile: `cargo bench` = workspace release profile (opt-level 3, thin LTO,
  1 codegen unit)
- plan: 3 warm-up then 20 samples per benchmark (large: 2 then 10), 3
  separate processes per scale, rounds interleaved
- command: `benchmarks/run-baseline.sh` with its defaults

Load (1/5/15 min) before and after each process:

- run1-fixtures.json: load before 2.58 5.29 6.24, after 2.58 5.29 6.24
- run2-fixtures.json: load before 2.56 4.06 5.52, after 2.56 4.06 5.52
- run3-fixtures.json: load before 2.61 3.53 4.98, after 2.61 3.53 4.98
- run1-small.json: load before 2.58 5.29 6.24, after 2.69 5.27 6.23
- run2-small.json: load before 2.56 4.06 5.52, after 2.56 4.06 5.52
- run3-small.json: load before 2.61 3.53 4.98, after 2.61 3.53 4.98
- run1-crossover.json: load before 2.69 5.27 6.23, after 2.90 5.22 6.21
- run2-crossover.json: load before 2.56 4.06 5.52, after 2.47 3.99 5.48
- run3-crossover.json: load before 2.61 3.53 4.98, after 3.38 3.67 5.00
- run1-large.json: load before 2.90 5.22 6.21, after 3.60 4.52 5.75
- run2-large.json: load before 2.47 3.99 5.48, after 3.53 3.86 5.17
- run3-large.json: load before 2.75 3.44 4.84, after 3.67 3.65 4.68

## Results

Median and interquartile range pool every sample of the three processes;
"run medians" is the range of the three per-process medians (run-to-run
noise). Heap figures come from the counting allocator, one untimed call
each; see the README for what they include and exclude. Lookup, iteration
and by-type queries allocate nothing to report.

| workload | entities | bench | median ms | IQR ms | MAD ms | run medians ms | heap retained MB | heap peak MB |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| meshing_coverage | 230 | step.read.eager | 0.1716 | 0.1700..0.1744 | 0.0024 | 0.1708..0.1718 | 0.09 | 0.11 |
| meshing_coverage | 230 | step.read.lazy | 0.1444 | 0.1413..0.1530 | 0.0043 | 0.1433..0.1521 | 0.06 | 0.07 |
| meshing_coverage | 230 | step.read.mapped | 0.1510 | 0.1496..0.1540 | 0.0018 | 0.1502..0.1516 | 0.04 | 0.06 |
| meshing_coverage | 230 | step.decode_all.1 | 0.1478 | 0.1462..0.1502 | 0.0019 | 0.1476..0.1486 | 0.04 | 0.05 |
| meshing_coverage | 230 | step.decode_all.8 | 0.1909 | 0.1776..0.2065 | 0.0141 | 0.1874..0.1961 | 0.04 | 0.05 |
| meshing_coverage | 230 | step.write | 0.1465 | 0.1446..0.1501 | 0.0023 | 0.1446..0.1486 | 0.01 | 0.06 |
| meshing_coverage | 230 | model.construct | 0.0207 | 0.0201..0.0212 | 0.0005 | 0.0206..0.0209 | 0.03 | 0.04 |
| meshing_coverage | 230 | model.lookup | 0.0022 | 0.0022..0.0022 | 0.0000 | 0.0022..0.0022 |  |  |
| meshing_coverage | 230 | model.iterate | 0.0022 | 0.0022..0.0022 | 0.0000 | 0.0022..0.0022 |  |  |
| meshing_coverage | 230 | model.by_type | 0.0028 | 0.0028..0.0029 | 0.0000 | 0.0028..0.0029 |  |  |
| meshing_coverage | 230 | model.traverse | 0.0158 | 0.0153..0.0164 | 0.0005 | 0.0157..0.0159 | 0.00 | 0.01 |
| meshing_coverage | 230 | model.reverse_index | 0.0256 | 0.0251..0.0262 | 0.0005 | 0.0254..0.0258 | 0.03 | 0.04 |
| issue_098_wall_W | 1,031 | step.read.eager | 0.6544 | 0.6473..0.6631 | 0.0077 | 0.6470..0.6592 | 0.35 | 0.39 |
| issue_098_wall_W | 1,031 | step.read.lazy | 0.5905 | 0.5851..0.5978 | 0.0064 | 0.5885..0.5921 | 0.23 | 0.37 |
| issue_098_wall_W | 1,031 | step.read.mapped | 0.6010 | 0.5920..0.6078 | 0.0082 | 0.5928..0.6043 | 0.17 | 0.31 |
| issue_098_wall_W | 1,031 | step.decode_all.1 | 0.6135 | 0.6075..0.6184 | 0.0056 | 0.6126..0.6177 | 0.17 | 0.19 |
| issue_098_wall_W | 1,031 | step.decode_all.8 | 0.3225 | 0.2981..0.3456 | 0.0246 | 0.3008..0.3359 | 0.17 | 0.19 |
| issue_098_wall_W | 1,031 | step.write | 0.6472 | 0.6387..0.6662 | 0.0108 | 0.6378..0.6631 | 0.06 | 0.31 |
| issue_098_wall_W | 1,031 | model.construct | 0.0650 | 0.0638..0.0664 | 0.0013 | 0.0643..0.0655 | 0.07 | 0.16 |
| issue_098_wall_W | 1,031 | model.lookup | 0.0099 | 0.0099..0.0100 | 0.0000 | 0.0099..0.0100 |  |  |
| issue_098_wall_W | 1,031 | model.iterate | 0.0100 | 0.0099..0.0100 | 0.0000 | 0.0099..0.0100 |  |  |
| issue_098_wall_W | 1,031 | model.by_type | 0.0065 | 0.0065..0.0065 | 0.0000 | 0.0065..0.0065 |  |  |
| issue_098_wall_W | 1,031 | model.traverse | 0.1504 | 0.1494..0.1535 | 0.0012 | 0.1497..0.1506 | 0.00 | 0.01 |
| issue_098_wall_W | 1,031 | model.reverse_index | 0.0997 | 0.0983..0.1018 | 0.0015 | 0.0990..0.1005 | 0.13 | 0.15 |
| shared_point_faceted_brep | 6,393 | step.read.eager | 3.06 | 3.04..3.08 | 0.0261 | 3.02..3.08 | 1.54 | 1.55 |
| shared_point_faceted_brep | 6,393 | step.read.lazy | 2.77 | 2.76..2.81 | 0.0229 | 2.76..2.79 | 0.90 | 1.46 |
| shared_point_faceted_brep | 6,393 | step.read.mapped | 2.84 | 2.81..2.89 | 0.0417 | 2.81..2.89 | 0.67 | 1.23 |
| shared_point_faceted_brep | 6,393 | step.decode_all.1 | 3.02 | 2.99..3.05 | 0.0259 | 2.99..3.05 | 0.86 | 0.92 |
| shared_point_faceted_brep | 6,393 | step.decode_all.8 | 1.05 | 0.9806..1.20 | 0.0903 | 1.03..1.14 | 0.86 | 0.92 |
| shared_point_faceted_brep | 6,393 | step.write | 2.86 | 2.83..2.88 | 0.0261 | 2.83..2.87 | 0.22 | 1.38 |
| shared_point_faceted_brep | 6,393 | model.construct | 0.3840 | 0.3811..0.3898 | 0.0038 | 0.3802..0.3907 | 0.30 | 0.67 |
| shared_point_faceted_brep | 6,393 | model.lookup | 0.0654 | 0.0648..0.0659 | 0.0006 | 0.0647..0.0661 |  |  |
| shared_point_faceted_brep | 6,393 | model.iterate | 0.0645 | 0.0642..0.0648 | 0.0004 | 0.0640..0.0647 |  |  |
| shared_point_faceted_brep | 6,393 | model.by_type | 0.0280 | 0.0279..0.0282 | 0.0001 | 0.0279..0.0282 |  |  |
| shared_point_faceted_brep | 6,393 | model.traverse | 0.4984 | 0.4655..0.5311 | 0.0330 | 0.4676..0.5383 | 0.00 | 0.32 |
| shared_point_faceted_brep | 6,393 | model.reverse_index | 0.7227 | 0.7040..0.7694 | 0.0268 | 0.7038..0.7793 | 0.80 | 0.80 |
| small (1800 walls) | 18,008 | step.read.eager | 10.65 | 10.61..10.73 | 0.0565 | 10.63..10.69 | 6.48 | 6.72 |
| small (1800 walls) | 18,008 | step.read.lazy | 9.52 | 9.47..9.63 | 0.0800 | 9.41..9.64 | 3.50 | 5.75 |
| small (1800 walls) | 18,008 | step.read.mapped | 9.47 | 9.06..9.54 | 0.1561 | 7.88..9.55 | 2.58 | 4.83 |
| small (1800 walls) | 18,008 | step.decode_all.1 | 10.44 | 10.38..11.45 | 0.0784 | 10.36..11.47 | 3.79 | 4.04 |
| small (1800 walls) | 18,008 | step.decode_all.8 | 3.28 | 3.12..3.51 | 0.2370 | 3.16..3.32 | 3.79 | 4.04 |
| small (1800 walls) | 18,008 | step.write | 10.81 | 10.74..10.90 | 0.0820 | 10.76..10.89 | 0.92 | 5.56 |
| small (1800 walls) | 18,008 | model.construct | 1.19 | 1.17..1.19 | 0.0102 | 1.17..1.19 | 1.08 | 2.58 |
| small (1800 walls) | 18,008 | model.lookup | 0.2174 | 0.2138..0.2315 | 0.0055 | 0.2132..0.2323 |  |  |
| small (1800 walls) | 18,008 | model.iterate | 0.2256 | 0.2203..0.2341 | 0.0059 | 0.2205..0.2348 |  |  |
| small (1800 walls) | 18,008 | model.by_type | 0.0251 | 0.0250..0.0251 | 0.0001 | 0.0249..0.0251 |  |  |
| small (1800 walls) | 18,008 | model.traverse | 2.68 | 2.67..2.80 | 0.0144 | 2.67..2.82 | 0.00 | 0.00 |
| small (1800 walls) | 18,008 | model.reverse_index | 2.08 | 2.07..2.10 | 0.0147 | 2.08..2.10 | 2.18 | 2.58 |
| crossover (8000 walls) | 80,008 | step.read.eager | 48.98 | 48.09..50.73 | 0.9181 | 48.18..49.14 | 27.60 | 27.60 |
| crossover (8000 walls) | 80,008 | step.read.lazy | 15.39 | 14.37..17.24 | 1.65 | 14.60..17.35 | 14.54 | 20.81 |
| crossover (8000 walls) | 80,008 | step.read.mapped | 15.28 | 13.73..17.60 | 1.72 | 13.56..18.01 | 10.36 | 16.64 |
| crossover (8000 walls) | 80,008 | step.decode_all.1 | 47.75 | 47.18..48.46 | 0.6117 | 47.07..48.47 | 16.85 | 17.85 |
| crossover (8000 walls) | 80,008 | step.decode_all.8 | 7.30 | 7.21..12.56 | 0.1656 | 7.22..12.71 | 16.85 | 17.85 |
| crossover (8000 walls) | 80,008 | step.write | 55.70 | 54.72..58.02 | 1.30 | 54.74..57.37 | 4.18 | 23.90 |
| crossover (8000 walls) | 80,008 | model.construct | 6.29 | 6.17..6.44 | 0.1311 | 6.15..6.45 | 4.36 | 10.36 |
| crossover (8000 walls) | 80,008 | model.lookup | 1.52 | 1.51..1.53 | 0.0102 | 1.51..1.53 |  |  |
| crossover (8000 walls) | 80,008 | model.iterate | 1.56 | 1.55..1.58 | 0.0124 | 1.55..1.58 |  |  |
| crossover (8000 walls) | 80,008 | model.by_type | 0.1207 | 0.1199..0.1218 | 0.0009 | 0.1190..0.1211 |  |  |
| crossover (8000 walls) | 80,008 | model.traverse | 13.47 | 13.17..13.79 | 0.3076 | 12.61..13.72 | 0.00 | 0.00 |
| crossover (8000 walls) | 80,008 | model.reverse_index | 12.20 | 11.90..13.12 | 0.5059 | 11.75..13.38 | 9.14 | 10.31 |
| large (180000 walls) | 1,800,008 | step.read.eager | 1261 | 1235..1271 | 19.94 | 1231..1281 | 561.03 | 561.03 |
| large (180000 walls) | 1,800,008 | step.read.lazy | 433 | 411..453 | 20.63 | 404..451 | 280.21 | 408.34 |
| large (180000 walls) | 1,800,008 | step.read.mapped | 423 | 403..439 | 19.17 | 403..428 | 179.74 | 307.87 |
| large (180000 walls) | 1,800,008 | step.decode_all.1 | 1341 | 1329..1353 | 12.39 | 1328..1348 | 379.03 | 395.03 |
| large (180000 walls) | 1,800,008 | step.decode_all.8 | 247 | 244..322 | 9.66 | 244..328 | 379.03 | 395.03 |
| large (180000 walls) | 1,800,008 | step.write | 1707 | 1690..1769 | 23.00 | 1692..1786 | 100.47 | 492.15 |
| large (180000 walls) | 1,800,008 | model.construct | 235 | 223..235 | 2.45 | 223..235 | 83.73 | 179.73 |
| large (180000 walls) | 1,800,008 | model.lookup | 93.23 | 92.54..106 | 1.85 | 92.87..106 |  |  |
| large (180000 walls) | 1,800,008 | model.iterate | 96.73 | 95.84..108 | 1.76 | 95.34..109 |  |  |
| large (180000 walls) | 1,800,008 | model.by_type | 5.47 | 5.35..6.70 | 0.1684 | 5.30..6.71 |  |  |
| large (180000 walls) | 1,800,008 | model.traverse | 628 | 625..704 | 8.63 | 625..704 | 0.00 | 0.00 |
| large (180000 walls) | 1,800,008 | model.reverse_index | 573 | 569..644 | 4.59 | 569..647 | 184.88 | 184.88 |

## Extra-work probe

`--probe` walks every traversal twice. Measured right after the table, in
one process each, load 2.96 before and 3.05 after:

| workload | `model.traverse` plain | with `--probe` | ratio | visited (asserted) |
| --- | ---: | ---: | ---: | --- |
| small (1800 walls) | 2.67 ms (IQR 2.66..2.68) | 5.35 ms (5.35..5.36) | 2.00 | 32,400 -> 64,800 |
| crossover (8000 walls) | 13.39 ms (13.28..13.51) | 28.99 ms (27.38..29.24) | 2.17 | 144,000 -> 288,000 |

The asserted visit count doubles exactly and the time about doubles, so the
benchmark measures the walk and the walk is not optimised away. The same two
processes also show the noise floor between processes: rows the probe does
not touch moved by up to 12% (`step.read.mapped` on small, -12%;
`step.read.eager`, `step.write`, +7..9%), which is why `compare` asks
for three runs a side and for the per-run medians to separate.

## Reading the numbers

Observations for orientation, not claims:

- The lazy and mapped reads beat the eager read most where the input is at
  or above 4 MiB and validation runs on 8 threads (crossover 3.2x, large
  2.9x); below it (small, fixtures) they are 10-20% faster.
- `decode_all(8)` after a lazy read is 5.4x faster than `decode_all(1)`
  on the large model; lazy read plus `decode_all(8)` (680 ms) is about
  half the eager read (1,261 ms) at the large scale.
- The decoded large model holds ~561 MB of heap for 1.8 M entities (about
  327 B per entity); a mapped lazy model holds 180 MB before decoding, the
  owned-buffer lazy model 280 MB (the extra ~100 MB is the copied source).
- `step.write` peaks at ~4.9x its output size on the heap (492 MB for a
  100 MB file): it builds the whole generic exchange before writing.
- `model.construct` retains 84 MB on the large scale: the storage, order
  and type index alone, since the entities it inserts were allocated
  before the call.
- `model.traverse` and `model.reverse_index` grow faster than linearly
  between small and large (234x and 275x for 100x the entities); cache
  misses are the likely reason, unverified.
