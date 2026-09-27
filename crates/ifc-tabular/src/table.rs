//! `IfcTable`, `IfcTableRow` and `IfcTableColumn`.

use ifc_model::{Entity, EntityId, Transaction, Value};

use crate::error::{TabularError, TabularResult};

const TABLE: &str = "IFCTABLE";
const ROW: &str = "IFCTABLEROW";
const COLUMN: &str = "IFCTABLECOLUMN";

/// Stage an `IfcTableRow`.
///
/// `cells` are `IfcValue`s: pass `Value::Typed { .. }` to declare a measure,
/// or a bare literal to leave the value dimensionless. This crate does not
/// choose for you.
///
/// # Errors
///
/// Refuses an empty cell list: `RowCells` is `LIST [1:?]`.
pub fn add_table_row(
    tx: &mut Transaction,
    cells: Vec<Value>,
    is_heading: bool,
) -> TabularResult<EntityId> {
    if cells.is_empty() {
        return Err(TabularError::EmptyList {
            entity: ROW,
            attribute: "RowCells",
        });
    }
    let attributes = vec![Value::List(cells), Value::Bool(is_heading)];
    Ok(tx.create(Entity::new(ROW, attributes)))
}

/// Attributes of an `IfcTableColumn`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ColumnDraft<'a> {
    /// `Identifier`.
    pub identifier: Option<&'a str>,
    /// `Name`.
    pub name: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `Unit`, an `IfcUnit` reference.
    pub unit: Option<EntityId>,
    /// `ReferencePath`, an `IfcReference` reference.
    ///
    /// Taken as an id rather than modelled here: `IfcReference` is a
    /// property-path concept this crate does not own.
    pub reference_path: Option<EntityId>,
}

fn optional_text(value: Option<&str>) -> Value {
    value.map_or(Value::Null, |text| Value::Text(text.into()))
}

/// Stage an `IfcTableColumn`.
///
/// Every attribute is OPTIONAL in the schema, so a column carrying nothing
/// is legal. A blank identifier is not: it would name a column that cannot
/// be referenced.
///
/// # Errors
///
/// Refuses a whitespace-only `Identifier`.
pub fn add_table_column(tx: &mut Transaction, draft: ColumnDraft<'_>) -> TabularResult<EntityId> {
    if draft.identifier.is_some_and(|id| id.trim().is_empty()) {
        return Err(TabularError::BlankRequired {
            entity: COLUMN,
            attribute: "Identifier",
        });
    }
    let attributes = vec![
        optional_text(draft.identifier),
        optional_text(draft.name),
        optional_text(draft.description),
        draft.unit.map_or(Value::Null, Value::Ref),
        draft.reference_path.map_or(Value::Null, Value::Ref),
    ];
    Ok(tx.create(Entity::new(COLUMN, attributes)))
}

/// One staged row: its id, its width, and whether it is the heading.
///
/// The width travels with the id because a staged entity cannot be read
/// back out of a `Transaction`, and WR1 is stated over cell counts. Taking
/// the count from the caller keeps the rule checkable before commit rather
/// than deferring it to a validator that runs later, or never.
pub type StagedRow = (EntityId, usize, bool);

/// Stage an `IfcTable`.
///
/// # Errors
///
/// Refuses a ragged table (WR1) and more than one heading row (WR2).
///
/// # Derived attributes
///
/// `NumberOfCellsInRow`, `NumberOfHeadings` and `NumberOfDataRows` are
/// DERIVEd under new names rather than redeclaring an inherited attribute,
/// so they occupy no instance slots and are not written at all. Contrast
/// `IfcSIUnit`, whose DERIVE redeclares `SELF\IfcNamedUnit.Dimensions` and
/// therefore does take a slot, written as `*`.
pub fn add_table(
    tx: &mut Transaction,
    name: Option<&str>,
    rows: &[StagedRow],
    columns: &[EntityId],
) -> TabularResult<EntityId> {
    if let Some((_, first_width, _)) = rows.first() {
        for (index, (_, width, _)) in rows.iter().enumerate() {
            if width != first_width {
                return Err(TabularError::RaggedRow {
                    entity: TABLE,
                    row: index,
                    expected: *first_width,
                    found: *width,
                });
            }
        }
    }
    let headings = rows.iter().filter(|(_, _, heading)| *heading).count();
    if headings > 1 {
        return Err(TabularError::TooManyHeadings {
            entity: TABLE,
            found: headings,
        });
    }
    let refs = |ids: &[EntityId]| {
        if ids.is_empty() {
            Value::Null
        } else {
            Value::List(ids.iter().copied().map(Value::Ref).collect())
        }
    };
    let row_ids: Vec<EntityId> = rows.iter().map(|(id, _, _)| *id).collect();
    let attributes = vec![optional_text(name), refs(&row_ids), refs(columns)];
    Ok(tx.create(Entity::new(TABLE, attributes)))
}
