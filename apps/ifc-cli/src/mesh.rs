//! `ifc mesh` — compile a file's geometry to triangle meshes.
//!
//! This is the reference consumer: the application chooses the codec and the
//! geometry providers, which library crates must never do.

use geom_boolmesh::BoolmeshBoolean;
use geom_compile::ScalarCompiler;
use geom_core::Tolerance as GeomTolerance;
use geom_kernel::{ExecutionOptions, GeometryCompiler, MeshBoolean};
use geom_mesh::TriMesh;
use ifc_geometry::constraint::local::PlacementResolver;
use ifc_geometry::lower::lower_representation;
use ifc_geometry::lower::{lower_representation_item, LoweringSession, Tolerance};
use ifc_geometry::Slots;
use ifc_geometry::{units, Transform};
use ifc_model::{EntityId, Model};
use std::collections::BTreeMap;

/// Representation families worth attempting; anything else is not geometry.
const ITEM_TYPES: &[&str] = &[
    "IFCEXTRUDEDAREASOLID",
    "IFCBOOLEANCLIPPINGRESULT",
    "IFCBOOLEANRESULT",
    "IFCMAPPEDITEM",
    "IFCFACETEDBREP",
    "IFCREVOLVEDAREASOLID",
    "IFCSURFACECURVESWEPTAREASOLID",
    "IFCFIXEDREFERENCESWEPTAREASOLID",
    "IFCSWEPTDISKSOLID",
    "IFCTRIANGULATEDFACESET",
    "IFCPOLYGONALFACESET",
];

/// Outcome for one representation item.
pub enum Outcome {
    /// Compiled to a mesh.
    Meshed(TriMesh),
    /// The IFC layer could not lower it. Carries the reason.
    NotLowered(String),
    /// Lowering succeeded but no provider implements the family.
    NotCompiled(String),
}

/// Per-file totals.
#[derive(Default)]
pub struct Summary {
    /// Items compiled to meshes.
    pub meshed: usize,
    /// Items the IFC layer could not lower.
    pub not_lowered: usize,
    /// Items lowered but not compilable by the wired providers.
    pub not_compiled: usize,
    /// Triangles across all produced meshes.
    pub triangles: usize,
    /// The produced meshes, in discovery order.
    ///
    /// Retained so a caller can validate geometry rather than only counts;
    /// the corpus gate checks volumes against an independent oracle.
    pub meshes: Vec<TriMesh>,
}

/// Compile every geometry item in `model`.
///
/// Failures are collected rather than fatal: a partial model is the normal
/// case while families remain unimplemented, and a caller needs the totals to
/// know what was actually produced.
pub fn compile_model(model: &Model, verbose: bool) -> Summary {
    let scale = units::resolve(model);
    let tolerance = Tolerance::building_scale();
    let compiler = ScalarCompiler::new(BoolmeshBoolean::new());
    let options = ExecutionOptions::new(GeomTolerance::MILLIMETRE);
    let mut summary = Summary::default();

    for type_name in ITEM_TYPES {
        for &id in model.ids_of_type(type_name) {
            match compile_item(model, &scale, tolerance, &compiler, &options, id) {
                Outcome::Meshed(mesh) => {
                    summary.meshed += 1;
                    summary.triangles += mesh.triangle_count();
                    summary.meshes.push(mesh);
                }
                Outcome::NotLowered(reason) => {
                    summary.not_lowered += 1;
                    if verbose {
                        println!("  #{id} {type_name}: not lowered: {reason}");
                    }
                }
                Outcome::NotCompiled(reason) => {
                    summary.not_compiled += 1;
                    if verbose {
                        println!("  #{id} {type_name}: not compiled: {reason}");
                    }
                }
            }
        }
    }
    summary
}

