//! `relations`, `materials`, `cost` and `schedule` (#472) through the
//! built binary, over the selector fixtures in IFC2X3, IFC4 and IFC4X3 and
//! the synthetic cost and schedule model.

mod support;

use support::{fixture, run};

const RELEASES: [&str; 3] = [
    "synthetic-relations/selector_ifc2x3.ifc",
    "synthetic-relations/selector_ifc4.ifc",
    "synthetic-relations/selector_ifc4x3.ifc",
];
const COST_SCHEDULE: &str = "synthetic-cost-schedule/synthetic_cost_schedule.ifc";

#[test]
fn relations_list_each_kind_and_every_relationship_in_every_release() {
    for release in RELEASES {
        let file = fixture(release);
        let out = run(&["relations", &file, "#17"]);
        assert_eq!(out.code, 0, "{release}: {}", out.stderr);
        for line in [
            "#17 IfcWall \"Wall A\"\n",
            "  container      #13 IfcBuildingStorey \"Level 0\"\n",
            "  type           #16 IfcWallType \"WT-200\"\n",
            "  material       #54 IfcMaterialLayerSet \"Concrete 200\"\n",
            "  materials      #52 IfcMaterial \"Concrete\"\n",
            "  openings       #21 IfcOpeningElement \"Door opening\", #23 IfcOpeningElement \"Recess\"\n",
            "    #20 IfcRelDefinesByType.RelatedObjects\n",
            "    #33 IfcRelContainedInSpatialStructure.RelatedElements\n",
        ] {
            assert!(out.stdout.contains(line), "{release}: {line:?} in\n{}", out.stdout);
        }
        let json = run(&["relations", &file, "#21", "--format", "json"]).json();
        assert_eq!(json["voids"][0]["id"], 17, "{release}");
        assert_eq!(json["fillings"][0]["id"], 28, "{release}");

        let up = run(&[
            "relations",
            &file,
            "#38",
            "--traverse-up",
            "--format",
            "json",
        ])
        .json();
        let chain: Vec<u64> = up
            .as_array()
            .unwrap()
            .iter()
            .map(|label| label["id"].as_u64().unwrap())
            .collect();
        assert_eq!(chain, [38, 37, 12, 11], "{release}");
    }
    let missing = run(&["relations", &fixture(RELEASES[1]), "#999"]);
    assert_eq!(missing.code, 2);
    assert!(
        missing.stderr.contains("missing-entity"),
        "{}",
        missing.stderr
    );
}

#[test]
fn materials_show_each_definition_with_its_parts() {
    for release in RELEASES {
        let file = fixture(release);
        let out = run(&["materials", &file, "IfcWall"]);
        assert_eq!(out.code, 0, "{release}: {}", out.stderr);
        assert!(
            out.stdout.starts_with(
                "#17 IfcWall \"Wall A\"\n  #54 IfcMaterialLayerSet \"Concrete 200\"\n"
            ),
            "{release}: {}",
            out.stdout
        );
        assert!(!out.stdout.contains("Wall C"), "no material, not listed");
        let json = run(&["materials", &file, "IfcDoor", "--format", "json"]).json();
        assert_eq!(
            json["entities"][0]["materials"][0]["name"], "Oak",
            "{release}"
        );
        let layer = &run(&["materials", &file, "IfcWall", "--format", "json"]).json()["entities"]
            [0]["materials"][0]["parts"][0];
        assert_eq!(layer["material"], "Concrete", "{release}");
        assert_eq!(layer["thickness"], 0.2, "{release}");
    }
}

#[test]
fn cost_shows_each_schedule_as_a_tree_with_both_totals() {
    let file = fixture(COST_SCHEDULE);
    let out = run(&["cost", &file]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.stdout.starts_with(
            "#31 IfcCostSchedule \"Budget\" (BUDGET)\n  \
             #38 A.1 \"Substructure\" direct 500, nested 500\n    \
             value \"Subtotal\" 500\n    \
             #39 A.1.1 \"Excavation\" direct 320\n"
        ),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("#44 A.3 \"Preliminaries\" direct 0\n"),
        "{}",
        out.stdout
    );
    let json = run(&["cost", &file, "--depth", "0", "--format", "json"]).json();
    assert_eq!(json[0]["items"][0]["items"]["truncated"], true);
    assert_eq!(json[0]["items"][0]["items"]["count"], 2);
    assert_eq!(json[0]["items"][1]["rolled_up_total"], 45.5);
}

#[test]
fn schedule_shows_each_work_schedule_as_a_task_tree() {
    let file = fixture(COST_SCHEDULE);
    let out = run(&["schedule", &file]);
    assert_eq!(out.code, 0, "{}", out.stderr);
    assert!(
        out.stdout.contains(
            "#55 IfcWorkSchedule \"Construction schedule\" (PLANNED)\n  \
             #61 T-1 \"Excavate\" 2026-03-02T08:00:00 .. 2026-03-06T17:00:00 (P5D)\n    \
             #62 T-2 \"Pour foundations\""
        ),
        "{}",
        out.stdout
    );
    assert!(out
        .stdout
        .contains("#65 T-5 \"Practical completion\" milestone"));
    let json = run(&["schedule", &file, "--format", "json", "--depth", "0"]).json();
    let schedule = json
        .as_array()
        .unwrap()
        .iter()
        .find(|schedule| schedule["id"] == 55)
        .unwrap();
    assert_eq!(schedule["tasks"][0]["subtasks"]["count"], 1);
    let none = run(&["schedule", &fixture(RELEASES[1])]);
    assert_eq!(none.stdout, "no work schedule or work plan\n");
}
