//! The directrix of a sweep as a polyline (#661).
//!
//! A swept disk (a pipe, a reinforcing bar) or a directrix sweep is described
//! by [`crate::body_description`] with its directrix by reference and its
//! `StartParam`/`EndParam` as authored. This is the sibling for a caller that
//! wants the path as points and says how closely: the directrix is lowered
//! exactly as the sweep's own lowering lowers it, trimmed to the sweep's
//! range, and handed to a [`PolylineCompiler`], which chord-samples it within
//! the caller's [`Tolerance`] and certifies the bound. No code here samples a
//! curve (ADR 0004).
//!
//! # Frames
//!
//! The directrix of a swept disk lies in the sweep's item frame. The
//! directrix of a swept-area sweep lies in the sweep's `Position` frame, which
//! is placed into the item frame. `frame` places the points from the item
//! frame: pass [`crate::BodyItem::item_world`] for world coordinates and
//! [`Transform::identity`] for the item frame. The range is applied to the
//! curve before it is placed (the `Position` and `frame` as one placement),
//! because the kernel grows its bound by a placement's stretch and refuses a
//! trim of a placed curve.
//!
//! # A trimmed or relational directrix with a range
//!
//! A range on a directrix that is itself a curve relation (an
//! `IfcTrimmedCurve`, an `IfcIndexedPolyCurve` with `Segments`, an offset
//! curve) is trimmed again, which the kernel refuses; it surfaces as
//! `CompilationRefused` naming the sweep. It is not worked around here.
//!
//! # The path is the authored directrix
//!
//! An `IfcSweptDiskSolidPolygonal`'s `FilletRadius` is not applied: the
//! polyline is the authored directrix, with sharp corners.

use axiolid_contracts::ExecutionOptions;
use axiolid_model::{
    CurveRelation, GeometryNode, Instance, TrimSelector, TrimmingPreference as KernelPreference,
};
use ifc_model::{Entity, EntityId, Model};

use super::{refused, CompiledPolyline, PolylineCompiler, Tolerance};
use crate::error::GeometryResult;
use crate::lower::curve::lower_sweep_directrix;
use crate::lower::swept::directrix_parameter_trims;
use crate::lower::LoweringSession;
use crate::resource::placement::axis_placement_transform;
use crate::slots::Slots;
use crate::solid::swept::swept_area_slot;
use crate::solid::swept::{
    FixedReferenceSweptAreaSolid, SurfaceCurveSweptAreaSolid, SweptDiskSolid,
};
use crate::transform::Transform;
use crate::units::{self, UnitScale};

#[cfg(feature = "compile-reference-backend")]
use super::default_backend;

