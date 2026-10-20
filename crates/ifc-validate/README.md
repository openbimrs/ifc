# ifc-validate

Schema conformance for an IFC model: attribute arity and types, cardinality,
GlobalId and reference integrity, and WHERE rules, checked against the
declared release's own tables in a separate pass from parsing. Rules it cannot
evaluate are reported as such, never passed silently.

```bash
cargo add ifc-validate
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`validate` feature, with the PSD/QTO catalog composed in for the
`Pset_`/`Qto_` name check.

## EXPRESS rules from the release's own text

The IFC schemas are CC BY-ND 4.0, so this crate bundles their structure and
rule labels but never their rule text, and does not generate code from it.
`validate` runs a hand-written subset of the WHERE rules. Given the
release's `.exp` file, which the caller supplies (`scripts/fetch-ifc-schemas.sh`
fetches all five), the `express` module interprets every entity WHERE rule,
type WHERE rule, global rule and FUNCTION of the release under ISO 10303-11
three-valued logic, within step and depth budgets, and checks INVERSE
cardinality; UNKNOWN results are reported as indeterminate, never as a pass
or a violation.

- API documentation: [docs.rs/ifc-validate](https://docs.rs/ifc-validate)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-validate)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
