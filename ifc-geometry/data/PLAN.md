# geometry schema data plan

Status: implemented and verified.

## Work queue

- [x] `GDATA-MOVE` - relocate the committed artifacts out of `references/`.
  - Evidence: `scripts/check-leakage.py` passes on the tracked tree; the
    directory name no longer collides with the reserved one.
- [x] `GDATA-NOTICE` - state source, license, and redistribution reasoning.
  - Evidence: `NOTICE.md` records the CC BY-ND 4.0 source and why structural
    facts are not a derivative of the schema text.
- [x] `GDATA-GATE` - make the licensing check executable rather than manual.
  - Evidence: wired into `scripts/gate.sh`; 3/3 mutation probes caught
    (`references/` path, XSD payload under an innocuous name, PDF payload).

## Completion log

Append the proof command and the material decision.

- `GDATA-MOVE` - `python3 scripts/check-leakage.py` passes; `cargo test -p
  ifc-geometry --test declaration_manifest` 3 passing after retargeting the
  two `include_str!` paths to `../data/`. Decision: rename rather than
  whitelist. The detector's rule is correct -- `references/` is where
  unredistributable payloads live -- so the long-term-correct repair is to
  stop violating it, not to record an exception.
- `GDATA-GATE` - the check existed since the docs work but ran only by hand,
  which is exactly how a tracked `ifc-geometry/references/` survived. A gate
  nobody executes is not a gate.
