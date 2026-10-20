# openbim-ifc-cli

The `openbim-ifc` command: validate, convert, inspect, query and edit IFC
files (STEP, ifcXML and ifcZIP), and exchange their values as CSV, TSV or
XLSX tables, from a shell or a CI job, with exit codes and JSON or SARIF output
for scripts and code scanning. Pure Rust, one static binary, over the
[`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade.

```sh
cargo install openbim-ifc-cli          # or: cargo binstall openbim-ifc-cli
curl -fsSL https://raw.githubusercontent.com/openbimrs/ifc/main/crates/openbim-ifc-cli/install.sh | sh
nix run github:openbimrs/ifc -- --version
```

```powershell
irm https://raw.githubusercontent.com/openbimrs/ifc/main/crates/openbim-ifc-cli/install.ps1 | iex
```

Each release also attaches Debian packages (`openbim-ifc_<version>_amd64.deb`,
`_arm64.deb`) and a Homebrew formula.

```sh
openbim-ifc validate model.ifc                     # exit 1 on findings
openbim-ifc validate --format sarif models/*.ifc > ifc.sarif
openbim-ifc convert model.ifc model.ifcxml --layout xsd
openbim-ifc info model.ifc
openbim-ifc psets model.ifc '2O2Fr$t4X7Zf8NOew3FLOH' --format csv
openbim-ifc tree model.ifc --elements
openbim-ifc lint model.ifc
openbim-ifc select model.ifc 'IfcWall, Pset_WallCommon.IsExternal=TRUE' --value type.Name
openbim-ifc edit model.ifc 'IfcWall, type=WT-200' --set Pset_WallCommon.FireRating F120 -o edited.ifc
openbim-ifc export-table model.ifc IfcWall -c Name -c type.Name -o walls.xlsx
openbim-ifc import-table model.ifc walls.xlsx --dry-run
openbim-ifc relations model.ifc '#17'
```

| Command | Answers | Formats |
| --- | --- | --- |
| `validate` | Is each file legal against the schema it declares? | human, json, sarif |
| `convert` | The model in STEP or ifcXML (native or XSD layout) | by extension or `--to` |
| `info` | Header, declared schema, entity and type counts | human, json |
| `psets` | Property and quantity sets of one object or type | table, json, csv |
| `tree` | Project, site, building, storey, space and their elements | human, json |
| `lint` | Products with geometry that no model viewer will draw | human, json, sarif |
| `select` | The entities a selector selects, with values read by path | table, json, csv |
| `edit` | Values set by path on selected entities, one checked transaction | human, json |
| `export-table` | Selected entities as a table, one column per value path | csv, tsv, xlsx |
| `import-table` | A table's edited cells written back by key, one checked transaction | human, json |
| `relations` | What one entity is related to, and every relationship naming it | human, json |
| `materials` | Material definitions with layers, profiles or constituents | human, json |
| `cost`, `schedule` | Cost schedules and work schedules as trees | human, json |

Exit codes: 0 success, 1 findings (`validate`, `lint`), 2 a usage error, an
unreadable file or a refusal, 3 a refused edit (`edit`, `import-table`;
nothing written). A refusal names its kind on stderr
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
