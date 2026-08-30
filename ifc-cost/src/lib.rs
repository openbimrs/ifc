//! `ifc-cost` — cost semantics as a **view** over the model.
//!
//! # This crate owns no data
//!
//! It borrows a `&Model` and interprets entities that happen to be cost
//! entities. That is the whole design: the model stores structure, this crate
//! supplies meaning, and the two are separable.
//!
//! The consequences are the point:
//!
//! - **Optional.** An application that never touches cost compiles none of
//!   this. It is a cargo feature of the `ifc` facade crate.
//! - **Non-destructive.** Because the model holds the same entities with or
//!   without this crate, a file with cost data round-trips identically in
//!   either build. Verified in `ifc/tests/costing_roundtrip.rs`, which runs
//!   with no domain crate compiled at all.
//! - **Replaceable.** A different interpretation of the same entities is
//!   another crate, not a fork of the model.
//!
//! # Modules
//!
//! | Module | Role |
//! | --- | --- |
//! | [`view`] | [`CostView`]: the entry point, borrows a `&Model` |
//! | [`item`] | `IfcCostItem`: the individual line |
//! | [`schedule`] | `IfcCostSchedule`: the containing document |
//! | [`value`] | `IfcCostValue` and applied monetary values |
//! | [`quantity`] | Quantities a cost is computed against |
//! | [`relation`] | Nesting and control assignment |
//! | [`currency`] | Monetary unit agreement |
//! | [`rollup`] | Summing a cost tree |
//! | [`error`] | Why a cost lookup failed |
//!
//! ```
//! use ifc_cost::CostView;
//! use ifc_model::{Entity, EntityId, Model, Value};
//!
//! let mut model = Model::new();
//! model.insert(
//!     EntityId(1),
//!     Entity::new("IFCCOSTITEM", vec![
//!         Value::Text("guid".into()), Value::Null, Value::Text("Excavation".into()),
//!     ]),
//! );
//!
//! let view = CostView::new(&model);
//! assert_eq!(view.items().next().unwrap().name(), Some("Excavation"));
//! ```

pub mod currency;
pub mod error;
pub mod item;
pub mod quantity;
pub mod relation;
pub mod rollup;
pub mod schedule;
pub mod value;
pub mod view;

pub use currency::{monetary_units, project_currency, CurrencyError};
pub use error::CostError;
pub use item::CostItem;
pub use quantity::CostQuantity;
pub use relation::{
    children_of, controlled_by, controls_of, descendants_of, parent_of, parents_of,
    CostRelationError, MAX_NESTING_DEPTH,
};
pub use rollup::{consistency, direct_total, grand_total, rolled_up_total, roots, Consistency};
pub use schedule::CostSchedule;
pub use value::{ArithmeticOperator, CostValue, UnitBasis};
pub use view::CostView;
