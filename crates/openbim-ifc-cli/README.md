# openbim-ifc-cli

The `openbim-ifc` command: validate, convert and inspect IFC files (STEP and
ifcXML) from a shell or a CI job, with exit codes and JSON or SARIF output
for scripts and code scanning. Pure Rust, one static binary, over the
[`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade.

```sh
cargo install openbim-ifc-cli          # or: cargo binstall openbim-ifc-cli
curl -fsSL https://raw.githubusercontent.com/openbimrs/ifc/main/crates/openbim-ifc-cli/install.sh | sh
```

```sh
openbim-ifc validate model.ifc                     # exit 1 on findings
openbim-ifc validate --format sarif models/*.ifc > ifc.sarif
openbim-ifc convert model.ifc model.ifcxml --layout xsd
openbim-ifc info model.ifc
openbim-ifc psets model.ifc '2O2Fr$t4X7Zf8NOew3FLOH' --format csv
openbim-ifc tree model.ifc --elements
openbim-ifc lint model.ifc
```

| Command | Answers | Formats |
| --- | --- | --- |
| `validate` | Is each file legal against the schema it declares? | human, json, sarif |
| `convert` | The model in STEP or ifcXML (native or XSD layout) | by extension or `--to` |
| `info` | Header, declared schema, entity and type counts | human, json |
| `psets` | Property and quantity sets of one object or type | table, json, csv |
| `tree` | Project, site, building, storey, space and their elements | human, json |
| `lint` | Products with geometry that no model viewer will draw | human, json, sarif |

Exit codes: 0 success, 1 findings (`validate`, `lint`), 2 a usage error, an
unreadable file or a refusal. A refusal names its kind on stderr
(`unsupported-schema`, `missing-entity`, ...) and never becomes an empty or
guessed answer.

- Guide: [openbimrs.github.io/ifc/guide/cli](https://openbimrs.github.io/ifc/guide/cli)
- Releases and checksums: [GitHub releases](https://github.com/openbimrs/ifc/releases)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)

## Design notes

- The binary depends on the facade and `clap` only;
  `crates/ifc-model/tests/package_architecture.rs` refuses an `ifc-*`
  dependency and any facade feature that links the geometry kernel.
- Every command reads with the facade's codecs and answers with its views;
  this crate parses arguments, chooses a codec and formats results. A
  question the facade cannot answer yet belongs in the facade first.
- JSON and SARIF are written by a small writer in `src/json.rs`, so the
  binary links no serialization framework; the tests parse every document
  with `serde_json` and check SARIF against the 2.1.0 schema's shape.
