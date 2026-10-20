//! Hand-written models for the spot checks.

use ifc_model::{Codec, Model};
use ifc_schema::SchemaVersion;
use ifc_step::StepCodec;
use ifc_validate::{Finding, FindingDetail, Report, RuleDetail};

/// A STEP file declaring `version`, holding `data`.
pub fn step(version: SchemaVersion, data: &str) -> Model {
    let token = match version {
        SchemaVersion::Ifc2x3 => "IFC2X3",
        SchemaVersion::Ifc4 => "IFC4",
        SchemaVersion::Ifc4x1 => "IFC4X1",
        SchemaVersion::Ifc4x2 => "IFC4X2",
        _ => "IFC4X3_ADD2",
    };
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('spot check'),'2;1');\n\
         FILE_NAME('spot.ifc','2026-10-20T00:00:00',(''),(''),'','','');\n\
         FILE_SCHEMA(('{token}'));\nENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .unwrap_or_else(|error| panic!("the spot fixture parses: {error}"))
}

/// The findings reported under `rule`.
pub fn under<'r>(report: &'r Report, rule: &str) -> Vec<&'r Finding> {
    report
        .findings()
        .iter()
        .filter(|finding| finding.rule == rule)
        .collect()
}

/// The rule detail of a finding.
pub fn detail(finding: &Finding) -> &RuleDetail {
    match &finding.detail {
        Some(FindingDetail::Rule(detail)) => detail,
        other => panic!("{finding} carries no rule detail: {other:?}"),
    }
}

/// A geometric context and a 2D polyline (#1-#5), shared by the
/// representation checks: a `Curve2D` item in every release. The
/// context's `TrueNorth` is unset, so no direction rule applies to it.
pub const GEOMETRY: &str = "\
#1=IFCCARTESIANPOINT((0.,0.,0.));
#2=IFCAXIS2PLACEMENT3D(#1,$,$);
#3=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#2,$);
#4=IFCCARTESIANPOINT((1.,0.));
#9=IFCCARTESIANPOINT((0.,0.));
#5=IFCPOLYLINE((#9,#4));";
