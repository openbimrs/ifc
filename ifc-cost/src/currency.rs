//! Currency and dimensional agreement for cost arithmetic.
//!
//! # Why this exists
//!
//! `IfcMonetaryUnit` states a currency as a plain string (`"EUR"`, `"GBP"`).
//! Nothing in the schema stops a file carrying cost values in two currencies,
//! and nothing marks which one a given `IfcCostValue` is in -- the currency
//! lives on the project's `UnitsInContext`, not on the value.
//!
//! So summing two cost values is only meaningful when the file states exactly
//! one monetary unit. When it states several, this crate reports the ambiguity
//! instead of adding numbers that are not comparable. A silent sum across EUR
//! and GBP is a wrong total that looks right, which is the worst failure mode
//! a cost tool has.
//!
//! # Slots, verified against IFC4 EXPRESS
//!
//! ```text
//! IfcMonetaryUnit    0 Currency          (IfcLabel, not an enum in IFC4)
//! IfcProject         8 UnitsInContext -> IfcUnitAssignment
//! IfcUnitAssignment  0 Units             SET of unit entities
//! ```

use ifc_model::{EntityId, Model, Value};

/// Why a monetary total could not be trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrencyError {
    /// The file states no monetary unit at all.
    ///
    /// Amounts are still readable; they simply have no stated currency, so a
    /// total is a bare number rather than money.
    Unstated,
    /// The file states more than one currency.
    ///
    /// Every distinct currency found, sorted for determinism. Summing across
    /// them requires an exchange rate this crate does not have and will not
    /// invent.
    Ambiguous {
        /// The currencies found, sorted.
        currencies: Vec<String>,
    },
}

/// The single currency this file's costs are expressed in.
///
/// # Errors
///
/// [`CurrencyError::Unstated`] when no `IfcMonetaryUnit` exists, or
/// [`CurrencyError::Ambiguous`] when the file states more than one.
pub fn project_currency(model: &Model) -> Result<String, CurrencyError> {
    let mut found: Vec<String> = Vec::new();
    for (_, entity) in model.of_type("IFCMONETARYUNIT") {
        // IFC4 relaxed Currency from IfcCurrencyEnum to IfcLabel, so it is a
        // quoted string. Accept an enum token too: IFC2X3 files converted
        // forward sometimes retain the older form.
        let currency = match entity.attribute(0) {
            Some(Value::Text(text)) => Some(text.to_string()),
            Some(Value::Enum(token)) => Some(token.to_string()),
            _ => None,
        };
        if let Some(currency) = currency {
            if !found.contains(&currency) {
                found.push(currency);
            }
        }
    }

    match found.len() {
        0 => Err(CurrencyError::Unstated),
        1 => Ok(found.remove(0)),
        _ => {
            found.sort();
            Err(CurrencyError::Ambiguous { currencies: found })
        }
    }
}

/// Every `IfcMonetaryUnit` in the file, in file order.
#[must_use]
pub fn monetary_units(model: &Model) -> Vec<EntityId> {
    model.ids_of_type("IFCMONETARYUNIT").to_vec()
}
