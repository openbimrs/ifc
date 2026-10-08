# Changelog -- openbim-ifc-cli

All notable changes to the `openbim-ifc-cli` crate, which installs the
`openbim-ifc` command, are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this crate follows Semantic Versioning independently of its siblings:
a release here does not imply a release of any other crate in the family.
The command line (commands, flags, exit codes and the JSON and SARIF
documents) is this crate's public interface.

## [Unreleased]

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

Semver: a new crate, first release 0.1.0.
