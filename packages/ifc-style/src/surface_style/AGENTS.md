# ifc-style surface_style instructions

Scope: surface shading, rendering, lighting, and refraction semantics. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `STYLE-SURFACE`; keep progress and blockers there.

## Owns

- surface style element selects
- shading/rendering scalar/colour values
- lighting/refraction descriptors

## Does not own

- BRDF compilation
- GPU material types
- surface geometry

## Growth map

`shading.rs`, `rendering.rs`, `lighting.rs`, `refraction.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Keep views, resolution, validation, and neutral output in
separate files.
