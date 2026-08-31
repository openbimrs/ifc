# ifc-style texture instructions

Scope: texture descriptors and coordinate associations. Follow crate
`../../AGENTS.md`; read `PLAN.md` only for assigned or remaining texture work.

## Owns

- texture metadata, repeat flags, and schema-specific mode
- image/blob references as inert data
- strict texture-transform and coordinate/map associations

## Does not own

- image decoding, loading, or other I/O
- UV generation from geometry
- renderer handles

## Implementation map

- `surface.rs`: shared texture descriptors and typed transforms
- `image.rs`: image texture metadata
- `coordinate.rs`: texture-coordinate associations
- `map.rs`: strict indexed texture-map relationships

URLs and blobs remain model data and are never fetched by this crate.
