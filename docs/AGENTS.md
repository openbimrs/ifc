# AGENTS.md — docs/

- `ROADMAP.md` — validation-gated stages. Follow `../vendor/solibri`'s
  convention: no stage is "done" without (a) a cross-check against fixtures/an
  oracle and (b) a measured wall-clock where performance is claimed.
- `adr/` — architecture decision records, one file per decision,
  `NNNN-short-title.md`, numbered sequentially. Use `adr/_template.md`. This
  mirrors `../poing/docs/adr/`'s convention on this host — same shape, so
  anyone jumping between the two repos doesn't relearn a format.
- `CHANGELOG.md` — Keep a Changelog format
  (https://keepachangelog.com/en/1.1.0/), Semantic Versioning. One entry per
  change under `## [Unreleased]` as work lands; cut a version section on
  release. Mirrors `../poing/CHANGELOG.md`'s convention.
- `api/` — **generated only**, from `cargo doc --workspace --no-deps`. Do not
  hand-edit anything under here once it exists; regenerate instead. (Not
  created yet — no crates to document.)

## Agent rules

1. Write the ADR *before* landing a hard-to-reverse decision (crate layering,
   parser strategy, geometry kernel choice, dependency adoption) — not after,
   as an afterthought.
2. Every ADR needs a real "Alternatives considered" table — a decision with no
   stated alternative is usually not a decision, it's an unexamined default.
3. Changelog entries describe user/API-visible change, not implementation
   diary — "what changed" not "what I did."
4. Don't create `docs/api/` by hand; run the generator once crates exist.
