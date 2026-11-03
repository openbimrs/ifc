//! The units a property or quantity edit, and an authoring batch, read their
//! numbers in (#487, #638).
//!
//! A writer that computes in SI (metre, kilogram, kelvin, radian) must not
//! need the model's units to write a value: it states the SI value and the
//! edit converts it into the unit that applies. [`PropertyEditOptions`] with
//! [`InputUnits::Si`] asks for that; [`InputUnits::Raw`], the default,
//! writes every number as given, which is what the edit has always done.
//!
//! # What `Si` converts
//!
//! A real or integer payload of a measure type whose unit type
//! `ifc_properties::measure_unit_type` knows: length, area, volume, plane
//! angle, mass, pressure, temperature and the other mapped measures. The
//! result is `ifc_properties::si_conversion` of the measure and its unit.
//! A dimensionless measure (`IfcRatioMeasure`), a non-numeric payload and a
//! value that is not a typed measure pass through unchanged; the edit's own
//! checks refuse an invalid one afterwards.
//!
//! # The unit that applies
//!
//! The `Unit` stated by the property being written (a single, list or
//! quantity member; for an enumerated value, the unit of its
//! `IfcPropertyEnumeration`), else the project unit of the measure's unit
//! type. IFC states no SI default, so a model that declares no unit of the
//! type is refused, never written in an assumed one.
//!
//! # Refused
//!
//! - A measure the units table has no verified correspondence for, a project
//!   without a unit of the type, an offset unit and a non-finite number are
//!   refused with `PropertyEditFailure::UnitConversion`.
//! - An integer payload of a non-quantity property is refused as an invalid
//!   value rather than converted: the converted number is generally not an
//!   integer. A quantity takes the integer as a real.
//!
//! The conversion runs before the template and value checks, which see the
//! converted number, and it is the converted number that is stored.
//!
//! # Authoring batches
//!
//! [`AuthoringOptions`] with [`InputUnits::Si`] does the same for
//! `apply_authoring_with` and `stage_authoring_with` (feature `authoring`).
//! It converts the attribute values of `Create`, `Edit`, `ReassignClass`,
//! `Project`, `Spatial`, `Product` and `TypeObject`, a `Placement` location
//! and a `CopyProduct` offset; values the planner derives or copies are
//! already in the model's units. The rules, and what is refused, are in the
//! `authoring` module documentation ("SI input"). One difference from
//! property edits: monetary and logarithmic measures pass unchanged there
//! (a reviewed table), where a property edit refuses every measure without a
//! unit mapping.
#![cfg(feature = "properties")]

/// How a property edit reads its numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum InputUnits {
    /// Numbers are written as given, in whatever unit applies.
    #[default]
    Raw,
    /// Numbers are SI values and are converted into the unit that applies.
    /// A number that cannot be converted is refused as `UnitConversion`
    /// (`InvalidValue` for an integer), whichever check `Raw` would have
    /// refused it with.
    Si,
}

/// Options of `stage_property_edits_with` and
/// `apply_property_edits_with`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PropertyEditOptions {
    /// How numbers are read.
    pub input_units: InputUnits,
}

impl PropertyEditOptions {
    /// These options with `input_units` set.
    #[must_use]
    pub const fn with_input_units(mut self, input_units: InputUnits) -> Self {
        self.input_units = input_units;
        self
    }
}

/// Options of `stage_authoring_with` and `apply_authoring_with` (#638).
#[cfg(feature = "authoring")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct AuthoringOptions {
    /// How the batch's numbers are read.
    pub input_units: InputUnits,
}

#[cfg(feature = "authoring")]
impl AuthoringOptions {
    /// These options with `input_units` set.
    #[must_use]
    pub const fn with_input_units(mut self, input_units: InputUnits) -> Self {
        self.input_units = input_units;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_read_raw_unless_si_is_asked_for() {
        assert_eq!(InputUnits::default(), InputUnits::Raw);
        let options = PropertyEditOptions::default();
        assert_eq!(options.input_units, InputUnits::Raw);
        assert_eq!(
            options.with_input_units(InputUnits::Si).input_units,
            InputUnits::Si
        );
    }

    #[cfg(feature = "authoring")]
    #[test]
    fn a_batch_reads_raw_unless_si_is_asked_for() {
        let options = AuthoringOptions::default();
        assert_eq!(options.input_units, InputUnits::Raw);
        assert_eq!(
            options.with_input_units(InputUnits::Si).input_units,
            InputUnits::Si
        );
    }
}
