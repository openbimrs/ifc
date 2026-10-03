// The PSD/QTO catalog loader behind `IfcModel.loadCatalog` (#318).
//
// The wasm module embeds no catalog. The package ships one snapshot file per
// edition in `catalog/`, and this loader reads the one a release needs,
// hands its bytes to `IfcModel.loadCatalogBytes` (which checks them against
// the edition's pinned SHA-256), and remembers the load for the module
// instance. scripts/build-npm-pkg.sh installs it on every target's
// `IfcModel`: the ES module targets import it; the CommonJS one imports it
// on first use.
//
// Where the bytes come from:
// - `options.bytes`: the caller's, for one release;
// - `options.baseUrl`: `<baseUrl>/<file>`, fetched (or read, for `file:`);
// - otherwise the package's own file, relative to this module. Node reads it
//   from disk; a browser fetches it; a bundler sees the `new URL(...,
//   import.meta.url)` literal below and emits the file as an asset.

// One literal per file, so a bundler (webpack 5, Vite, Parcel) can find
// and emit each one; a computed path would hide them.
const PACKAGED = {
  "ifc2x3-tc1.bin": () => new URL("./catalog/ifc2x3-tc1.bin", import.meta.url),
  "ifc4-add2-tc1.bin": () => new URL("./catalog/ifc4-add2-tc1.bin", import.meta.url),
  "ifc4x3-add2.bin": () => new URL("./catalog/ifc4x3-add2.bin", import.meta.url),
};

/** Every release with a catalog edition, loaded when no release is named. */
const RELEASES = ["IFC2X3", "IFC4", "IFC4X3"];

/** An `IfcError` with `code`, as the module's own errors are. */
function ifcError(code, message) {
  const error = new Error(message);
  error.name = "IfcError";
  error.code = code;
  return error;
}

async function read(file, baseUrl) {
  let url;
  if (baseUrl === undefined || baseUrl === null) {
    const packaged = PACKAGED[file];
    if (packaged === undefined) {
      throw ifcError("io", `the package ships no catalog file ${file}`);
    }
    url = packaged();
  } else {
    const base = String(baseUrl);
    url = new URL(file, base.endsWith("/") ? base : `${base}/`);
  }
  try {
    if (url.protocol === "file:") {
      // Node only; never reached in a browser, and kept out of bundles.
      const { readFile } = await import(/* webpackIgnore: true */ /* @vite-ignore */ "node:fs/promises");
      return new Uint8Array(await readFile(url));
    }
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    return new Uint8Array(await response.arrayBuffer());
  } catch (cause) {
    throw ifcError("io", `cannot read the PSD/QTO catalog ${url}: ${cause.message ?? cause}`);
  }
}

function asBytes(bytes) {
  if (bytes instanceof Uint8Array) return bytes;
  if (bytes instanceof ArrayBuffer) return new Uint8Array(bytes);
  throw ifcError("invalid-value", "loadCatalog: `bytes` must be a Uint8Array or an ArrayBuffer");
}

/** The `loadCatalog(release?, options?)` of one module instance's `IfcModel`. */
export function catalogLoader(IfcModel) {
  // Reads in flight, by file: concurrent loads of one edition read it once.
  const pending = new Map();

  function loadOne(release, options) {
    // Throws `unsupported-schema` for a release without a catalog.
    const file = IfcModel.catalogFile(release);
    if (IfcModel.catalogLoaded(release)) return undefined;
    if (options.bytes !== undefined) {
      IfcModel.loadCatalogBytes(release, asBytes(options.bytes));
      return undefined;
    }
    let loading = pending.get(file);
    if (loading === undefined) {
      loading = read(file, options.baseUrl)
        .then((bytes) => IfcModel.loadCatalogBytes(release, bytes))
        .finally(() => pending.delete(file));
      pending.set(file, loading);
    }
    return loading;
  }

  return async function loadCatalog(release, options = {}) {
    const releases = release === undefined || release === null ? RELEASES : [release];
    if (options.bytes !== undefined && releases.length !== 1) {
      throw ifcError("invalid-value", "loadCatalog: `bytes` belong to one release; name it");
    }
    await Promise.all(releases.map((one) => loadOne(one, options)));
  };
}
