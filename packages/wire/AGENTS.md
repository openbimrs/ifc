# AGENTS.md — packages/wire/

Encoding substrate. Sits **below** both `packages/ifc/` and
`packages/openbim/`.

| Crate | Scope |
| --- | --- |
| `wire-xml` | XML recognition: BOM handling, content sniffing |
| `wire-zip` | ZIP framing recognition |

## Why this layer exists

`packages/ifc/` must never depend on `packages/openbim/` (see
`../openbim/AGENTS.md`). But `ifc-xml` reads XML and so do BCF, IDS, IDM, LOIN
and ICDD; `ifc-zip` will read ZIP and so do BCF and ICDD.

The shared piece therefore cannot live in `openbim/`. It lives here, below
both. That is the entire justification for this directory — if a proposed
addition is not needed by *both* subtrees, it does not belong here.

## Scope: recognition, not parsing

These crates answer *"what is this byte stream?"*. They do not build element
trees, extract archive entries, or bind schemas — format crates do that, using
`quick-xml` or `zip` directly.

The boundary is deliberate. A shared "XML utilities" crate has no natural
stopping point and ends up absorbing every format's parsing quirks, at which
point it depends on all of them in spirit if not in Cargo.

## Detect by content, never by extension

A file named `.bcf` may be a ZIP or a bare XML document depending on which tool
wrote it, and openBIM files are routinely misnamed in the wild. Dispatching on
the extension produces errors that read like file corruption rather than a
wrong-container guess — which is how an afternoon gets lost.

`../../vendor/solibri/crates/codec/src/container/mod.rs` documents two concrete
instances of this trap and is worth reading before changing sniffing logic.

## `wire-xml` and `wire-zip` are separate on purpose

A consumer reading a plain `.ids` file must not link a ZIP implementation.
Merging them would be one fewer manifest and one more unnecessary dependency
for the most common case.

## No `wire-rdf` yet

ICDD is the only RDF consumer in the workspace. Creating `wire-rdf` now would
be a one-consumer abstraction; add it when a second consumer appears, or keep
the RDF stack inside `openbim-icdd` where it costs nobody else anything.
