//! Geometry for the hosts (#328, #367, ADR 0021), at three levels.
//!
//! - **Placements** (feature `placements`, default): per product, the world
//!   placement as a 4x4 column-major matrix in metres and the Body
//!   representation a viewer draws, with its identifier, type and context.
//!   Kernel-free: the facade's `product_placements`.
//! - **Graphs** (feature `graph`, #367): per product, the Body lowered
//!   into Axiolid's neutral geometry graph, exact, serialised in Axiolid's
//!   versioned wire format (1.0 to 1.3) as JSON or CBOR ([`graph`]).
//!   Computes nothing; the facade's `product_graphs`.
//! - **Meshes** (feature `mesh`, opt-in): per product, triangles compiled by
//!   the reference backend, `f32` positions relative to the product's world
//!   placement and `u32` indices. The facade's `product_meshes`; it links an
//!   execution provider, which ADR 0004 keeps out of every default build.
//!   Each mesh carries its surface colour (#449): RGB and transparency, and
//!   whether the representation item, its presentation layer or its
//!   material supplied it; `null` when nothing colours the product, with a
//!   typed `colour_refusal` when its colour cannot be resolved.
//!
//! A product that cannot be placed, selected or meshed is a record with a
//! typed [`GeometryRefusal`], never an error that aborts the call: one
//! broken wall does not hide the building. The refusal codes are the
//! shared binding codes `unsupported`, `invalid-model`, `missing-reference`
//! and `budget-exceeded`. Without its feature an operation refuses with
//! `feature-disabled`.

use crate::record::{Field, Record, ToRecord};
use crate::{BindingError, IfcModel};

pub mod graph;

/// Why one product has no placement, representation, graph or mesh.
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

/// The surface colour a mesh is painted with (#449).
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceColour {
    /// Red, in `[0, 1]`.
    pub red: f64,
    /// Green, in `[0, 1]`.
    pub green: f64,
    /// Blue, in `[0, 1]`.
    pub blue: f64,
    /// `Transparency` in `[0, 1]`, 0 opaque; `None` when the file states
    /// none, which the standard reads as opaque.
    pub transparency: Option<f64>,
    /// Where the style was assigned: `item` (an `IfcStyledItem` on the
    /// representation item), `layer` (an `IfcPresentationLayerWithStyle`)
    /// or `material` (the `IfcMaterial`'s
    /// `IfcMaterialDefinitionRepresentation`), in that precedence.
    pub source: String,
    /// The `IfcStyledItem`, the layer assignment or the `IfcMaterial`.
    pub source_entity: u64,
    /// The `IfcSurfaceStyle` the colour is read from.
    pub surface_style: u64,
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
    /// The surface colour the Body's items agree on; `None` when nothing
    /// colours the product or its colour is refused.
    pub colour: Option<SurfaceColour>,
    /// Why the colour could not be resolved: `ambiguous` (two styled items
    /// or two colours at one level, a layer or profile set of several
    /// materials, a constituent set no shape aspect resolves, or Body items
    /// of different colours), or the shared codes `invalid-model`,
    /// `missing-reference`, `unsupported` and `budget-exceeded`. Never
    /// refuses the mesh itself.
    pub colour_refusal: Option<GeometryRefusal>,
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
            colour: None,
            colour_refusal: None,
        };
        match mesh {
            Ok(mesh) => {
                record.transform = Some(ifc::column_major(&mesh.world));
                record.positions = mesh.positions;
                record.indices = mesh.indices;
                match mesh.colour {
                    Ok(colour) => record.colour = colour.as_ref().map(surface_colour),
                    Err(error) => record.colour_refusal = Some(colour_refusal(&error)),
                }
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

/// The host record of a resolved colour.
#[cfg(feature = "mesh")]
fn surface_colour(colour: &ifc::SurfaceColour) -> SurfaceColour {
    use ifc::ColourSource as S;
    let (source, entity) = match colour.source {
        S::StyledItem(id) => ("item", id),
        S::PresentationLayer(id) => ("layer", id),
        S::Material { material, .. } => ("material", material),
        // A source added to the facade after this binding: still typed.
        _ => ("other", colour.surface_style),
    };
    let [red, green, blue] = colour.rgb;
    SurfaceColour {
        red,
        green,
        blue,
        transparency: colour.transparency,
        source: source.to_owned(),
        source_entity: entity.0,
        surface_style: colour.surface_style.0,
    }
}

/// The typed refusal for a colour that cannot be resolved.
#[cfg(feature = "mesh")]
fn colour_refusal(error: &ifc::ColourError) -> GeometryRefusal {
    use ifc::material::MaterialError as M;
    use ifc::style::StyleError as S;
    use ifc::ColourError as E;
    let (code, entity) = match error {
        E::Geometry(error) => return refusal(error),
        E::AmbiguousMaterial { item, .. } => ("ambiguous", Some(*item)),
        E::ItemsDiffer { product, .. } => ("ambiguous", Some(*product)),
        E::UnsupportedSchema { .. } => ("unsupported", None),
        E::Style(S::AmbiguousStyleAssignment { item, .. }) => ("ambiguous", Some(*item)),
        E::Style(S::AmbiguousSurfaceColour { owner, .. }) => ("ambiguous", Some(*owner)),
        E::Style(S::UnknownEntity { id }) => ("missing-reference", Some(*id)),
        E::Style(S::DanglingReference { source_id, .. }) => ("missing-reference", Some(*source_id)),
        E::Style(S::UnsupportedSchema { .. }) => ("unsupported", None),
        E::Material(M::AmbiguousAssignment { object, .. } | M::AmbiguousType { object, .. }) => {
            ("ambiguous", Some(*object))
        }
        E::Material(M::UnsupportedSchema { .. }) => ("unsupported", None),
        E::Material(M::UnknownEntity { id }) => ("missing-reference", Some(*id)),
        _ => ("invalid-model", None),
    };
    GeometryRefusal {
        code: code.to_owned(),
        entity: entity.map(|id| id.0),
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
                ("colour", Field::record(self.colour.as_ref())),
                (
                    "colour_refusal",
                    Field::record(self.colour_refusal.as_ref()),
                ),
            ],
        )
    }
}

impl ToRecord for SurfaceColour {
    fn to_record(&self) -> Record {
        Record::new(
            "SurfaceColour",
            vec![
                ("red", Field::Real(self.red)),
                ("green", Field::Real(self.green)),
                ("blue", Field::Real(self.blue)),
                (
                    "transparency",
                    Field::optional(self.transparency.as_ref(), |t| Field::Real(*t)),
                ),
                ("source", Field::Text(self.source.clone())),
                ("source_entity", Field::Id(self.source_entity)),
                ("surface_style", Field::Id(self.surface_style)),
            ],
        )
    }
}
