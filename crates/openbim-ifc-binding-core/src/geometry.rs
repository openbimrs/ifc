//! Geometry for the hosts (#328, ADR 0021), at two levels.
//!
//! - **Placements** (feature `placements`, default): per product, the world
//!   placement as a 4x4 column-major matrix in metres and the Body
//!   representation a viewer draws, with its identifier, type and context.
//!   Kernel-free: the facade's `product_placements`.
//! - **Meshes** (feature `mesh`, opt-in): per product, triangles compiled by
//!   the reference backend, `f32` positions relative to the product's world
//!   placement and `u32` indices. The facade's `product_meshes`; it links an
//!   execution provider, which ADR 0004 keeps out of every default build.
//!
//! The serialised neutral representation between the two is deferred until
//! Axiolid promises its model as a stable format (ADR 0021).
//!
//! A product that cannot be placed, selected or meshed is a record with a
//! typed [`GeometryRefusal`], never an error that aborts the call: one
//! broken wall does not hide the building. The refusal codes are the
//! shared binding codes `unsupported`, `invalid-model`, `missing-reference`
//! and `budget-exceeded`. Without its feature an operation refuses with
//! `feature-disabled`.

use crate::record::{Field, Record, ToRecord};
use crate::{BindingError, IfcModel};

/// Why one product has no placement, representation or mesh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeometryRefusal {
    /// `unsupported` (valid IFC this build does not interpret, or a mesh
    /// the backend refused), `invalid-model` (geometry the file states
    /// wrongly), `missing-reference` (a reference to an entity the file
    /// lacks) or `budget-exceeded` (a cycle, or a chain or aggregate over
    /// its limit).
    pub code: String,
    /// The entity at fault, when the refusal names one.
    pub entity: Option<u64>,
    /// A one-line explanation.
    pub message: String,
}

/// The `IfcShapeRepresentation` selected as a product's Body, and its
/// context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedRepresentation {
    /// The representation's entity id.
    pub id: u64,
    /// `RepresentationIdentifier`, e.g. `Body`.
    pub identifier: Option<String>,
    /// `RepresentationType`, e.g. `SweptSolid`.
    pub representation_type: Option<String>,
    /// `ContextOfItems`, when the file contains it.
    pub context: Option<u64>,
    /// The context's `ContextType`, e.g. `Model` (a sub-context's derived
    /// from its parent).
    pub context_type: Option<String>,
    /// The context's `ContextIdentifier`, e.g. `Body`.
    pub context_identifier: Option<String>,
    /// A sub-context's `TargetView` literal, e.g. `MODEL_VIEW`.
    pub target_view: Option<String>,
}

/// One product's world placement and selected Body representation.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductPlacement {
    /// The product's entity id.
    pub id: u64,
    /// Its `GlobalId`.
    pub global_id: Option<String>,
    /// Its entity type, upper-case: `IFCWALL`, ...
    pub type_name: String,
    /// The world placement, 4x4 column-major, metres; `None` when refused.
    pub transform: Option<[f64; 16]>,
    /// The Body representation; `None` when the product has no solid
    /// representation (an axis only) or its selection was refused.
    pub representation: Option<SelectedRepresentation>,
    /// Why the placement or the selection was refused, the placement's
    /// reason first.
    pub refusal: Option<GeometryRefusal>,
}

/// One product's Body as triangles.
#[derive(Debug, Clone, PartialEq)]
pub struct ProductMesh {
    /// The product's entity id.
    pub id: u64,
    /// Its `GlobalId`.
    pub global_id: Option<String>,
    /// Its entity type, upper-case.
    pub type_name: String,
    /// The world placement, 4x4 column-major, metres: a vertex's world
    /// position is this matrix applied to it. `None` when refused.
    pub transform: Option<[f64; 16]>,
    /// `x, y, z` per vertex, metres, relative to `transform`.
    pub positions: Vec<f32>,
    /// Three vertex indices per triangle.
    pub indices: Vec<u32>,
    /// Why there is no mesh. Empty arrays and no refusal is a product
    /// with no Body representation (an axis only).
    pub refusal: Option<GeometryRefusal>,
}

