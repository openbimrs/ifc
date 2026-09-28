//! Authoring for constituent and profile compositions.
//!
//! Split from the parent module, which stages materials, layers, lists and
//! associations. Every entity staged here is IFC4 onwards: IFC2X3 declares
//! no constituent, profile, or profile-set usage, so each function refuses
//! an IFC2X3 model with [`crate::MaterialError::EntityNotInSchema`] before
//! staging anything.

use ifc_model::{EntityId, Model, Transaction, Value};

use super::{invalid, optional_text, reals, refs, require_exists, require_type};
use crate::release::Release;
use crate::MaterialResult;

/// Authored fields for `IfcMaterialConstituent`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ConstituentDraft<'a> {
    /// `IfcMaterialConstituent.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcMaterialConstituent.Description`, if given.
    pub description: Option<&'a str>,
    /// `IfcMaterialConstituent.Material`, an `IfcMaterial` reference.
    pub material: EntityId,
    /// `IfcMaterialConstituent.Fraction`, if given. A ratio in `0.0..=1.0`.
    pub fraction: Option<f64>,
    /// `IfcMaterialConstituent.Category`, if given.
    pub category: Option<&'a str>,
}

impl<'a> ConstituentDraft<'a> {
    /// Starts a draft for a constituent made of `material`.
    #[must_use]
    pub const fn new(material: EntityId) -> Self {
        Self {
            name: None,
            description: None,
            material,
            fraction: None,
            category: None,
        }
    }

    /// Sets `Name`.
    #[must_use]
    pub const fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `Description`.
    #[must_use]
    pub const fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `Fraction`, a ratio in `0.0..=1.0`.
    #[must_use]
    pub const fn fraction(mut self, value: f64) -> Self {
        self.fraction = Some(value);
        self
    }

    /// Sets `Category`.
    #[must_use]
    pub const fn category(mut self, value: &'a str) -> Self {
        self.category = Some(value);
        self
    }
}

/// Stage an `IfcMaterialConstituent`. IFC4 onwards.
///
/// `Fraction` is an `IfcNormalisedRatioMeasure`: values outside `0..=1` are
/// refused because a constituent cannot be a negative or >100% share of its
/// set, and a wrong fraction silently misstates a composition.
pub fn create_constituent(
    tx: &mut Transaction,
    model: &Model,
    draft: ConstituentDraft<'_>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALCONSTITUENT";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    if let Some(fraction) = draft.fraction {
        if !fraction.is_finite() || !(0.0..=1.0).contains(&fraction) {
            return Err(invalid(
                ENTITY,
                "Fraction",
                "expected a normalised ratio in 0..=1",
            ));
        }
    }
    require_type(tx, model, release, draft.material, &["IFCMATERIAL"])?;
    let record = release.record(
        ENTITY,
        vec![
            ("Name", optional_text(draft.name)),
            ("Description", optional_text(draft.description)),
            ("Material", Value::Ref(draft.material)),
            ("Fraction", draft.fraction.map_or(Value::Null, Value::Real)),
            ("Category", optional_text(draft.category)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Stage an `IfcMaterialConstituentSet`. IFC4 onwards. Must name at least
/// one constituent: an empty set describes no composition at all.
pub fn create_constituent_set(
    tx: &mut Transaction,
    model: &Model,
    constituents: &[EntityId],
    name: Option<&str>,
    description: Option<&str>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALCONSTITUENTSET";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    if constituents.is_empty() {
        return Err(invalid(
            ENTITY,
            "MaterialConstituents",
            "expected at least one constituent",
        ));
    }
    for &constituent in constituents {
        require_type(tx, model, release, constituent, &["IFCMATERIALCONSTITUENT"])?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("Name", optional_text(name)),
            ("Description", optional_text(description)),
            ("MaterialConstituents", refs(constituents)),
        ],
    )?;
    Ok(tx.create(record))
}

/// Authored fields for `IfcMaterialProfile`.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ProfileDraft<'a> {
    /// `IfcMaterialProfile.Name`, if given.
    pub name: Option<&'a str>,
    /// `IfcMaterialProfile.Description`, if given.
    pub description: Option<&'a str>,
    /// `IfcMaterialProfile.Material`, an `IfcMaterial` reference, if given.
    pub material: Option<EntityId>,
    /// `IfcMaterialProfile.Profile`, an `IfcProfileDef` reference.
    pub profile: EntityId,
    /// `IfcMaterialProfile.Priority`, if given. Must be in `0..=100`.
    pub priority: Option<i64>,
    /// `IfcMaterialProfile.Category`, if given.
    pub category: Option<&'a str>,
}

