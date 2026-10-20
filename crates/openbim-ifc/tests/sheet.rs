//! Tables of values by path (#474) over the selector model in IFC2X3, IFC4
//! and IFC4X3: export, an unchanged import that writes nothing, edits by
//! row and column as one transaction, and refusals that leave the model
//! unchanged.
//!
//! `test/fixtures/synthetic-relations/selector_*.ifc` (see
//! `tests/selector.rs` for its map): walls #17 and #18 share type #16
//! `WT-200`, whose `Pset_WallCommon` states `FireRating` `F90`; #17 states
//! `IsExternal` TRUE and `Qto_WallBaseQuantities.Length` 5, #18
//! `IsExternal` FALSE; #19 has no type and no sets.

#![cfg(all(
    feature = "step",
    feature = "sheet",
    feature = "properties",
    feature = "property-catalog",
    feature = "ifc2x3",
    feature = "ifc4",
    feature = "ifc4x3"
))]

use ifc::selector::{
    apply_text_edits, get_value, select, SelectedValue, Selector, SelectorError, SelectorIndex,
    SetTarget, TextEdit, ValuePath,
};
use ifc::sheet::{
    export, import, plan_import, KeyColumn, Sheet, SheetFormat, SheetOptions, SheetRefusal,
    SheetRefusalReason,
};
use ifc::{Codec, EntityId, Model, Schema, SchemaVersion, StepCodec};

const FIXTURES: [(&str, SchemaVersion); 3] = [
    ("selector_ifc2x3.ifc", SchemaVersion::Ifc2x3),
    ("selector_ifc4.ifc", SchemaVersion::Ifc4),
    ("selector_ifc4x3.ifc", SchemaVersion::Ifc4x3),
];

const COLUMNS: [&str; 9] = [
    "Name",
    "Description",
    "type.Name",
    "Pset_WallCommon.FireRating",
    "Pset_WallCommon.IsExternal",
    "Qto_WallBaseQuantities.Length",
    "container.Name",
    "class",
    "id",
];

fn read(name: &str) -> Model {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    StepCodec
        .read_path(std::path::Path::new(&path))
        .unwrap_or_else(|error| panic!("{name}: {error}"))
}

