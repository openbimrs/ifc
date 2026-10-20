//! Unit conversion and a model's length unit (#469).
//!
//! A unit is named as UTF-8 text: an SI unit with its prefix written before
//! it (`METRE`, `MILLIMETRE`, `MILLI METRE`, `SQUARE_MILLIMETRE`,
//! `KILOGRAM`; `METER` spellings accepted), or a named conversion-based
//! unit of IFC4 Table 673 with exact factors (`inch`, `foot`, `yard`,
//! `mile`, `square foot`, `gallon US`, `pound`, `degree` ...) or `US survey
//! foot`.
//!
//! `openbim_ifc_v0_1_model_convert_length_unit` answers with a
//! `LengthUnitReport` record tape: from and to (`TEXT`), length factor
//! (`REAL`), area and volume factors (`REAL` or `NULL` when that unit does
//! not follow), the new length, area and volume unit ids (`REF` or `NULL`),
//! converted (`LIST` of `ConvertedValues`: entity type, attribute, kind
//! `TEXT` -- `length`, `area`, `volume`, `inverse-length`,
//! `inverse-area`, `map-scale` -- entities and values `INTEGER`), kept
//! (`LIST` of `KeptValues`: entity type, attribute, reason `TEXT` --
//! `explicit-unit`, `map-unit`, `unitless-parameter`, `unit-not-followed`
//! -- values), entities changed and values converted (`INTEGER`), created
//! and removed (`LIST`s of `REF`), applied (`BOOL`, false for a dry run).
//! The call never applies a conversion it cannot report: it plans first,
//! and a size query or a short buffer answers `BufferTooSmall` with the
//! sizes and leaves the model unchanged. The plan lists the ids the
//! commit creates, so the applied report has the planned one's size.

use openbim_ifc_binding_core::{convert_unit, format_length, LengthUnitOptions, ToRecord};

use crate::buffer::{fill_c_string, put, text};
use crate::domains::Out;
use crate::model::{done, with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Tape};

/// `openbim_ifc_v0_1_model_convert_length_unit` flag: keep the area and
/// volume units and their values instead of replacing them by the square
/// and cube of the new length unit.
pub const OPENBIM_IFC_LENGTH_UNIT_KEEP_AREA_AND_VOLUME: u32 = 1;
/// `openbim_ifc_v0_1_model_convert_length_unit` flag: keep the replaced
/// unit entities even when nothing else refers to them.
pub const OPENBIM_IFC_LENGTH_UNIT_KEEP_REPLACED_UNITS: u32 = 2;
/// `openbim_ifc_v0_1_model_convert_length_unit` flag: report only.
pub const OPENBIM_IFC_LENGTH_UNIT_DRY_RUN: u32 = 4;
/// `openbim_ifc_v0_1_format_length` style: decimal, in `display_unit`
/// (default the value's unit) with `places` digits after the point.
pub const OPENBIM_IFC_LENGTH_FORMAT_DECIMAL: u32 = 0;
/// `openbim_ifc_v0_1_format_length` style: feet and inches, `5' - 6 1/2"`,
/// rounded to `1/places` of an inch.
pub const OPENBIM_IFC_LENGTH_FORMAT_FEET_INCHES: u32 = 1;
/// `openbim_ifc_v0_1_format_length` style: inches, `66 1/2"`, rounded to
/// `1/places` of an inch.
pub const OPENBIM_IFC_LENGTH_FORMAT_INCHES: u32 = 2;
/// `openbim_ifc_v0_1_format_length` flag: write a whole number of feet as
/// `5'` rather than `5' - 0"`.
pub const OPENBIM_IFC_LENGTH_FORMAT_SUPPRESS_ZERO_INCHES: u32 = 1;

const ALL_FLAGS: u32 = 7;