/// `sweep`'s directrix as a polyline in `frame`, sampled by `backend` within
/// `tolerance`.
///
/// Accepts `IfcSweptDiskSolid`, `IfcSweptDiskSolidPolygonal`,
/// `IfcFixedReferenceSweptAreaSolid`,
/// `IfcDirectrixDerivedReferenceSweptAreaSolid` and
/// `IfcSurfaceCurveSweptAreaSolid`. `StartParam`/`EndParam` are applied in
/// the directrix's own parameterisation, as the sweep's lowering applies them;
/// an `IfcCompositeCurve` directrix is cut structurally. The result's
/// `curve` is the directrix and its `deviation` the backend's certified bound.
/// The `FilletRadius` of an `IfcSweptDiskSolidPolygonal` is not applied: the
/// polyline is the authored directrix, with sharp corners.
///
/// The directrix of a swept-area sweep is placed by the sweep's `Position`
/// (identity when absent) and then by `frame`; a swept disk has no `Position`.
/// A ranged directrix that is a curve relation is refused as
/// `CompilationRefused` (kernel limit, #693).
///
/// Refused, each naming the sweep:
///
/// - any other entity, as [`GeometryError::Unsupported`](crate::GeometryError);
/// - an IFC4X3 `IfcLengthMeasure` `StartParam`/`EndParam`, which is a distance
///   along the directrix, as `Unsupported`;
/// - one of `StartParam`/`EndParam` without the other, as `Degenerate`;
/// - a dangling directrix or `Position`, as `MissingEntity`;
/// - a directrix the backend refuses, as `CompilationRefused`.
///
/// # Errors
///
/// As above, plus whatever lowering the directrix and unit resolution refuse.
pub fn compile_sweep_directrix_with<B: PolylineCompiler>(
    backend: &B,
    model: &Model,
    sweep: EntityId,
    frame: Transform,
    tolerance: Tolerance,
) -> GeometryResult<CompiledPolyline> {
    let scale = units::resolve(model)?;
    let mut session = LoweringSession::new(model, &scale);
    let entity = session.entity(sweep, sweep)?;
    let type_name = entity.type_name.to_ascii_uppercase();
    let (directrix, start, end, position) = match type_name.as_str() {
        "IFCSWEPTDISKSOLID" | "IFCSWEPTDISKSOLIDPOLYGONAL" => {
            let view = SweptDiskSolid::new(sweep, entity);
            (
                view.directrix()?,
                view.start_param(),
                view.end_param(),
                Transform::identity(),
            )
        }
        "IFCFIXEDREFERENCESWEPTAREASOLID" | "IFCDIRECTRIXDERIVEDREFERENCESWEPTAREASOLID" => {
            let view = FixedReferenceSweptAreaSolid::new(sweep, entity);
            let (start, end) = directrix_parameter_trims(
                &session,
                sweep,
                &type_name,
                view.start_param()?,
                view.end_param()?,
            )?;
            (
                view.directrix()?,
                start,
                end,
                sweep_position(&session, sweep, entity, &scale)?,
            )
        }
        "IFCSURFACECURVESWEPTAREASOLID" => {
            let view = SurfaceCurveSweptAreaSolid::new(sweep, entity);
            let (start, end) = directrix_parameter_trims(
                &session,
                sweep,
                &type_name,
                view.start_param()?,
                view.end_param()?,
            )?;
            (
                view.directrix()?,
                start,
                end,
                sweep_position(&session, sweep, entity, &scale)?,
            )
        }
        _ => {
            return Err(session.unsupported(sweep, &type_name, "not a sweep along a directrix"));
        }
    };
    Slots::new(sweep, entity).resolve(model, directrix)?;

    let (curve, range) = lower_sweep_directrix(
        &mut session,
        sweep,
        &type_name,
        directrix,
        Transform::identity(),
        start,
        end,
    )?;
    // Trim first, then place: a placed curve cannot be trimmed.
    let mut node = curve;
    if let Some((start, end)) = range {
        // TODO(#693): a directrix that is itself a curve relation is refused
        // by the kernel here (blocked by axiolid/kernel#424).
        node = session.node_for(
            directrix,
            GeometryNode::CurveRelation(CurveRelation::Trimmed {
                basis: node,
                start: vec![TrimSelector::Parameter(start)],
                end: vec![TrimSelector::Parameter(end)],
                sense_agreement: true,
                preference: KernelPreference::Parameter,
            }),
        )?;
    }
    let frame = frame.compose(&position);
    if !frame.is_identity(0.0) {
        node = session.node_for(
            directrix,
            GeometryNode::Instance(Instance {
                source: node,
                transform: frame.to_geom(),
            }),
        )?;
    }
    let lowered = session.finish(node)?;
    let outcome = backend
        .compile_polyline(
            &lowered.graph,
            lowered.root,
            &ExecutionOptions::new(tolerance),
        )
        .map_err(|error| refused(sweep, error, &lowered.provenance))?;
    Ok(CompiledPolyline {
        curve: Some(directrix),
        closed: outcome.closed,
        deviation: outcome.deviation,
        points: outcome.points,
    })
}

/// The `Position` of an `IfcSweptAreaSolid`, in metres; the identity when
/// absent.
fn sweep_position(
    session: &LoweringSession<'_>,
    sweep: EntityId,
    entity: &Entity,
    scale: &UnitScale,
) -> GeometryResult<Transform> {
    let Some(position_id) = Slots::new(sweep, entity).opt_ref(swept_area_slot::POSITION) else {
        return Ok(Transform::identity());
    };
    let position = session.entity(sweep, position_id)?;
    Ok(axis_placement_transform(session.model(), position_id, position)?.to_metres(scale))
}

/// [`compile_sweep_directrix_with`] with [`default_backend`].
///
/// # Errors
///
/// As [`compile_sweep_directrix_with`].
#[cfg(feature = "compile-reference-backend")]
pub fn compile_sweep_directrix(
    model: &Model,
    sweep: EntityId,
    frame: Transform,
    tolerance: Tolerance,
) -> GeometryResult<CompiledPolyline> {
    compile_sweep_directrix_with(&default_backend(), model, sweep, frame, tolerance)
}
