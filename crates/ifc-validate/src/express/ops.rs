//! Operators: arithmetic, comparison, membership and aggregate algebra
//! (ISO 10303-11 12.1-12.6).

use std::cmp::Ordering;
use std::rc::Rc;

use super::ast::{AggKind, BinOp, Logical};
use super::eval::overflow;
use super::machine::{Fault, Machine, R};
use super::value::{Agg, Inst, Val};

/// Comparisons deeper than this between entity values give UNKNOWN rather
/// than recurse further; entity graphs may be cyclic.
const MAX_VALUE_DEPTH: usize = 16;

impl Machine<'_> {
    pub(crate) fn binary(&self, op: BinOp, left: &Val, right: &Val) -> R<Val> {
        match op {
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.compare(op, left, right)
            }
            BinOp::InstEq => Ok(Val::Logical(self.instance_eq(left, right)?)),
            BinOp::InstNe => Ok(Val::Logical(self.instance_eq(left, right)?.not())),
            BinOp::In => Ok(Val::Logical(self.member(left, right)?)),
            BinOp::Xor => {
                let l = super::eval::logical(left)?;
                let r = super::eval::logical(right)?;
                Ok(Val::Logical(l.xor(r)))
            }
            BinOp::And | BinOp::Or | BinOp::Complex => {
                unreachable!("evaluated before reaching binary()")
            }
            BinOp::Like => Err(Fault::Unsupported("the LIKE operator".into())),
            BinOp::Add | BinOp::Sub | BinOp::Mul
                if matches!(left.bare(), Val::Agg(_)) || matches!(right.bare(), Val::Agg(_)) =>
            {
                self.aggregate_op(op, left, right)
            }
            BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::IntDiv
            | BinOp::Mod
            | BinOp::Pow => arithmetic(op, left, right),
        }
    }

    /// `=`, `<>`, `<`, `<=`, `>`, `>=`: a LOGICAL, UNKNOWN with `?`.
    pub(crate) fn compare(&self, op: BinOp, left: &Val, right: &Val) -> R<Val> {
        if left.is_indet() || right.is_indet() {
            return Ok(Val::Logical(Logical::Unknown));
        }
        if matches!(op, BinOp::Eq | BinOp::Ne) {
            let eq = self.value_eq(left, right, 0)?;
            return Ok(Val::Logical(if op == BinOp::Eq { eq } else { eq.not() }));
        }
        if let (Val::Agg(a), Val::Agg(b)) = (left.bare(), right.bare()) {
            // Subset and superset (12.6.4).
            let (small, large) = match op {
                BinOp::Le => (a, b),
                BinOp::Ge => (b, a),
                _ => return Err(Fault::Data("`<` or `>` between aggregates".into())),
            };
            let mut all = Logical::True;
            for item in &small.items {
                all = all.and(self.member(item, &Val::Agg(large.clone()))?);
            }
            return Ok(Val::Logical(all));
        }
        let ordering = order(left.bare(), right.bare())?;
        let holds = match op {
            BinOp::Lt => ordering == Ordering::Less,
            BinOp::Le => ordering != Ordering::Greater,
            BinOp::Gt => ordering == Ordering::Greater,
            _ => ordering != Ordering::Less,
        };
        Ok(Val::logical(holds))
    }

    /// Value equality (12.2.1): numbers by value, entity instances by
    /// identity or attribute-wise, aggregates element-wise.
    pub(crate) fn value_eq(&self, left: &Val, right: &Val, depth: usize) -> R<Logical> {
        self.step()?;
        let (left, right) = (left.bare(), right.bare());
        Ok(match (left, right) {
            (Val::Indet, _) | (_, Val::Indet) => Logical::Unknown,
            (Val::Int(a), Val::Int(b)) => truth(a == b),
            (Val::Int(_) | Val::Real(_), Val::Int(_) | Val::Real(_)) => {
                truth(number(left) == number(right))
            }
            (Val::Str(a), Val::Str(b)) | (Val::Binary(a), Val::Binary(b)) => truth(a == b),
            (Val::Enum(a), Val::Enum(b)) => truth(a.eq_ignore_ascii_case(b)),
            (Val::Logical(a), Val::Logical(b)) => truth(a == b),
            (Val::Inst(a), Val::Inst(b)) => {
                if same_instance(a, b) {
                    Logical::True
                } else if depth >= MAX_VALUE_DEPTH {
                    Logical::Unknown
                } else {
                    self.entity_value_eq(a, b, depth)?
                }
            }
            (Val::Agg(a), Val::Agg(b)) => {
                if a.items.len() != b.items.len() {
                    return Ok(Logical::False);
                }
                let ordered = !matches!(a.kind, AggKind::Set | AggKind::Bag)
                    && !matches!(b.kind, AggKind::Set | AggKind::Bag);
                let mut all = Logical::True;
                if ordered {
                    for (x, y) in a.items.iter().zip(&b.items) {
                        all = all.and(self.value_eq(x, y, depth + 1)?);
                        if all == Logical::False {
                            break;
                        }
                    }
                } else {
                    for x in &a.items {
                        let mut found = Logical::False;
                        for y in &b.items {
                            found = found.or(self.value_eq(x, y, depth + 1)?);
                            if found == Logical::True {
                                break;
                            }
                        }
                        all = all.and(found);
                    }
                }
                all
            }
            _ => Logical::False,
        })
    }

    /// Two entity instances of one type whose attributes are value-equal.
    fn entity_value_eq(&self, a: &Inst, b: &Inst, depth: usize) -> R<Logical> {
        let (ea, eb) = (self.entity_of(a)?, self.entity_of(b)?);
        if ea != eb {
            return Ok(Logical::False);
        }
        let mut all = Logical::True;
        for &(owner, idx) in &self.tables.layout[ea] {
            let name = &self.tables.schema.entities[owner].explicit[idx].name;
            let x = self.attribute(&Val::Inst(a.clone()), name)?;
            let y = self.attribute(&Val::Inst(b.clone()), name)?;
            all = all.and(if x.is_indet() && y.is_indet() {
                Logical::True
            } else {
                self.value_eq(&x, &y, depth + 1)?
            });
            if all == Logical::False {
                break;
            }
        }
        Ok(all)
    }

    /// Instance equality (12.2.2): identity for instances, value equality
    /// for everything else.
    pub(crate) fn instance_eq(&self, left: &Val, right: &Val) -> R<Logical> {
        match (left.bare(), right.bare()) {
            (Val::Inst(a), Val::Inst(b)) => Ok(truth(same_instance(a, b))),
            (Val::Agg(a), Val::Agg(b)) => {
                if a.items.len() != b.items.len() {
                    return Ok(Logical::False);
                }
                let mut all = Logical::True;
                for (x, y) in a.items.iter().zip(&b.items) {
                    all = all.and(self.instance_eq(x, y)?);
                }
                Ok(all)
            }
            (Val::Inst(_), _) | (_, Val::Inst(_)) => {
                if left.is_indet() || right.is_indet() {
                    Ok(Logical::Unknown)
                } else {
                    Ok(Logical::False)
                }
            }
            _ => self.value_eq(left, right, 0),
        }
    }

    /// `item IN aggregate` (12.2.3), by instance equality.
    pub(crate) fn member(&self, item: &Val, aggregate: &Val) -> R<Logical> {
        if item.is_indet() || aggregate.is_indet() {
            return Ok(Logical::Unknown);
        }
        let Val::Agg(agg) = aggregate.bare() else {
            return Err(Fault::Data(format!(
                "IN over {}, not an aggregate",
                aggregate.kind_name()
            )));
        };
        self.contains(&agg.items, item)
    }

    /// Whether `items` holds `item` (TRUE only), by instance equality.
    pub(crate) fn holds(&self, items: &[Val], item: &Val) -> R<bool> {
        Ok(self.contains(items, item)? == Logical::True)
    }

    /// `items` with repeated members dropped, by instance equality, as a
    /// SET holds them. Instances, integers, strings and enumeration items
    /// are compared by hashing; anything else pairwise, each comparison a
    /// step.
    pub(crate) fn dedupe(&self, items: &[Val]) -> R<Vec<Val>> {
        let keys: Option<Vec<Key>> = items.iter().map(key).collect();
        if let Some(keys) = keys {
            let mut seen = std::collections::HashSet::with_capacity(keys.len());
            return Ok(items
                .iter()
                .zip(keys)
                .filter(|(_, key)| seen.insert(key.clone()))
                .map(|(item, _)| item.clone())
                .collect());
        }
        let mut out: Vec<Val> = Vec::with_capacity(items.len());
        for item in items {
            if !self.holds(&out, item)? {
                out.push(item.clone());
            }
        }
        Ok(out)
    }

    /// Whether `items` holds `item`, by instance equality.
    fn contains(&self, items: &[Val], item: &Val) -> R<Logical> {
        let mut found = Logical::False;
        for candidate in items {
            self.step()?;
            found = found.or(self.instance_eq(item, candidate)?);
            if found == Logical::True {
                break;
            }
        }
        Ok(found)
    }

    /// `+` (union), `-` (difference) and `*` (intersection) with an
    /// aggregate operand (12.6).
    fn aggregate_op(&self, op: BinOp, left: &Val, right: &Val) -> R<Val> {
        if left.is_indet() || right.is_indet() {
            return Ok(Val::Indet);
        }
        let as_items = |v: &Val| match v.bare() {
            Val::Agg(agg) => (Some(agg.kind), agg.items.clone()),
            other => (None, vec![other.clone()]),
        };
        let (lk, litems) = as_items(left);
        let (rk, ritems) = as_items(right);
        let set = lk == Some(AggKind::Set) || (lk.is_none() && rk == Some(AggKind::Set));
        let kind = lk.or(rk).unwrap_or(AggKind::Bag);
        let mut out = Vec::new();
        match op {
            BinOp::Add => {
                out = litems;
                out.extend(ritems);
                if set {
                    out = self.dedupe(&out)?;
                }
            }
            BinOp::Sub => {
                let mut remove = ritems;
                for item in litems {
                    let hit = remove
                        .iter()
                        .position(|r| matches!(self.instance_eq(&item, r), Ok(Logical::True)));
                    match hit {
                        Some(at) if !set => {
                            remove.remove(at);
                        }
                        Some(_) => {}
                        None => out.push(item),
                    }
                }
            }
            _ => {
                for item in litems {
                    if self.contains(&ritems, &item)? == Logical::True {
                        out.push(item);
                    }
                }
            }
        }
        let kind = if matches!(op, BinOp::Mul) && kind == AggKind::List {
            AggKind::Bag
        } else {
            kind
        };
        Ok(Val::Agg(Rc::new(Agg::new(kind, out))))
    }
}