fn each_release(mut check: impl FnMut(&str, SchemaVersion, Model, &'static Schema)) {
    for (name, version) in FIXTURES {
        let schema = ifc::schema::for_version(version).expect("bundled");
        check(name, version, read(name), schema);
    }
}

fn walls(model: &Model, schema: &Schema) -> Vec<EntityId> {
    select(model, schema, &Selector::parse("IfcWall").unwrap()).unwrap()
}

fn paths(columns: &[&str]) -> Vec<ValuePath> {
    columns
        .iter()
        .map(|text| ValuePath::parse(text).unwrap())
        .collect()
}

fn exported(model: &Model, schema: &Schema, options: &SheetOptions) -> Sheet {
    let index = SelectorIndex::new(model, schema);
    export(&index, &walls(model, schema), &paths(&COLUMNS), options).unwrap()
}

fn value(model: &Model, schema: &Schema, id: u64, path: &str) -> SelectedValue {
    get_value(
        model,
        schema,
        EntityId(id),
        &ValuePath::parse(path).unwrap(),
    )
    .unwrap()
}

fn text(value: &str) -> SelectedValue {
    SelectedValue::Text(value.to_owned())
}

/// Set the cell of `column` in the row keyed `#id`'s entity.
fn set(sheet: &mut Sheet, id: u64, column: &str, text: &str) {
    let at = sheet
        .header
        .iter()
        .position(|header| header == column)
        .unwrap_or_else(|| panic!("no column {column}"));
    let id_at = sheet
        .header
        .iter()
        .position(|header| header == "id")
        .unwrap();
    let row = sheet
        .rows
        .iter_mut()
        .find(|row| row[id_at] == id.to_string())
        .unwrap_or_else(|| panic!("no row for #{id}"));
    row[at] = text.to_owned();
}

fn bytes(model: &Model) -> Vec<u8> {
    StepCodec.write_bytes(model).unwrap()
}

#[test]
fn export_reads_every_column_in_every_release() {
    each_release(|name, _, model, schema| {
        let sheet = exported(&model, schema, &SheetOptions::default());
        let mut header = vec!["GlobalId"];
        header.extend(COLUMNS);
        assert_eq!(sheet.header, header, "{name}");
        let rows: Vec<Vec<&str>> = sheet
            .rows
            .iter()
            .map(|row| row[1..].iter().map(String::as_str).collect())
            .collect();
        assert_eq!(
            rows,
            [
                ["Wall A", "", "WT-200", "F90", "TRUE", "5", "Level 0", "IfcWall", "17"],
                ["Wall B", "", "WT-200", "F90", "FALSE", "", "Level 0", "IfcWall", "18"],
                ["Wall C", "", "", "", "", "", "Level 1", "IfcWall", "19"],
            ],
            "{name}"
        );
        let guid = value(&model, schema, 17, "GlobalId");
        assert_eq!(text(&sheet.rows[0][0]), guid, "{name}");
    });
}

#[test]
fn an_unchanged_table_imports_as_a_no_op_in_every_format() {
    each_release(|name, _, mut model, schema| {
        let before = bytes(&model);
        let revision = model.revision();
        let options = SheetOptions::default().with_null("-");
        let sheet = exported(&model, schema, &options);
        let mut formats = vec![SheetFormat::Csv, SheetFormat::Tsv];
        if cfg!(feature = "sheet-xlsx") {
            formats.push(SheetFormat::Xlsx);
        }
        for format in formats {
            let read_back = Sheet::read(&sheet.write(format).unwrap(), format).unwrap();
            assert_eq!(read_back, sheet, "{name} {format:?}");
            let plan = plan_import(&SelectorIndex::new(&model, schema), &read_back, &options)
                .unwrap_or_else(|refused| panic!("{name} {format:?}: {refused:?}"));
            assert!(plan.is_empty(), "{name} {format:?}: {:?}", plan.changes);
            assert!(plan.transaction.is_empty());
            let changes = import(&mut model, schema, &read_back, &options).unwrap();
            assert!(changes.is_empty());
            assert_eq!(model.revision(), revision, "{name}: no new revision");
            assert_eq!(bytes(&model), before, "{name} {format:?}");
        }
        // Keyed by id, the same.
        let by_id = SheetOptions::default().with_key(KeyColumn::Id);
        let sheet = exported(&model, schema, &by_id);
        assert_eq!(sheet.header[0], "id");
        assert!(
            !sheet.header[1..].contains(&"id".to_owned()),
            "the key once"
        );
        assert!(import(&mut model, schema, &sheet, &by_id)
            .unwrap()
            .is_empty());
    });
}

#[test]
fn edits_are_written_by_path_as_one_transaction() {
    each_release(|name, _, mut model, schema| {
        let options = SheetOptions::default();
        let mut sheet = exported(&model, schema, &options);
        set(&mut sheet, 17, "Name", "Wall A1");
        set(&mut sheet, 17, "Pset_WallCommon.FireRating", "F120");
        set(&mut sheet, 17, "Pset_WallCommon.IsExternal", "false");
        set(&mut sheet, 17, "Qto_WallBaseQuantities.Length", "5.5");
        // Two rows give the type they share the same new name: one write.
        set(&mut sheet, 17, "type.Name", "WT-250");
        set(&mut sheet, 18, "type.Name", "WT-250");
        set(&mut sheet, 19, "Description", "Parapet");

        let plan = plan_import(&SelectorIndex::new(&model, schema), &sheet, &options)
            .unwrap_or_else(|refused| panic!("{name}: {refused:?}"));
        let changed: Vec<(usize, &str)> = plan
            .changes
            .iter()
            .map(|change| (change.row, change.path.as_str()))
            .collect();
        assert_eq!(
            changed,
            [
                (2, "Name"),
                (2, "type.Name"),
                (2, "Pset_WallCommon.FireRating"),
                (2, "Pset_WallCommon.IsExternal"),
                (2, "Qto_WallBaseQuantities.Length"),
                (4, "Description"),
            ],
            "{name}"
        );
        let rename = &plan.changes[1].change;
        assert_eq!(
            rename.target,
            SetTarget::Attribute {
                entity: EntityId(16),
                name: "Name".to_owned()
            }
        );
        assert_eq!(
            (&rename.before, &rename.after),
            (&text("WT-200"), &text("WT-250"))
        );

        let revision = model.revision();
        plan.commit(&mut model).unwrap().expect("a commit");
        assert!(model.revision() > revision);
        // Through the file and back.
        let model = StepCodec.read_bytes(&bytes(&model)).unwrap();
        assert_eq!(value(&model, schema, 17, "Name"), text("Wall A1"), "{name}");
        assert_eq!(value(&model, schema, 16, "Name"), text("WT-250"), "{name}");
        assert_eq!(
            value(&model, schema, 17, "Pset_WallCommon.FireRating"),
            text("F120"),
            "{name}: the occurrence's own value"
        );
        assert_eq!(
            value(&model, schema, 18, "Pset_WallCommon.FireRating"),
            text("F90"),
            "{name}: still the type's"
        );
        assert_eq!(
            value(&model, schema, 17, "Pset_WallCommon.IsExternal"),
            SelectedValue::Bool(false)
        );
        assert_eq!(
            value(&model, schema, 17, "Qto_WallBaseQuantities.Length"),
            SelectedValue::Real(5.5)
        );
        assert_eq!(value(&model, schema, 19, "Description"), text("Parapet"));
        // What was imported now exports, spelled canonically, and imports
        // as a no-op.
        set(&mut sheet, 17, "Pset_WallCommon.IsExternal", "FALSE");
        assert_eq!(exported(&model, schema, &options), sheet, "{name}");
        let mut model = model;
        assert!(import(&mut model, schema, &sheet, &options)
            .unwrap()
            .is_empty());
    });
}

#[test]
fn null_text_and_enumerations_are_typed_by_the_release() {
    each_release(|name, version, mut model, schema| {
        let options = SheetOptions::default()
            .with_key(KeyColumn::Id)
            .with_null("$");
        let mut columns = vec!["id", "Name", "Description"];
        if version != SchemaVersion::Ifc2x3 {
            columns.push("PredefinedType");
        }
        let index = SelectorIndex::new(&model, schema);
        let mut sheet = export(&index, &walls(&model, schema), &paths(&columns), &options).unwrap();
        assert_eq!(sheet.rows[0][2], "$", "{name}: NULL as the null text");
        set(&mut sheet, 17, "Description", "");
        set(&mut sheet, 18, "Name", "$");
        if version != SchemaVersion::Ifc2x3 {
            set(&mut sheet, 19, "PredefinedType", "movable");
        }
        import(&mut model, schema, &sheet, &options).unwrap_or_else(|r| panic!("{name}: {r:?}"));
        assert_eq!(value(&model, schema, 17, "Description"), text(""), "{name}");
        assert_eq!(value(&model, schema, 18, "Name"), SelectedValue::Null);
        if version != SchemaVersion::Ifc2x3 {
            assert_eq!(
                value(&model, schema, 19, "PredefinedType"),
                SelectedValue::Enum("MOVABLE".to_owned())
            );
        }
    });
}

#[test]
fn a_property_no_object_states_needs_a_typed_value() {
    each_release(|name, _, mut model, schema| {
        let options = SheetOptions::default().with_key(KeyColumn::Id);
        let header = ["id", "Pset_WallCommon.AcousticRating"]
            .map(str::to_owned)
            .to_vec();
        let row = |text: &str| vec!["19".to_owned(), text.to_owned()];
        let untyped = Sheet::new(header.clone(), vec![row("R50")]);
        let refused = import(&mut model, schema, &untyped, &options).unwrap_err();
        assert!(
            matches!(
                &refused[..],
                [SheetRefusal {
                    row: Some(2),
                    column: Some(2),
                    reason: SheetRefusalReason::Value(SelectorError::InvalidText { .. }),
                    ..
                }]
            ),
            "{name}: {refused:?}"
        );
        let typed = Sheet::new(header, vec![row("IFCLABEL(R50)")]);
        import(&mut model, schema, &typed, &options).unwrap_or_else(|r| panic!("{name}: {r:?}"));
        assert_eq!(
            value(&model, schema, 19, "Pset_WallCommon.AcousticRating"),
            text("R50")
        );
    });
}

/// An expected refusal: its row, its column, and a check of its reason.
type Expected = (
    Option<usize>,
    Option<usize>,
    fn(&SheetRefusalReason) -> bool,
);

/// `sheet` refused with exactly `expected` (row, column, reason) refusals,
/// and the model, its file and its revision unchanged.
fn assert_refused(
    model: &mut Model,
    schema: &Schema,
    sheet: &Sheet,
    options: &SheetOptions,
    expected: &[Expected],
    name: &str,
) {
    let before = bytes(model);
    let revision = model.revision();
    let refused = import(model, schema, sheet, options).unwrap_err();
    assert_eq!(refused.len(), expected.len(), "{name}: {refused:#?}");
    for (refusal, (row, column, reason)) in refused.iter().zip(expected) {
        assert_eq!(
            (refusal.row, refusal.column),
            (*row, *column),
            "{name}: {refusal}"
        );
        assert!(reason(&refusal.reason), "{name}: {refusal}");
    }
    assert_eq!(model.revision(), revision, "{name}");
    assert_eq!(
        bytes(model),
        before,
        "{name}: a refused import writes nothing"
    );
}

#[test]
fn every_refusal_is_reported_by_row_and_column_and_nothing_is_written() {
    each_release(|name, _, mut model, schema| {
        let options = SheetOptions::default();
        let base = exported(&model, schema, &options);
        let column = |header: &str| base.header.iter().position(|h| h == header).unwrap() + 1;

        // Several refusals at once, every one reported.
        let mut sheet = base.clone();
        set(&mut sheet, 17, "Qto_WallBaseQuantities.Length", "five");
        set(&mut sheet, 17, "type.Name", "WT-1");
        set(&mut sheet, 18, "type.Name", "WT-2");
        set(&mut sheet, 19, "class", "IfcSlab");
        sheet.rows[2][0] = "1rA9DY70b2pFWTs3_FH5PZ".to_owned();
        let mut ragged = base.rows[0].clone();
        ragged.pop();
        sheet.rows.push(ragged);
        sheet.rows.push(base.rows[1].clone());
        sheet.rows.push(vec![String::new(); base.header.len()]);
        assert_refused(
            &mut model,
            schema,
            &sheet,
            &options,
            &[
                (
                    Some(2),
                    Some(column("Qto_WallBaseQuantities.Length")),
                    |r| {
                        matches!(
                            r,
                            SheetRefusalReason::Value(SelectorError::InvalidText { .. })
                        )
                    },
                ),
                (Some(3), Some(column("type.Name")), |r| {
                    matches!(
                        r,
                        SheetRefusalReason::Value(SelectorError::ConflictingWrite { .. })
                    )
                }),
                (Some(4), Some(1), |r| {
                    matches!(r, SheetRefusalReason::UnknownKey(_))
                }),
                (Some(5), None, |r| {
                    matches!(
                        r,
                        SheetRefusalReason::RaggedRow {
                            cells: 9,
                            columns: 10
                        }
                    )
                }),
                (Some(6), Some(1), |r| {
                    matches!(r, SheetRefusalReason::DuplicateKey { first: 3 })
                }),
            ],
            name,
        );

        // A computed column changed.
        let mut sheet = base.clone();
        set(&mut sheet, 19, "class", "IfcSlab");
        assert_refused(
            &mut model,
            schema,
            &sheet,
            &options,
            &[(Some(4), Some(column("class")), |r| {
                matches!(
                    r,
                    SheetRefusalReason::Value(SelectorError::NotSettable { .. })
                )
            })],
            name,
        );

        // The header: a path that does not parse, with its span, and a
        // repeated one.
        let mut sheet = base.clone();
        sheet.header[2] = "Description.".to_owned();
        sheet.header[3] = "Name".to_owned();
        assert_refused(
            &mut model,
            schema,
            &sheet,
            &options,
            &[
                (
                    Some(1),
                    Some(3),
                    |r| matches!(r, SheetRefusalReason::Header(error) if error.span == (12..12)),
                ),
                (Some(1), Some(4), |r| {
                    matches!(r, SheetRefusalReason::DuplicateColumn { first: 2 })
                }),
            ],
            name,
        );

        // No key column.
        let sheet = Sheet::new(vec!["Name".to_owned()], vec![vec!["x".to_owned()]]);
        assert_refused(
            &mut model,
            schema,
            &sheet,
            &options,
            &[(Some(1), None, |r| {
                matches!(r, SheetRefusalReason::NoKeyColumn(None))
            })],
            name,
        );

        // A required attribute written as NULL, keyed by id.
        let by_id = SheetOptions::default().with_key(KeyColumn::Id);
        let mut sheet = base.clone();
        sheet.rows[0][0] = String::new();
        assert_refused(
            &mut model,
            schema,
            &sheet,
            &by_id,
            &[(Some(2), Some(1), |r| {
                matches!(
                    r,
                    SheetRefusalReason::Value(SelectorError::TypeMismatch { .. })
                )
            })],
            name,
        );
    });
}

#[test]
fn a_plan_for_a_model_that_changed_since_is_refused_at_commit() {
    let (name, version) = FIXTURES[1];
    let schema = ifc::schema::for_version(version).unwrap();
    let mut model = read(name);
    let options = SheetOptions::default();
    let mut sheet = exported(&model, schema, &options);
    set(&mut sheet, 17, "Name", "Wall A1");
    let plan = plan_import(&SelectorIndex::new(&model, schema), &sheet, &options).unwrap();
    ifc::selector::set_value(
        &mut model,
        schema,
        EntityId(18),
        &ValuePath::parse("Name").unwrap(),
        &ifc::Value::Text("Wall B1".into()),
    )
    .unwrap();
    let before = bytes(&model);
    let refused = plan.commit(&mut model).unwrap_err();
    assert!(matches!(
        &refused[..],
        [SheetRefusal {
            reason: SheetRefusalReason::Value(SelectorError::Conflict(_)),
            ..
        }]
    ));
    assert_eq!(bytes(&model), before);
}

#[test]
fn text_edits_write_every_selected_entity_in_one_transaction() {
    each_release(|name, _, mut model, schema| {
        let walls = walls(&model, schema);
        let path = ValuePath::parse("Description").unwrap();
        let edits: Vec<TextEdit> = walls
            .iter()
            .map(|id| TextEdit::new(*id, path.clone(), "Load-bearing"))
            .collect();
        let changes = apply_text_edits(&mut model, schema, &edits, "")
            .unwrap_or_else(|refused| panic!("{name}: {refused:?}"));
        assert_eq!(changes.len(), 3, "{name}");
        for id in [17, 18, 19] {
            assert_eq!(
                value(&model, schema, id, "Description"),
                text("Load-bearing")
            );
        }
        // Again: nothing to change.
        assert!(apply_text_edits(&mut model, schema, &edits, "")
            .unwrap()
            .is_empty());
        // One refused edit refuses them all.
        let before = bytes(&model);
        let mut edits: Vec<TextEdit> = walls
            .iter()
            .map(|id| TextEdit::new(*id, path.clone(), "Shear wall"))
            .collect();
        edits.push(TextEdit::new(
            EntityId(17),
            ValuePath::parse("Qto_WallBaseQuantities.Length").unwrap(),
            "long",
        ));
        let refused = apply_text_edits(&mut model, schema, &edits, "").unwrap_err();
        assert_eq!(refused.len(), 1, "{name}: {refused:?}");
        assert_eq!(refused[0].edit, Some(3));
        assert_eq!(bytes(&model), before, "{name}");
    });
}
