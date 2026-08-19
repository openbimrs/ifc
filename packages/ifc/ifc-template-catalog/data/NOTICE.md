# IFC4 ADD2 TC1 catalog provenance

`ifc4-add2-tc1.bin` is generated from the buildingSMART IFC4 ADD2 TC1 PSD/QTO publication. Normal builds require neither XML, network access, nor the reference checkout.

- Source: https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD2_TC1/HTML/
- Inventory: 420 PSD, 93 QTO, 2,550 properties, 257 quantities
- Ordered source digest: `57227d4c82f9903bc59cb5bade18a49f2c5f2c9363d0293ccb68fed8765d36e3`

- Artifact: 1,537,163 bytes
- Artifact SHA-256: `9a3d0fa550027a89b6ede563daeac05d8f8155f30d25115ece1804dcc67b3eb8`
- Generator: `cargo run -p ifc-template-catalog --features generation --bin ifc-template-catalog-generate -- <HTML-root>`

Upstream names, descriptions, aliases, GUIDs, applicability, units, and type declarations are copyright buildingSMART International Limited and published under CC BY-ND 4.0: https://technical.buildingsmart.org/standards/ifc/ifc-schema-specifications/

The binary is a deterministic format shift of that official catalog without semantic edits. Crate code and Nehirde correction overlays are MIT-licensed and remain separate; overlays never rewrite the official artifact. Redistribution must preserve this attribution and the upstream license.
