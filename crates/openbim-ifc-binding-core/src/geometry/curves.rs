//! Polylines per product for the hosts (#458): the curves of a curve
//! representation (`Axis`, `FootPrint`, an annotation's curves), compiled
//! by the reference backend. The facade's `product_curves`.
//!
//! One product is one record and one `f32` array: every polyline's
//! vertices back to back, `x, y, z` in metres relative to the product's
//! world placement, as a mesh's are, so a host draws them with the same
//! matrix. Each [`ProductPolyline`] names its slice (`first_vertex`,
//! `vertex_count`), whether it is closed (its last vertex repeats its
//! first), and the certified bound on how far the exact curve lies from it.
//! A product the backend refuses carries a typed [`GeometryRefusal`]; a
//! trimmed, composite, offset or intrinsic curve is `unsupported`, naming
//! the curve, until the reference compiler polylines it (#506).

use super::GeometryRefusal;
use crate::record::{Field, Record, ToRecord};
use crate::{BindingError, IfcModel};

/// One curve of a product, as a slice of its positions.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductPolyline {
    /// The IFC curve it was compiled from, when the lowering names one.
    pub curve: Option<u64>,
    /// Its first vertex in the product's positions.
    pub first_vertex: usize,
    /// How many vertices it has.
    pub vertex_count: usize,
    /// Whether it is closed: its last vertex is its first.
    pub closed: bool,
    /// The certified bound, metres, on the distance from every point of
    /// the exact curve to the polyline: `0` for a polyline the file
    /// authored, at most `chord_requested` otherwise.
    pub chord_bound: f64,
    /// The chord budget, metres: one millimetre.
    pub chord_requested: f64,
}

/// One product's curves as polylines.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductCurves {
    /// The product's entity id.
    pub id: u64,
    /// Its `GlobalId`.
    pub global_id: Option<String>,
    /// Its entity type, upper-case.
    pub type_name: String,
    /// The world placement, 4x4 column-major, metres. `None` when refused.
    pub transform: Option<[f64; 16]>,
    /// `x, y, z` per vertex, metres, relative to `transform`: every
    /// polyline's vertices in turn.
    pub positions: Vec<f32>,
    /// The curves, in authored order. Empty with no refusal: a product
    /// without the representation, or one with no curves.
    pub polylines: Vec<ProductPolyline>,
    /// Why there are no polylines.
    pub refusal: Option<GeometryRefusal>,
}

impl IfcModel {
    /// The curves of each of `products`' `representation` as polylines
    /// (#458), or, with `None`, of every product that has a shape, in id
    /// order. `representation` is `Plan` by default in the hosts (the best
    /// drawing curves), or `Axis`, `FootPrint`, `Annotation`, `Body`,
    /// `Surface`. Chord budget: one millimetre.
    ///
    /// Refused as a whole as [`Self::product_meshes`] is: `unsupported-schema`,
    /// `feature-disabled` without the `mesh` feature, and `invalid-value`
    /// for a representation name it does not know.
    pub fn product_curves(
        &self,
        products: Option<&[u64]>,
        representation: &str,
    ) -> Result<Vec<ProductCurves>, BindingError> {
        #[cfg(feature = "mesh")]
        {
            curves(self, products, super::purpose(representation)?)
        }
        #[cfg(not(feature = "mesh"))]
        {
            let _ = (products, representation);
            Err(BindingError::FeatureDisabled("mesh"))
        }
    }
}

#[cfg(feature = "mesh")]
fn curves(
    model: &IfcModel,
    products: Option<&[u64]>,
    purpose: ifc::RepresentationPurpose,
) -> Result<Vec<ProductCurves>, BindingError> {
    let products = super::ids(products);
    let mut out = Vec::new();
    for (product, curves) in ifc::product_curves(&model.inner, products.as_deref(), purpose) {
        let (global_id, type_name) = super::product_identity(model, product.0)?;
        let mut record = ProductCurves {
            id: product.0,
            global_id,
            type_name,
            transform: None,
            positions: Vec::new(),
            polylines: Vec::new(),
            refusal: None,
        };
        match curves {
            Ok(curves) => {
                record.transform = Some(ifc::column_major(&curves.world));
                for polyline in curves.polylines {
                    record.polylines.push(ProductPolyline {
                        curve: polyline.curve.map(|id| id.0),
                        first_vertex: record.positions.len() / 3,
                        vertex_count: polyline.positions.len() / 3,
                        closed: polyline.closed,
                        chord_bound: polyline.chord_bound,
                        chord_requested: polyline.chord_requested,
                    });
                    record.positions.extend(polyline.positions);
                }
            }
            Err(error) => record.refusal = Some(super::refusal(&error)),
        }
        out.push(record);
    }
    Ok(out)
}

impl ToRecord for ProductPolyline {
    fn to_record(&self) -> Record {
        Record::new(
            "ProductPolyline",
            vec![
                ("curve", Field::id(self.curve)),
                ("first_vertex", Field::Count(self.first_vertex)),
                ("vertex_count", Field::Count(self.vertex_count)),
                ("closed", Field::Bool(self.closed)),
                ("chord_bound", Field::Real(self.chord_bound)),
                ("chord_requested", Field::Real(self.chord_requested)),
            ],
        )
    }
}

/// The curves' metadata: the positions cross separately, in each host's
/// typed-array form, as a mesh's do.
impl ToRecord for ProductCurves {
    fn to_record(&self) -> Record {
        Record::new(
            "ProductCurves",
            vec![
                ("id", Field::Id(self.id)),
                ("global_id", Field::text(self.global_id.clone())),
                ("type_name", Field::Text(self.type_name.clone())),
                ("transform", super::matrix(self.transform.as_ref())),
                ("vertex_count", Field::Count(self.positions.len() / 3)),
                ("polylines", Field::records(&self.polylines)),
                ("refusal", Field::record(self.refusal.as_ref())),
            ],
        )
    }
}
