//! The domain views (#123) as value tapes.
//!
//! Each domain record of the shared core crosses as one `LIST` of its
//! field values in declaration order, nested records as nested `LIST`s
//! (`openbim_ifc_binding_core::Record::to_tagged`). An id is a `REF`, an
//! absent field `NULL`, a count an `INTEGER`, a resolved parameter a
//! `REAL`, and an IFC value its own tagged node, typed wrapper included.
//! The field order of every record is listed in the binding docs
//! (`docs/bindings/c.md`, "Domain record tapes"); the core's record
//! definitions are the source of truth.
//!
//! As for validation, the C protocol sizes buffers with a first call, so
//! a size query runs the read; a host that wants one run passes buffers it
//! expects to be large enough and retries only on `BufferTooSmall`.

use openbim_ifc_binding_core::record::{to_records, Record};
use openbim_ifc_binding_core::value::Tagged;
use openbim_ifc_binding_core::{BindingError, IfcModel, ToRecord};

use crate::buffer::{put, text};
use crate::model::{done, fill_tape, with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Tape};

/// The output side every domain export shares.
struct Out {
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
}

impl Out {
    /// Write `value` as a tape.
    ///
    /// # Safety
    /// The buffers as for `openbim_ifc_v0_1_entity_attribute`.
    unsafe fn write(&self, value: &Tagged) -> Result<OpenbimIfcStatus, BindingError> {
        let tape = Tape::encode(value);
        // SAFETY: forwarded caller contract.
        done(unsafe {
            fill_tape(
                &tape,
                self.nodes,
                self.node_capacity,
                self.out_nodes_required,
                self.strings,
                self.string_capacity,
                self.out_strings_required,
            )
        })
    }
}

/// A list of records as one tape, its length written to `out_count`.
///
/// # Safety
/// `out_count` checked non-null by the caller; the buffers as for `Out`.
unsafe fn list(
    records: &[Record],
    out_count: *mut usize,
    out: &Out,
) -> Result<OpenbimIfcStatus, BindingError> {
    // SAFETY: caller contract above.
    if let Err(status) = unsafe { put(out_count, records.len()) } {
        return done(status);
    }
    let tape = Tagged::List(records.iter().map(Record::to_tagged).collect());
    // SAFETY: caller contract above.
    unsafe { out.write(&tape) }
}

/// Run a list-returning domain read on `model`.
///
/// # Safety
/// As for [`list`].
unsafe fn list_export(
    model: OpenbimIfcModel,
    out_count: *mut usize,
    out: Out,
    read: impl FnOnce(&IfcModel) -> Result<Vec<Record>, BindingError>,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_count.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        with_model(model, |m| {
            let records = read(m)?;
            // SAFETY: checked non-null; caller contract.
            unsafe { list(&records, out_count, &out) }
        })
    })
}

/// Run a single-record domain read on `model`; `None` is a `NULL` tape.
///
/// # Safety
/// The buffers as for `Out`.
unsafe fn record_export(
    model: OpenbimIfcModel,
    out: Out,
    read: impl FnOnce(&IfcModel) -> Result<Option<Record>, BindingError>,
) -> OpenbimIfcStatus {
    boundary(|| {
        with_model(model, |m| {
            let value = read(m)?.map_or(Tagged::Null, |record| record.to_tagged());
            // SAFETY: caller contract.
            unsafe { out.write(&value) }
        })
    })
}

/// The property sets of `object`: a `LIST` of `PropertySet` records, its
/// own sets first, then those its type object holds; `out_count` gets their
/// number. `PropertySet`: id (`REF`), global id, name, type name, source
/// (`occurrence` or `type`), source id (`REF` or `NULL`), properties
/// (`LIST` of `Property`). `Property`: id, name, type name, kind, value
/// type, unit (`REF` or `NULL`), value (tagged, typed), enumeration,
/// bounds, table, usage, discrimination, quality, members (`LIST` of
/// `Property`).
///
/// `MissingEntity`, `WrongEntityType`, `UnsupportedSchema` (a release other
/// than IFC2X3, IFC4 or IFC4X3), `InvalidModel`, `FeatureDisabled`.
///
/// # Safety
/// `out_count` valid for one write; otherwise as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_property_sets(
    model: OpenbimIfcModel,
    object: u64,
    out_count: *mut usize,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    // SAFETY: caller contract above.
    unsafe {
        list_export(model, out_count, out, |m| {
            Ok(to_records(&m.property_sets(object)?))
        })
    }
}

/// The effective unit of a `measure_type` value (`IFCAREAMEASURE`, UTF-8,
/// `measure_len` bytes): `unit` when non-zero (a property's stated unit),
/// otherwise the project default. The tape is one `ResolvedUnit`: unit
/// (`REF` or `NULL`), from project (`BOOL`), dimensions (`LIST` of seven
/// `INTEGER`s, SI exponents L M T I Θ N J), scale (`REAL`), offset (`REAL`).
///
/// `InvalidValue` for a type that is no measure, `Unsupported`,
/// `InvalidModel`, `FeatureDisabled`.
///
/// # Safety
/// `measure_type` valid for `measure_len` reads; otherwise as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_resolve_unit(
    model: OpenbimIfcModel,
    measure_type: *const u8,
    measure_len: usize,
    unit: u64,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    // SAFETY: caller contract above.
    let measure = match unsafe { text(measure_type, measure_len) } {
        Ok(measure) => measure,
        Err(status) => return status,
    };
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    // SAFETY: caller contract above.
    unsafe {
        record_export(model, out, |m| {
            let unit = m.resolve_unit(measure, (unit != 0).then_some(unit))?;
            Ok(Some(unit.to_record()))
        })
    }
}

