# NOTICE

Provenance for the committed artifacts in this directory.

## Source

Derived from the official IFC4 ADD2 TC1 EXPRESS schema (`IFC4.exp`), published
by buildingSMART International at <https://standards.buildingsmart.org/> under
CC BY-ND 4.0.

The schema itself is **not** redistributed here. It is read from a local
read-only checkout, which is gitignored, absent from a published clone, and
never a build dependency.

## What these files are

| File | Content |
|---|---|
| `absolute-slots.txt` | Positional STEP slot index per geometry entity, with inherited attributes flattened first. |
| `ifc4-add2-tc1-geometry-declarations.tsv` | The set of geometry-resource declarations: resource, kind, name, abstractness, type kind. |
| `ifc4-add2-tc1-geometry-support.tsv` | The above, joined with this repository's own implementation-ownership mapping (`bridge_owner`, `neutral_owner`, `status`). |
| `ifc4-where-rules.tsv` | Every WHERE rule declared on a geometry entity, by entity and rule **label**, with this repository's support state. No rule expression. |
| `ifc4-representation-item-dispositions.tsv` | This repository's own analysis: how each representation item is handled (disposition, owner, rationale). No schema text. |

## Why redistribution is permitted

CC BY-ND 4.0 forbids distributing *modified* copies of the licensed work. These
files are not a copy of the schema, modified or otherwise:

- They contain no EXPRESS source text -- no `ENTITY`, `SUBTYPE OF`, `WHERE`,
  `SELECT`, or rule bodies, verbatim or paraphrased. A WHERE rule appears
  only by its label, which names it the way an entity name does.
  `ifc4-where-rules.tsv` carried the rule expressions verbatim until they
  were removed; `scripts/check-leakage.py` now fails on EXPRESS rule syntax
  in any committed or packaged data file.
- They record structural facts (a name, its slot index, its declaring
  supertype), which are the interface a STEP reader must agree with to parse a
  file at all. Facts are not copyrightable subject matter.
- The support table's owner and status columns, the rule support states and
  the representation-item dispositions are this repository's own analysis,
  not upstream content.

The same reasoning is applied in `crates/ifc-template-catalog/data/NOTICE.md` for
generated PSD/QTO catalogs.

## Why they are committed rather than generated at build time

`declaration_manifest.rs` `include_str!`s the two TSVs, so coverage is an
executable audit that runs in CI and in a fresh clone, where the local schema
checkout does not exist. Regeneration requires the local schema
checkout and is a maintenance task, not a build step.

## Directory naming

This directory is deliberately **not** called `references/`.
`scripts/check-leakage.py` rejects that name anywhere in the published source
tree, because it is where unredistributable standards payloads live. Committed
derived artifacts belong in `data/`, matching `crates/ifc-template-catalog/data/`.
