# ifc-xml

The ifcXML (ISO 10303-28) codec for the IFC model: the same `ifc_model::Codec`
trait as `ifc-step`, over the same model. Two layouts:

- **Its own lossless layout**, read and written. With a schema it writes
  conformant named attributes and reads strictly: every value is typed from
  its declaration (`Name="1"` is a label, not an integer) and a name the
  entity does not declare is a typed error, never a value in a free slot.
  `SchemaReading::Lenient` restores inference. Without a schema it falls back
  to marked positional names; both round-trip losslessly.
- **The buildingSMART XSD configuration** of IFC4 ADD2 TC1 and IFC4X3 ADD2,
  read and written (`XmlCodec::xsd`): nested and inline entities, `ref`/`href`
  references, inverse attributes, `-wrapper` typed values and list
  attributes, read into the same model as the document's STEP form. The
  writer writes that model back as a document that validates against
  `IFC4.xsd` and reads back to the same model. Each attribute's form comes
  from one derivation both share, checked against both XSDs. What either
  direction cannot carry exactly is refused with a typed error
  (`XmlError::Unrepresentable` for a model the configuration cannot
  represent), never approximated.

A strict release profile (`XmlCodec::strict`, `with_schema_and_profile`)
fixes the namespace and schema token of the native layout; it does not make
the output valid against the release XSD, and does not claim to. Use
`XmlCodec::xsd` for that.

```bash
cargo add ifc-xml
```

## Checking output against the official XSD

An opt-in test validates output with `xmllint` (libxml2) against the release
XSD. Every IFC4 fixture written with `XmlCodec::xsd` must validate with no
error. Strict native output must be well-formed, rooted in the XSD's
`ifcXML` element and target namespace, and depart only as documented (the
`schema` root attribute, upper-case entity elements, the header order). The
published `IFC4X3_ADD2.xsd` does not compile in any conforming validator,
which the test records; for that release it checks well-formedness and the
namespace. The XSDs are CC BY-ND 4.0 and never committed; normal builds and
the gate do not need them, the network or `xmllint`. Once run, anything
missing is a failure:

```bash
scripts/fetch-ifc-schemas.sh          # IFC4.xsd, IFC4X3_ADD2.xsd into references/ifc-spec
sudo apt-get install libxml2-utils    # xmllint; or set XMLLINT=<path>
cargo test -p ifc-xml --test xsd_output -- --ignored --nocapture
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`ifcxml` feature.

- API documentation: [docs.rs/ifc-xml](https://docs.rs/ifc-xml)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-xml)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
