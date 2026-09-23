// Corpus round trip through the WASM binding (#34).
//
// For every committed fixture: parse, write, re-parse, and require the same
// ids, types and attribute values. Covers what the handwritten smoke test
// cannot: real exporter output and the hostile string-literal fixture.
//
// Usage: IFC_WASM_PKG=<package-dir> node --test corpus.mjs
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const pkg = path.resolve(process.env.IFC_WASM_PKG ?? "pkg");
const { IfcModel } = createRequire(import.meta.url)(
  path.join(pkg, "openbim_ifc_wasm.js"),
);

const root = path.resolve(fileURLToPath(import.meta.url), "../../../..");
const fixtures = execFileSync("git", ["ls-files", "*.ifc"], {
  cwd: path.join(root, "test/fixtures"),
  encoding: "utf8",
})
  .split("\n")
  .filter(Boolean)
  .map((name) => path.join(root, "test/fixtures", name));

test("the corpus is non-empty", () => {
  assert.ok(fixtures.length >= 10, `found ${fixtures.length} fixtures`);
});

for (const file of fixtures) {
  test(path.relative(root, file), () => {
    const model = IfcModel.parse(readFileSync(file));
    assert.ok(model.size > 0, "parsed no entities");
    const reparsed = IfcModel.parse(model.write());
    assert.deepEqual(reparsed.ids(), model.ids());
    for (const id of model.ids()) {
      assert.equal(reparsed.typeOf(id), model.typeOf(id), `#${id} type`);
      assert.deepEqual(reparsed.attributes(id), model.attributes(id), `#${id}`);
    }
  });
}
