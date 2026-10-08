// Drive the browser playground (#331) in headless Chrome against a built site.
//
//   node docs/scripts/check-playground.mjs <site-dir> [<package-dir>]
//
// <site-dir> is a built site (docs/.vitepress/dist). With <package-dir>, the
// site is copied to a scratch directory first and the module installed
// there (docs/scripts/playground.mjs), so the gate leaves the site it built
// untouched; without it the site must already carry /playground-files/pkg/, as
// the Pages workflow installs it.
//
// The site is served under /ifc/, its base path on GitHub Pages, with the
// same clean URLs. The check opens /ifc/playground, picks a fixture through
// the real file input, and requires the header, the spatial tree, a
// selected wall's property sets, a validation report and the meshes of a
// second fixture, with no page error and no request that leaves the site
// or sends anything (every request a GET to the local server).
//
// Chrome is found as tools/check-package.mjs finds it: $CHROME_BIN, the
// usual names on PATH, then a Playwright download. IFC_SKIP_BROWSER=1
// skips the check with a warning when there is none.
import { existsSync, readdirSync } from "node:fs";
import { cp, mkdtemp, readFile, rm, stat } from "node:fs/promises";
import { createServer } from "node:http";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "playwright-core";

import { install } from "./playground.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const [siteArg, pkgArg] = process.argv.slice(2);
if (!siteArg) {
  console.error("usage: node docs/scripts/check-playground.mjs <site-dir> [<package-dir>]");
  process.exit(2);
}
const BASE = "/ifc/";
const TIMEOUT_MS = 60_000;
const PROPERTIES = path.join(root, "test/fixtures/synthetic-properties/synthetic_properties.ifc");
const GEOMETRY = path.join(root, "test/fixtures/synthetic-bindings/binding_geometry.ifc");

const TYPES = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".mjs": "text/javascript",
  ".css": "text/css",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm",
  ".ifc": "application/octet-stream",
  ".woff2": "font/woff2",
};

function findChrome() {
  if (process.env.CHROME_BIN) return process.env.CHROME_BIN;
  const names = ["google-chrome-stable", "google-chrome", "chromium", "chromium-browser", "chrome"];
  for (const dir of (process.env.PATH ?? "").split(path.delimiter)) {
    for (const name of names) {
      if (dir && existsSync(path.join(dir, name))) return path.join(dir, name);
    }
  }
  const mac = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
  if (existsSync(mac)) return mac;
  const playwright = path.join(os.homedir(), ".cache/ms-playwright");
  if (existsSync(playwright)) {
    for (const build of readdirSync(playwright).filter((d) => /^chromium-\d+$/.test(d)).sort().reverse()) {
      for (const sub of ["chrome-linux64", "chrome-linux"]) {
        const exe = path.join(playwright, build, sub, "chrome");
        if (existsSync(exe)) return exe;
      }
    }
  }
  return undefined;
}

