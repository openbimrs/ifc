# Changelog

All notable changes to **nehirde** are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

One entry per change under `## [Unreleased]` as you land work; cut a version
section on release.

## [Unreleased]

### Added
- Repo scaffold: empty Cargo workspace (`crates/*`), `docs/` (roadmap, ADRs,
  this changelog), `references/` symlinks to IfcOpenShell + ifc-lite clones on
  `/mnt/backup/`, `test/fixtures/` with 18 edge-case `.ifc` files pulled from
  those two repos, `target` symlinked to `/mnt/backup/build-cache/` (sparse
  root disk), progressive `AGENTS.md` context files.