/// Lower and compile one item.
///
/// Each item gets a fresh session so one malformed record cannot poison the
/// memoisation caches of unrelated items.
fn compile_item(
    model: &Model,
    scale: &units::UnitScale,
    tolerance: Tolerance,
    compiler: &ScalarCompiler<BoolmeshBoolean>,
    options: &ExecutionOptions,
    id: EntityId,
) -> Outcome {
    let mut session = LoweringSession::new(model, scale, tolerance);
    let root = match lower_representation_item(&mut session, id, Transform::identity()) {
        Ok(root) => root,
        Err(error) => return Outcome::NotLowered(error.to_string()),
    };
    let lowered = match session.finish(root) {
        Ok(lowered) => lowered,
        Err(error) => return Outcome::NotLowered(error.to_string()),
    };
    match compiler.compile(&lowered.graph, lowered.root, options) {
        Ok(mesh) => Outcome::Meshed(mesh),
        Err(error) => Outcome::NotCompiled(error.to_string()),
    }
}

/// One product's net solid, after its openings have been cut.
pub struct Product {
    /// Entity id of the product.
    pub id: EntityId,
    /// IFC type name.
    pub type_name: String,
    /// Net solid.
    pub mesh: TriMesh,
    /// Openings subtracted.
    pub voids_applied: usize,
}

/// Compile products, applying `IfcRelVoidsElement` openings.
///
/// This is product-level assembly, which deliberately does not live in
/// `ifc-geometry`: that crate lowers representation *items*, and a void is a
/// relationship between two products. Doing it here keeps the geometry layer
/// free of relationship semantics.
pub fn compile_products(model: &Model) -> Vec<Product> {
    let scale = units::resolve(model);
    let tolerance = Tolerance::building_scale();
    let compiler = ScalarCompiler::new(BoolmeshBoolean::new());
    let options = ExecutionOptions::new(GeomTolerance::MILLIMETRE);

    // Relating element -> its opening elements.
    let mut voids: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
    for &rel in model.ids_of_type("IFCRELVOIDSELEMENT") {
        let Some(entity) = model.get(rel) else {
            continue;
        };
        let slots = Slots::new(rel, entity);
        // IfcRelVoidsElement: ..., RelatingBuildingElement(4), RelatedOpeningElement(5)
        let (Some(building), Some(opening)) = (slots.opt_ref(4), slots.opt_ref(5)) else {
            continue;
        };
        voids.entry(building).or_default().push(opening);
    }

    // Every entity that OWNS an IfcProductDefinitionShape, i.e. every product
    // with geometry. Selecting by "has a representation" rather than by type
    // name keeps this schema-independent: `IFCPRODUCT` is abstract and never
    // appears as a concrete type in the model index.
    let mut subjects: Vec<EntityId> = Vec::new();
    for &shape in model.ids_of_type("IFCPRODUCTDEFINITIONSHAPE") {
        for (id, entity) in model.iter() {
            if entity
                .attributes
                .iter()
                .any(|v| matches!(v, ifc_model::Value::Ref(r) if *r == shape))
                && !subjects.contains(&id)
            {
                subjects.push(id);
            }
        }
    }
    subjects.sort_unstable();
    let no_openings: Vec<EntityId> = Vec::new();
    let mut out = Vec::new();
    for building in subjects {
        let openings = voids.get(&building).unwrap_or(&no_openings);
        let Some(subject) = product_mesh(model, &scale, tolerance, &compiler, &options, building)
        else {
            continue;
        };
        let tools: Vec<TriMesh> = openings
            .iter()
            .filter_map(|&o| product_mesh(model, &scale, tolerance, &compiler, &options, o))
            .collect();
        let applied = tools.len();
        let mesh = if tools.is_empty() {
            subject
        } else {
            match compiler
                .boolean_provider()
                .subtract_many(&subject, &tools, &options)
            {
                Ok(mesh) => mesh,
                // A failed cut must not silently yield the uncut solid: report
                // zero voids applied so the caller can tell the difference.
                Err(_) => {
                    out.push(Product {
                        id: building,
                        type_name: type_name_of(model, building),
                        mesh: subject,
                        voids_applied: 0,
                    });
                    continue;
                }
            }
        };
        out.push(Product {
            id: building,
            type_name: type_name_of(model, building),
            mesh,
            voids_applied: applied,
        });
    }
    out
}

/// IFC type name, or a placeholder when the record is missing.
fn type_name_of(model: &Model, id: EntityId) -> String {
    model
        .get(id)
        .map_or_else(|| "(unknown)".to_owned(), |e| e.type_name.to_string())
}

