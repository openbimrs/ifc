//! The trim of a directrix-driven sweep, and which kind of measure it is.
//!
//! `StartParam`/`EndParam` of `IfcSurfaceCurveSweptAreaSolid` and
//! `IfcFixedReferenceSweptAreaSolid` change declaration between releases:
//!
//! ```text
//! IFC2X3  IfcParameterValue
//! IFC4    OPTIONAL IfcParameterValue
//! IFC4X3  OPTIONAL IfcCurveMeasureSelect
//!         = SELECT (IfcLengthMeasure, IfcParameterValue)
//! ```
//!
//! In IFC4X3 the file says, through the SELECT member it writes, whether the
//! trim is a curve parameter or a distance along the directrix. The two are
//! different quantities: a parameter lives in the directrix's own
//! parameterisation (an angle on a conic, a vector multiple on a line), a
//! length in the model's length unit. Merging them reads a 5 m trim on a
//! circle as five radians. [`TrimMeasure`] keeps them apart (#210).

use ifc_model::Value;

use crate::error::GeometryResult;
use crate::slots::Slots;

/// One end of a directrix sweep's trim, as the file states it.
///
/// Values are in the file's units, as everywhere in [`crate::solid`];
/// converting a [`Self::Length`] is lowering's job.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum TrimMeasure {
    /// An `IfcParameterValue` in the directrix's own parameter space: a bare
    /// value in IFC2X3 and IFC4, `IFCPARAMETERVALUE(..)` in IFC4X3.
    Parameter(f64),
    /// An `IfcLengthMeasure`, a distance along the directrix in the file's
    /// length unit: `IFCLENGTHMEASURE(..)`, IFC4X3 only.
    Length(f64),
}

impl TrimMeasure {
    /// The parameter value, or `None` for a length.
    pub fn parameter(self) -> Option<f64> {
        match self {
            Self::Parameter(value) => Some(value),
            Self::Length(_) => None,
        }
    }
}

/// Read an optional trim slot.
///
/// `$` is `None`. A bare number is the `IfcParameterValue` of IFC2X3 and
/// IFC4. A typed value must be one of the two `IfcCurveMeasureSelect`
/// members; any other wrapper is refused rather than guessed at.
pub(super) fn read_trim(
    slots: &Slots<'_>,
    index: usize,
    attribute: &'static str,
) -> GeometryResult<Option<TrimMeasure>> {
    let Some(value) = slots.opt(index) else {
        return Ok(None);
    };
    const EXPECTED: &str = "an IfcParameterValue or IFCLENGTHMEASURE(..)";
    let kind_error = || slots.kind_error(attribute, EXPECTED, value);
    let measure = match value {
        Value::Typed {
            type_name,
            value: inner,
        } => {
            let number = inner.as_f64().ok_or_else(kind_error)?;
            if type_name.eq_ignore_ascii_case("IFCPARAMETERVALUE") {
                TrimMeasure::Parameter(number)
            } else if type_name.eq_ignore_ascii_case("IFCLENGTHMEASURE") {
                TrimMeasure::Length(number)
            } else {
                return Err(kind_error());
            }
        }
        other => TrimMeasure::Parameter(other.as_f64().ok_or_else(kind_error)?),
    };
    Ok(Some(measure))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::entity;
    use ifc_model::EntityId;

    fn typed(name: &str, value: f64) -> Value {
        Value::Typed {
            type_name: name.into(),
            value: Box::new(Value::Real(value)),
        }
    }

    fn read(value: Value) -> GeometryResult<Option<TrimMeasure>> {
        let e = entity("IFCSURFACECURVESWEPTAREASOLID", vec![value]);
        read_trim(&Slots::new(EntityId(1), &e), 0, "StartParam")
    }

    #[test]
    fn the_select_member_decides_the_kind() {
        assert_eq!(read(Value::Null).unwrap(), None);
        assert_eq!(
            read(Value::Real(0.5)).unwrap(),
            Some(TrimMeasure::Parameter(0.5))
        );
        assert_eq!(
            read(Value::Integer(2)).unwrap(),
            Some(TrimMeasure::Parameter(2.0))
        );
        assert_eq!(
            read(typed("IFCPARAMETERVALUE", 0.5)).unwrap(),
            Some(TrimMeasure::Parameter(0.5))
        );
        assert_eq!(
            read(typed("IfcLengthMeasure", 5.0)).unwrap(),
            Some(TrimMeasure::Length(5.0))
        );
    }

    #[test]
    fn a_wrapper_outside_the_select_is_refused() {
        for value in [
            typed("IFCNONNEGATIVELENGTHMEASURE", 1.0),
            typed("IFCREAL", 1.0),
            Value::Text("1".into()),
        ] {
            assert!(read(value).is_err());
        }
    }
}