/** Serve `dir` at BASE as GitHub Pages does: `/x` serves `x.html`, `/x/` `x/index.html`. */
async function serve(dir, requests) {
  const server = createServer(async (request, response) => {
    requests.push(`${request.method} ${request.url}`);
    const url = new URL(request.url, "http://localhost");
    if (request.method !== "GET" || !url.pathname.startsWith(BASE)) {
      response.statusCode = request.method === "GET" ? 404 : 405;
      response.end();
      return;
    }
    let file = path.join(dir, path.normalize(decodeURIComponent(url.pathname.slice(BASE.length))));
    if (!file.startsWith(dir)) {
      response.statusCode = 403;
      response.end();
      return;
    }
    const kind = await stat(file).catch(() => undefined);
    if (kind?.isDirectory()) file = path.join(file, "index.html");
    else if (!kind && existsSync(`${file}.html`)) file = `${file}.html`;
    try {
      const body = await readFile(file);
      response.setHeader("content-type", TYPES[path.extname(file)] ?? "application/octet-stream");
      response.end(body);
    } catch {
      response.statusCode = 404;
      response.end();
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return server;
}

const chrome = findChrome();
if (chrome === undefined) {
  if (process.env.IFC_SKIP_BROWSER) {
    console.warn("warning: IFC_SKIP_BROWSER set and no Chrome found; playground check NOT run");
    process.exit(0);
  }
  console.error("no Chrome or Chromium found; set CHROME_BIN, or IFC_SKIP_BROWSER=1 to skip the playground check");
  process.exit(1);
}

let site = path.resolve(siteArg);
let scratch;
if (pkgArg) {
  scratch = await mkdtemp(path.join(os.tmpdir(), "openbim-ifc-playground-"));
  await cp(site, scratch, { recursive: true });
  await install(path.resolve(pkgArg), scratch);
  site = scratch;
}
if (!existsSync(path.join(site, "playground-files/pkg/web/openbim_ifc_wasm.js"))) {
  console.error(`${site} carries no playground module; run docs/scripts/playground.mjs first`);
  process.exit(1);
}

const requests = [];
const server = await serve(site, requests);
const origin = `http://127.0.0.1:${server.address().port}`;
const problems = [];
const browser = await chromium.launch({
  executablePath: chrome,
  // Hosted CI runners restrict the user namespaces Chrome's sandbox needs;
  // the page is this script's own, served from localhost. SwiftShader gives
  // the viewer a WebGL2 context without a GPU.
  args: ["--no-sandbox", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"],
});
try {
  const page = await browser.newPage();
  page.setDefaultTimeout(TIMEOUT_MS);
  page.on("pageerror", (error) => problems.push(`page error: ${error.message}`));
  page.on("console", (message) => {
    // Chrome's own /favicon.ico probe 404s at the server root, as on Pages.
    if (message.type() === "error" && !message.location().url.endsWith("/favicon.ico")) {
      problems.push(`console error: ${message.text()} (${message.location().url})`);
    }
  });
  page.on("request", (request) => {
    if (!request.url().startsWith(origin) || request.method() !== "GET") {
      problems.push(`request leaves the page: ${request.method()} ${request.url()}`);
    }
  });

  await page.goto(`${origin}${BASE}playground`);
  const text = (selector) => page.locator(selector).first().innerText();

  // 1. A local file through the real picker: header, tree, property sets.
  await page.locator('[data-testid="file"]').setInputFiles(PROPERTIES);
  await page.locator('[data-testid="header"]').waitFor();
  const header = await text('[data-testid="header"]');
  if (!header.includes("IFC4")) problems.push(`header lacks the schema:\n${header}`);
  const status = await text('[data-testid="status"]');
  if (!status.includes("synthetic_properties.ifc: IFC4, 68 entities")) problems.push(`status: ${status}`);
  const tree = await text('[data-testid="tree"]');
  for (const expected of ["IFCPROJECT #22 Properties", "IFCBUILDINGSTOREY #25 Level 0", "IFCWALL #30 Wall A", "IFCWALL #31 Wall B"]) {
    if (!tree.includes(expected)) problems.push(`tree lacks "${expected}":\n${tree}`);
  }
  await page.getByRole("button", { name: "IFCWALL #30 Wall A" }).click();
  await page.locator('[data-testid="pset"]').first().waitFor();
  const sets = await page.locator('[data-testid="pset"]').allInnerTexts();
  const joined = sets.join("\n");
  for (const expected of ["Pset_WallCommon (its own)", "IsExternal", "Qto_WallBaseQuantities", "Pset_WallCommon (from its type)", "F30"]) {
    if (!joined.includes(expected)) problems.push(`property sets lack "${expected}":\n${joined}`);
  }

  // 2. Validation lists its findings.
  await page.locator('[data-testid="validate"]').click();
  const report = await page.locator('[data-testid="report"]').innerText();
  if (!/^Conformant: 0 error\(s\)/.test(report)) problems.push(`validation report: ${report}`);
  if ((await page.locator('[data-testid="findings"] tbody tr').count()) === 0) {
    problems.push("validation listed no findings (the fixture has rules it does not evaluate)");
  }

  // 3. A second file, from the sample picker, and its meshes.
  await page.locator('[data-testid="sample"]').selectOption("binding_geometry.ifc");
  await page.getByText("binding_geometry.ifc: IFC4").waitFor();
  await page.locator('[data-testid="meshes"]').click();
  const meshes = page.locator('[data-testid="mesh-status"][data-drawn="2"]');
  await meshes.waitFor();
  const meshStatus = await meshes.innerText();
  if (!meshStatus.startsWith("2 meshes")) problems.push(`mesh status: ${meshStatus}`);
  if (meshStatus.includes("not drawn")) problems.push(`the viewer did not draw: ${meshStatus}`);

  console.log(`playground: ${status}; ${report.trim()} ${meshStatus}`);
} catch (error) {
  problems.push(String(error?.stack ?? error));
} finally {
  await browser.close();
  server.close();
  if (scratch) await rm(scratch, { recursive: true, force: true });
}

// Chrome asks for /favicon.ico by itself; anything but a GET would be the
// page sending data.
const sent = requests.filter((line) => !line.startsWith("GET "));
for (const line of sent) problems.push(`the page sent a request: ${line}`);
if (problems.length > 0) {
  console.error(`playground check FAILED:\n${problems.join("\n")}\nrequests:\n${requests.join("\n")}`);
  process.exit(1);
}
console.log(`playground check ok (${requests.length} requests, all GET)`);
