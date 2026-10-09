# Changelog -- openbim-ifc-cli

All notable changes to the `openbim-ifc-cli` crate, which installs the
`openbim-ifc` command, are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.
The command line (commands, flags, exit codes and the JSON and SARIF
documents) is this crate's public interface.

## [Unreleased]

## [0.1.0] - 2026-10-09

### Added (#329, the first part)

- The `openbim-ifc` command, over the `openbim-ifc` facade only:
  - `validate FILE...`: each file against the schema its header declares;
    `--format human|json|sarif`, `--max-findings`, `--deny-warnings`,
    `--include-unsupported`. Exit 0 when every file passes, 1 on an error,
    an evaluation error, a truncated report or (with `--deny-warnings`) a
    warning, 2 when a file could not be validated.
  - `convert IN OUT`: STEP to ifcXML and back, the output format from the
    extension or `--to`; `--layout native|xsd` for ifcXML output;
    `--input-layout auto|native|xsd`; `-` writes standard output.
  - `info FILE`: header, declared schema and whether it is bundled, entity
    and per-type counts; `--format human|json`.
  - `psets FILE ENTITY`: the exactly resolved property and quantity sets of
    an object or type object, by `#id` or `GlobalId`, occurrence and
    inherited; `--format table|json|csv`.
  - `tree FILE`: the spatial structure with element counts or (`--elements`)
    the elements, and the anomalies the tree found; `--format human|json`.
  - `lint FILE...`: products no model viewer will draw (exit 1 on a
    finding); `--format human|json|sarif`.
- Typed refusals with a stable kind on stderr (`usage`, `io`, `parse`,
  `unsupported-schema`, `unsupported`, `missing-entity`,
  `wrong-entity-type`, `missing-reference`, `invalid-model`,
  `budget-exceeded`, `write`) and exit code 2.
- Prebuilt binaries on each `openbim-ifc-cli-v*` GitHub release for Linux
  (x86_64, aarch64; static musl), macOS (x86_64, arm64) and Windows
  (x86_64), with `SHA256SUMS`; `install.sh` and a Homebrew formula
  generated for the release; `cargo binstall` metadata.

### Added (#376, #377, #378)

- Debian packages `openbim-ifc_<version>_amd64.deb` and `_arm64.deb` on
  each release, packed from the same static binaries by
  `scripts/package.py deb` (the binary in `/usr/bin`, the README and the
  licence in `/usr/share/doc/openbim-ifc/`, no dependencies) and listed in
  `SHA256SUMS`; `scripts/check-deb.sh` inspects them and installs, runs and
  removes them with `dpkg` on Ubuntu 22.04 (x86_64 and aarch64) in CI and
  in every release build. No apt repository.
- A Nix flake at the repository root: `packages.<system>.openbim-ifc`
  (and `default`) built from source with the pinned toolchain and
  `Cargo.lock`, and `apps.<system>.default`, so
  `nix run github:openbimrs/ifc -- --version` works on x86_64 and aarch64
  Linux and macOS; checked in `.github/workflows/nix.yml`.
- `install.ps1` for Windows (`irm .../install.ps1 | iex`): picks the x64 or
  Arm64 archive, refuses a checksum mismatch, installs to
  `%LOCALAPPDATA%\Programs\openbim-ifc\bin` (or `-Prefix`) and offers to
  add it to the user `PATH`; honours `OPENBIM_IFC_BASE_URL` like
  `install.sh`. `scripts/check-install.ps1` tests it on Windows in CI and in
  every release build, and the release attaches it.
- A Windows Arm64 binary (`aarch64-pc-windows-msvc`) on each release.

Semver: a new crate, first release 0.1.0.

[Unreleased]: https://github.com/openbimrs/ifc/compare/openbim-ifc-cli-v0.1.0...HEAD
[0.1.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-cli-v0.1.0
