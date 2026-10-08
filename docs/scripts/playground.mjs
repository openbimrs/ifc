// Put the browser playground's module beside a docs site (#331).
//
//   node docs/scripts/playground.mjs <package-dir> <site-dir>
//
// <package-dir> is the npm package crates/openbim-ifc-wasm/scripts/build-npm-pkg.sh
// builds (crates/openbim-ifc-wasm/pkg); <site-dir> is the built site
// (docs/.vitepress/dist, as the Pages workflow does) or docs/public for
// `npm run docs:dev`. It copies, under <site-dir>/playground-files/ (not
// playground/: a directory beside playground.html would make the page's
// clean URL /playground ambiguous on a static host):
//
//   pkg/web/       the `@openbim/ifc/web` build the page imports
//   pkg/mesh/web/  the `@openbim/ifc/mesh/web` build, fetched only when a
//                  reader asks for meshes
//   pkg/catalog.mjs, pkg/package.json
//                  the PSD/QTO catalog loader both builds import, and the
//                  version the page names
//   samples/       two fixtures from test/fixtures for the sample picker
//
// The page itself (docs/playground.md) is part of the VitePress build and
// works without these files: it then says the module is missing.
import { cp, mkdir, rm } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");

/** The fixtures the page offers as samples, by the name it shows. */
export const SAMPLES = {
  "synthetic_properties.ifc": "test/fixtures/synthetic-properties/synthetic_properties.ifc",
  "binding_geometry.ifc": "test/fixtures/synthetic-bindings/binding_geometry.ifc",
};

/** Copy the module and the samples from `pkg` into `site`/playground-files. */
export async function install(pkg, site) {
  const target = path.join(site, "playground-files");
  await rm(target, { recursive: true, force: true });
  await mkdir(path.join(target, "pkg/mesh"), { recursive: true });
  for (const dir of ["web", "mesh/web"]) {
    await cp(path.join(pkg, dir), path.join(target, "pkg", dir), {
      recursive: true,
      // Declarations are for TypeScript, not for the page.
      filter: (source) => !source.endsWith(".d.ts"),
    });
  }
  for (const file of ["catalog.mjs", "package.json"]) {
    await cp(path.join(pkg, file), path.join(target, "pkg", file));
  }
  await mkdir(path.join(target, "samples"), { recursive: true });
  for (const [name, source] of Object.entries(SAMPLES)) {
    await cp(path.join(root, source), path.join(target, "samples", name));
  }
  return target;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [pkg, site] = process.argv.slice(2);
  if (!pkg || !site) {
    console.error("usage: node docs/scripts/playground.mjs <package-dir> <site-dir>");
    process.exit(2);
  }
  console.log(`playground module installed in ${await install(path.resolve(pkg), path.resolve(site))}`);
}
