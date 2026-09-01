//! `ifc-structural` -- bounded structural-analysis semantics.
//!
//! This crate exposes schema-resolved borrowed projections for analysis models,
//! load/result groups, idealized members and connections, applied actions, core
//! static load values, and their relationship graph across IFC2X3, IFC4 and
//! IFC4X3. Selected analysis models and loads can be staged through
//! `ifc_model::Transaction`.
//!
//! It does not solve structures, generate FEM meshes, evaluate geometry, or
//! claim computed reaction/result authoring.

mod action;
mod authoring;
mod connection;
mod error;
mod load;
mod member;
mod model;
mod query;
mod reaction;
mod view;

mod condition;
mod result;

pub use action::{ActionKind, CoordinateSystem, StructuralAction};
pub use authoring::{stage_analysis_model, stage_load, AnalysisModelDraft, LoadDraft};
pub use condition::{
    AxisValues, BoundaryCondition, BoundaryConditionKind, ConnectionCondition,
    ConnectionConditionKind, FailureLimits, StiffnessValue,
};
pub use connection::{ConnectionKind, StructuralConnection};
pub use error::{StructuralError, StructuralResult};
pub use load::{LoadConfiguration, LoadKind, StaticLoad};
pub use member::{Member, MemberKind};
pub use model::{AnalysisModel, AnalysisModelType, LoadGroup, ResultGroup};
pub use query::{ActivityAssignment, MemberConnection};
pub use result::{Reaction, ReactionKind};
pub use view::StructuralView;
