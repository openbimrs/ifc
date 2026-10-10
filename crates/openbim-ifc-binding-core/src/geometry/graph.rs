//! Geometry, Level 2 (#367, ADR 0021): each product's Body as Axiolid's
//! neutral geometry graph, serialised in Axiolid's versioned wire format.
//!
//! The payload is the envelope Axiolid defines (axiolid/kernel#267, Axiolid
//! ADR 0085), JSON text or CBOR bytes:
//!
//! ```json
//! {"format":"axiolid-geometry-graph","version":"1.1","graph":{"nodes":[...],"roots":[...]}}
//! ```
//!
//! Nodes are in insertion order and a reference is the index of an earlier
//! node; the graph is in world coordinates, metres, with the product's
//! placement already applied. A host evaluates it with its own kernel, or
//! reads it back with `GeometryGraph::from_json` / `from_cbor` in Rust.
//! The envelope's `version` is the lowest wire format version its content
//! needs: `1.0`, or `1.1` when a station in it carries a seam-snapping
//! window (#423; a station on an offset whose length is a quadrature).
//! `axiolid-model` 0.3.9 still labels every payload `1.1`
//! (axiolid/kernel#297), so a host reads either; a reader on
//! `axiolid-model` 0.3.8 or older refuses a `1.1` payload.
//! [`GEOMETRY_FORMAT_VERSION`] is the newest version this build writes. A
//! minor version of the wire format reaches every binding as a minor
//! release, a major one is a breaking change of every binding (ADR 0021).

use crate::record::{Field, Record, ToRecord};

use super::GeometryRefusal;
#[cfg(feature = "graph")]
use super::{ids, product_identity, refusal};
use crate::{BindingError, IfcModel};

/// The wire format's name, the envelope's `format` entry.
pub const GEOMETRY_FORMAT: &str = "axiolid-geometry-graph";

/// The newest wire format version this build writes, `MAJOR.MINOR`. A
/// payload's envelope carries the lowest version its content needs, so its
/// `version` entry is this or an older minor of the same major (module
/// documentation).
pub const GEOMETRY_FORMAT_VERSION: &str = "1.1";

/// How [`IfcModel::product_geometry`](crate::IfcModel::product_geometry)
/// encodes each graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum GeometryEncoding {
    /// JSON text (RFC 8259), UTF-8.
    #[default]
    Json,
    /// CBOR bytes (RFC 8949), smaller than the JSON.
    Cbor,
}

impl GeometryEncoding {
    /// `json` or `cbor`; `None` for anything else.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "json" => Some(Self::Json),
            "cbor" => Some(Self::Cbor),
            _ => None,
        }
    }

    /// `json` or `cbor`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Cbor => "cbor",
        }
    }
}

/// One product's Body as a serialised neutral geometry graph.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductGeometry {
    /// The product's entity id.
    pub id: u64,
    /// Its `GlobalId`.
    pub global_id: Option<String>,
    /// Its entity type, upper-case.
    pub type_name: String,
    /// The world placement, 4x4 column-major, metres: the graph already has
    /// it applied, so a host never applies it again. `None` when refused.
    pub transform: Option<[f64; 16]>,
    /// The encoding of `payload`.
    pub encoding: GeometryEncoding,
    /// The wire payload: UTF-8 JSON text or CBOR bytes. `None` with no
    /// refusal is a product with no Body representation (an axis only).
    pub payload: Option<Vec<u8>>,
    /// Why there is no graph.
    pub refusal: Option<GeometryRefusal>,
}

impl ProductGeometry {
    /// The payload as text, for [`GeometryEncoding::Json`].
    #[must_use]
    pub fn json(&self) -> Option<&str> {
        match self.encoding {
            GeometryEncoding::Json => std::str::from_utf8(self.payload.as_deref()?).ok(),
            GeometryEncoding::Cbor => None,
        }
    }
}

