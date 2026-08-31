# ifc-style texture plan

Status: implemented under `STYLE-TEXTURE`. Last updated: 2026-08-31.
Follow `AGENTS.md`; record remaining scope without reopening completed tasks.

## Work queue

- [x] `TEX-SURF` - texture descriptor views
  - Proof: IFC2X3/IFC4/IFC4X3 mode-drift and typed-transform tests.
- [x] `TEX-IMAGE` - URI/blob metadata without loading
  - Proof: focused borrowed-projection tests.
- [x] `TEX-COORD` - coordinate association views
  - Proof: strict wrong-reference and requiredness tests.
- [x] `TEX-MAP` - texture map relationships
  - Proof: indexed-map `TexCoords` type/requiredness tests.

## Completion log

- Texture modes resolve by schema attribute name rather than fixed slot.
- Texture transforms and coordinate references reject dangling or wrong-kind targets.
- External texture identifiers remain inert data; no network or file I/O occurs.
