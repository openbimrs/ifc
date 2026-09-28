//! Constructors and builder setters for the drafts of the parent module,
//! kept apart so the writer stays under the 800-line limit.

#[allow(clippy::wildcard_imports)]
use super::*;

impl AnalysisModelDraft {
    /// Starts a draft from its required fields; every other field is unset.
    #[must_use]
    pub fn new(global_id: impl Into<String>, predefined_type: AnalysisModelType) -> Self {
        Self {
            global_id: global_id.into(),
            owner_history: None,
            name: None,
            description: None,
            object_type: None,
            predefined_type,
            orientation_of_2d_plane: None,
            loaded_by: Vec::new(),
            result_groups: Vec::new(),
            shared_placement: None,
        }
    }

    /// Sets `owner_history`: `OwnerHistory`, validated against the
    /// model/transaction if present.
    #[must_use]
    pub fn owner_history(mut self, value: EntityId) -> Self {
        self.owner_history = Some(value);
        self
    }

    /// Sets `name`: `Name`.
    #[must_use]
    pub fn name(mut self, value: impl Into<String>) -> Self {
        self.name = Some(value.into());
        self
    }

    /// Sets `description`: `Description`.
    #[must_use]
    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }

    /// Sets `object_type`: `ObjectType`; required non-blank when
    /// `predefined_type` is `UserDefined`.
    #[must_use]
    pub fn object_type(mut self, value: impl Into<String>) -> Self {
        self.object_type = Some(value.into());
        self
    }

    /// Sets `orientation_of_2d_plane`: `OrientationOf2DPlane`, an
    /// `IfcAxis2Placement3D` reference.
    #[must_use]
    pub fn orientation_of_2d_plane(mut self, value: EntityId) -> Self {
        self.orientation_of_2d_plane = Some(value);
        self
    }

    /// Sets `loaded_by`: `LoadedBy`, `IfcStructuralLoadGroup` references;
    /// members must be unique.
    #[must_use]
    pub fn loaded_by(mut self, value: Vec<EntityId>) -> Self {
        self.loaded_by = value;
        self
    }

    /// Sets `result_groups`: `HasResults`, `IfcStructuralResultGroup`
    /// references; members must be unique.
    #[must_use]
    pub fn result_groups(mut self, value: Vec<EntityId>) -> Self {
        self.result_groups = value;
        self
    }

    /// Sets `shared_placement`: `SharedPlacement`; only staged when the target
    /// schema declares the attribute.
    #[must_use]
    pub fn shared_placement(mut self, value: EntityId) -> Self {
        self.shared_placement = Some(value);
        self
    }
}
