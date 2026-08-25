# OpenBIM.rs IFC

Canonical repository: <https://github.com/openbimrs/ifc>
Integration repository: <https://github.com/openbimrs/openbim>

Read `AGENTS.md` before changing the repository and the nearest nested
`AGENTS.md` before editing a crate. Keep this repository independently buildable;
the parent OpenBIM.rs workspace pins it as a submodule but is not required for
standalone development.

## Verification

Run `./scripts/gate.sh`. It is the authoritative local and CI gate and decides
success from command exit codes.

## Project conventions

- Rust 2021, MSRV 1.88, MIT except where package/data notices say otherwise.
- Pure Rust; IFC bridges depend only on format-neutral Axiolid representation
  crates and never on concrete execution providers.
- `ifc-model` stays schema-, codec-, and domain-agnostic.
- Domain projections borrow the model; unknown records and values round-trip.
- Do not vendor official IFC/ISO schemas without confirmed redistribution rights.
- Keep a changelog and distinguish implemented behavior from scaffolding.
