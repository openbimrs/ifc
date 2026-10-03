//! Validation and the reachability lint, as value tapes (#244).
//!
//! Both results are lists of records, so both cross as one tape each. The
//! C protocol sizes a buffer with a first call, so a size query runs the
//! check; a host that wants a single run passes buffers it expects to be
//! large enough and retries only on `BufferTooSmall`.

use openbim_ifc_binding_core::{unreachable, validation};

use crate::buffer::put;
use crate::model::{done, fill_tape, with_model, OpenbimIfcModel};
use crate::status::{boundary, OpenbimIfcStatus};
use crate::tape::{OpenbimIfcValueNode, Tape};

/// Counts from one validation run.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OpenbimIfcValidationSummary {
    /// Number of findings on the tape.
    pub finding_count: usize,
    /// Schema violations.
    pub errors: usize,
    /// Implemented rules that could not be decided for an instance.
    pub evaluation_errors: usize,
    /// Legal but suspicious conditions.
    pub warnings: usize,
    /// Rules this validator does not evaluate.
    pub unsupported: usize,
    /// 1 when there are no errors and no evaluation errors, else 0.
    pub conformant: u32,
    /// 1 when the run hit `max_findings`: the counts are lower bounds.
    pub truncated: u32,
}

/// Validate against the schema the header declares.
///
/// `out_summary` gets the counts, also when the tape does not fit. The tape
/// is a `LIST` of findings, sorted by severity, rule, entity and slot; each
/// finding is a `LIST` of seven values: severity (`TEXT`: `error`,
/// `evaluation-error`, `warning` or `unsupported`), rule id (`TEXT`),
/// entity (`REF`, or `NULL` for the file), attribute index (`INTEGER` or
/// `NULL`), attribute name (`TEXT` or `NULL`), path (`TEXT`) and message
/// (`TEXT`). `max_findings` 0 is the validator's default budget (10,000).
/// `UnsupportedSchema` when the declared schema is not bundled;
/// `FeatureDisabled` in a build without the `validate` feature.
///
/// # Safety
/// `out_summary` valid for one write; otherwise as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_validate(
    model: OpenbimIfcModel,
    max_findings: usize,
    out_summary: *mut OpenbimIfcValidationSummary,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_summary.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        with_model(model, |m| {
            let report = m.validate((max_findings != 0).then_some(max_findings))?;
            let summary = OpenbimIfcValidationSummary {
                finding_count: report.findings.len(),
                errors: report.summary.errors,
                evaluation_errors: report.summary.evaluation_errors,
                warnings: report.summary.warnings,
                unsupported: report.summary.unsupported,
                conformant: u32::from(report.conformant),
                truncated: u32::from(report.truncated),
            };
            // SAFETY: checked non-null; caller contract above.
            if let Err(status) = unsafe { put(out_summary, summary) } {
                return done(status);
            }
            let tape = Tape::encode(&validation::findings_to_tagged(&report.findings));
            // SAFETY: caller contract above.
            done(unsafe {
                fill_tape(
                    &tape,
                    nodes,
                    node_capacity,
                    out_nodes_required,
                    strings,
                    string_capacity,
                    out_strings_required,
                )
            })
        })
    })
}

/// Products no viewer will draw, in id order, and how many.
///
/// The tape is a `LIST` of products; each is a `LIST` of four values: the
/// product (`REF`), the reason (`TEXT`: `not-contained-in-spatial-structure`,
/// `no-representation-in-model-context` or `representation-without-context`),
/// the target views its geometry was found in instead (`LIST` of `TEXT`,
/// empty unless the reason is the second), and a one-line message (`TEXT`).
/// `FeatureDisabled` in a build without the `unreachable` feature.
///
/// # Safety
/// `out_count` valid for one write; otherwise as for
/// `openbim_ifc_v0_1_entity_attribute`.
#[no_mangle]
pub unsafe extern "C" fn openbim_ifc_v0_1_model_unreachable_products(
    model: OpenbimIfcModel,
    out_count: *mut usize,
    nodes: *mut OpenbimIfcValueNode,
    node_capacity: usize,
    out_nodes_required: *mut usize,
    strings: *mut u8,
    string_capacity: usize,
    out_strings_required: *mut usize,
) -> OpenbimIfcStatus {
    boundary(|| {
        if out_count.is_null() {
            return OpenbimIfcStatus::NullPointer;
        }
        with_model(model, |m| {
            let products = m.unreachable_products()?;
            // SAFETY: checked non-null; caller contract above.
            if let Err(status) = unsafe { put(out_count, products.len()) } {
                return done(status);
            }
            let tape = Tape::encode(&unreachable::products_to_tagged(&products));
            // SAFETY: caller contract above.
            done(unsafe {
                fill_tape(
                    &tape,
                    nodes,
                    node_capacity,
                    out_nodes_required,
                    strings,
                    string_capacity,
                    out_strings_required,
                )
            })
        })
    })
}