impl IfcModel {
    /// The world placement and selected Body of each of `products`, or,
    /// with `None`, of every product that has a shape, in id order.
    ///
    /// An id the model lacks is a record refused with `missing-reference`.
    /// Refused as a whole only with `unsupported-schema` (the header names
    /// a release this build does not bundle, as for the domain views) and
    /// `feature-disabled` without the `placements` feature.
    pub fn product_placements(
        &self,
        products: Option<&[u64]>,
    ) -> Result<Vec<ProductPlacement>, BindingError> {
        #[cfg(feature = "placements")]
        {
            placements(self, products)
        }
        #[cfg(not(feature = "placements"))]
        {
            let _ = products;
            Err(BindingError::FeatureDisabled("placements"))
        }
    }

    /// The Body mesh of each of `products`, or, with `None`, of every
    /// product that has a shape, in id order. Tolerance: one millimetre.
    ///
    /// Refused as a whole only as [`Self::product_placements`] is, and with
    /// `feature-disabled` without the `mesh` feature.
    pub fn product_meshes(
        &self,
        products: Option<&[u64]>,
    ) -> Result<Vec<ProductMesh>, BindingError> {
        #[cfg(feature = "mesh")]
        {
            meshes(self, products)
        }
        #[cfg(not(feature = "mesh"))]
        {
            let _ = products;
            Err(BindingError::FeatureDisabled("mesh"))
        }
    }
}

#[cfg(feature = "placements")]
fn ids(products: Option<&[u64]>) -> Option<Vec<ifc::EntityId>> {
    products.map(|ids| ids.iter().copied().map(ifc::EntityId).collect())
}

/// `GlobalId` and entity type of a product, or of an id the model lacks.
#[cfg(feature = "placements")]
fn product_identity(model: &IfcModel, id: u64) -> Result<(Option<String>, String), BindingError> {
    let identity = model.identity(id)?;
    let type_name = model
        .inner
        .get(ifc::EntityId(id))
        .map_or_else(String::new, |entity| entity.type_name.to_string());
    Ok((identity.global_id, type_name))
}

#[cfg(feature = "placements")]
fn placements(
    model: &IfcModel,
    products: Option<&[u64]>,
) -> Result<Vec<ProductPlacement>, BindingError> {
    let products = ids(products);
    let mut out = Vec::new();
    for placement in ifc::product_placements(&model.inner, products.as_deref()) {
        let id = placement.product.0;
        let (global_id, type_name) = product_identity(model, id)?;
        let refusal = placement
            .world
            .as_ref()
            .err()
            .or(placement.body.as_ref().err())
            .map(refusal);
        let representation = placement.body.ok().flatten().map(|body| {
            let context = body.context.as_ref();
            SelectedRepresentation {
                id: body.id.0,
                identifier: body.identifier.clone(),
                representation_type: body.representation_type.clone(),
                context: context.map(|c| c.id.0),
                context_type: context.and_then(|c| c.context_type.clone()),
                context_identifier: context.and_then(|c| c.identifier.clone()),
                target_view: context.and_then(|c| c.target_view.clone()),
            }
        });
        out.push(ProductPlacement {
            id,
            global_id,
            type_name,
            transform: placement.world.ok().map(|world| ifc::column_major(&world)),
            representation,
            refusal,
        });
    }
    Ok(out)
}

#[cfg(feature = "mesh")]
fn meshes(model: &IfcModel, products: Option<&[u64]>) -> Result<Vec<ProductMesh>, BindingError> {
    let products = ids(products);
    let mut out = Vec::new();
    for (product, mesh) in ifc::product_meshes(&model.inner, products.as_deref()) {
        let (global_id, type_name) = product_identity(model, product.0)?;
        let mut record = ProductMesh {
            id: product.0,
            global_id,
            type_name,
            transform: None,
            positions: Vec::new(),
            indices: Vec::new(),
            refusal: None,
        };
        match mesh {
            Ok(mesh) => {
                record.transform = Some(ifc::column_major(&mesh.world));
                record.positions = mesh.positions;
                record.indices = mesh.indices;
            }
            Err(error) => record.refusal = Some(refusal(&error)),
        }
        out.push(record);
    }
    Ok(out)
}

