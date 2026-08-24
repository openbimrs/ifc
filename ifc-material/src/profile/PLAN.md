# ifc-material profile plan

Status: IFC4 views implemented under `MAT-PROFILE`. Last updated: 2026-08-20.
Follow `AGENTS.md`; claim one task and record blockers/decisions beneath it.

## Work queue

- [x] `MATPROF-DEF` - material/name/description/priority/category projection
  - Proof: `profiles::reads_profiles_sets_offsets_and_usage` plus crate clippy.
- [x] `MATPROF-SET` - ordered semantic membership and composite indicator
  - Requires: `MATPROF-DEF`.
  - Proof: `profiles.rs` pins source-order LIST projection and composite-profile access.
- [x] `MATPROF-USAGE` - authored cardinal points, extent, and tapering set
  - Requires: `MATPROF-SET`.
  - Proof: `profiles.rs` cardinal/tapering checks and `strict_decoding::malformed_optional_values_and_where_rules_are_errors`.
- [ ] `MATPROF-CROSS` - shared fixture with geometry's material-usage projection
  - Requires: `MATPROF-USAGE`, `INPUT-MAT`.
  - Proof: both projections join by EntityId without crate dependencies or duplicate slot parsing.

## Completion log

`MATPROF-*` - `tests/profiles.rs` and `errors.rs`; geometry cross-projection
remains separately owned by `MATPROF-CROSS`.