/// A hashable stand-in for instance equality, where one is exact.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Key {
    Model(ifc_model::EntityId),
    Local(usize),
    Int(i64),
    Str(std::sync::Arc<str>),
    Enum(String),
    Logical(u8),
}

/// The key of a value, or `None` when only a pairwise comparison is exact
/// (a REAL, which equals an INTEGER of its value; an aggregate; `?`).
fn key(value: &Val) -> Option<Key> {
    Some(match value.bare() {
        Val::Inst(Inst::Model(id)) => Key::Model(*id),
        Val::Inst(Inst::Local(local)) => Key::Local(Rc::as_ptr(local) as usize),
        Val::Int(v) => Key::Int(*v),
        Val::Str(text) => Key::Str(text.clone()),
        Val::Enum(item) => Key::Enum(item.to_ascii_uppercase()),
        Val::Logical(l) => Key::Logical(l.rank()),
        _ => return None,
    })
}

fn truth(value: bool) -> Logical {
    if value {
        Logical::True
    } else {
        Logical::False
    }
}

fn same_instance(a: &Inst, b: &Inst) -> bool {
    match (a, b) {
        (Inst::Model(x), Inst::Model(y)) => x == y,
        (Inst::Local(x), Inst::Local(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

#[allow(clippy::cast_precision_loss)]
fn number(value: &Val) -> f64 {
    match value {
        Val::Int(v) => *v as f64,
        Val::Real(v) => *v,
        _ => f64::NAN,
    }
}

fn order(left: &Val, right: &Val) -> R<Ordering> {
    match (left, right) {
        (Val::Int(a), Val::Int(b)) => Ok(a.cmp(b)),
        (Val::Int(_) | Val::Real(_), Val::Int(_) | Val::Real(_)) => number(left)
            .partial_cmp(&number(right))
            .ok_or_else(|| Fault::Data("comparing NaN".into())),
        (Val::Str(a), Val::Str(b)) => Ok(a.cmp(b)),
        (Val::Logical(a), Val::Logical(b)) => Ok(a.rank().cmp(&b.rank())),
        _ => Err(Fault::Data(format!(
            "ordering {} against {}",
            left.kind_name(),
            right.kind_name()
        ))),
    }
}

fn arithmetic(op: BinOp, left: &Val, right: &Val) -> R<Val> {
    let (l, r) = (left.bare(), right.bare());
    if l.is_indet() || r.is_indet() {
        return Ok(Val::Indet);
    }
    if let (BinOp::Add, Val::Str(a), Val::Str(b)) = (op, l, r) {
        return Ok(Val::Str(format!("{a}{b}").into()));
    }
    match (l, r) {
        (Val::Int(a), Val::Int(b)) => int_arithmetic(op, *a, *b),
        (Val::Int(_) | Val::Real(_), Val::Int(_) | Val::Real(_)) => {
            let (a, b) = (number(l), number(r));
            Ok(match op {
                BinOp::Add => Val::Real(a + b),
                BinOp::Sub => Val::Real(a - b),
                BinOp::Mul => Val::Real(a * b),
                BinOp::Div if b == 0.0 => Val::Indet,
                BinOp::Div => Val::Real(a / b),
                BinOp::Pow => Val::Real(a.powf(b)),
                _ => return Err(Fault::Data("DIV or MOD of a REAL".into())),
            })
        }
        _ => Err(Fault::Data(format!(
            "arithmetic on {} and {}",
            l.kind_name(),
            r.kind_name()
        ))),
    }
}

#[allow(clippy::cast_precision_loss)]
fn int_arithmetic(op: BinOp, a: i64, b: i64) -> R<Val> {
    Ok(match op {
        BinOp::Add => Val::Int(a.checked_add(b).ok_or_else(overflow)?),
        BinOp::Sub => Val::Int(a.checked_sub(b).ok_or_else(overflow)?),
        BinOp::Mul => Val::Int(a.checked_mul(b).ok_or_else(overflow)?),
        BinOp::Div if b == 0 => Val::Indet,
        BinOp::Div => Val::Real(a as f64 / b as f64),
        BinOp::IntDiv | BinOp::Mod if b == 0 => Val::Indet,
        BinOp::IntDiv => Val::Int(a.checked_div(b).ok_or_else(overflow)?),
        BinOp::Mod => Val::Int(a.rem_euclid(b)),
        BinOp::Pow => match u32::try_from(b) {
            Ok(exp) => Val::Int(a.checked_pow(exp).ok_or_else(overflow)?),
            Err(_) => Val::Real((a as f64).powf(b as f64)),
        },
        _ => unreachable!("only arithmetic operators reach here"),
    })
}
