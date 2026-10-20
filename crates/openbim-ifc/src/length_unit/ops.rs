//! The two entry points; see the parent module for what they do.

use ifc_model::Model;
use ifc_properties::{
    exact_project_unit, exact_schema, resolve_unit, unit_conversion, ConversionUnit, ExactUnitError,
};

use super::report::{
    LengthUnitError, LengthUnitOptions, LengthUnitReport, StagedLengthUnitConversion, Tally,
};
use super::target::{self, TargetUnits};
use super::walk;

/// Plan the conversion of `model`'s length unit to `target` and stage it on
/// one transaction, without committing.
///
/// `target` is a length unit: an SI `METRE` with any prefix
/// (`ConversionUnit::parse("MILLIMETRE")`) or a named length unit (`inch`,
/// `foot`, `yard`, `mile`, `US survey foot`). The staged report says what the
/// commit will change; a target equal to the current unit stages nothing.
///
/// # Errors
///
/// [`LengthUnitError::Target`] for a target that is not a length unit;
/// [`LengthUnitError::Schema`] for a release other than IFC2X3, IFC4 or
/// IFC4X3; [`LengthUnitError::ProjectUnit`] when the project's length, area
/// or volume unit cannot be resolved exactly; [`LengthUnitError::Refused`]
/// with every value the conversion cannot carry; and
/// [`LengthUnitError::Authoring`] if a new unit cannot be written.
pub fn stage_length_unit_conversion(
    model: &Model,
    target: &ConversionUnit,
    options: &LengthUnitOptions,
) -> Result<StagedLengthUnitConversion, LengthUnitError> {
    if !target.is_length() || target.offset != 0.0 {
        return Err(LengthUnitError::Target {
            target: target.name.clone(),
            reason: "not a length unit",
        });
    }
    target::require_writable(target)?;
    let schema = exact_schema(model).map_err(LengthUnitError::Schema)?;
    let old = project_unit(model, "LENGTHUNIT")?.ok_or_else(|| LengthUnitError::ProjectUnit {
        unit_type: "LENGTHUNIT".into(),
        error: ifc_properties::UnitConversionError::Resolve(ExactUnitError::NoProjectUnit {
            unit_type: "LENGTHUNIT".into(),
        }),
    })?;
    let (area, volume) = if options.area_and_volume {
        (
            project_unit(model, "AREAUNIT")?,
            project_unit(model, "VOLUMEUNIT")?,
        )
    } else {
        (None, None)
    };
    let units = TargetUnits::plan(target, &old.1, area.as_ref(), volume.as_ref())?;
    let mut tally = Tally::new(&old.1, target, &units);
    let mut transaction = ifc_model::Transaction::new(model);
    if units.is_identity() {
        return Ok(tally.staged(transaction));
    }
    walk::plan(model, schema, &units, &mut tally, &mut transaction)?;
    target::stage(
        model,
        &units,
        old.0,
        area,
        volume,
        options,
        &mut transaction,
        &mut tally,
    )?;
    Ok(tally.staged(transaction))
}

/// Convert `model`'s length unit to `target` and commit, all or nothing.
///
/// # Errors
///
/// As [`stage_length_unit_conversion`], and [`LengthUnitError::Conflict`]
/// when the staged transaction fails its preflight. The model is unchanged
/// on every error.
pub fn convert_length_unit(
    model: &mut Model,
    target: &ConversionUnit,
    options: &LengthUnitOptions,
) -> Result<LengthUnitReport, LengthUnitError> {
    let staged = stage_length_unit_conversion(model, target, options)?;
    let mut report = staged.report;
    if staged.transaction.is_empty() {
        return Ok(report);
    }
    let applied = staged
        .transaction
        .commit(model)
        .map_err(LengthUnitError::Conflict)?;
    report.created = applied.created;
    report.removed = applied.removed.into_iter().map(|(id, _)| id).collect();
    report.revision = Some(applied.revision);
    report.applied = true;
    Ok(report)
}

/// The project unit of `unit_type` and its conversion, `None` when the
/// project assigns none of that type.
fn project_unit(
    model: &Model,
    unit_type: &str,
) -> Result<Option<(ifc_model::EntityId, ConversionUnit)>, LengthUnitError> {
    let failed = |error| LengthUnitError::ProjectUnit {
        unit_type: unit_type.into(),
        error,
    };
    let id = match exact_project_unit(model, unit_type) {
        Ok(id) => id,
        Err(ExactUnitError::NoProjectUnit { .. }) if unit_type != "LENGTHUNIT" => return Ok(None),
        Err(error) => return Err(failed(error.into())),
    };
    let unit = resolve_unit(model, id).map_err(failed)?;
    // The unit must measure what its type says; the resolver checks
    // `IfcCorrectDimensions`, this checks the conversion is defined.
    unit_conversion(&unit, &unit).map_err(failed)?;
    Ok(Some((id, unit)))
}
