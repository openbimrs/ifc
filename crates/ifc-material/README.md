# ifc-material

Material definitions as borrowed views over `ifc-model`: materials, layer and
profile sets, constituents, usages, relationships and material property sets,
bound to the release the file declares, with transaction-staged authoring.

```bash
cargo add ifc-material
```

The [`openbim-ifc`](https://crates.io/crates/openbim-ifc) facade also provides it behind its
`material` feature.

To resolve the material of many objects, build one `MaterialIndex` and ask it
per object: it reads the material and typing relationships once, where
`MaterialView::assigned_material` reads them per call.

- API documentation: [docs.rs/ifc-material](https://docs.rs/ifc-material)
- Reference page: [openbimrs.github.io/ifc](https://openbimrs.github.io/ifc/reference/crates/ifc-material)
- Source and issues: [github.com/openbimrs/ifc](https://github.com/openbimrs/ifc)
