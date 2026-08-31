//! Structural analysis, load and result groups.

mod analysis;
mod load_group;
mod result_group;

pub use analysis::{AnalysisModel, AnalysisModelType};
pub use load_group::LoadGroup;
pub use result_group::ResultGroup;
