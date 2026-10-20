//! Values during evaluation (ISO 10303-11 clause 8).
//!
//! Aggregates have value semantics: they are shared behind an `Rc` and
//! copied on write. Entity instances are references: a file instance by its
//! id, an instance a function builds with an entity constructor by a shared
//! cell, so `U[2].DirectionRatios[1] := ...` changes the instance every
//! holder sees, as EXPRESS requires. File instances are never written.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use ifc_model::EntityId;

use super::ast::{AggKind, Logical, Name};

/// One value.
#[derive(Debug, Clone)]
pub(crate) enum Val {
    /// `?`.
    Indet,
    Logical(Logical),
    Int(i64),
    Real(f64),
    Str(Arc<str>),
    Binary(Arc<str>),
    /// An enumeration item, upper-cased.
    Enum(Arc<str>),
    Inst(Inst),
    Agg(Rc<Agg>),
    /// A value of a defined, enumeration or select-member type, by type
    /// index: what `TYPEOF` reports, wrapping the underlying value.
    Typed(Rc<(usize, Val)>),
}

/// An entity instance.
#[derive(Debug, Clone)]
pub(crate) enum Inst {
    /// An instance in the file.
    Model(EntityId),
    /// An instance built by an entity constructor during evaluation.
    Local(Rc<RefCell<LocalInst>>),
}

/// An instance an entity constructor built.
#[derive(Debug, Clone)]
pub(crate) struct LocalInst {
    /// The most specific entity, by index.
    pub entity: usize,
    /// Explicit attribute values by upper-cased name.
    pub attrs: HashMap<Name, Val>,
}

/// An aggregate value.
#[derive(Debug, Clone)]
pub(crate) struct Agg {
    pub kind: AggKind,
    /// The index of the first element: 1, or an `ARRAY`'s declared low
    /// bound.
    pub lower: i64,
    pub items: Vec<Val>,
}

impl Agg {
    pub(crate) fn new(kind: AggKind, items: Vec<Val>) -> Self {
        Self {
            kind,
            lower: 1,
            items,
        }
    }
}

impl Val {
    pub(crate) fn logical(value: bool) -> Self {
        Self::Logical(if value { Logical::True } else { Logical::False })
    }

    pub(crate) fn agg(kind: AggKind, items: Vec<Val>) -> Self {
        Self::Agg(Rc::new(Agg::new(kind, items)))
    }

    pub(crate) fn is_indet(&self) -> bool {
        match self {
            Self::Indet => true,
            Self::Typed(typed) => typed.1.is_indet(),
            _ => false,
        }
    }

    /// The value with every defined-type wrapper removed.
    pub(crate) fn bare(&self) -> &Val {
        let mut current = self;
        while let Self::Typed(typed) = current {
            current = &typed.1;
        }
        current
    }

    /// A short description for messages.
    pub(crate) fn kind_name(&self) -> &'static str {
        match self.bare() {
            Self::Indet => "?",
            Self::Logical(_) => "LOGICAL",
            Self::Int(_) => "INTEGER",
            Self::Real(_) => "REAL",
            Self::Str(_) => "STRING",
            Self::Binary(_) => "BINARY",
            Self::Enum(_) => "an enumeration item",
            Self::Inst(_) => "an entity instance",
            Self::Agg(agg) => agg.kind.keyword(),
            Self::Typed(_) => unreachable!("bare() removes wrappers"),
        }
    }
}

impl Logical {
    pub(crate) fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }

    pub(crate) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    pub(crate) fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }

    pub(crate) fn xor(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (a, b) => {
                if a == b {
                    Self::False
                } else {
                    Self::True
                }
            }
        }
    }

    /// FALSE < UNKNOWN < TRUE.
    pub(crate) fn rank(self) -> u8 {
        match self {
            Self::False => 0,
            Self::Unknown => 1,
            Self::True => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ISO 10303-11 Tables 13-15.
    #[test]
    fn three_valued_logic_follows_the_standard_tables() {
        use Logical::{False as F, True as T, Unknown as U};
        assert_eq!(U.and(F), F);
        assert_eq!(U.and(T), U);
        assert_eq!(U.or(T), T);
        assert_eq!(U.or(F), U);
        assert_eq!(U.not(), U);
        assert_eq!(T.xor(F), T);
        assert_eq!(T.xor(T), F);
        assert_eq!(U.xor(F), U);
    }

    #[test]
    fn a_wrapped_indeterminate_is_indeterminate() {
        let wrapped = Val::Typed(Rc::new((0, Val::Indet)));
        assert!(wrapped.is_indet());
        assert!(!Val::Int(0).is_indet());
    }
}