/// Convert `value` from the unit named `from` (`from_len` UTF-8 bytes) to
/// the unit named `to`, written to `out_value`. Needs no model.
/// `InvalidValue` for text that names no unit; `TypeMismatch` for units of
/// different dimensions; `Unsupported` for a unit with an offset (degree
/// Fahrenheit, #110); `InvalidArgument` for text that is not UTF-8.
///
/// # Safety
/// `from` and `to` valid for their lengths; `out_value` valid for one
/// write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_convert_unit(
    value: f64,
    from: *const u8,
    from_len: usize,
    to: *const u8,
    to_len: usize,
    out_value: *mut f64,
) -> OpenbimIfcStatus {
    boundary(|| {
        // SAFETY: caller contract above.
        let (from, to) = match unsafe { (text(from, from_len), text(to, to_len)) } {
            (Ok(from), Ok(to)) => (from, to),
            (Err(status), _) | (_, Err(status)) => return status,
        };
        match convert_unit(value, from, to) {
            // SAFETY: caller contract above.
            Ok(converted) => unsafe { put(out_value, converted) }
                .err()
                .unwrap_or(OpenbimIfcStatus::Ok),
            Err(error) => OpenbimIfcStatus::from(&error),
        }
    })
}

/// Write `value`, a length in the unit named `unit`, as NUL-terminated
/// text. `style` is `OPENBIM_IFC_LENGTH_FORMAT_DECIMAL` (in `display_unit`,
/// or the value's unit when `display_unit` is null, with `places` decimals),
/// `OPENBIM_IFC_LENGTH_FORMAT_FEET_INCHES` or
/// `OPENBIM_IFC_LENGTH_FORMAT_INCHES` (rounded to `1/places` of an inch);
/// `flags`: `OPENBIM_IFC_LENGTH_FORMAT_SUPPRESS_ZERO_INCHES`. Needs no
/// model. `InvalidValue` for a unit name, a zero denominator or a length
/// too large; `TypeMismatch` for a unit that is not a length;
/// `OutOfRange` for more than 255 decimals; `InvalidArgument` for an
/// unknown style or flag, or text that is not UTF-8.
///
/// # Safety
/// `unit` valid for `unit_len`; `display_unit` null or valid for
/// `display_unit_len`; `buffer` and `out_required` as for
/// `openbim_ifc_v0_1_new_guid`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openbim_ifc_v0_1_format_length(
    value: f64,
    unit: *const u8,
    unit_len: usize,
    style: u32,
    places: u32,
    display_unit: *const u8,
    display_unit_len: usize,
    flags: u32,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        let style = match style {
            OPENBIM_IFC_LENGTH_FORMAT_DECIMAL => "decimal",
            OPENBIM_IFC_LENGTH_FORMAT_FEET_INCHES => "feet-inches",
            OPENBIM_IFC_LENGTH_FORMAT_INCHES => "inches",
            _ => return OpenbimIfcStatus::InvalidArgument,
        };
        if flags & !OPENBIM_IFC_LENGTH_FORMAT_SUPPRESS_ZERO_INCHES != 0 {
            return OpenbimIfcStatus::InvalidArgument;
        }
        // SAFETY: caller contract above.
        let unit = match unsafe { text(unit, unit_len) } {
            Ok(unit) => unit,
            Err(status) => return status,
        };
        let display = if display_unit.is_null() {
            None
        } else {
            // SAFETY: caller contract above.
            match unsafe { text(display_unit, display_unit_len) } {
                Ok(display) => Some(display),
                Err(status) => return status,
            }
        };
        let suppress = flags & OPENBIM_IFC_LENGTH_FORMAT_SUPPRESS_ZERO_INCHES != 0;
        match format_length(value, unit, style, places, display, suppress) {
            // SAFETY: caller contract above.
            Ok(formatted) => unsafe { fill_c_string(&formatted, buffer, capacity, out_required) },
            Err(error) => OpenbimIfcStatus::from(&error),
        }
    })
}

