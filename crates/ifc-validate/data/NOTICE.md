# NOTICE

## `express-coverage.tsv`

How much of each bundled IFC release the EXPRESS evaluator can evaluate:
per release, the number of entity WHERE rules, type WHERE rules, global
rules, functions and derived attributes the schema declares, how many of
each are evaluable, and the names of any that are not.

It is computed from the official EXPRESS schemas (IFC2X3 TC1, IFC4 ADD2 TC1,
IFC4.1 FINAL, IFC4.2 FINAL, IFC4.3 ADD2), published by buildingSMART
International under CC BY-ND 4.0, by
`tests/express_rules/schemas.rs::the_committed_coverage_table_is_current`,
which fails when the file is stale. The schemas are fetched into the
gitignored `references/ifc-spec/` by `scripts/fetch-ifc-schemas.sh` and never
committed.

The file holds counts and declaration names, which are facts about the
schemas, and this repository's own judgement of what its evaluator
supports. It holds no EXPRESS source text: no rule, function or derive
expression, verbatim or paraphrased. The rules themselves are read from the
caller's copy of the schema at run time (`ifc_validate::express`).
