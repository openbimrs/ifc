# ifc-template-catalog

Typed, versioned IFC PSD/QTO metadata, separate from authored IFC property data.

```rust
use ifc_template_catalog::{catalog::CatalogProfile, definition::CatalogEdition, embedded::load_catalog};
let catalog = load_catalog(CatalogEdition::Ifc4Add2Tc1, CatalogProfile::Corrected)?;
let wall = catalog.get("Qto_WallBaseQuantities").unwrap();
# Ok::<(), Box<dyn std::error::Error>>(())
```

See `AGENTS.md` for boundaries and `PLAN.md` for task state.
