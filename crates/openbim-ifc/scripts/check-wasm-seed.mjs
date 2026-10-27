// Run by check-wasm-seed.sh: node check-wasm-seed.mjs <js-random-dir> <no-source-dir>
//
// With `js-random`, two separate Node processes each draw two seeds; all four
// must be 32 hex digits and distinct (a counter from 0 would repeat across
// processes). Without it, loading and calling `freshSeed` must refuse.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import path from 'node:path';

const [withSource, withoutSource] = process.argv.slice(2).map((dir) => path.resolve(dir));
assert.ok(withSource && withoutSource, 'usage: check-wasm-seed.mjs <js-random-dir> <no-source-dir>');

const draw = () =>
  JSON.parse(
    execFileSync(
      process.execPath,
      [
        '-e',
        `const m = require(${JSON.stringify(path.join(withSource, 'wasm_fresh_seed.js'))});` +
          'console.log(JSON.stringify([m.freshSeed(), m.freshSeed()]));',
      ],
      { encoding: 'utf8' },
    ),
  );

const seeds = [...draw(), ...draw()];
for (const seed of seeds) {
  assert.match(seed, /^[0-9a-f]{32}$/);
}
assert.equal(new Set(seeds).size, seeds.length, `seeds repeat: ${seeds.join(' ')}`);

const refused = createRequire(import.meta.url)(path.join(withoutSource, 'wasm_fresh_seed.js'));
assert.throws(() => refused.freshSeed(), (error) => String(error).includes('no random source'));

console.log(`ok: ${seeds.length} distinct seeds across 2 processes; the build without js-random refuses`);