/// The spatial containment tree as one `SpatialTree` record: release
/// (`TEXT` or `NULL`), roots (`LIST` of `REF`), nodes (`LIST` of
/// `SpatialNode`: id, global id, name, type name, kind, parent, children,
/// elements, referenced), orphans, dangling (`LIST` of `[relation,
/// target]`), anomalies (`LIST` of `[kind, relation, subject, kept]`).
///
/// `UnsupportedSchema` when the header names no bundled release;
/// `FeatureDisabled`.
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_spatial_tree(
    model: OpenbimIfcModel,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    // SAFETY: caller contract above.
    unsafe { record_export(model, out, |m| Ok(Some(m.spatial_tree()?.to_record()))) }
}

/// The classifications of `object`: a `LIST` of `Classification` records,
/// its own then its type object's; `out_count` gets their number.
/// `Classification`: relationship (`REF`), global id, source, type object,
/// target (`REF`), kind (`reference`, `system` or `notation`),
/// identification, name, location, notation (`LIST` of `TEXT`), parents
/// (`LIST` of `REF`), system (`[id, name, source, edition]` or `NULL`).
///
/// `MissingEntity`, `UnsupportedSchema`, `InvalidModel`,
/// `MissingReference`, `BudgetExceeded`, `FeatureDisabled`.
///
/// # Safety
/// As for `openbim_ifc_v0_1_model_property_sets`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_classifications(
    model: OpenbimIfcModel,
    object: u64,
    out_count: *mut usize,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    // SAFETY: caller contract above.
    unsafe {
        list_export(model, out_count, out, |m| {
            Ok(to_records(&m.classifications(object)?))
        })
    }
}

/// The material association of `object`, its own or its type object's, as
/// one `MaterialAssignment` record, or a `NULL` tape when there is none:
/// relationship, global id, source, type object, target, type name, kind,
/// set, name, materials, layers, profiles, constituents (each a `LIST` of
/// records), usage (record or `NULL`).
///
/// `MissingEntity`, `UnsupportedSchema`, `InvalidModel`,
/// `MissingReference`, `FeatureDisabled`.
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_material(
    model: OpenbimIfcModel,
    object: u64,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    // SAFETY: caller contract above.
    unsafe {
        record_export(model, out, |m| {
            Ok(m.material(object)?.map(|material| material.to_record()))
        })
    }
}

/// Every system as one `Systems` record: systems (`LIST` of `System`: id,
/// global id, type name, name, long name, predefined type, members,
/// serviced buildings, serviced facilities) and anomalies (`LIST` of
/// `[kind, subject, other, message]`).
///
/// `UnsupportedSchema`, `FeatureDisabled`.
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_systems(
    model: OpenbimIfcModel,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    // SAFETY: caller contract above.
    unsafe { record_export(model, out, |m| Ok(Some(m.systems()?.to_record()))) }
}

/// Every cost schedule and cost item as one `Cost` record: schedules
/// (`LIST` of `CostSchedule`), items (`LIST` of `CostItem`, each with its
/// `CostValue` tree, applied values tagged), anomalies.
///
/// `UnsupportedSchema`, `MissingReference`, `BudgetExceeded`,
/// `FeatureDisabled`.
///
/// # Safety
/// As for `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_cost(
    model: OpenbimIfcModel,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    // SAFETY: caller contract above.
    unsafe { record_export(model, out, |m| Ok(Some(m.cost()?.to_record()))) }
}

/// Every coordinate operation, resolved: a `LIST` of `MapConversion`
/// records, empty when the model has none; `out_count` gets their number.
/// `MapConversion`: operation, kind, source, source kind, target CRS
/// (`ProjectedCrs` record), eastings, northings, orthogonal height, x axis
/// (two `REAL`s), scale, factors (three `REAL`s or `NULL`), project unit
/// and map unit (`[name, metres per unit]`), map unit declared (`BOOL`),
/// linear (three columns of three `REAL`s), translation (three `REAL`s).
///
/// `UnsupportedSchema` (anything but IFC4 and IFC4X3), `Unsupported`,
/// `InvalidModel`, `MissingReference`, `FeatureDisabled`.
///
/// # Safety
/// As for `openbim_ifc_v0_1_model_property_sets`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_georeferencing(
    model: OpenbimIfcModel,
    out_count: *mut usize,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    let out = Out {
        nodes,
        node_capacity,
        out_nodes_required,
        strings,
        string_capacity,
        out_strings_required,
    };
    // SAFETY: caller contract above.
    unsafe {
        list_export(model, out_count, out, |m| {
            Ok(to_records(&m.georeferencing()?))
        })
    }
}