/// The typed refusal for a geometry error.
#[cfg(feature = "placements")]
fn refusal(error: &ifc::geometry::GeometryError) -> GeometryRefusal {
    use ifc::geometry::GeometryError as E;
    let code = match error {
        E::MissingEntity { .. } => "missing-reference",
        E::CyclicChain { .. } | E::ChainTooDeep { .. } | E::AggregateTooLarge { .. } => {
            "budget-exceeded"
        }
        E::Unsupported { .. } => "unsupported",
        #[cfg(feature = "mesh")]
        E::CompilationRefused { .. } => "unsupported",
        // A net refusal is as typed as its cause; gross meshes never raise it.
        E::OpeningNotSubtracted { cause, .. } => return refusal(cause),
        E::MissingAttribute { .. }
        | E::WrongValueKind { .. }
        | E::WrongEntityType { .. }
        | E::Degenerate { .. }
        | E::Units(_) => "invalid-model",
        // A refusal added to the bridge after this binding, or one only an
        // evaluator raises: still typed, never dropped.
        other if other.is_unsupported() => "unsupported",
        _ => "invalid-model",
    };
    GeometryRefusal {
        code: code.to_owned(),
        entity: error.entity().map(|id| id.0),
        message: error.to_string(),
    }
}

/// A column-major matrix as a list of 16 reals, or `NULL`.
fn matrix(transform: Option<&[f64; 16]>) -> Field {
    Field::optional(transform, |m| {
        Field::List(m.iter().copied().map(Field::Real).collect())
    })
}

impl ToRecord for GeometryRefusal {
    fn to_record(&self) -> Record {
        Record::new(
            "GeometryRefusal",
            vec![
                ("code", Field::Text(self.code.clone())),
                ("entity", Field::id(self.entity)),
                ("message", Field::Text(self.message.clone())),
            ],
        )
    }
}

impl ToRecord for SelectedRepresentation {
    fn to_record(&self) -> Record {
        Record::new(
            "SelectedRepresentation",
            vec![
                ("id", Field::Id(self.id)),
                ("identifier", Field::text(self.identifier.clone())),
                (
                    "representation_type",
                    Field::text(self.representation_type.clone()),
                ),
                ("context", Field::id(self.context)),
                ("context_type", Field::text(self.context_type.clone())),
                (
                    "context_identifier",
                    Field::text(self.context_identifier.clone()),
                ),
                ("target_view", Field::text(self.target_view.clone())),
            ],
        )
    }
}

impl ToRecord for ProductPlacement {
    fn to_record(&self) -> Record {
        Record::new(
            "ProductPlacement",
            vec![
                ("id", Field::Id(self.id)),
                ("global_id", Field::text(self.global_id.clone())),
                ("type_name", Field::Text(self.type_name.clone())),
                ("transform", matrix(self.transform.as_ref())),
                (
                    "representation",
                    Field::record(self.representation.as_ref()),
                ),
                ("refusal", Field::record(self.refusal.as_ref())),
            ],
        )
    }
}

/// The mesh's metadata: the arrays cross separately, in each host's
/// typed-array form (`Float32Array`, `array('f')`, a caller `float`
/// buffer), never as a list of reals.
impl ToRecord for ProductMesh {
    fn to_record(&self) -> Record {
        Record::new(
            "ProductMesh",
            vec![
                ("id", Field::Id(self.id)),
                ("global_id", Field::text(self.global_id.clone())),
                ("type_name", Field::Text(self.type_name.clone())),
                ("transform", matrix(self.transform.as_ref())),
                ("vertex_count", Field::Count(self.positions.len() / 3)),
                ("triangle_count", Field::Count(self.indices.len() / 3)),
                ("refusal", Field::record(self.refusal.as_ref())),
            ],
        )
    }
}