/// Convert `value` from unit entity `from` to unit entity `to` of the model
/// (an `IfcSIUnit`, `IfcConversionBasedUnit` or `IfcDerivedUnit`), written
/// to `out_value`. `MissingEntity`; `TypeMismatch` for units of different
/// dimensions; `Unsupported` for a monetary, context-dependent or offset
/// unit; `InvalidModel` for a unit that does not resolve;
/// `UnsupportedSchema`; `FeatureDisabled`.
///
/// # Safety
/// `out_value` valid for one write.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_convert_unit_value(
    model: OpenbimIfcModel,
    value: f64,
    from: u64,
    to: u64,
    out_value: *mut f64,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            let converted = m.convert_unit_value(value, from, to)?;
            // SAFETY: caller contract above.
            done(
                unsafe { put(out_value, converted) }
                    .err()
                    .unwrap_or(OpenbimIfcStatus::Ok),
            )
        })
    })
}

/// Convert the model's length unit to the unit named `target`
/// (`target_len` UTF-8 bytes): an SI `METRE` with any prefix, or `inch`,
/// `foot`, `yard`, `mile`, `US survey foot`. Every length, area and volume
/// value is rescaled as one checked transaction. `flags`:
/// `OPENBIM_IFC_LENGTH_UNIT_KEEP_AREA_AND_VOLUME`,
/// `OPENBIM_IFC_LENGTH_UNIT_KEEP_REPLACED_UNITS`,
/// `OPENBIM_IFC_LENGTH_UNIT_DRY_RUN`. The answer is the `LengthUnitReport`
/// tape of the module docs; a size query or a short buffer plans the
/// conversion and answers `BufferTooSmall` without applying it.
///
/// `InvalidValue` for a target that is not a length unit;
/// `Unsupported` naming the values the conversion cannot carry (a
/// parameter on a curve whose parameter space IFC leaves undefined, a
/// p-curve off a plane, polynomial coefficients ...) and for an offset
/// unit (#110); `InvalidModel` when the project length unit does not
/// resolve; `UnsupportedSchema`; `InvalidArgument` for an unknown flag or
/// text that is not UTF-8; `FeatureDisabled`.
///
/// # Safety
/// `target` valid for `target_len`; the buffers as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_convert_length_unit(
    model: OpenbimIfcModel,
    target: *const u8,
    target_len: usize,
    flags: u32,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    if flags & !ALL_FLAGS != 0 {
        return OpenbimIfcStatus::InvalidArgument;
    }
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    boundary(|| {
        // SAFETY: caller contract above.
        let target = match unsafe { text(target, target_len) } {
            Ok(target) => target.to_owned(),
            Err(status) => return status,
        };
        let options = |dry_run| {
            let mut options = LengthUnitOptions::new();
            options.area_and_volume = flags & OPENBIM_IFC_LENGTH_UNIT_KEEP_AREA_AND_VOLUME == 0;
            options.remove_replaced_units =
                flags & OPENBIM_IFC_LENGTH_UNIT_KEEP_REPLACED_UNITS == 0;
            options.dry_run = dry_run;
            options
        };
        let dry_run = flags & OPENBIM_IFC_LENGTH_UNIT_DRY_RUN != 0;
        with_model(model, |m| {
            let planned = m.convert_length_unit(&target, options(true))?;
            if dry_run {
                // SAFETY: caller contract.
                return unsafe { out.write(&planned.to_record().to_tagged()) };
            }
            // The plan lists the ids the commit creates, so the applied
            // report has the planned one's size.
            let mut sized = planned.clone();
            sized.applied = true;
            let sized = sized.to_record().to_tagged();
            let tape = Tape::encode(&sized);
            let fits =
                tape.nodes.len() <= out.node_capacity && tape.strings.len() <= out.string_capacity;
            if !fits {
                // SAFETY: caller contract; too small, so only sizes are written.
                return unsafe { out.write(&sized) };
            }
            let report = m.convert_length_unit(&target, options(false))?;
            // SAFETY: caller contract.
            unsafe { out.write(&report.to_record().to_tagged()) }
        })
    })
}