impl IfcModel {
    /// The Body of each of `products`, or, with `None`, of every product
    /// that has a shape, in id order, as Axiolid's geometry graph encoded
    /// as `encoding` ([`GEOMETRY_FORMAT`] [`GEOMETRY_FORMAT_VERSION`]).
    ///
    /// A product whose placement, lowering or encoding is refused is a
    /// record with a typed refusal. Refused as a whole only as
    /// [`Self::product_placements`] is, and with `feature-disabled` without
    /// the `graph` feature.
    pub fn product_geometry(
        &self,
        products: Option<&[u64]>,
        encoding: GeometryEncoding,
    ) -> Result<Vec<ProductGeometry>, BindingError> {
        #[cfg(feature = "graph")]
        {
            graphs(self, products, encoding)
        }
        #[cfg(not(feature = "graph"))]
        {
            let _ = (products, encoding);
            Err(BindingError::FeatureDisabled("graph"))
        }
    }
}

#[cfg(feature = "graph")]
fn graphs(
    model: &IfcModel,
    products: Option<&[u64]>,
    encoding: GeometryEncoding,
) -> Result<Vec<ProductGeometry>, BindingError> {
    let products = ids(products);
    let mut out = Vec::new();
    for (product, graph) in ifc::product_graphs(&model.inner, products.as_deref()) {
        let (global_id, type_name) = product_identity(model, product.0)?;
        let mut record = ProductGeometry {
            id: product.0,
            global_id,
            type_name,
            transform: None,
            encoding,
            payload: None,
            refusal: None,
        };
        match graph {
            Ok(lowered) => {
                let payload = lowered.graph.as_ref().map(|g| encode(g, encoding));
                match payload.transpose() {
                    Ok(payload) => {
                        record.transform = Some(ifc::column_major(&lowered.world));
                        record.payload = payload;
                    }
                    Err(error) => record.refusal = Some(wire_refusal(product.0, &error)),
                }
            }
            Err(error) => record.refusal = Some(refusal(&error)),
        }
        out.push(record);
    }
    Ok(out)
}

#[cfg(feature = "graph")]
fn encode(
    graph: &ifc::geometry::GeometryGraph,
    encoding: GeometryEncoding,
) -> Result<Vec<u8>, ifc::geometry::wire::WireError> {
    match encoding {
        GeometryEncoding::Json => graph.to_json().map(String::into_bytes),
        GeometryEncoding::Cbor => graph.to_cbor(),
    }
}

/// A graph the wire writer refuses: in practice a non-finite number the
/// file stated, which neither encoding carries. The file's fault.
#[cfg(feature = "graph")]
fn wire_refusal(product: u64, error: &ifc::geometry::wire::WireError) -> GeometryRefusal {
    GeometryRefusal {
        code: "invalid-model".to_owned(),
        entity: Some(product),
        message: format!("the geometry graph cannot be written: {error}"),
    }
}

/// The record's metadata: the payload crosses separately, as a string or
/// a byte array in each host's idiom, never inside the record.
impl ToRecord for ProductGeometry {
    fn to_record(&self) -> Record {
        Record::new(
            "ProductGeometry",
            vec![
                ("id", Field::Id(self.id)),
                ("global_id", Field::text(self.global_id.clone())),
                ("type_name", Field::Text(self.type_name.clone())),
                ("transform", super::matrix(self.transform.as_ref())),
                ("encoding", Field::Text(self.encoding.name().to_owned())),
                (
                    "payload_size",
                    Field::Count(self.payload.as_ref().map_or(0, Vec::len)),
                ),
                ("refusal", Field::record(self.refusal.as_ref())),
            ],
        )
    }
}

#[cfg(all(test, feature = "graph"))]
mod tests {
    use super::*;

    #[test]
    fn the_constants_are_the_wire_formats() {
        use ifc::geometry::wire::{FORMAT_NAME, FORMAT_VERSION};
        assert_eq!(GEOMETRY_FORMAT, FORMAT_NAME);
        assert_eq!(GEOMETRY_FORMAT_VERSION, FORMAT_VERSION.to_string());
    }
}
