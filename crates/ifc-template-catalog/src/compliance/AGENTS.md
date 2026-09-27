# compliance instructions

Purpose: Format-neutral template application walking and authored-set compliance checks.

Follow `../../AGENTS.md`.

Boundaries:
- No dependency on `ifc-model` or codecs.
- Adapters implement `TemplateSink` in consumer crates.
- Missing members are optional unless policy explicitly requires all.
- Validation never mutates authored values or catalog templates.
