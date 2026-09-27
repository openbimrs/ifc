//! Reading `IfcTable` with its rows and columns.
//!
//! # WR1 and WR2 as the schema states them
//!
//! ```text
//! WR1 : SIZEOF(QUERY( Temp <* Rows |
//!         HIINDEX(Temp.RowCells) <> HIINDEX(Rows[1].RowCells))) = 0;
//! WR2 : { 0 <= NumberOfHeadings <= 1 };   -- rows with IsHeading TRUE
//! ```
//!
//! Both are identical in IFC4 ADD2 TC1 and IFC4X3 ADD2. `RowCells` and
//! `IsHeading` are OPTIONAL, and a comparison with an indeterminate value
//! is UNKNOWN, which `QUERY` does not select. So a row without cells is
//! never ragged, a table whose `Rows[1]` has no cells (or does not
//! resolve) has no WR1 reference width, and a row without `IsHeading` is
//! not a heading. The checks below follow that reading exactly rather
//! than a stricter one of their own.

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::Schema;

use super::decode::{Need, Slots};
use super::issue::TabularIssue;

pub(crate) const TABLE: &str = "IFCTABLE";
const ROW: &str = "IFCTABLEROW";
const COLUMN: &str = "IFCTABLECOLUMN";

/// A borrowed `IfcTable`, its resolved rows and columns, and every
/// defect found on the way.
#[derive(Debug, Clone)]
pub struct Table<'m> {
    id: EntityId,
    name: Option<&'m str>,
    rows: Vec<TableRow<'m>>,
    columns: Vec<TableColumn<'m>>,
    issues: Vec<TabularIssue>,
}

impl<'m> Table<'m> {
    /// The table's id.
    #[must_use]
    pub const fn id(&self) -> EntityId {
        self.id
    }

    /// `Name`.
    #[must_use]
    pub const fn name(&self) -> Option<&'m str> {
        self.name
    }

    /// Rows in `Rows` order that resolved to an `IfcTableRow`.
    ///
    /// A member that did not resolve is absent here and reported in
    /// [`Self::issues`].
    #[must_use]
    pub fn rows(&self) -> &[TableRow<'m>] {
        &self.rows
    }

    /// Columns in `Columns` order that resolved to an `IfcTableColumn`.
    #[must_use]
    pub fn columns(&self) -> &[TableColumn<'m>] {
        &self.columns
    }

    /// The row whose `IsHeading` is TRUE, when exactly one is.
    #[must_use]
    pub fn heading(&self) -> Option<&TableRow<'m>> {
        let mut headings = self.rows.iter().filter(|row| row.is_heading == Some(true));
        let first = headings.next();
        if headings.next().is_some() {
            None
        } else {
            first
        }
    }

    /// Rows that are not headings, in order.
    pub fn data_rows(&self) -> impl Iterator<Item = &TableRow<'m>> {
        self.rows.iter().filter(|row| row.is_heading != Some(true))
    }

    /// Every defect found: malformed slots, dangling or mistyped
    /// references, WR1 and WR2.
    #[must_use]
    pub fn issues(&self) -> &[TabularIssue] {
        &self.issues
    }
}

/// A borrowed `IfcTableRow`.
#[derive(Debug, Clone, Copy)]
pub struct TableRow<'m> {
    id: EntityId,
    cells: Option<&'m [Value]>,
    is_heading: Option<bool>,
}

impl<'m> TableRow<'m> {
    /// The row's id.
    #[must_use]
    pub const fn id(&self) -> EntityId {
        self.id
    }

    /// `RowCells` as written: each cell is an `IfcValue`, typed or bare.
    ///
    /// `None` when the slot is unset or malformed.
    #[must_use]
    pub const fn cells(&self) -> Option<&'m [Value]> {
        self.cells
    }

    /// `IsHeading`; `None` when unset or malformed.
    #[must_use]
    pub const fn is_heading(&self) -> Option<bool> {
        self.is_heading
    }
}

/// A borrowed `IfcTableColumn`.
#[derive(Debug, Clone, Copy)]
pub struct TableColumn<'m> {
    id: EntityId,
    identifier: Option<&'m str>,
    name: Option<&'m str>,
    description: Option<&'m str>,
    unit: Option<EntityId>,
    reference_path: Option<EntityId>,
}

