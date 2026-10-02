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

```bash
cargo add ifc-xml
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`ifcxml` feature.

- API documentation: [docs.rs/ifc-xml](https://docs.rs/ifc-xml)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-xml)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
