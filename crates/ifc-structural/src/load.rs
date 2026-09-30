//! Static structural load values and the reinforcement-area result.
//!
//! [`StaticLoad`] projects every `IfcStructuralLoadStatic` subtype declared
//! by IFC2X3, IFC4 and IFC4X3, classified with `Schema::is_a` from the most
//! specific subtype up, so a subtype is never read as its supertype's
//! narrower attribute list. `IfcSurfaceReinforcementArea` is a sibling of
//! `IfcStructuralLoadStatic` under `IfcStructuralLoadOrResult` (IFC4 and
//! later), not a static load, so it has its own projection,
//! [`SurfaceReinforcementArea`].

use ifc_model::EntityId;

use crate::error::{StructuralError, StructuralResult};
use crate::view::Record;

mod dynamic;
mod reinforcement;

pub use dynamic::LoadConfiguration;
pub use reinforcement::SurfaceReinforcementArea;

/// Which `IfcStructuralLoadStatic` subtype a static load value carries.
///
/// Each variant names the most specific subtype this crate reads; a record
/// of an undeclared further subtype is classified as its nearest listed
/// ancestor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LoadKind {
    /// `IfcStructuralLoadSingleForce`: force/moment at a point.
    SingleForce,
    /// `IfcStructuralLoadSingleForceWarping`: a single force plus the
    /// `WarpingMoment` (bimoment) of restrained warping.
    SingleForceWarping,
    /// `IfcStructuralLoadSingleDisplacement`: prescribed translations and
    /// rotations at a point.
    SingleDisplacement,
    /// `IfcStructuralLoadSingleDisplacementDistortion`: a single
    /// displacement plus the cross-section `Distortion`.
    SingleDisplacementDistortion,
    /// `IfcStructuralLoadLinearForce`: force/moment per unit length along a curve.
    LinearForce,
    /// `IfcStructuralLoadPlanarForce`: force per unit area over a surface.
    PlanarForce,
    /// `IfcStructuralLoadTemperature`: constant plus through-thickness temperature deltas.
    Temperature,
}

/// Borrowed projection of an `IfcStructuralLoadStatic` subtype: single,
/// linear or planar force, single displacement, or temperature.
#[derive(Debug, Clone, Copy)]
pub struct StaticLoad<'m, 's> {
    record: Record<'m, 's>,
    kind: LoadKind,
}

impl<'m, 's> StaticLoad<'m, 's> {
    /// Classify `record`'s [`LoadKind`] from its declared IFC type.
    ///
    /// Classification uses `Schema::is_a`, most specific subtype first, so
    /// `IfcStructuralLoadSingleForceWarping` is never read as a plain
    /// single force. Fails with [`StructuralError::WrongType`] if the type
    /// is not an `IfcStructuralLoadStatic` subtype this crate reads.
    pub(crate) fn from_record(record: Record<'m, 's>) -> StructuralResult<Self> {
        // Subtypes before their supertypes: the first match wins.
        const ORDER: [(&str, LoadKind); 7] = [
            (
                "IfcStructuralLoadSingleForceWarping",
                LoadKind::SingleForceWarping,
            ),
            ("IfcStructuralLoadSingleForce", LoadKind::SingleForce),
            (
                "IfcStructuralLoadSingleDisplacementDistortion",
                LoadKind::SingleDisplacementDistortion,
            ),
            (
                "IfcStructuralLoadSingleDisplacement",
                LoadKind::SingleDisplacement,
            ),
            ("IfcStructuralLoadLinearForce", LoadKind::LinearForce),
            ("IfcStructuralLoadPlanarForce", LoadKind::PlanarForce),
            ("IfcStructuralLoadTemperature", LoadKind::Temperature),
        ];
        let Some(kind) = ORDER
            .iter()
            .find(|(ancestor, _)| record.schema.is_a(&record.entity.type_name, ancestor))
            .map(|(_, kind)| *kind)
        else {
            return Err(StructuralError::WrongType {
                id: record.id,
                expected: "core static structural load",
                actual: record.entity.type_name.to_string(),
            });
        };
        Ok(Self { record, kind })
    }

    #[must_use]
    /// The `IfcStructuralLoadStatic` entity id.
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    #[must_use]
    /// Which core static load subtype this value carries.
    pub fn kind(&self) -> LoadKind {
        self.kind
    }

    /// `Name`, inherited from `IfcStructuralLoad`. Legally absent.
    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

    /// The load's numeric components in schema-declared order for its [`LoadKind`]:
    /// six force/moment axes for a single force, followed by `WarpingMoment`
    /// for the warping form; three translations and three rotations
    /// (`DisplacementX..Z`, `RotationalDisplacementRX..RZ`) for a single
    /// displacement, followed by `Distortion` for the distortion form; six
    /// per-length axes for a linear force; three per-area axes for a planar
    /// force; or three temperature deltas (constant, through-Y, through-Z).
    /// Each component is legally absent
    /// individually; `IfcStructuralLoadTemperature` uses the underscored
    /// `DeltaT_Constant`/`DeltaT_Y`/`DeltaT_Z` names in schemas where the
    /// unscored `DeltaTConstant` attribute does not exist.
    pub fn components(&self) -> StructuralResult<Vec<Option<f64>>> {
        let names: &[&'static str] = match self.kind {
            LoadKind::SingleForce => &[
                "ForceX", "ForceY", "ForceZ", "MomentX", "MomentY", "MomentZ",
            ],
            LoadKind::SingleForceWarping => &[
                "ForceX",
                "ForceY",
                "ForceZ",
                "MomentX",
                "MomentY",
                "MomentZ",
                "WarpingMoment",
            ],
            LoadKind::SingleDisplacement => &[
                "DisplacementX",
                "DisplacementY",
                "DisplacementZ",
                "RotationalDisplacementRX",
                "RotationalDisplacementRY",
                "RotationalDisplacementRZ",
            ],
            LoadKind::SingleDisplacementDistortion => &[
                "DisplacementX",
                "DisplacementY",
                "DisplacementZ",
                "RotationalDisplacementRX",
                "RotationalDisplacementRY",
                "RotationalDisplacementRZ",
                "Distortion",
            ],
            LoadKind::LinearForce => &[
                "LinearForceX",
                "LinearForceY",
                "LinearForceZ",
                "LinearMomentX",
                "LinearMomentY",
                "LinearMomentZ",
            ],
            LoadKind::PlanarForce => &["PlanarForceX", "PlanarForceY", "PlanarForceZ"],
            LoadKind::Temperature => {
                if self.record.has_attribute("DeltaTConstant") {
                    &["DeltaTConstant", "DeltaTY", "DeltaTZ"]
                } else {
                    &["DeltaT_Constant", "DeltaT_Y", "DeltaT_Z"]
                }
            }
        };
        names
            .iter()
            .map(|name| self.record.optional_number(name))
            .collect()
    }
}
