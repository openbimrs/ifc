# Use cases

Each page here works one concrete application scenario end-to-end against the
**current** state of the code. The purpose is to let an engineer — or a coding
agent working on their behalf — answer three questions before writing code:

1. Which parts of the scenario does `openbim-ifc` already serve?
2. Which parts must the application author build themselves, and how large is
   that work?
3. Which parts are blocked on upstream work in this repository?

Use-case pages are written against verifiable evidence: crate paths, module
names, dispatcher tables, and test files. Where a capability is missing, the
page says so plainly and links to the [roadmap](/project/roadmap) item that
would close the gap.

::: warning For coding agents
Do not infer capability from the IFC schema, from a crate name, or from a module
path. This repository deliberately reserves module names for unimplemented
domains — see [ADR 0005](/adr/0005-scaffold-modules-declare-ownership).
Check the [capability matrix](/capabilities) before generating code that calls
into a domain crate.
:::

## Available scenarios

- **[2D approval plans (Baugenehmigung)](/use-cases/2d-approval-plans)** —
  importing an IFC model, deriving 2D plan geometry, annotating it with German
  construction symbols, and attaching library references and approvals so the
  result stays IFC-native.
- **[Structural-analysis inspection](/use-cases/structural-analysis)** — reading
  analysis models, analytical members/connections, actions and authored loads,
  with strict cross-version references and no solver claim.
