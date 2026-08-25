# Contributing

1. Clone `https://github.com/openbimrs/ifc.git`.
2. Read `HERMES.md`, `AGENTS.md`, and the nearest nested context file.
3. Keep changes inside the declared dependency boundaries.
4. Add tests for behavioral changes.
5. Run `./scripts/gate.sh` before opening a pull request.

Do not commit official IFC/ISO schema files unless redistribution rights have
been verified explicitly. Use generated manifests or local reference checkouts
for specification-derived work.

Changes consumed by `openbimrs/openbim` land here first. After the child commit
is pushed and green, the integration repository updates its submodule pin.
