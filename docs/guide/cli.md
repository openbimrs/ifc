# Command line

`openbim-ifc` validates, converts and inspects IFC files from a shell or a CI
job. It is one binary over the [`openbim-ifc`](/reference/crates/openbim-ifc)
facade: every answer is a facade call, so the command reads exactly what the
library reads. Source: [`openbim-ifc-cli`](/reference/crates/openbim-ifc-cli).

## Install

On Linux and macOS, the install script picks the archive for the machine,
checks it against the release's `SHA256SUMS` and installs the binary to
`~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/openbimrs/ifc/main/crates/openbim-ifc-cli/install.sh | sh
curl -fsSL https://raw.githubusercontent.com/openbimrs/ifc/main/crates/openbim-ifc-cli/install.sh | sh -s -- --version 0.1.0 --prefix /usr/local
```

It refuses to install when the checksum does not match. Without `--version`
it installs the newest `openbim-ifc-cli` release; `--print-target` shows the
archive it would choose.

With a Rust toolchain:

```bash
cargo install openbim-ifc-cli      # builds from source
cargo binstall openbim-ifc-cli     # takes the release binary
```

With Homebrew, once the tap exists (see [Homebrew](#homebrew)):

```bash
brew install openbimrs/tap/openbim-ifc
```

Or download an archive from an `openbim-ifc-cli-v*`
[GitHub release](https://github.com/openbimrs/ifc/releases) and check it
yourself:

| Platform | Archive |
| --- | --- |
| Linux x86_64 (static) | `openbim-ifc-v<version>-x86_64-unknown-linux-musl.tar.gz` |
| Linux aarch64 (static) | `openbim-ifc-v<version>-aarch64-unknown-linux-musl.tar.gz` |
| macOS Intel | `openbim-ifc-v<version>-x86_64-apple-darwin.tar.gz` |
| macOS Apple silicon | `openbim-ifc-v<version>-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `openbim-ifc-v<version>-x86_64-pc-windows-msvc.zip` |

```bash
sha256sum --check --ignore-missing SHA256SUMS
```

## Commands

| Command | Answers | Formats |
| --- | --- | --- |
| `validate FILE...` | Is each file legal against the schema its header declares? | `human`, `json`, `sarif` |
| `convert IN OUT` | The model in STEP or ifcXML, native or XSD layout | from the extension, or `--to` |
| `info FILE` | Header, declared schema, entity and type counts | `human`, `json` |
| `psets FILE ENTITY` | Property and quantity sets of one object or type object | `table`, `json`, `csv` |
| `tree FILE` | Project, site, building, storey and space, with their elements | `human`, `json` |
| `lint FILE...` | Products with geometry that no model viewer will draw | `human`, `json`, `sarif` |

`openbim-ifc <command> --help` lists every option. Each command reads STEP or
ifcXML, chosen by content first and extension second.

### validate

```bash
openbim-ifc validate model.ifc
openbim-ifc validate --format json --deny-warnings models/*.ifc
```

Each file is validated against the tables of the release its own
`FILE_SCHEMA` declares; a file declaring none, or a release this build does
not bundle, is refused rather than checked against another release. A human
report prints one line per finding, `file:line: severity [rule] #12.Name:
message`, with the line of the entity's record in a STEP file, then a
summary per file.

A file fails on an error, on an evaluation error (a rule that could not be
decided for an instance), and on a truncated report (`--max-findings`
reached, so the counts are lower bounds); with `--deny-warnings`, on a
warning too. Rules this validator does not evaluate are counted in the
summary and never fail a file; `--include-unsupported` lists them.

### convert

```bash
openbim-ifc convert model.ifc model.ifcxml                 # native layout
openbim-ifc convert model.ifc model.ifcxml --layout xsd    # buildingSMART XSD layout
openbim-ifc convert model.ifcxml model.ifc
openbim-ifc convert model.ifc - --to ifcxml | gzip > model.ifcxml.gz
```

The native layout is this library's lossless ifcXML for any release, with
attribute names from the declared release's tables when the build bundles
them. The XSD layout is the buildingSMART configuration of IFC4 ADD2 TC1 and
IFC4X3 ADD2; a model it cannot carry exactly is refused with the codec's
reason and nothing is written. On reading, the layout is detected from the
root element (the native layout carries a `schema` attribute, the XSD layout
declares its release's namespace); `--input-layout` names it when neither is
present. An existing output is replaced only with `--force`.

### info, psets, tree

```bash
openbim-ifc info model.ifc --all-types
openbim-ifc psets model.ifc '#1234'
openbim-ifc psets model.ifc '2O2Fr$t4X7Zf8NOew3FLOH' --format csv > wall.csv
openbim-ifc tree model.ifc --elements
```

`psets` takes `#12`, `12` or a 22-character `GlobalId` and resolves exactly:
the object's own sets first, then the sets its type object holds (marked
`type #id`), an occurrence property overriding an inherited one of the same
set and name. A complex property's members follow it as `Parent.Member`
rows in the table and CSV, and stay inside its value in JSON. Quote a
`GlobalId` in the shell: it may contain `$`.

`tree` prints the containers from the project down, with each container's
element count or, with `--elements`, its elements, followed by what the tree
could not honour: a second parent, a containment in something that is no
container, a relationship naming an entity the file lacks.

### lint

```bash
openbim-ifc lint model.ifc
```

Reports products with geometry that a model viewer will not draw: outside
the spatial structure, or with geometry only in contexts a model viewer
skips. Openings, parts, containers and products without a shape are
legitimately outside the containment tree and are not reported. Any finding
fails the run.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success; for `validate` and `lint`, nothing that fails the run |
| 1 | `validate` or `lint` found something that fails the run |
| 2 | A usage error, an unreadable or unparseable file, or a refusal |

A refusal prints its kind on stderr, `openbim-ifc: <kind>: <detail>`, and
is never turned into an empty or guessed answer:

| Kind | Meaning |
| --- | --- |
| `usage` | The arguments contradict each other or the input |
| `io` | A file could not be read or written |
| `parse` | Not a STEP or ifcXML file, or it does not parse |
| `unsupported-schema` | No schema declared, or one this build bundles no tables for |
| `unsupported` | Something this tool or the library does not support for this input |
| `missing-entity` | No entity with that id or `GlobalId` |
| `wrong-entity-type` | The entity cannot carry what was asked for |
| `missing-reference` | A reference names an entity the file lacks |
| `invalid-model` | The file is malformed or ambiguous where an exact answer is needed |
| `budget-exceeded` | A traversal hit its bound before it could answer |
| `write` | The output format refused the model |

`validate` and `lint` take several files; one that cannot be checked is
reported (in JSON as `error`, in SARIF as a tool execution notification),
the others are still checked, and the run exits 2.

## In CI

`--format sarif` writes SARIF 2.1.0: one result per finding with the file,
the line of the entity's record, and the entity or attribute (`#12`,
`#12.Name`) as a logical location. Errors and evaluation errors are
`error`, warnings `warning`, unevaluated rules `note`. On GitHub:

```yaml
- run: openbim-ifc validate --format sarif models/*.ifc > ifc.sarif
  continue-on-error: true
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: ifc.sarif
```

`--format json` of `validate` is one document: `tool`, `version`, the
run's `passed` verdict, and per file `file`, `schema`, `passed`,
`conformant`, `truncated`, `summary` (`errors`, `evaluation_errors`,
`warnings`, `unsupported`) and `findings`, each with `severity`, `rule`,
`path`, `entity`, `attribute_index`, `attribute_name`, `line` and `message`;
a file that could not be checked has `error` (`kind`, `message`) instead.

## Homebrew

Each `openbim-ifc-cli-v*` release attaches `openbim-ifc.rb`, a formula
generated from the release's `SHA256SUMS` by
`crates/openbim-ifc-cli/scripts/package.py` (template:
`crates/openbim-ifc-cli/packaging/openbim-ifc.rb.in`). The release workflow
also pushes it to the tap repository when it can. Setting up the tap is a
maintainer's step, done once:

1. Create the public repository `openbimrs/homebrew-tap` with a `Formula/`
   directory (an empty `Formula/.gitkeep` and a README are enough).
   Homebrew finds it as `openbimrs/tap`.
2. Create a fine-grained personal access token (or a GitHub App token)
   with **Contents: read and write** on `openbimrs/homebrew-tap` only.
3. Store it in `openbimrs/ifc` as the Actions secret `HOMEBREW_TAP_TOKEN`.
4. On the next `openbim-ifc-cli-v*` tag, the `cli-assets` job of
   `release.yml` commits `Formula/openbim-ifc.rb` to the tap. For a release
   made before the token existed, copy `openbim-ifc.rb` from that release
   into `Formula/` by hand.
5. Check it: `brew install openbimrs/tap/openbim-ifc && brew test openbim-ifc`.

Without the secret the job still attaches the formula to the release and
notes that no tap was updated.