impl<'m> TableColumn<'m> {
    /// The column's id.
    #[must_use]
    pub const fn id(&self) -> EntityId {
        self.id
    }

    /// `Identifier`.
    #[must_use]
    pub const fn identifier(&self) -> Option<&'m str> {
        self.identifier
    }

    /// `Name`.
    #[must_use]
    pub const fn name(&self) -> Option<&'m str> {
        self.name
    }

    /// `Description`.
    #[must_use]
    pub const fn description(&self) -> Option<&'m str> {
        self.description
    }

    /// `Unit`, an `IfcUnit` reference.
    #[must_use]
    pub const fn unit(&self) -> Option<EntityId> {
        self.unit
    }

    /// `ReferencePath`, an `IfcReference` reference.
    #[must_use]
    pub const fn reference_path(&self) -> Option<EntityId> {
        self.reference_path
    }
}

/// Read an `IfcTable` the caller has already type-checked.
pub(crate) fn read_table<'m>(
    model: &'m Model,
    schema: &'m Schema,
    id: EntityId,
    entity: &'m Entity,
) -> Table<'m> {
    let mut issues = Vec::new();
    let mut slots = Slots::open(model, schema, id, entity, TABLE, &mut issues);
    let name = slots.text("Name", Need::Optional);
    let row_refs = slots.records("Rows", Need::Optional, ROW);
    let column_refs = slots.records("Columns", Need::Optional, COLUMN);

    let rows: Vec<Option<TableRow<'m>>> = row_refs
        .into_iter()
        .map(|found| found.map(|(row, record)| read_row(model, schema, row, record, &mut issues)))
        .collect();
    let columns = column_refs
        .into_iter()
        .flatten()
        .map(|(column, record)| read_column(model, schema, column, record, &mut issues))
        .collect();

    check_wr1(id, &rows, &mut issues);
    check_wr2(id, &rows, &mut issues);

    Table {
        id,
        name,
        rows: rows.into_iter().flatten().collect(),
        columns,
        issues,
    }
}

fn read_row<'m>(
    model: &'m Model,
    schema: &'m Schema,
    id: EntityId,
    entity: &'m Entity,
    issues: &mut Vec<TabularIssue>,
) -> TableRow<'m> {
    let mut slots = Slots::open(model, schema, id, entity, ROW, issues);
    TableRow {
        id,
        cells: slots.values("RowCells", Need::Optional),
        is_heading: slots.boolean("IsHeading", Need::Optional),
    }
}

fn read_column<'m>(
    model: &'m Model,
    schema: &'m Schema,
    id: EntityId,
    entity: &'m Entity,
    issues: &mut Vec<TabularIssue>,
) -> TableColumn<'m> {
    let mut slots = Slots::open(model, schema, id, entity, COLUMN, issues);
    TableColumn {
        id,
        identifier: slots.text("Identifier", Need::Optional),
        name: slots.text("Name", Need::Optional),
        description: slots.text("Description", Need::Optional),
        unit: slots.reference("Unit", Need::Optional),
        reference_path: slots.reference("ReferencePath", Need::Optional),
    }
}

/// WR1, measured against `Rows[1]`: the first list position, resolved.
fn check_wr1(table: EntityId, rows: &[Option<TableRow<'_>>], issues: &mut Vec<TabularIssue>) {
    let Some(expected) = rows
        .first()
        .and_then(|row| row.as_ref())
        .and_then(|row| row.cells)
        .map(<[Value]>::len)
    else {
        return;
    };
    for (index, row) in rows.iter().enumerate() {
        let Some(row) = row else { continue };
        let Some(found) = row.cells.map(<[Value]>::len) else {
            continue;
        };
        if found != expected {
            issues.push(TabularIssue::RaggedRow {
                table,
                row: row.id,
                index,
                expected,
                found,
            });
        }
    }
}

/// WR2: at most one row whose `IsHeading` is TRUE.
fn check_wr2(table: EntityId, rows: &[Option<TableRow<'_>>], issues: &mut Vec<TabularIssue>) {
    let found = rows
        .iter()
        .flatten()
        .filter(|row| row.is_heading == Some(true))
        .count();
    if found > 1 {
        issues.push(TabularIssue::TooManyHeadings { table, found });
    }
}
