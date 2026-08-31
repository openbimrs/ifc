# ifc-style surface_style plan

Status: implemented under `STYLE-SURFACE`. Last updated: 2026-08-31.
Follow `AGENTS.md`; record remaining scope without reopening completed tasks.

## Work queue

- [x] `SSURF-SHADE` - shading views
  - Proof: focused projection and authoring tests plus strict clippy.
- [x] `SSURF-RENDER` - rendering/reflection views
  - Proof: colour-or-factor and wrong-reference tests.
- [x] `SSURF-LIGHT` - lighting views
  - Proof: strict select and aggregate tests.
- [x] `SSURF-REFRACT` - refraction views
  - Proof: strict select and aggregate tests.

## Completion log

- `SSURF-SHADE`/`SSURF-RENDER` - selected transactional authoring and borrowed views pass.
- `SSURF-LIGHT`/`SSURF-REFRACT` - typed descriptors pass strict reference validation.
- Surface elements enforce `SET [1:5]` and one member per implemented semantic category.
