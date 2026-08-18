# AGENTS.md — references/ifc-spec/

The **official buildingSMART schema releases**. Real tree lives on
`/mnt/backup/references/ifc-spec/` (249 MB); `references/ifc-spec` is a symlink.
Nothing here is committed, and no crate may read these paths at build or run
time — they are the authority you *consult*, and code generation output gets
committed, not the input.

## What is here

| Path | Contents | Size |
| --- | --- | --- |
| `ifc2x3-tc1/IFC2X3_TC1.exp` | EXPRESS schema, IFC2x3 TC1 | 261 KB |
| `ifc2x3-tc1/express-longform/` | Long-form EXPRESS distribution | |
| `ifc2x3-tc1/psd/` | 317 property-set definition XMLs | 2 MB |
| `ifc4-add2-tc1/IFC4.exp` | EXPRESS schema, IFC4 ADD2 TC1 (the ISO one) | 364 KB |
| `ifc4-add2-tc1/IFC4.xsd` | ifcXML schema | 630 KB |
| `ifc4-add2-tc1/dist/` | Full HTML documentation + 420 psd/qto XMLs | 246 MB |
| `ifc4x3-add2/IFC4X3_ADD2.exp` | EXPRESS schema, IFC4x3 ADD2 | 394 KB |

Source: `https://standards.buildingsmart.org/IFC/RELEASE/...`.

## Re-fetch

The server **403s without a browser User-Agent** — this is the single most
likely reason a re-download "fails":

```bash
UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120 Safari/537.36"
B=/mnt/backup/references/ifc-spec
curl -A "$UA" -o $B/ifc2x3-tc1/IFC2X3_TC1.exp \
  https://standards.buildingsmart.org/IFC/RELEASE/IFC2x3/TC1/EXPRESS/IFC2X3_TC1.exp
curl -A "$UA" -o $B/ifc4-add2-tc1/IFC4.exp \
  https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/EXPRESS/IFC4.exp
curl -A "$UA" -o $B/ifc4x3-add2/IFC4X3_ADD2.exp \
  https://standards.buildingsmart.org/IFC/RELEASE/IFC4_3/HTML/IFC4X3_ADD2.exp
```

Note the 4x3 `.exp` sits under `HTML/`, not `EXPRESS/`. Directory listings are
browsable, so `curl -A "$UA" <dir>/` is how you find a moved file.

## Measured facts (recomputed 2026-08-18, from these files)

| Schema | Entities | Types | Functions | Global rules |
| --- | --- | --- | --- | --- |
| IFC2x3 TC1 | 653 | — | — | — |
| IFC4 ADD2 TC1 | 776 | 397 | 47 | 2 |
| IFC4x3 ADD2 | 876 | — | — | — |

IFC4x3 adds **116 entities** over IFC4 and removes **16**.

**The rename that shapes our design:** `IfcBuildingElement` → `IfcBuiltElement`
in 4x3 (also gone: `IfcProxy`, the `*StandardCase` family, `IfcDoorStyle`,
`IfcWindowStyle`, `IfcPresentationStyleAssignment`). Entity *names* differ
across versions, which is exactly why the schema is data and not generated
types — see `docs/adr/0001` and `docs/adr/0005`.

## How to use this

Re-derive rather than trust a number in a doc; the schemas are right here:

```bash
# entity count
grep -cE '^ENTITY' ifc4-add2-tc1/IFC4.exp
# every subtype of a thing
grep -E '^ENTITY.*ProfileDef' ifc4-add2-tc1/IFC4.exp
# what 4x3 added
comm -13 <(grep -oE '^ENTITY [A-Za-z]+' ifc4-add2-tc1/IFC4.exp | sort) \
         <(grep -oE '^ENTITY [A-Za-z]+' ifc4x3-add2/IFC4X3_ADD2.exp | sort)
```

## Licence

buildingSMART specifications are published under **CC BY-ND 4.0**. Reference
freely, cite clause/entity names in prose, and do not redistribute the documents
from this repo. The `.exp`/`.xsd` files are the machine-readable schema; deriving
a data table from them is the intended use.

## Agent rules

1. **Never commit anything under this directory.** `.gitignore` covers
   `references/*`; only the `AGENTS.md` files are tracked.
2. **No build-time dependency.** A generator script may read these paths and
   write a committed Rust/data file; the resulting file is what crates use, so a
   clean checkout without `/mnt/backup` still builds.
3. **The schema is the authority over any doc, including this one.** If a count
   here disagrees with `grep`, the file is right and this file is stale — fix it.