impl<'a> ProfileDraft<'a> {
    /// Starts a draft for a material profile of `profile`, an
    /// `IfcProfileDef` reference.
    #[must_use]
    pub const fn new(profile: EntityId) -> Self {
        Self {
            name: None,
            description: None,
            material: None,
            profile,
            priority: None,
            category: None,
        }
    }

    /// Sets `Name`.
    #[must_use]
    pub const fn name(mut self, value: &'a str) -> Self {
        self.name = Some(value);
        self
    }

    /// Sets `Description`.
    #[must_use]
    pub const fn description(mut self, value: &'a str) -> Self {
        self.description = Some(value);
        self
    }

    /// Sets `Material`, an `IfcMaterial` reference.
    #[must_use]
    pub const fn material(mut self, value: EntityId) -> Self {
        self.material = Some(value);
        self
    }

    /// Sets `Priority`, in `0..=100`.
    #[must_use]
    pub const fn priority(mut self, value: i64) -> Self {
        self.priority = Some(value);
        self
    }

    /// Sets `Category`.
    #[must_use]
    pub const fn category(mut self, value: &'a str) -> Self {
        self.category = Some(value);
        self
    }
}

/// The profile attributes [`create_profile`] and
/// [`create_profile_with_offsets`] share, after checking them.
fn profile_values(
    tx: &Transaction,
    model: &Model,
    release: Release<'_>,
    entity: &'static str,
    draft: &ProfileDraft<'_>,
) -> MaterialResult<Vec<(&'static str, Value)>> {
    release.require_entity(entity, None)?;
    if let Some(priority) = draft.priority.filter(|value| !(0..=100).contains(value)) {
        return Err(invalid(entity, "Priority", priority.to_string()));
    }
    if let Some(material) = draft.material {
        require_type(tx, model, release, material, &["IFCMATERIAL"])?;
    }
    require_exists(tx, model, draft.profile)?;
    Ok(vec![
        ("Name", optional_text(draft.name)),
        ("Description", optional_text(draft.description)),
        ("Material", draft.material.map_or(Value::Null, Value::Ref)),
        ("Profile", Value::Ref(draft.profile)),
        (
            "Priority",
            draft.priority.map_or(Value::Null, Value::Integer),
        ),
        ("Category", optional_text(draft.category)),
    ])
}

/// Stage an `IfcMaterialProfile`. IFC4 onwards.
///
/// The profile reference must exist; an arbitrary missing id here would
/// produce a material profile with no cross-section.
pub fn create_profile(
    tx: &mut Transaction,
    model: &Model,
    draft: ProfileDraft<'_>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALPROFILE";
    let release = Release::of(model);
    let values = profile_values(tx, model, release, ENTITY, &draft)?;
    Ok(tx.create(release.record(ENTITY, values)?))
}

/// Stage an `IfcMaterialProfileSet`. IFC4 onwards. Must name at least one
/// profile.
pub fn create_profile_set(
    tx: &mut Transaction,
    model: &Model,
    profiles: &[EntityId],
    name: Option<&str>,
    description: Option<&str>,
    composite_profile: Option<EntityId>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALPROFILESET";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    if profiles.is_empty() {
        return Err(invalid(
            ENTITY,
            "MaterialProfiles",
            "expected at least one profile",
        ));
    }
    for &profile in profiles {
        require_type(tx, model, release, profile, &["IFCMATERIALPROFILE"])?;
    }
    if let Some(composite) = composite_profile {
        require_exists(tx, model, composite)?;
    }
    let record = release.record(
        ENTITY,
        vec![
            ("Name", optional_text(name)),
            ("Description", optional_text(description)),
            ("MaterialProfiles", refs(profiles)),
            (
                "CompositeProfile",
                composite_profile.map_or(Value::Null, Value::Ref),
            ),
        ],
    )?;
    Ok(tx.create(record))
}

