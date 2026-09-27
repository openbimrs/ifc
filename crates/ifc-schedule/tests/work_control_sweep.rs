//! Sweep both `WorkControlKind` variants.
//!
//! `create_work_control` maps the kind to a type name through a match.
//! Only one arm was exercised, so `IfcWorkPlan` was never proven to
//! stage at all.

use ifc_model::{Model, Transaction};
use ifc_schedule::{create_work_control, WorkControlDraft, WorkControlKind};

const GUID: &str = "1hqA$FMcT8$hVvcqsRDBzZ";

fn draft() -> WorkControlDraft<'static> {
    WorkControlDraft {
        global_id: GUID,
        name: Some("Sweep"),
        description: None,
        identification: None,
        creation_date: "2026-09-22T09:00:00",
        purpose: None,
        duration: None,
        total_float: None,
        start_time: "2026-09-22T09:00:00",
        finish_time: None,
        predefined_type: None,
    }
}

/// Both work-control forms stage under their own type name.
#[test]
fn both_work_control_variants_stage() {
    for (kind, expected) in [
        (WorkControlKind::Plan, "IFCWORKPLAN"),
        (WorkControlKind::Schedule, "IFCWORKSCHEDULE"),
    ] {
        let mut model = Model::default();
        let mut tx = Transaction::new(&model);
        let id = create_work_control(&mut tx, kind, draft())
            .unwrap_or_else(|error| panic!("{expected} refused: {error:?}"));
        tx.commit(&mut model).expect("commit");
        assert_eq!(model.get(id).expect("staged").type_name.as_ref(), expected);
    }
}
