# AGENTS.md — apps/

Applications. This is the **one place in the workspace where binding to a
concrete geometry backend is correct** — libraries take the kernel by injection,
an application must choose one.

| App | Binary | Role |
| --- | --- | --- |
| `ifc-cli` | `ifc` | Reference consumer, and the performance harness |

## Why a CLI exists beyond being useful

1. **It proves the library is usable.** An API awkward to drive from a binary is
   awkward to drive from an application.
2. **It is the legitimate site of backend selection** —
   `geom-kernel = { workspace = true, features = ["scalar", "simd"] }`.
3. **It is where the `docs/ROADMAP.md` wall-clock numbers come from.** Numbers
   come from running this, never from assertion.

## Working commands

```bash
cargo run -p ifc-cli -- capabilities    # detected backends + selected boolean impl
cargo run -p ifc-cli -- --version
```

`capabilities` is a real diagnostic, not a placeholder: when a performance number
looks wrong, the first question is which backend actually ran. It currently
reports `mesh boolean: none (not implemented yet)`, which is the honest state.

## Rules

- Keep application concerns (arg parsing, output formatting, backend choice)
  here. If logic is useful to a library consumer, it belongs in a `packages/`
  crate.
- Do not let the CLI become the only place a capability is reachable.
