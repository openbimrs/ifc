//! The comparison harness: backends as values, one traversal for all.
//!
//! A [`Contestant`] is one backend bound to one contract area. The run loop
//! in `main` sees only `&[Box<dyn Contestant>]`, so which kernels take part
//! is decided where the list is built, never inside the traversal.
//!
//! Only the mesh area exists today. The shape is meant to grow: a
//! `MeshBoolean`, `MeshPlaneSection`, `Tessellator`, `CurveEvaluator` or
//! `ExactCompiler` contestant adds an [`Area`], an [`Artifact`] variant for
//! what it returns and a [`Measure`] variant for what agreement compares,
//! and the traversal, timing and report stay as they are.

use std::time::{Duration, Instant};

use axiolid_contracts::BackendId;
use axiolid_core::{Aabb, Point3, Tolerance};
use axiolid_mesh::TriMesh;
use axiolid_mesh_compile_contract::MeshCompiler;
use ifc_geometry::compile::compile_product_mesh_with;
use ifc_geometry::GeometryError;
use ifc_model::{EntityId, Model};

/// The contract a contestant is measured on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    /// `MeshCompiler`: a product's Body, compiled to triangles.
    Mesh,
}

impl Area {
    pub fn name(self) -> &'static str {
        match self {
            Self::Mesh => "MeshCompiler",
        }
    }
}

/// What one contestant returned for one product: the timed result.
pub enum Artifact {
    Mesh(TriMesh),
}

impl Artifact {
    /// Reduce to what agreement compares. Never inside the clock.
    pub fn measure(&self) -> Measure {
        match self {
            Self::Mesh(mesh) => Measure::Mesh(MeshMeasure::of(mesh)),
        }
    }
}

/// One product's result, after the clock stopped.
pub enum Outcome {
    /// The backend produced something; see [`Measure`].
    Produced(Measure),
    /// The product has no Body representation: not an error.
    NoBody,
    /// The backend, or lowering before it, refused. Data, not a crash.
    Refused(String),
}

/// One backend under one contract.
pub trait Contestant {
    fn backend(&self) -> BackendId;
    fn area(&self) -> Area;
    /// Run the contract for one product. This is what the clock measures.
    fn execute(&self, model: &Model, product: EntityId) -> Result<Option<Artifact>, String>;
}

/// Any [`MeshCompiler`], compiled through the crate's public seam.
pub struct MeshContestant<B> {
    backend: B,
    tolerance: Tolerance,
}

impl<B: MeshCompiler> MeshContestant<B> {
    pub fn new(backend: B, tolerance: Tolerance) -> Self {
        Self { backend, tolerance }
    }
}

impl<B: MeshCompiler> Contestant for MeshContestant<B> {
    fn backend(&self) -> BackendId {
        self.backend.descriptor().id
    }

    fn area(&self) -> Area {
        Area::Mesh
    }

    fn execute(&self, model: &Model, product: EntityId) -> Result<Option<Artifact>, String> {
        compile_product_mesh_with(&self.backend, model, product, self.tolerance)
            .map(|mesh| mesh.map(Artifact::Mesh))
            .map_err(|error| match error {
                // The kernel's own words; the product is already in the row.
                GeometryError::CompilationRefused { reason, .. } => reason,
                other => other.to_string(),
            })
    }
}

/// The comparable facts of a triangle mesh.
#[derive(Debug, Clone, Copy)]
pub struct MeshMeasure {
    pub vertices: usize,
    pub triangles: usize,
    /// `None` for a mesh with no vertices.
    pub bounds: Option<Aabb>,
    /// Sum of triangle areas, m².
    pub area: f64,
    /// Signed divergence volume, m³: positive when the triangles face
    /// outward. For a closed mesh it is the enclosed volume; for an open
    /// surface it is still a deterministic function of the triangles, so
    /// two backends that agree on the surface agree on it too.
    pub volume: f64,
}

impl MeshMeasure {
    fn of(mesh: &TriMesh) -> Self {
        let bounds = (!mesh.positions.is_empty()).then(|| mesh.bounds());
        // About the box centre: survey coordinates far from the origin
        // would otherwise cancel catastrophically in the volume sum.
        let centre = bounds.map_or(Point3::ZERO, |b| (b.min + b.max) / 2.0);
        let (mut area, mut volume) = (0.0, 0.0);
        for [a, b, c] in mesh.triangles() {
            let p = |i: u32| mesh.positions[i as usize] - centre;
            let (a, b, c) = (p(a), p(b), p(c));
            area += (b - a).cross(c - a).length() / 2.0;
            volume += a.dot(b.cross(c)) / 6.0;
        }
        Self {
            vertices: mesh.positions.len(),
            triangles: mesh.triangle_count(),
            bounds,
            area,
            volume,
        }
    }
}

