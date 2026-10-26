# Tables

A table puts the values of many entities side by side, one row per entity
and one column per [value path](/guide/selectors#values-by-path), so they
can be checked, sorted and edited in a spreadsheet and written back. It is
this library's counterpart of IfcOpenShell's `ifccsv`, with the same
selector language on both sides and a checked write.

| Host | Export | Import |
| --- | --- | --- |
| Rust | `ifc::sheet::export` (feature `sheet`) | `ifc::sheet::plan_import`, `import` |
| Command line | `openbim-ifc export-table` | `openbim-ifc import-table` |
| Python | `IfcModel.export_table` | `IfcModel.import_table` |
| JavaScript | `IfcModel.exportTable` | `IfcModel.importTable` |
| C | `openbim_ifc_v0_1_model_export_table` | `openbim_ifc_v0_1_model_import_table` |
| .NET | `IfcModel.ExportTable` | `IfcModel.ImportTable` |

## Columns and keys

The first column is the key: `GlobalId` by default, or `id`, the entity's
`#id` in this file. A `GlobalId` is stable across tools and files; only an
`IfcRoot` carries one, so a table of materials or other resources is keyed
by `id`, and exporting one keyed by `GlobalId` is refused rather than
written with empty keys. Every other column is headed by a value path in
its canonical text: an attribute (`Name`, `PredefinedType`), a property or
quantity (`Pset_WallCommon.FireRating`, `Qto_WallBaseQuantities.Length`),
or a path through a relation (`type.Name`, `container.Name`,
`materials.Name`, `type.Pset_WallCommon.FireRating`). Computed keys
(`class`, `id`, `count`) make useful read-only columns.

<!-- SNIPPET:tables-rust -->

```rust
use ifc::selector::{select, Selector, SelectorIndex, ValuePath};
use ifc::sheet::{export, plan_import, KeyColumn, Sheet, SheetFormat, SheetOptions};

let options = SheetOptions::default().with_key(KeyColumn::Id);
let columns = ["Name", "type.Name", "Pset_WallCommon.FireRating"]
    .map(|path| ValuePath::parse(path).unwrap());
let walls = select(&model, schema, &Selector::parse("IfcWall").unwrap()).unwrap();
let index = SelectorIndex::new(&model, schema);
let sheet = export(&index, &walls, &columns, &options).unwrap();
// Or Tsv, or Xlsx with the `sheet-xlsx` feature.
let csv = String::from_utf8(sheet.write(SheetFormat::Csv).unwrap()).unwrap();
assert!(csv.starts_with("id,Name,type.Name,Pset_WallCommon.FireRating\r\n17,Wall A,WT-200,F90"));

// Edit a cell, read the table back and plan: every change, or every
// refusal by row and column.
let edited = Sheet::read(
    csv.replace("17,Wall A,", "17,Wall A1,").as_bytes(),
    SheetFormat::Csv,
);
let plan =
    plan_import(&index, &edited.unwrap(), &options, ifc::fresh_seed()).expect("no refusal");
for cell in &plan.changes {
    let (before, after) = (&cell.change.before, &cell.change.after);
    println!("row {} {}: {before:?} -> {after:?}", cell.row, cell.path);
}
drop(index);
// One transaction: all of it, or nothing.
plan.commit(&mut model).unwrap();
```

<!-- /SNIPPET -->

## Cell text

Export writes each value as text that reads back as the same value: a
real as the shortest decimal of its exact value, a logical as `TRUE`,
`FALSE` or `UNKNOWN`, an enumeration item as it is spelled, an entity as
`#12`, `NULL` as the empty cell (or the text the null option names, as
IfcOpenShell's `--null` does). A list is written as its items joined with
`; ` and a property set as `name=value` pairs; both are for reading, not
for writing back.

Import reads a changed cell as a value of its target's type, never from the
look of the text:

- an attribute by its declared type in the release: the text verbatim for
  a string type (`IfcLabel`), an integer or a finite real for a number, an
  item of the enumeration in any case, `#12` for a reference;
- a property by the type it already states (`IFCLABEL`,
  `IFCLENGTHMEASURE`), inherited from the type object too, so `F120`
  becomes `IFCLABEL('F120')`;
- where nothing says which type to write, a property no object states yet
  or a SELECT attribute holding nothing, by the type the text names:
  `IFCLABEL(F120)`, `IfcLengthMeasure(2.5)`.

A list, a bounded or a complex value is not written from text. With the
default null text an empty string cannot be written; name another null
text to tell the two apart.

## Import

- A cell whose text is what its path reads now changes nothing. An
  exported table imported unchanged writes nothing and leaves the model's
  revision as it was; a model written after it is byte for byte the model
  written before.
- A changed cell is written through the checked set-by-path API: an
  attribute checked against its declared type (a required one cannot become
  `NULL`), a property through the checked property edit, a `Pset_` or
  `Qto_` set against the PSD/QTO catalog. A property an occurrence inherits
  from its type is written on the occurrence, overriding it; write the
  type's own value through a `type.` column.
- Two rows giving one target different values, such as two walls renaming
  the type they share differently, are refused; the same value is written
  once.
- Every refusal is collected, by row and column: a header that is no value
  path (with the byte span in it), a repeated column, no key column, a row
  of another width, an empty, unknown, ambiguous or repeated key, and each
  cell that cannot be written. Any refusal refuses the whole table:
  nothing is written unless everything can be.
- The writes are one transaction. A plan is a dry run: it lists every
  change (row, column, the entity written, the value before and after)
  without touching the model, and committing it later is refused if the
  model changed in between.

## Formats

CSV follows RFC 4180: UTF-8 (a leading byte-order mark is accepted),
fields quoted where they hold a comma, a quote, a line break or leading or
trailing whitespace, records ending in CRLF; a quote inside an unquoted
field is refused by row and column rather than guessed around. TSV has no
quoting: a tab, line break or backslash in a field is written `\t`, `\n`,
`\r` or `\\`, and any other escape is refused.

XLSX needs the facade feature `sheet-xlsx`: the pure-Rust `rust_xlsxwriter`
writes the workbook and `calamine` reads it, both on the `zip` and
`quick-xml` releases the workspace already links. Every cell is written as
text, so a `GlobalId`, a leading zero or a real's last digit survives a
spreadsheet application; the header row is bold, frozen and filterable.
Reading takes the first worksheet from cell A1: a number typed into a cell
reads as the shortest decimal of its value, a boolean as `TRUE` or `FALSE`,
and a date, a duration or an error cell is refused by row and column. The
command line, Python, C and .NET carry XLSX; the npm package leaves it out
for size and refuses it with `feature-disabled`. ODS is not supported.

## Compared with IfcOpenShell's ifccsv

| | ifccsv | `ifc::sheet` |
| --- | --- | --- |
| Row selection | Selector | Selector, grammar 1.0 |
| Key | `GlobalId` | `GlobalId` or `id` |
| Write-back | Each cell set through the API in turn | One checked transaction, typed by the target, all or nothing |
| Unchanged cells | Written again | Skipped; an unchanged table is a no-op |
| Refusals | An exception at the first | Every refusal, by row and column |
| Dry run | No | A plan listing every change |
| Formats | CSV, ODS, XLSX | CSV, TSV, XLSX |
| Null and booleans | `--null`, `--empty`, `--bool_true`, `--bool_false` | A null text; `TRUE`/`FALSE` |
| Summaries, grouping, sorting | Yes | No; a spreadsheet does them |
