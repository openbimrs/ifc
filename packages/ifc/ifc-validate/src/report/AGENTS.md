# ifc-validate report instructions

Scope: stable deterministic validation findings and summaries. Follow the crate `../../AGENTS.md`. Read `PLAN.md` only for
assigned task(s) `VAL-REPORT` and keep implementation state there.

## Owns

- finding path/rule/severity/evidence
- stable ordering and summary counts
- machine-readable report values

## Does not own

- printing/logging policy
- codec source loading
- throwing away unsupported-rule state

## Growth map

`finding.rs`, `path.rs`, `summary.rs`. These source owners already compile as private scaffold modules. Replace a module's planned-owner marker with its first real contract and tests; do not add parallel placeholders. Every graph operation has deterministic order and explicit limits.
