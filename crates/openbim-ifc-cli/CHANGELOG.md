# Changelog -- openbim-ifc-cli

All notable changes to the `openbim-ifc-cli` crate, which installs the
`openbim-ifc` command, are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.
The command line (commands, flags, exit codes and the JSON and SARIF
documents) is this crate's public interface.

## [Unreleased]

### Added (#472, #474)

- `edit FILE QUERY --set PATH VALUE... (-o FILE | --in-place | --dry-run)`
  writes values by path on the entities a selector selects, as one
  checked transaction, each value read as its target's type; `--dry-run`
  lists the changes and writes nothing; the input is never changed
  without `--in-place`, which keeps its format; `--null` names the text of
  `$`; `--format json` reports changes and refusals as one document.
- `export-table FILE QUERY [-c PATH]... [-o FILE] [--to csv|tsv|xlsx]
  [--key global-id|id] [--null TEXT]` writes the selected entities as a
  table, one column per value path.
- `import-table FILE TABLE (-o FILE | --in-place | --dry-run) [--from
  csv|tsv|xlsx] [--key ...] [--null TEXT]` applies a table back by key as
  one checked transaction; an unchanged table writes an identical model.
- Exit code 3: `edit` or `import-table` refused a planned change; every
  refusal is printed with its entity and path or its row and column and a
  kind (`invalid-text`, `type-mismatch`, `not-settable`,
  `conflicting-write`, `property-edit`, `invalid-header` with its span,
  `unknown-key`, `ragged-row`, ...), and nothing is written.
- The command now checks `Pset_` and `Qto_` writes against the embedded
  PSD/QTO catalog (facade feature `property-catalog`), which grows the
  binary.

Semver: minor (additive; exit codes 0, 1 and 2 keep their meaning).

## [0.3.0] - 2026-10-14

### Added (#464)

- `select FILE QUERY [--value PATH]... [--format table|json|csv]` lists
  the entities a selector selects, in ascending id order, with their
  class, `GlobalId`, `Name` and one column per value path. JSON carries
  the query in canonical form and the grammar version. A selector or path
  that does not parse, or names a class or attribute the declared release
  lacks, is refused with the new kind `invalid-selector` and its byte
  span (exit 2).

Semver: minor (additive).

## [0.2.0] - 2026-10-13

### Added (#450)

- `convert` reads and writes ifcZIP: an input archive is recognized by its
  ZIP signature on every command, and an output ending in `.ifczip`, or
  `--to ifczip`, is written as one Deflate entry named after the output
  file, STEP unless `--inner ifcxml` (with `--layout`) asks for ifcXML.
  An archive without exactly one model, or whose entry is in another
  format, is refused as `parse`; an encrypted entry or another compression
  method as `unsupported`; an entry past the decompression budget as
  `budget-exceeded`. `info` reports the format as `STEP in ifcZIP
  (model.ifc)`.

### Fixed

- A native-layout ifcXML document whose attributes are named by position
  (`a0`, as the bindings' `write_ifcxml` writes it) reads, with the
  declared release's tables resolving both kinds of name; a strict read
  refused it.

Semver: minor (additive).

### Changed (#463)

- An entity named by `GlobalId` is found with the facade's `by_guid`
  instead of the command's own scan. A `GlobalId` several entities carry
  still exits 2 naming every holder (previously the first two), and a
  22-character reference outside IFC's `GlobalId` alphabet or range is a
  usage error rather than a lookup that finds nothing.

Semver: patch.

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

[Unreleased]: https://github.com/openbimrs/ifc/compare/openbim-ifc-cli-v0.3.0...HEAD
[0.3.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-cli-v0.3.0
[0.2.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-cli-v0.2.0
[0.1.0]: https://github.com/openbimrs/ifc/releases/tag/openbim-ifc-cli-v0.1.0
