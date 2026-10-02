//! Survey of every alignment path over real IFC4X3 exports in a local
//! directory.
//!
//! The files are not committed (their licences are not established); fetch
//! them, for example, from buildingSMART/IFC4.x-IF (`tests/_archived/*`,
//! `IFC-files/*`) and run
//!
//! ```text
//! IFC_ALIGNMENT_EXPORTS=<dir> cargo test -p ifc-alignment \
//!     --test real_export_survey -- --ignored --nocapture
//! ```
//!
//! Every `.ifc` file under the directory (recursively) is read. For each
//! `IfcAlignment` the survey runs the horizontal plan, the strict and
//! partial horizontal lowerings, `VerticalLayout::resolve`,
//! `vertical_profile_law`, `CantLayout::resolve`, the gradient curve, the
//! stationing and the hierarchy, and prints how many each accepted and why
//! the rest were refused. It is a survey, not a gate: it never fails on a
//! refusal. Set `IFC_ALIGNMENT_SURVEY_VERBOSE=1` to print every refusal
//! with its file and entity.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ifc_alignment::{
    lower_gradient_curve, lower_horizontal_layout, lower_horizontal_layout_partial,
    lower_horizontal_plan, vertical_profile_law, AlignmentError, AlignmentUnits, AlignmentView,
    CantLayout, Stationing, VerticalLayout,
};
use ifc_model::{Codec, Model, Value};

#[derive(Default)]
struct Tally {
    accepted: usize,
    refused: BTreeMap<String, usize>,
}

impl Tally {
    fn record<T>(&mut self, result: &Result<T, AlignmentError>) {
        match result {
            Ok(_) => self.accepted += 1,
            Err(error) => {
                if std::env::var_os("IFC_ALIGNMENT_SURVEY_VERBOSE").is_some() {
                    CURRENT.with(|c| eprintln!("  {}: {error}", c.borrow()));
                }
                *self.refused.entry(class(error)).or_default() += 1;
            }
        }
    }
}

thread_local! {
    /// The file and alignment being surveyed, for verbose output.
    static CURRENT: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// The refusal with entity ids and values stripped, so equal causes group.
fn class(error: &AlignmentError) -> String {
    match error {
        AlignmentError::InvalidSegment { detail, .. } => format!("InvalidSegment: {detail}"),
        AlignmentError::Unsupported {
            type_name, detail, ..
        } => format!("Unsupported {type_name}: {detail}"),
        AlignmentError::SemanticViolation { rule, .. } => format!("SemanticViolation: {rule}"),
        AlignmentError::ProfileDiscontinuity { seam, .. } => {
            format!("ProfileDiscontinuity: {seam}")
        }
        other => {
            let text = format!("{other:?}");
            text.split([' ', '{', '(']).next().unwrap_or("").to_owned()
        }
    }
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "ifc") {
            out.push(path);
        }
    }
}

/// Project units, enough for a survey: an SI length prefix and a plane
/// angle in degrees. A file this misreads shows up as a refusal, not a
/// silent pass.
fn units(model: &Model) -> AlignmentUnits {
    let mut units = AlignmentUnits {
        length_to_metres: 1.0,
        angle_to_radians: 1.0,
    };
    for (_, entity) in model.iter() {
        let kind = entity.attributes.get(1).and_then(|v| match v {
            Value::Enum(name) => Some(name.to_string()),
            _ => None,
        });
        if entity.type_name.eq_ignore_ascii_case("IFCSIUNIT")
            && kind.as_deref() == Some("LENGTHUNIT")
        {
            if let Some(Value::Enum(prefix)) = entity.attributes.get(2) {
                units.length_to_metres = match &**prefix {
                    "MILLI" => 1e-3,
                    "CENTI" => 1e-2,
                    "DECI" => 1e-1,
                    "KILO" => 1e3,
                    _ => 1.0,
                };
            }
        }
        if entity
            .type_name
            .eq_ignore_ascii_case("IFCCONVERSIONBASEDUNIT")
            && kind.as_deref() == Some("PLANEANGLEUNIT")
        {
            units.angle_to_radians = core::f64::consts::PI / 180.0;
        }
    }
    units
}

#[test]
#[ignore = "needs IFC_ALIGNMENT_EXPORTS pointing at local real exports"]
fn survey_every_alignment_path() {
    let Ok(dir) = std::env::var("IFC_ALIGNMENT_EXPORTS") else {
        eprintln!("IFC_ALIGNMENT_EXPORTS is not set");
        return;
    };
    let mut paths = Vec::new();
    files(Path::new(&dir), &mut paths);
    paths.sort();
    let names = [
        "horizontal plan",
        "horizontal strict",
        "horizontal partial (complete)",
        "VerticalLayout::resolve",
        "vertical_profile_law",
        "CantLayout::resolve",
        "gradient curve",
        "stationing",
        "hierarchy",
    ];
    let mut tallies: Vec<Tally> = names.iter().map(|_| Tally::default()).collect();
    let mut read = 0;
    for path in &paths {
        let Ok(model) = ifc_step::StepCodec.read_path(path) else {
            continue;
        };
        let Ok(view) = AlignmentView::for_model(&model) else {
            continue;
        };
        read += 1;
        let units = units(&model);
        for alignment in view.alignments() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            CURRENT.with(|c| *c.borrow_mut() = format!("{name} #{}", alignment.0));
            let hierarchy = view.hierarchy(alignment);
            tallies[8].record(&hierarchy);
            let Ok(hierarchy) = hierarchy else { continue };
            tallies[7].record(&Stationing::resolve(&model, alignment, units));
            let cant = match hierarchy.cant.as_slice() {
                [only] => {
                    let resolved = CantLayout::resolve(&model, *only, units);
                    tallies[5].record(&resolved);
                    resolved.ok()
                }
                _ => None,
            };
            if let [horizontal] = hierarchy.horizontal.as_slice() {
                tallies[0].record(&lower_horizontal_plan(
                    &model,
                    *horizontal,
                    units,
                    cant.as_ref(),
                ));
                tallies[1].record(&lower_horizontal_layout(
                    &model,
                    *horizontal,
                    units,
                    cant.as_ref(),
                ));
                let partial =
                    lower_horizontal_layout_partial(&model, *horizontal, units, cant.as_ref());
                match partial {
                    Ok(p) if p.is_complete() => tallies[2].accepted += 1,
                    Ok(p) => {
                        for refused in &p.refused {
                            *tallies[2]
                                .refused
                                .entry(format!("segment: {}", class(&refused.reason)))
                                .or_default() += 1;
                        }
                    }
                    Err(error) => tallies[2].record::<()>(&Err(error)),
                }
            }
            for vertical in &hierarchy.vertical {
                tallies[3].record(&VerticalLayout::resolve(&model, *vertical, units));
                tallies[4].record(&vertical_profile_law(&model, *vertical, units));
            }
            if hierarchy.horizontal.len() == 1 && hierarchy.vertical.len() == 1 {
                tallies[6].record(&lower_gradient_curve(&model, alignment, units));
            }
        }
    }
    eprintln!("{read} IFC4X3 files read from {dir}");
    for (name, tally) in names.iter().zip(&tallies) {
        let refused: usize = tally.refused.values().sum();
        eprintln!("{name}: {} accepted, {refused} refused", tally.accepted);
        for (reason, count) in &tally.refused {
            eprintln!("    {count:4}  {reason}");
        }
    }
}
