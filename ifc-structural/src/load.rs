//! Core static structural load values.

use ifc_model::EntityId;

use crate::error::{StructuralError, StructuralResult};
use crate::view::Record;

mod dynamic;
mod r#static;

pub use dynamic::LoadConfiguration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadKind {
    SingleForce,
    LinearForce,
    PlanarForce,
    Temperature,
}

#[derive(Debug, Clone, Copy)]
pub struct StaticLoad<'m, 's> {
    record: Record<'m, 's>,
    kind: LoadKind,
}

impl<'m, 's> StaticLoad<'m, 's> {
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
    pub fn id(&self) -> EntityId {
        self.record.id
    }

    #[must_use]
    pub fn kind(&self) -> LoadKind {
        self.kind
    }

    pub fn name(&self) -> StructuralResult<Option<&'m str>> {
        self.record.optional_text("Name")
    }

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
