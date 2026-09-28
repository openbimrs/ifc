//! Which form a value takes in its slot: a typed parameter or a bare value.
//!
//! # What ISO 10303-21 requires
//!
//! Part 21 does not leave the form to the writer. It follows from the
//! attribute's declared type alone:
//!
//! - **Not a SELECT: bare.** §12.1.6 maps a value of a simple defined type
//!   as a value of the type it is defined over, and §12.1.7 maps an
//!   enumeration value as the enumeration constant itself.
//!   `IfcQuantityArea.AreaValue : IfcAreaMeasure` holds `12.5`, never
//!   `IFCAREAMEASURE(12.5)`.
//! - **A SELECT: typed, except for an entity.** §12.1.8 maps an entity value
//!   as an instance name, and a value of a simple defined type or enumeration
//!   as a typed parameter whose keyword names that type, which must be listed
//!   by the SELECT or a SELECT nested in it. This holds for every SELECT, not only the
//!   ambiguous ones: a SELECT listing a single defined type still takes
//!   `TYPE(value)`.
//!
//! A type aliasing a SELECT is encoded as that SELECT (§12.1.8, EXAMPLE 2), so
//! "is the declared type a SELECT" follows defined-type aliases; see
//! [`super::select::resolve_select`]. The same rules apply to each member of
//! an aggregate against the element type, and to the parameter inside a typed
//! wrapper against the wrapper's own type.
//!
//! # What "the right type" means outside a SELECT
//!
//! A wrapper written where a bare value belongs still names a type. When that
//! type is the declared type, or a defined type whose alias chain reaches it
//! (`IfcPositiveLengthMeasure = IfcLengthMeasure`), the value is of the right
//! type in the wrong form. Anything else -- `IFCLABEL('x')` in an area slot,
//! `IFCLENGTHMEASURE(1.)` where `IfcPositiveLengthMeasure` is declared, a
//! keyword the schema does not declare -- is the wrong type as well.
//!
//! # Rule ids and severity
//!
//! | Rule id | Finding | Severity |
//! |---|---|---|
//! | `type.typed.outside_select` | right type, typed where bare is required | error |
//! | `type.typed.wrong_type` | typed with another type, where bare is required | error |
//! | `type.select.untyped` | bare, where a SELECT requires the typed form | error |
//!
//! The first is the one that invites a warning: the value is unambiguous
//! and many readers unwrap it. It is an error all the same, deliberately.
//! [`crate::Severity::Warning`] means the file is *legal* but suspicious, and
//! this one is not legal: §12.1.6 requires the bare value.
//! IfcOpenShell's `ifcopenshell.validate`, whose verdicts name the
//! `ifcopenshell-validate` fixtures, treats a wrapped value in a slot
//! declared as a defined type or enumeration as invalid, and a bare value
//! in a SELECT slot likewise; a warning would call conformant a file the
//! reference validator rejects. The finding has its own id, so a caller that
//! tolerates the form filters that id without losing
//! `type.typed.wrong_type`.

use ifc_schema::{Schema, TypeKind};

use super::scalar::{primitive_of, Primitive};

/// How many alias hops are followed from a wrapper's type.
///
/// IFC alias chains are two or three long; this only stops a malformed
/// cyclic table from looping.
const MAX_ALIAS_HOPS: usize = 16;

/// Whether a typed parameter naming `wrapper` holds a value of the
/// non-SELECT type `declared`.
///
/// `None` when `declared` gives no basis to judge: a token the schema does
/// not declare and that names no EXPRESS primitive, such as the `LIST` the
/// tables record for a nested aggregate's element.
#[must_use]
pub fn wrapper_fits(schema: &Schema, wrapper: &str, declared: &str) -> Option<bool> {
    if schema.type_def(declared).is_some() || schema.entity(declared).is_some() {
        return Some(aliases_to(schema, wrapper, declared));
    }
    // A slot declared directly as an EXPRESS primitive (`LIST OF INTEGER`)
    // names no defined type; the wrapper fits when its own type bottoms out
    // in that primitive. INTEGER is a specialisation of REAL in EXPRESS.
    let expected = Primitive::from_resolved(declared)?;
    let Some(actual) = primitive_of(schema, wrapper).filter(|_| is_declared(schema, wrapper))
    else {
        return Some(false);
    };
    Some(actual == expected || (expected == Primitive::Real && actual == Primitive::Integer))
}

/// Whether `wrapper` is `declared`, or aliases it through defined types.
fn aliases_to(schema: &Schema, wrapper: &str, declared: &str) -> bool {
    let mut current = wrapper.trim().to_string();
    for _ in 0..MAX_ALIAS_HOPS {
        if current.eq_ignore_ascii_case(declared) {
            return true;
        }
        let Some(TypeKind::Defined(target)) =
            schema.type_def(&current).map(|definition| &definition.kind)
        else {
            return false;
        };
        current = target.trim().to_string();
    }
    false
}

