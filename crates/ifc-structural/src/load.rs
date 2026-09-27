//! Core static structural load values.

use ifc_model::EntityId;

use crate::error::{StructuralError, StructuralResult};
use crate::view::Record;

mod dynamic;
mod r#static;

pub use dynamic::LoadConfiguration;

/// Which `IfcStructuralLoadStatic` subtype a static load value carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadKind {
    /// `IfcStructuralLoadSingleForce`: force/moment at a point.
    SingleForce,
    /// `IfcStructuralLoadLinearForce`: force/moment per unit length along a curve.
    LinearForce,
    /// `IfcStructuralLoadPlanarForce`: force per unit area over a surface.
    PlanarForce,
    /// `IfcStructuralLoadTemperature`: constant plus through-thickness temperature deltas.
    Temperature,
}

/// Borrowed projection of a core `IfcStructuralLoadStatic` subtype (force, planar force, or temperature).
#[derive(Debug, Clone, Copy)]
pub struct StaticLoad<'m, 's> {
    record: Record<'m, 's>,
    kind: LoadKind,
}

impl<'m, 's> StaticLoad<'m, 's> {
    /// Classify `record`'s [`LoadKind`] from its declared IFC type.
    ///
    /// Fails with [`StructuralError::WrongType`] if the type is none of
    /// `IfcStructuralLoadSingleForce`, `IfcStructuralLoadLinearForce`,
    /// `IfcStructuralLoadPlanarForce`, or `IfcStructuralLoadTemperature`.
    pub(crate) fn from_record(record: Record<'m, 's>) -> StructuralResult<Self> {
        let kind = if record.entity.is_type("IfcStructuralLoadSingleForce") {
            LoadKind::SingleForce
        } else if record.entity.is_type("IfcStructuralLoadLinearForce") {
            LoadKind::LinearForce
        } else if record.entity.is_type("IfcStructuralLoadPlanarForce") {
            LoadKind::PlanarForce
        } else if record.entity.is_type("IfcStructuralLoadTemperature") {
            LoadKind::Temperature
        } else {
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
    /// six force/moment axes for a single force, six per-length axes for a linear
    /// force, three per-area axes for a planar force, or three temperature deltas
    /// (constant, through-Y, through-Z). Each component is legally absent
    /// individually; `IfcStructuralLoadTemperature` uses the underscored
    /// `DeltaT_Constant`/`DeltaT_Y`/`DeltaT_Z` names in schemas where the
    /// unscored `DeltaTConstant` attribute does not exist.
    pub fn components(&self) -> StructuralResult<Vec<Option<f64>>> {
        let names: &[&'static str] = match self.kind {
            LoadKind::SingleForce => &[
                "ForceX", "ForceY", "ForceZ", "MomentX", "MomentY", "MomentZ",
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
