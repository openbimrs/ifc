// STEP parse time of one or more builds of the wasm module, in Node (#303).
//
//   node tools/bench-parse.mjs [--runs N] [--warmup W] [--json out.json] \
//        label=<nodejs pkg dir> [label=<dir> ...] -- file.ifc [file.ifc ...]
//
// Each <dir> is `wasm-bindgen --target nodejs` output (what
// scripts/build-npm-pkg.sh writes to pkg/). All builds load into one
// process and are timed interleaved -- run i parses every file with every
// build before run i+1 starts -- so drift in machine load or CPU clock
// lands on every build alike instead of on whichever ran last.
//
// One sample is `IfcModel.parse(bytes)` alone: the bytes are read once up
// front, and the model is freed outside the timed region. Per build and
// file it prints the median, the interquartile range and min..max over
// the measured runs, after the warm-up runs, plus the module's size raw,
// under gzip -9 and under brotli 11 (node:zlib), and the machine.
//
// scripts/bench-opt-level.sh builds the modules and runs this.
import { readFileSync, statSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { basename, join, resolve } from "node:path";
import os from "node:os";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

const require = createRequire(import.meta.url);

function usage(message) {
  console.error(`error: ${message}`);
  console.error(
    "usage: node bench-parse.mjs [--runs N] [--warmup W] [--json out] label=<pkg> ... -- file.ifc ...",
  );
  process.exit(2);
}

let runs = 30;
let warmup = 5;
let jsonOut = null;
const builds = [];
const files = [];
{
  const args = process.argv.slice(2);
  let inFiles = false;
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (inFiles) files.push(arg);
    else if (arg === "--") inFiles = true;
    else if (arg === "--runs") runs = Number(args[++i]);
    else if (arg === "--warmup") warmup = Number(args[++i]);
    else if (arg === "--json") jsonOut = args[++i];
    else if (arg.includes("=")) {
      const at = arg.indexOf("=");
      builds.push({ label: arg.slice(0, at), dir: resolve(arg.slice(at + 1)) });
    } else usage(`unexpected argument ${arg}`);
  }
}
if (!Number.isInteger(runs) || runs < 1) usage("--runs must be a positive integer");
if (!Number.isInteger(warmup) || warmup < 0) usage("--warmup must be a non-negative integer");
if (builds.length === 0) usage("no build given");
if (files.length === 0) usage("no file given");

for (const build of builds) {
  build.IfcModel = require(join(build.dir, "openbim_ifc_wasm.js")).IfcModel;
  const wasm = readFileSync(join(build.dir, "openbim_ifc_wasm_bg.wasm"));
  build.size = {
    raw: wasm.length,
    gzip9: gzipSync(wasm, { level: 9 }).length,
    brotli11: brotliCompressSync(wasm, {
      params: { [constants.BROTLI_PARAM_QUALITY]: 11 },
    }).length,
  };
}

const inputs = files.map((path) => ({
  name: basename(path),
  bytes: new Uint8Array(readFileSync(path)),
  size: statSync(path).size,
}));

// Every build must read every file to the same entity count: otherwise
// the builds are not doing the same work and the times do not compare.
for (const input of inputs) {
  const counts = builds.map((build) => {
    const model = build.IfcModel.parse(input.bytes);
    const size = model.size;
    model.free();
    return size;
  });
  if (counts.some((count) => count !== counts[0])) {
    console.error(`error: builds disagree on the entity count of ${input.name}: ${counts}`);
    process.exit(1);
  }
  input.entities = counts[0];
}

function parseOnce(build, input) {
  const start = process.hrtime.bigint();
  const model = build.IfcModel.parse(input.bytes);
  const elapsed = process.hrtime.bigint() - start;
  model.free();
  return Number(elapsed) / 1e6; // ms
}

const samples = new Map(); // `${label}\0${file}` -> ms[]
for (let i = 0; i < warmup + runs; i++) {
  for (const input of inputs) {
    for (const build of builds) {
      const ms = parseOnce(build, input);
      if (i < warmup) continue;
      const key = `${build.label}\0${input.name}`;
      if (!samples.has(key)) samples.set(key, []);
      samples.get(key).push(ms);
    }
  }
}

function quantile(sorted, q) {
  const pos = (sorted.length - 1) * q;
  const lo = Math.floor(pos);
  const hi = Math.ceil(pos);
  return sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo);
}

const machine = {
  cpu: os.cpus()[0]?.model ?? "unknown",
  cores: os.cpus().length,
  memoryGiB: Math.round(os.totalmem() / 2 ** 30),
  platform: `${os.platform()} ${os.release()} ${os.arch()}`,
  node: process.version,
  loadavg: os.loadavg().map((x) => Number(x.toFixed(2))),
};

const results = [];
for (const input of inputs) {
  const base = samples.get(`${builds[0].label}\0${input.name}`).slice().sort((a, b) => a - b);
  const baseMedian = quantile(base, 0.5);
  for (const build of builds) {
    const sorted = samples.get(`${build.label}\0${input.name}`).slice().sort((a, b) => a - b);
    const median = quantile(sorted, 0.5);
    results.push({
      build: build.label,
      file: input.name,
      bytes: input.size,
      entities: input.entities,
      medianMs: median,
      p25Ms: quantile(sorted, 0.25),
      p75Ms: quantile(sorted, 0.75),
      minMs: sorted[0],
      maxMs: sorted[sorted.length - 1],
      vsFirst: median / baseMedian,
    });
  }
}

const fmt = (x) => x.toFixed(x < 10 ? 3 : 1);
console.log(`machine: ${machine.cpu}, ${machine.cores} threads, ${machine.memoryGiB} GiB`);
console.log(`         ${machine.platform}, node ${machine.node}`);
console.log(`load average at end (1/5/15 min): ${machine.loadavg.join(" / ")}`);
console.log(`runs: ${runs} measured after ${warmup} warm-up, interleaved\n`);
console.log("| Build | Raw | gzip -9 | brotli 11 |");
console.log("| --- | ---: | ---: | ---: |");
for (const build of builds) {
  const { raw, gzip9, brotli11 } = build.size;
  console.log(`| ${build.label} | ${raw} | ${gzip9} | ${brotli11} |`);
}
console.log("\n| File | Entities | Build | Median ms | IQR ms | Min..max ms | vs first |");
console.log("| --- | ---: | --- | ---: | ---: | ---: | ---: |");
for (const r of results) {
  console.log(
    `| ${r.file} | ${r.entities} | ${r.build} | ${fmt(r.medianMs)} | ` +
      `${fmt(r.p25Ms)}..${fmt(r.p75Ms)} | ${fmt(r.minMs)}..${fmt(r.maxMs)} | ` +
      `${(r.vsFirst * 100 - 100).toFixed(1)}% |`,
  );
}

if (jsonOut) {
  const sizes = Object.fromEntries(builds.map((b) => [b.label, b.size]));
  writeFileSync(jsonOut, JSON.stringify({ machine, runs, warmup, sizes, results }, null, 2));
}
