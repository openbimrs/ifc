# Cloud setup

Use this setup command in Codex or Claude cloud environments, from the repository root:

```bash
#!/usr/bin/env bash
set -euo pipefail
exec ./scripts/cloud-setup.sh
```

The script supports Ubuntu/Debian Linux images with root or passwordless sudo.
Run it during the network-enabled setup phase. It installs image packages and
repository tools, warms dependency caches, and can be rerun. `--help` prints
usage without installing anything.

Tools installed by setup are exposed through `/usr/local/bin`, so subsequent
agent shells can use them after the setup process exits. Existing proxy and CA
settings are inherited. Setup does not run builds or tests or claim that the
gate passed; run the verification command printed at the end,
`scripts/gate.sh`.

## Required tools

A failure in this part stops setup with the failing command.

| Tool | Why |
| --- | --- |
| Rust 1.88.0 with rustfmt and clippy | `rust-toolchain.toml` and CI |
| `wasm32-unknown-unknown` target | the gate's browser WASM builds |
| `<arch>-unknown-linux-musl` target | the CLI's static build, `.deb` and release archives |
| `cargo fetch --locked` | warm crate cache |
| Node.js 22.12 or newer, `npm ci` | VitePress, TypeDoc and `playwright-core` (root lockfile); webpack (`crates/openbim-ifc-wasm/tools`) |
| `wasm-bindgen` CLI | the exact version in `Cargo.lock`; `build-npm-pkg.sh` refuses a mismatch |
| `uv` and `maturin` | the Python wheel and its test venv, versions as in `ci.yml` |
| `dpkg` (`dpkg-deb`) | the CLI's Debian package check |
| IFC schemas | `scripts/fetch-ifc-schemas.sh`, checksummed |

## Optional extras

When one cannot be installed, setup prints `cloud setup: skipped ...` and a
summary at the end, and still completes.

| Extra | Without it |
| --- | --- |
| Chrome or Chromium | The npm package and docs playground checks fail; set `CHROME_BIN` or `IFC_SKIP_BROWSER=1`. A browser already on `PATH` or in Playwright's cache is reused; otherwise the locked `playwright-core` downloads Chromium and its system libraries. |
| .NET 8 SDK and docfx | `build-api-docs.sh` skips the .NET API reference. Setup installs Ubuntu's `dotnet-sdk-8.0` package and restores docfx from `.config/dotnet-tools.json`. Debian images need Microsoft's package feed first. |
| Python tool cache | `check-python.sh` downloads its pinned pandas, mypy and pdoc during the gate instead. |

Nix is not needed in cloud sessions: the flake is checked only in CI
(`.github/workflows/nix.yml`), and setup does not install it.

The gate may still need network access for package verification, external
references or optional features. Restricted standards material is never
committed by setup. Fuzzing and upstream-parity toolchains are outside this
bootstrap.

Python image tools use the current interpreter's user site (or its active
virtual environment). On externally managed Debian interpreters, setup uses
`--break-system-packages --user`; use this script in a disposable cloud image.

## Testing the script

Test the bootstrap's command flow without downloads or system changes:

```bash
bash -n scripts/cloud-setup.sh
python3 scripts/test-cloud-setup.py
```

The tests run the script against command doubles in a temporary directory.
`PATH` holds only the doubles and a few plain host utilities, and
`OPENBIM_CLOUD_BIN_DIR` redirects the exposed-tools directory, so no real
package manager, compiler or network tool on the host can run. The
`image-smoke` job in `.github/workflows/cloud-setup.yml` runs the real script
on a fresh Ubuntu runner and checks the tools from a later shell.
