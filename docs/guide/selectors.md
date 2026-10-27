# Selectors

A selector is a short query that picks entities out of a model:

```text
IfcWall, Pset_WallCommon.IsExternal=TRUE, location="Level 1" + IfcSlab
```

reads "external walls on Level 1, and every slab". A value path such as
`type.Name` or `Pset_WallCommon.FireRating` reads one value of an entity,
and writes it. Both are the facade's `ifc::selector` module (feature
`selector`, [#464](https://github.com/openbimrs/ifc/issues/464)), bound in
every host and used by the [`select` command](/guide/cli#select).

The syntax follows IfcOpenShell's `ifcopenshell.util.selector`, so a query
written for one usually runs on the other. Where this library differs, it
refuses rather than guesses; every difference is listed
[below](#differences-from-ifcopenshell).

## Grammar, version 1.0

```text
selector   = list { "+" list } [ "+" ]
list       = facet { "," facet }
facet      = [ "!" ] GLOBALID                   instance
           | [ "!" ] CLASS                      class, subtypes included
           | ATTRIBUTE comparison value         explicit attribute
           | key "." key comparison value      property or quantity
           | KEYWORD comparison value           relationship
           | "query:" path comparison value    value path
path       = key { "." key }
comparison = [ "!" ] "=" | [ "!" ] "*=" | ">" | ">=" | "<" | "<="
value      = "NULL" | "TRUE" | "FALSE" | quoted | pattern | unquoted
key        = quoted | pattern | unquoted
quoted     = '"' characters '"'                \" and \\ are escapes
pattern    = "/" characters "/"                 no "/" inside

GLOBALID   22 characters of IFC's alphabet, the first 0-3
CLASS      "Ifc" (any case) then letters, digits and "_"
ATTRIBUTE  an upper-case letter, then letters, digits and "_"
KEYWORD    type | material | classification | location | container | group
unquoted   a key: anything but whitespace and , + " = < > ! * . /
           a value: the same, but "." and "/" are allowed after the start
```

Whitespace between tokens and `/* ... */` comments are ignored. The grammar
version is `GRAMMAR_VERSION` in Rust and its equivalent in each host; the
minor version grows when the grammar accepts more, the major version when
a selector that parsed before parses differently or not at all.

A query parses into a typed AST (`Selector`, `FacetList`, `Facet`), and
every error is a `ParseError` with the byte range it refuses:
`IfcWall, colour=red` is refused at bytes 9..15 with "unknown keyword". The
AST prints back in canonical form, which parses to the same facets:
`container = "Level0"` prints as `location=Level0`.

## How a selector selects

Each facet list works on one set of entities, facet by facet, left to
right:

- a class or `GlobalId` facet adds its entities to the set, and with `!`
  removes them;
- every other facet keeps only the entities it holds for;
- a list whose first facet is a filter or a `!` facet starts from every
  `IfcProduct` and `IfcTypeProduct` (or from the base set a caller passes).

The lists' sets are then united, and the answer is in ascending id order.
`IfcWall, IfcSlab` therefore means walls and slabs, `IfcWall, Name=A`
walls named A, and `IfcWall, Name=A, IfcSlab` walls named A and every slab.

| Facet | Holds for an entity when |
| --- | --- |
| `IfcWall` | it is an `IfcWall` or a subtype, in the declared release |
| `2O2Fr$t4X7Zf8NOew3FLOH` | its `GlobalId` is this one |
| `Name=Wall` | its explicit attribute compares; `PredefinedType` reads the type object's when that states one other than `NOTDEFINED`, and a `USERDEFINED` one as its user-defined name |
| `Pset_WallCommon.FireRating=F90` | the property or quantity compares, its type object's included (an occurrence's own overrides) |
| `type=WT-200` | its type object's `Name` or `GlobalId` compares; a type object compares itself |
| `material=Concrete` | the `Name` or `Category` of a material it is made of compares, through usages, layer, profile and constituent sets and lists; its type's when it has none of its own |
| `classification="04 22 00"` | a classification reference's `Identification` (`ItemReference` in IFC2X3) or `Name` compares, its type's included |
| `location="Level 1"`, `container=...` | the `Name` or `GlobalId` of a structure above it compares: its spatial container, else the whole it is a part of, and so on up to the project |
| `group="Fire doors"` | the `Name` or `GlobalId` of a group it is assigned to compares |
| `query:type.Name=WT-200` | the value the path reads compares |

### Comparisons

| Value | Operators | Holds when |
| --- | --- | --- |
| text | `=`, `*=` | a string or enumeration item equals, or contains, the text |
| a number as text | every operator | an integer or real compares with the text read as a number; `*=` never holds |
| `NULL` | `=`, `!=` | there is no value: `$`, an absent property, no relation |
| `TRUE`, `FALSE` | `=`, `!=` | a boolean (or a logical's `.T.` / `.F.`) is that value |
| `/pattern/` | `=`, `!=` | the pattern matches a string or enumeration item from its start, as Python's `re.match` does |

Text never orders: `Name>A` holds for nothing. A comparison against
several values (a list property, the names of several materials) holds when
it holds for any of them, and `!=` and `!*=` are its complement: `!=` means
"none equals". Patterns use the `regex-lite` syntax: Perl-like classes,
repetition and alternation, no Unicode classes, no look-around.

## Values by path

A path reads one value from an entity, key by key: on an entity a key is a
relation keyword, else an explicit attribute of its type (any case), else a
property or quantity set name; on a property set it names a property; on a
list it applies to each item, or picks one by index. `NULL` ends the path.

| Key | On an entity |
| --- | --- |
| `type` | its type object (a type object itself) |
| `material`, `mat` | its material definition, a usage read as the set it uses |
| `materials`, `mats` | every `IfcMaterial` it is made of |
| `container` | its spatial container |
| `parent` | its container, else the whole it is a part of |
| `space`, `storey`, `building`, `site` | the nearest such structure above it |
| `types`, `occurrences` | the objects a type object types |
| `classification` | its classification references, its type's included |
| `group` | the groups it is assigned to |
| `class` | its class, as the release spells it |
| `id` | its entity id |
| `predefined_type` | the predefined type a `PredefinedType` facet compares |
| `count` | on any value: a list's length, else 1 |

`occurrences.Name` is the names of a type's occurrences,
`Pset_WallCommon` the whole set as name and value pairs,
`Pset_WallCommon./.*Rating/` every rating it holds, `types.0.Name` the
first occurrence's name.

### Writing

A path ending in an attribute name writes that attribute of every entity
the keys before it reach (`Name`, `type.Name`, `materials.Category`); one
ending in a set and a property name writes that property through the
checked property edit (`Pset_WallCommon.FireRating`,
`type.Pset_WallCommon.FireRating`), creating the set or the property when
the object states neither. All targets are staged on one transaction and
every check runs first, so a refused write changes nothing:

- an attribute takes the value its declared type in the release admits:
  bare for a defined type, typed in a `SELECT`, an enumeration item by
  name, a reference to an entity of the declared type, `$` only where the
  attribute is optional; a derived attribute is refused;
- a property takes a typed value, or a plain one that the property already
  states the type of; `Pset_` and `Qto_` sets are checked against the
  PSD/QTO catalog;
- a pattern, a relation or computed key (`type`, `count`) and a prefix
  that reaches no entity are refused as not settable.

Each host passes plain values as it does for attributes written by name:
a host string becomes a label or the enumeration item it names, a host
integer an `INTEGER` or a `REAL` as declared.

Many writes from text, such as the `--set` values of `openbim-ifc edit` or
the cells of a [table](/guide/tables), are planned as one transaction by
`SelectorIndex::plan_text_edits`: each text is read as a value of its
target's declared or stated type, a value already held is left alone, two
writes giving one target different values are refused, and every refusal
is reported with the write it belongs to; nothing is written unless
everything can be.

## In Rust

<!-- SNIPPET:selector-rust -->

```rust
use ifc::selector::{set_value, Selector, SelectorIndex, ValuePath};

// One index per model: each relationship and property scan runs once,
// on first use, for every query and path after it.
let index = SelectorIndex::new(&model, schema);
let query = Selector::parse("IfcWall, Pset_WallCommon.FireRating=F90 + IfcDoor")?;
let type_name = ValuePath::parse("type.Name")?;
for id in index.select(&query)? {
    printed.push(format!("#{} {:?}", id.0, index.value(id, &type_name)?));
}
// #17 Text("WT-200"), #18 Text("WT-200"), #28 Text("D-90")

// A refusal names the byte span of what it refuses.
let error = Selector::parse("IfcWall, colour=red").unwrap_err();
assert_eq!(error.span, 9..15);

// Writes are checked against the release and staged on one transaction.
let mut model = model.clone();
let renamed = ifc::Value::Text("WT-300".into());
set_value(
    &mut model,
    schema,
    EntityId(17),
    &type_name,
    &renamed,
    ifc::fresh_seed()?,
)?;
```

<!-- /SNIPPET -->

`SelectorIndex` borrows the model, so nothing it caches goes stale; drop it
before committing a write. `stage_set_value` returns the transaction
uncommitted.

### Cost

Class facets read the model's type index, subtypes through the release's
tree; `GlobalId` facets a `GlobalIdIndex`; property facets the exact
resolver's `PropertyIndex`, which validates the file's property
relationships once; relationship facets one scan of their relationship
kind. Each is built on first use and kept by the `SelectorIndex`, so a
query costs its relationships plus its working set, never a pass per
element.

### Refusals

| Refusal | When |
| --- | --- |
| `ParseError` | the text does not follow the grammar; `span` is its byte range |
| `UnknownClass`, `UnknownAttribute` | a class, or an attribute no entity of the release declares, with the facet's span; checked before any entity is read |
| `DuplicateGlobalId` | an instance facet names a `GlobalId` several entities carry |
| `Ambiguous`, `Malformed`, `MissingReference` | a relationship the facet reads is contradictory (two types for one object), not the reference the schema declares, or dangling |
| `Property` | the exact property resolver refuses the file or an entity |
| `UnsupportedValue` | a table property, which has no one value to compare |
| `FeatureDisabled` | a property facet or path in a build without `properties` |
| `NotSettable`, `TypeMismatch`, `UntypedValue`, `PropertyEdit`, `Attribute` | a write that names no single value, does not fit, has no type to take, or that the property edit or attribute check refuses |

## In every host

| Host | Select | Read | Write | A refused selector |
| --- | --- | --- | --- | --- |
| Python | `model.select(query, base=None)`, `model.select_ids(...)` | `model.get_value(entity, path)` | `model.set_value(entity, path, value)` | `IfcError` with `code == "invalid-selector"` and `span` |
| JavaScript | `model.select(query, base?)` | `model.getValue(id, path)` | `model.setValue(id, path, value)` | `IfcError` with `code` and `span` |
| C | `openbim_ifc_v0_1_model_select`, `_model_select_from` | `openbim_ifc_v0_1_entity_get_value` | `openbim_ifc_v0_1_entity_set_value` | `INVALID_SELECTOR`, `openbim_ifc_v0_1_last_error_span` |
| .NET | `model.Select(query)`, `model.Select(query, baseIds)` | `model.GetValue(id, path)` | `model.SetValue(id, path, value)` | `IfcException` with `Code` and `Span` |
| Command line | `openbim-ifc select FILE QUERY` | `--value PATH` | | `invalid-selector` on stderr, exit 2 |

Python and JavaScript read a property set as a mapping (`properties` with
`entries`); C and .NET as a list of two-item lists, each the member's name
and value. Spans are byte offsets into the UTF-8 query. The cookbooks have
a recipe for each host:
[Python](/cookbook/python#select-entities-and-read-values-by-path),
[JavaScript](/cookbook/javascript#select-entities-and-read-values-by-path),
[C](/cookbook/c#select-entities-and-read-values-by-path),
[.NET](/cookbook/dotnet#select-entities-and-read-values-by-path).

## Differences from IfcOpenShell

Compared with `ifcopenshell.util.selector` at IfcOpenShell `1a6336bd`:

| Topic | IfcOpenShell | This library |
| --- | --- | --- |
| Unknown class | selects nothing | refused (`UnknownClass`) with its span |
| Attribute no entity declares | compares as `None` | refused (`UnknownAttribute`) with its span |
| A list starting with `!IfcWall` or `!GlobalId` | selects nothing | starts from every product and type product, then removes |
| Class spelling | `Ifc` case-sensitive | any case (`IFCWALL`) |
| Unquoted values | no `.`, so `4.5` needs quotes | `.`, `/` and `:` allowed after the first character |
| Quoted escapes | `\"` | `\"` and `\\` |
| Ordering operators on text, `*=` on numbers | fall back to equality | never hold |
| A number against an integer value | `int()` of the text, so `4.5` never matches | read as a number, so `Length>4.5` works for integers too |
| `NULL`, `TRUE`, `FALSE` and patterns | any operator, compared by identity | `=` and `!=` only; others are a parse error |
| A property facet with a pattern | the first matching property decides | any matching property holds |
| `!=` on `material`, `classification`, `location`, `group` | per facet; `location` and `group` ignore the operator but for negation | every operator, and `!=` is always the complement of `=` |
| `container` keyword | absent | an alias of `location` |
| IFC2X3 classification notations | not read | the notation facets' values compare |
| `parent=` facet | supported | not supported: a parse error |
| Path keys `x`, `y`, `z`, `easting`, `northing`, `elevation`, `rotation_*`, `profiles`, `styles`, `item`, `system`, `zone` | supported | not supported: read as a property set name, so `NULL` |
| A key after a scalar (`Name.x`) | returns the scalar | `NULL` |
| Contradictory or dangling relationships, unreadable property sets | skipped or `None` | refused with a typed error |
| Result order | a set | ascending entity id |
| Writing | coerces strings broadly; `class` reclasses an entity and `predefined_type` sets the predefined type or the user-defined name | checked against the declared type; `class`, `predefined_type` and relation keys are not settable: write `PredefinedType`, and change classes and relations through authoring |
| Pattern syntax | Python `re` | `regex-lite`: no Unicode classes, look-around or back-references |
