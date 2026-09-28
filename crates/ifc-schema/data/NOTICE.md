# NOTICE

Provenance for the bundled schema artifacts in this directory.

## Source

| File | Derived from |
|---|---|
| `ifc2x3-tc1.bin` | IFC2x3 TC1 EXPRESS schema (`IFC2X3_TC1.exp`) |
| `ifc4-add2-tc1.bin` | IFC4 ADD2 TC1 EXPRESS schema (`IFC4.exp`) |
| `ifc4x1-final.bin` | IFC4.1 FINAL EXPRESS schema (`IFC4x1.exp`) |
| `ifc4x2-final.bin` | IFC4.2 FINAL EXPRESS schema (`IFC4x2.exp`) |
| `ifc4x3-add2.bin` | IFC4.3 ADD2 EXPRESS schema (`IFC4X3_ADD2.exp`) |

The schemas are published by buildingSMART International at
<https://standards.buildingsmart.org/> under CC BY-ND 4.0. IFC4.2 was
published under `IFC/DEV/IFC4_2/FINAL/`, which buildingSMART no longer
serves; the fetch script reads the archived copy of that URL and verifies it
against the same pinned checksum. They are not
redistributed here: `scripts/fetch-ifc-schemas.sh` fetches them into the
gitignored `references/ifc-spec/`, which is never a build dependency.

## What these files are

Each file is a compact binary table, written by `tools/generate.rs`, of the
structure a STEP reader and a validator must agree with:

- entity names, supertypes and abstractness;
- explicit attributes in declaration order: name, declared (innermost)
  type name, optional flag, and each aggregation level's kind, bounds and
  uniqueness;
- the names of derived attributes;
- WHERE rules by **label** only;
- INVERSE attributes (name, inverse entity and attribute, cardinality) and
  UNIQUE rules (label and attribute names);
- defined types, enumeration items and select members.

## Why redistribution is permitted

CC BY-ND 4.0 forbids distributing *modified* copies of the licensed work.
These files are not a copy of the schema, modified or otherwise: they hold
no EXPRESS source text, no WHERE-rule or function bodies, and no
documentation. What they record are facts about the interface -- names,
order and types -- that any reader of an IFC file must reproduce to parse
it at all. Facts are not copyrightable subject matter.

WHERE-rule expressions were bundled verbatim until they were removed; the
generator now drops them before encoding.
`tests/real_schemas.rs::bundled_schemas_carry_rule_labels_but_no_rule_text`
keeps it that way, and `scripts/check-leakage.py` fails on EXPRESS rule
syntax in any committed or packaged data file.

The same reasoning is applied in `crates/ifc-geometry/data/NOTICE.md`.
