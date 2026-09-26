//! Index every product of an IFC file in Axiolid's BVH (#36).
//!
//! This repository computes each product's world-space bounds; Axiolid owns
//! the index. The two compose with no glue beyond `SpatialItem::new`.
//!
//! ```text
//! cargo run -p ifc-geometry --features compile-reference-backend \
//!     --example product_bvh -- path/to/model.ifc
//! ```
//!
//! Without an argument it indexes a corpus fixture. It prints how each
//! product was bounded, then every pair of products whose boxes overlap,
//! which is the broad phase of a clash test.

use std::path::PathBuf;

use axiolid_core::Tolerance;
use axiolid_spatial::{Bvh, SpatialItem};
use ifc_geometry::compile::product_bounds;
use ifc_geometry::geometric_products;
use ifc_model::Codec;
use ifc_step::StepCodec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../test/fixtures/ifclite-geometry/mapped_instances_multi_item.ifc")
        },
        PathBuf::from,
    );
    let model = StepCodec.read_bytes(&std::fs::read(&path)?)?;

    let mut items = Vec::new();
    for product in geometric_products(&model) {
        let type_name = model.get(product).map_or("?", |e| &*e.type_name);
        match product_bounds(&model, product, Tolerance::MILLIMETRE) {
            Ok(Some(bounds)) => {
                println!(
                    "#{:<6} {type_name:<28} {:?} {:?} .. {:?}",
                    product.0, bounds.source, bounds.aabb.min, bounds.aabb.max
                );
                items.push(SpatialItem::new(product, bounds.aabb));
            }
            Ok(None) => println!("#{:<6} {type_name:<28} no body", product.0),
            Err(error) => println!("#{:<6} {type_name:<28} refused: {error}", product.0),
        }
    }

    let bvh = Bvh::build(items);
    println!(
        "\nindexed {} products ({} rejected)",
        bvh.len(),
        bvh.rejected_items()
    );
    for pair in bvh.overlap_pairs(0.0).pairs {
        println!("overlap: #{} #{}", pair.a.0, pair.b.0);
    }
    Ok(())
}
