# ifc-style surface_style instructions

Scope: surface shading, rendering, lighting, and refraction semantics. Follow
crate `../../AGENTS.md`; read `PLAN.md` only for assigned or remaining
surface-style work.

## Owns

- strict surface-style element selects and cardinality
- shading/rendering scalar and colour values
- lighting/refraction descriptors
- bounded per-category uniqueness validation

## Does not own

- BRDF compilation
- GPU material types
- surface geometry

## Implementation map

- `shading.rs`: shading projections
- `rendering.rs`: rendering and colour-or-factor projections
- `lighting.rs`: lighting projections
- `refraction.rs`: refraction projections

Keep schema-specific validation explicit; do not claim general `WHERE` evaluation.