/// What agreement compares, per area.
#[derive(Debug, Clone, Copy)]
pub enum Measure {
    Mesh(MeshMeasure),
}

/// The allowed difference between two backends' numbers.
///
/// `|a - b| <= relative * max(|a|, |b|, 1)`: relative to the magnitude,
/// with a floor of one unit (1 m, 1 m², 1 m³) so values near zero are not
/// held to an impossible relative bound.
#[derive(Debug, Clone, Copy)]
pub struct Agreement {
    pub relative: f64,
}

impl Agreement {
    fn allowed(self, a: f64, b: f64) -> f64 {
        self.relative * a.abs().max(b.abs()).max(1.0)
    }
}

/// One metric on which two backends disagree, beyond the tolerance.
pub struct Divergence {
    pub metric: String,
    pub left: String,
    pub right: String,
    pub difference: f64,
    pub allowed: f64,
}

/// Compare two measures of one product, listing every metric that differs.
pub fn compare(left: &Measure, right: &Measure, agreement: Agreement) -> Vec<Divergence> {
    let (Measure::Mesh(left), Measure::Mesh(right)) = (left, right);
    let mut out = Vec::new();
    let mut scalar = |metric: &str, a: f64, b: f64| {
        let (difference, allowed) = ((a - b).abs(), agreement.allowed(a, b));
        // A NaN on either side is a divergence, not a pass.
        if difference.is_nan() || difference > allowed {
            out.push(Divergence {
                metric: metric.to_owned(),
                left: format!("{a:.9}"),
                right: format!("{b:.9}"),
                difference,
                allowed,
            });
        }
    };
    scalar("volume (m³)", left.volume, right.volume);
    scalar("area (m²)", left.area, right.area);
    match (left.bounds, right.bounds) {
        (Some(l), Some(r)) => {
            for (axis, i) in [("x", 0), ("y", 1), ("z", 2)] {
                scalar(&format!("bbox.min.{axis} (m)"), l.min[i], r.min[i]);
                scalar(&format!("bbox.max.{axis} (m)"), l.max[i], r.max[i]);
            }
        }
        (None, None) => {}
        (l, r) => out.push(Divergence {
            metric: "bbox".into(),
            left: format!("{l:?}"),
            right: format!("{r:?}"),
            difference: f64::INFINITY,
            allowed: 0.0,
        }),
    }
    out
}

/// One contestant over one fixture.
pub struct Run {
    /// The first pass, timed on its own: cold caches, first allocations.
    pub first: Duration,
    /// The following passes, each one full pass over the fixture's products.
    pub timed: Vec<Duration>,
    /// One outcome per product, in product order, from the first pass.
    pub outcomes: Vec<Outcome>,
}

impl Run {
    pub fn median(&self) -> Option<Duration> {
        let mut sorted = self.timed.clone();
        sorted.sort();
        let n = sorted.len();
        match n {
            0 => None,
            _ if n % 2 == 1 => Some(sorted[n / 2]),
            _ => Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2),
        }
    }

    pub fn range(&self) -> Option<(Duration, Duration)> {
        Some((*self.timed.iter().min()?, *self.timed.iter().max()?))
    }
}

/// Time `contestant` over every product of one model.
///
/// One pass compiles every product once, refusals included: a kernel that
/// refuses fast is measured refusing. The pass's artifacts are held until
/// the clock stops, so their drop is never timed, then measured once from
/// the first pass. Parsing and product discovery happen before this and are
/// never timed; lowering is inside `compile_product_mesh_with` and so is
/// timed, identically for every backend.
pub fn run(
    contestant: &dyn Contestant,
    model: &Model,
    products: &[EntityId],
    iterations: usize,
) -> Run {
    let pass = || {
        let start = Instant::now();
        let results: Vec<_> = products
            .iter()
            .map(|product| std::hint::black_box(contestant.execute(model, *product)))
            .collect();
        (start.elapsed(), results)
    };
    let (first, results) = pass();
    let outcomes = results
        .iter()
        .map(|result| match result {
            Ok(Some(artifact)) => Outcome::Produced(artifact.measure()),
            Ok(None) => Outcome::NoBody,
            Err(reason) => Outcome::Refused(reason.clone()),
        })
        .collect();
    drop(results);
    let timed = (0..iterations).map(|_| pass().0).collect();
    Run {
        first,
        timed,
        outcomes,
    }
}