/// Compile a product's own representation, ignoring its voids.
///
/// Products carry `ObjectPlacement` and `Representation` in the last two
/// slots of `IfcProduct`, which every element subtype inherits.
fn product_mesh(
    model: &Model,
    scale: &units::UnitScale,
    tolerance: Tolerance,
    compiler: &ScalarCompiler<BoolmeshBoolean>,
    options: &ExecutionOptions,
    id: EntityId,
) -> Option<TriMesh> {
    let entity = model.get(id)?;
    let slots = Slots::new(id, entity);
    // Slot 5 is `ObjectPlacement`: without it every product lands at the
    // origin, which looks plausible for a single solid and silently ruins
    // every boolean between two differently placed products.
    //
    // `world_transform` returns the chain in FILE units, while lowering has
    // already converted the representation to metres. Applying the raw
    // transform would scale the geometry a second time, so convert the
    // placement to metres before composing the two.
    let placement = slots
        .opt_ref(5)
        .and_then(|p| PlacementResolver::new().world_transform(model, p).ok())
        .map_or_else(Transform::identity, |t| t.to_metres(scale));
    let shape = slots.opt_ref(6)?;
    let shape_entity = model.get(shape)?;
    let representations = Slots::new(shape, shape_entity).opt_ref_list(2);

    let mut combined: Option<TriMesh> = None;
    for representation in representations {
        let mut session = LoweringSession::new(model, scale, tolerance);
        let Ok(root) = lower_representation(&mut session, representation) else {
            continue;
        };
        let Ok(lowered) = session.finish(root) else {
            continue;
        };
        let Ok(mut mesh) = compiler.compile(&lowered.graph, lowered.root, options) else {
            continue;
        };
        // Lowering produces geometry in the product's local frame; the
        // placement chain puts it in the world. Applying it here keeps the
        // geometry layer placement-agnostic.
        for position in &mut mesh.positions {
            let p = placement.apply([position.x, position.y, position.z]);
            *position = geom_core::Point3::new(p[0], p[1], p[2]);
        }
        match &mut combined {
            Some(existing) => merge(existing, &mesh),
            None => combined = Some(mesh),
        }
    }
    combined
}

/// Concatenate `source` into `target`, rebasing indices.
fn merge(target: &mut TriMesh, source: &TriMesh) {
    let offset = target.positions.len() as u32;
    target.positions.extend_from_slice(&source.positions);
    target
        .indices
        .extend(source.indices.iter().map(|&i| i + offset));
}

/// Signed volume by the divergence theorem, centred on the mesh centroid.
///
/// Centring is not cosmetic. The formula sums `a . (b x c)` over triangles,
/// whose terms scale with the CUBE of the distance to the origin while the
/// true volume does not. On survey coordinates (~1.5e6) the terms reach 1e19
/// and a cubic-metre result is reconstructed from differences sixteen digits
/// down -- the naive sum returns ~8% low. Translating to the centroid first
/// makes the terms proportional to the object, not to its map position.
pub fn signed_volume(mesh: &TriMesh) -> f64 {
    if mesh.positions.is_empty() {
        return 0.0;
    }
    let centre = mesh
        .positions
        .iter()
        .fold(geom_core::Point3::ZERO, |acc, p| acc + *p)
        / mesh.positions.len() as f64;
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let a = mesh.positions[t[0] as usize] - centre;
            let b = mesh.positions[t[1] as usize] - centre;
            let c = mesh.positions[t[2] as usize] - centre;
            a.dot(b.cross(c))
        })
        .sum::<f64>()
        / 6.0
}

/// Every directed edge exactly once, and paired with its opposite.
///
// Used by the `differential` subcommand. This module is also `#[path]`-included
// by the corpus test, which does not call it, so dead-code analysis fires there.
#[allow(dead_code)]
pub fn edge_manifold(mesh: &TriMesh) -> bool {
    use std::collections::HashSet;
    let mut seen: HashSet<(u32, u32)> = HashSet::new();
    for t in mesh.indices.chunks_exact(3) {
        for e in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            if !seen.insert(e) {
                return false;
            }
        }
    }
    seen.iter().all(|&(a, b)| seen.contains(&(b, a)))
}