/// Stage an `IfcMaterialProfileSetUsage`. IFC4 onwards.
///
/// `CardinalPoint` selects the cross-section reference point and is an
/// `IfcCardinalPointReference` in `1..=9`; anything else names no point.
pub fn create_profile_set_usage(
    tx: &mut Transaction,
    model: &Model,
    for_profile_set: EntityId,
    cardinal_point: Option<i64>,
    reference_extent: Option<f64>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALPROFILESETUSAGE";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    require_type(
        tx,
        model,
        release,
        for_profile_set,
        &["IFCMATERIALPROFILESET"],
    )?;
    if let Some(point) = cardinal_point.filter(|value| !(1..=9).contains(value)) {
        return Err(invalid(ENTITY, "CardinalPoint", point.to_string()));
    }
    let record = release.record(
        ENTITY,
        vec![
            ("ForProfileSet", Value::Ref(for_profile_set)),
            (
                "CardinalPoint",
                cardinal_point.map_or(Value::Null, Value::Integer),
            ),
            (
                "ReferenceExtent",
                reference_extent.map_or(Value::Null, Value::Real),
            ),
        ],
    )?;
    Ok(tx.create(record))
}

/// Stage an `IfcMaterialProfileWithOffsets`. IFC4 onwards.
///
/// The offset variant of [`create_profile`]. `OffsetValues` is an
/// `ARRAY [1:2]`: two finite lengths, so a single value or three is
/// not an under-specified profile but a malformed one.
///
/// # Errors
///
/// Refuses non-finite offsets, a priority outside `0..=100`, and a
/// `Profile` or `Material` reference whose target is the wrong type.
pub fn create_profile_with_offsets(
    tx: &mut Transaction,
    model: &Model,
    draft: ProfileDraft<'_>,
    offset_values: [f64; 2],
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALPROFILEWITHOFFSETS";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    if offset_values.iter().any(|value| !value.is_finite()) {
        return Err(invalid(ENTITY, "OffsetValues", "expected finite lengths"));
    }
    let mut values = profile_values(tx, model, release, ENTITY, &draft)?;
    values.push(("OffsetValues", reals(&offset_values)));
    Ok(tx.create(release.record(ENTITY, values)?))
}

/// Stage an `IfcMaterialProfileSetUsageTapering`. IFC4 onwards.
///
/// Five slots: three inherited, then its own two.
///
/// # Errors
///
/// Refuses a set that is not an `IfcMaterialProfileSet`, and a
/// cardinal point outside 1..=9 at either end.
pub fn create_profile_set_usage_tapering(
    tx: &mut Transaction,
    model: &Model,
    for_profile_set: EntityId,
    for_profile_end_set: EntityId,
    cardinal_point: Option<i64>,
    cardinal_end_point: Option<i64>,
    reference_extent: Option<f64>,
) -> MaterialResult<EntityId> {
    const ENTITY: &str = "IFCMATERIALPROFILESETUSAGETAPERING";
    let release = Release::of(model);
    release.require_entity(ENTITY, None)?;
    for set in [for_profile_set, for_profile_end_set] {
        require_type(tx, model, release, set, &["IFCMATERIALPROFILESET"])?;
    }
    // Both ends carry the same 1..=9 cardinal point range.
    for (point, attribute) in [
        (cardinal_point, "CardinalPoint"),
        (cardinal_end_point, "CardinalEndPoint"),
    ] {
        if let Some(value) = point.filter(|value| !(1..=9).contains(value)) {
            return Err(invalid(ENTITY, attribute, value.to_string()));
        }
    }
    let record = release.record(
        ENTITY,
        vec![
            ("ForProfileSet", Value::Ref(for_profile_set)),
            (
                "CardinalPoint",
                cardinal_point.map_or(Value::Null, Value::Integer),
            ),
            (
                "ReferenceExtent",
                reference_extent.map_or(Value::Null, Value::Real),
            ),
            ("ForProfileEndSet", Value::Ref(for_profile_end_set)),
            (
                "CardinalEndPoint",
                cardinal_end_point.map_or(Value::Null, Value::Integer),
            ),
        ],
    )?;
    Ok(tx.create(record))
}