/// Whether the schema declares `name` as a type at all.
fn is_declared(schema: &Schema, name: &str) -> bool {
    schema.type_def(name).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::type_check::{check_value, Mismatch};
    use ifc_model::Value;

    /// A schema shaped like the one ISO 10303-21:2016 §12.1.8 uses to show
    /// nested SELECTs and a defined type aliasing a SELECT (`Computed_Load`),
    /// plus an attribute declared as that alias.
    const ALIASED_SELECT: &str = "\
SCHEMA ALIASED_SELECT;
TYPE Load = SELECT (Load_Choice, Other_Load); END_TYPE;
TYPE Load_Choice = SELECT (Rated_Load, Computed_Load); END_TYPE;
TYPE Other_Load = SELECT (Guessed_Load); END_TYPE;
TYPE Rated_Load = REAL; END_TYPE;
TYPE Guessed_Load = REAL; END_TYPE;
TYPE Computed_Load = Number_Or_Flag; END_TYPE;
TYPE Number_Or_Flag = SELECT (Plain_Number, Flag); END_TYPE;
TYPE Plain_Number = REAL; END_TYPE;
TYPE Flag = ENUMERATION OF (unbounded, unknown); END_TYPE;
ENTITY Beam;
  span : Number_Or_Flag;
  load : Load;
  computed : Computed_Load;
END_ENTITY;
END_SCHEMA;
";

    fn wrap(type_name: &str, value: Value) -> Value {
        Value::Typed {
            type_name: type_name.into(),
            value: Box::new(value),
        }
    }

    /// The forms §12.1.8 prescribes are accepted and the others are not.
    /// `Computed_Load` aliases the SELECT `Number_Or_Flag`, so its value is
    /// typed: the alias is followed, not taken for a simple defined type.
    #[test]
    fn nested_and_aliased_selects_are_judged_as_part_21_writes_them() {
        let schema = Schema::from_express(ALIASED_SELECT);
        let number = || wrap("PLAIN_NUMBER", Value::Real(7.5));
        assert_eq!(check_value(&schema, "Number_Or_Flag", &number()), None);
        let flag = wrap("FLAG", Value::Enum("UNKNOWN".into()));
        assert_eq!(check_value(&schema, "Number_Or_Flag", &flag), None);
        // Two SELECTs down: Load -> Load_Choice -> Rated_Load.
        let rated = wrap("RATED_LOAD", Value::Real(3.0));
        assert_eq!(check_value(&schema, "Load", &rated), None);
        // COMPUTED_LOAD(PLAIN_NUMBER(1.5)): the alias's parameter is typed.
        let computed = wrap("COMPUTED_LOAD", number());
        assert_eq!(check_value(&schema, "Load", &computed), None);

        // An attribute declared as the alias is a SELECT slot too.
        assert_eq!(check_value(&schema, "Computed_Load", &number()), None);
        assert!(matches!(
            check_value(&schema, "Computed_Load", &Value::Real(1.0)),
            Some(Mismatch::UntypedSelectValue { .. })
        ));
        assert!(matches!(
            check_value(&schema, "Load", &wrap("COMPUTED_LOAD", Value::Real(1.5))),
            Some(Mismatch::UntypedSelectValue { .. })
        ));
        // PLAIN_NUMBER is reachable from Load only inside COMPUTED_LOAD,
        // never as a keyword of Load itself.
        assert!(matches!(
            check_value(&schema, "Load", &number()),
            Some(Mismatch::SelectMember { .. })
        ));
        // A simple defined type takes the bare value, and a wrapper there is
        // the wrong form.
        assert_eq!(check_value(&schema, "Rated_Load", &Value::Real(1.0)), None);
        assert!(matches!(
            check_value(&schema, "Rated_Load", &wrap("RATED_LOAD", Value::Real(1.0))),
            Some(Mismatch::TypedOutsideSelect { .. })
        ));
    }

    #[test]
    fn the_declared_type_and_its_specialisations_fit() {
        let schema = ifc_schema::ifc4();
        assert_eq!(
            wrapper_fits(schema, "IFCAREAMEASURE", "IfcAreaMeasure"),
            Some(true)
        );
        // IfcPositiveLengthMeasure = IfcLengthMeasure: every positive length
        // is a length.
        assert_eq!(
            wrapper_fits(schema, "IFCPOSITIVELENGTHMEASURE", "IfcLengthMeasure"),
            Some(true)
        );
        assert_eq!(
            wrapper_fits(schema, "IFCWALLTYPEENUM", "IfcWallTypeEnum"),
            Some(true)
        );
    }

    #[test]
    fn another_type_does_not_fit() {
        let schema = ifc_schema::ifc4();
        assert_eq!(
            wrapper_fits(schema, "IFCLABEL", "IfcAreaMeasure"),
            Some(false)
        );
        // Same primitive, different quantity.
        assert_eq!(
            wrapper_fits(schema, "IFCLENGTHMEASURE", "IfcAreaMeasure"),
            Some(false)
        );
        // The generalisation is not the specialisation: a length is not
        // known to be positive.
        assert_eq!(
            wrapper_fits(schema, "IFCLENGTHMEASURE", "IfcPositiveLengthMeasure"),
            Some(false)
        );
        assert_eq!(
            wrapper_fits(schema, "IFCNOSUCHMEASURE", "IfcAreaMeasure"),
            Some(false)
        );
    }

    #[test]
    fn a_primitive_slot_is_judged_by_primitive() {
        let schema = ifc_schema::ifc4();
        assert_eq!(wrapper_fits(schema, "IFCINTEGER", "INTEGER"), Some(true));
        assert_eq!(wrapper_fits(schema, "IFCINTEGER", "REAL"), Some(true));
        assert_eq!(wrapper_fits(schema, "IFCLABEL", "REAL"), Some(false));
        assert_eq!(wrapper_fits(schema, "IFCNOSUCHTYPE", "REAL"), Some(false));
        // No basis: the element of a nested aggregate.
        assert_eq!(wrapper_fits(schema, "IFCLENGTHMEASURE", "LIST"), None);
    }
}
