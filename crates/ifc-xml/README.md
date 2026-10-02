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
  read only (`XmlCodec::xsd`): nested and inline entities, `ref`/`href`
  references, inverse attributes, `-wrapper` typed values and list
  attributes, read into the same model as the document's STEP form. What it
  cannot read exactly is refused with a typed error.

A strict release profile (`XmlCodec::strict`, `with_schema_and_profile`)
fixes the namespace and schema token of the native layout; it does not make
the output valid against the release XSD, and does not claim to. Writing the
XSD configuration is [#274](https://github.com/openbimrs/ifc/issues/274).

```bash
cargo add ifc-xml
```

## Checking strict output against the official XSD

An opt-in test writes every IFC4 and IFC4X3 fixture with the strict profile
and validates it with `xmllint` (libxml2) against the release XSD. It checks
what the profile claims: well-formed XML whose root is the XSD's `ifcXML`
element in the XSD's target namespace, and no XSD error beyond the documented
native-layout departures (the `schema` root attribute, upper-case entity
elements, the header order). The published `IFC4X3_ADD2.xsd` does not compile
in any conforming validator, which the test records. The XSDs are CC BY-ND
4.0 and never committed; normal builds and the gate do not need them, the
network or `xmllint`. Once run, anything missing is a failure:

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
