//! Which unit type a measure type is expressed in, per declared release.
//!
//! IFC states this correspondence by name and in prose ("usually measured
//! in"), not in a normative function: `IfcAreaMeasure` goes with `AREAUNIT`,
//! `IfcThermalTransmittanceMeasure` with `THERMALTRANSMITTANCEUNIT`. So the
//! mapping is derived from the release's own tables rather than hand-listed:
//!
//! 1. The name must be a type the release declares, and a member of
//!    `IfcValue` that is not an `IfcSimpleValue`.
//! 2. `IfcXMeasure` maps to `XUNIT` when that is a member of the release's
//!    `IfcUnitEnum` or `IfcDerivedUnitEnum`.
//! 3. Otherwise a defined type is followed to its base, so
//!    `IfcPositiveLengthMeasure = IfcLengthMeasure` maps to `LENGTHUNIT` and
//!    `IfcNormalisedRatioMeasure = IfcRatioMeasure` is dimensionless.
//!
//! Anything left over is refused, never guessed. That includes names whose
//! unit enum is spelt differently (`IfcThermalConductivityMeasure` has no
//! `THERMALCONDUCTIVITYUNIT`), non-scalar measures such as
//! `IfcCompoundPlaneAngleMeasure`, and the logarithmic ones below, for which
//! `value * scale` is meaningless.

use std::collections::BTreeSet;
use std::sync::Arc;

use ifc_schema::TypeKind;

use super::release::Release;
use super::unit::ExactUnitError;
use super::value::select_accepts_type;

/// Measures that carry no unit by definition.
const DIMENSIONLESS: [&str; 3] = ["IFCCOUNTMEASURE", "IFCRATIOMEASURE", "IFCNUMERICMEASURE"];

/// Decibel and pH measures: a derived unit for them has no linear scale.
const LOGARITHMIC: [&str; 3] = [
    "IFCSOUNDPOWERLEVELMEASURE",
    "IFCSOUNDPRESSURELEVELMEASURE",
    "IFCPHMEASURE",
];

/// What a measure type needs from a unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum MeasureUnit {
    /// No unit applies.
    Dimensionless,
    /// A named unit of this `IfcUnitEnum`.
    Named(Arc<str>),
    /// An `IfcDerivedUnit` of this `IfcDerivedUnitEnum`.
    Derived(Arc<str>),
}

impl MeasureUnit {
    pub(super) fn unit_type(&self) -> Option<&Arc<str>> {
        match self {
            Self::Dimensionless => None,
            Self::Named(unit_type) | Self::Derived(unit_type) => Some(unit_type),
        }
    }
}

pub(super) fn measure_unit(
    release: Release,
    measure_type: &str,
) -> Result<MeasureUnit, ExactUnitError> {
    let schema = release.schema;
    let name = measure_type.to_ascii_uppercase();
    if schema.type_def(&name).is_none() {
        return Err(ExactUnitError::MeasureNotInSchema {
            measure_type: measure_type.into(),
            schema: release.version,
        });
    }
    if !select_accepts_type(schema, "IFCVALUE", &name)
        || select_accepts_type(schema, "IFCSIMPLEVALUE", &name)
    {
        return Err(ExactUnitError::NotAMeasure {
            measure_type: measure_type.into(),
        });
    }
    let unmapped = || ExactUnitError::UnmappedMeasureType {
        measure_type: measure_type.into(),
    };
    if LOGARITHMIC.contains(&name.as_str()) || !is_scalar(release, &name) {
        return Err(unmapped());
    }
    let named = enum_members(release, "IFCUNITENUM");
    let derived = enum_members(release, "IFCDERIVEDUNITENUM");
    let mut current = name;
    let mut seen = BTreeSet::new();
    while seen.insert(current.clone()) {
        if DIMENSIONLESS.contains(&current.as_str()) {
            return Ok(MeasureUnit::Dimensionless);
        }
        if let Some(stem) = current
            .strip_prefix("IFC")
            .and_then(|rest| rest.strip_suffix("MEASURE"))
        {
            let unit_type = format!("{stem}UNIT");
            if named.contains(&unit_type) {
                return Ok(MeasureUnit::Named(unit_type.into()));
            }
            if derived.contains(&unit_type) {
                return Ok(MeasureUnit::Derived(unit_type.into()));
            }
        }
        match defined_base(release, &current) {
            Some(base) => current = base,
            None => break,
        }
    }
    Err(unmapped())
}

/// The members of one of the release's enumerations, upper-cased.
fn enum_members(release: Release, name: &str) -> BTreeSet<String> {
    match release
        .schema
        .type_def(name)
        .map(|definition| &definition.kind)
    {
        Some(TypeKind::Enumeration(members)) => members
            .iter()
            .map(|member| member.to_ascii_uppercase())
            .collect(),
        _ => BTreeSet::new(),
    }
}

/// The declared type a defined type aliases, when that is itself a type.
fn defined_base(release: Release, name: &str) -> Option<String> {
    let TypeKind::Defined(rhs) = &release.schema.type_def(name)?.kind else {
        return None;
    };
    let base = first_word(rhs).to_ascii_uppercase();
    release.schema.type_def(&base).map(|_| base)
}

/// Whether the measure's underlying type is one number, not a list.
fn is_scalar(release: Release, name: &str) -> bool {
    let mut current = name.to_ascii_uppercase();
    let mut seen = BTreeSet::new();
    while seen.insert(current.clone()) {
        let Some(TypeKind::Defined(rhs)) = release
            .schema
            .type_def(&current)
            .map(|definition| &definition.kind)
        else {
            return false;
        };
        let base = first_word(rhs).to_ascii_uppercase();
        match base.as_str() {
            "REAL" | "NUMBER" | "INTEGER" => return true,
            _ if release.schema.type_def(&base).is_some() => current = base,
            _ => return false,
        }
    }
    false
}

fn first_word(rhs: &str) -> &str {
    rhs.split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .find(|part| !part.is_empty())
        .unwrap_or("")
}
