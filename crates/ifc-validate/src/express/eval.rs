//! Evaluating expressions (ISO 10303-11 clause 12) with three-valued logic.
//!
//! `?` propagates: an operator or attribute reference with an indeterminate
//! operand yields `?` (or UNKNOWN for a comparison), never an error. `AND`
//! stops at FALSE and `OR` at TRUE, as IfcOpenShell's evaluation does, so a
//! guard such as `NOT EXISTS(X) OR (X.Y > 0)` never reads past its guard.

use std::rc::Rc;

use super::ast::{AggKind, BinOp, Callee, Expr, Logical, UnOp};
use super::machine::{Fault, Machine, R};
use super::value::{Agg, Val};

/// The variables of one evaluation context: `SELF` and the frame slots.
pub(crate) struct Cx {
    pub this: Val,
    pub frame: Vec<Val>,
}

impl Cx {
    pub(crate) fn new(this: Val, frame: usize) -> Self {
        Self {
            this,
            frame: vec![Val::Indet; frame],
        }
    }
}

/// A value as a logical, `?` reading as UNKNOWN.
pub(crate) fn logical(value: &Val) -> R<Logical> {
    match value.bare() {
        Val::Logical(l) => Ok(*l),
        Val::Indet => Ok(Logical::Unknown),
        other => Err(Fault::Data(format!(
            "{} where a LOGICAL is required",
            other.kind_name()
        ))),
    }
}

impl Machine<'_> {
    pub(crate) fn eval(&self, expr: &Expr, cx: &mut Cx) -> R<Val> {
        self.step()?;
        Ok(match expr {
            Expr::Int(value) => Val::Int(*value),
            Expr::Real(value) => Val::Real(*value),
            Expr::Str(text) => Val::Str(text.clone()),
            Expr::Bits(bits) => Val::Binary(super::builtins::binary_literal(bits).into()),
            Expr::Logical(value) => Val::Logical(*value),
            Expr::Indeterminate => Val::Indet,
            Expr::SelfRef => cx.this.clone(),
            Expr::Pi => Val::Real(std::f64::consts::PI),
            Expr::ConstE => Val::Real(std::f64::consts::E),
            Expr::Var(slot) => cx.frame[*slot].clone(),
            Expr::EnumItem(item) => Val::Enum(item.clone()),
            Expr::Population(index) => Val::Agg(self.population(*index)),
            Expr::Name(name) | Expr::Call(Callee::Unresolved(name), _) => {
                return Err(Fault::Unsupported(format!("the unresolved name {name}")))
            }
            Expr::Call(callee, args) => return self.call(callee, args, cx),
            Expr::Aggregate(elems) => {
                let mut items = Vec::new();
                for elem in elems {
                    let value = self.eval(&elem.value, cx)?;
                    match &elem.repeat {
                        None => items.push(value),
                        Some(count) => {
                            let count = self.eval(count, cx)?;
                            let Val::Int(count) = count.bare() else {
                                return Ok(Val::Indet);
                            };
                            for _ in 0..*count {
                                self.step()?;
                                items.push(value.clone());
                            }
                        }
                    }
                }
                Val::agg(AggKind::List, items)
            }
            Expr::Interval {
                low,
                low_strict,
                item,
                high_strict,
                high,
            } => {
                let low = self.eval(low, cx)?;
                let item = self.eval(item, cx)?;
                let high = self.eval(high, cx)?;
                let op = |strict: bool| if strict { BinOp::Lt } else { BinOp::Le };
                let first = logical(&self.compare(op(*low_strict), &low, &item)?)?;
                let second = logical(&self.compare(op(*high_strict), &item, &high)?)?;
                Val::Logical(first.and(second))
            }
            Expr::Query {
                var,
                source,
                condition,
            } => {
                let source = self.eval(source, cx)?;
                let Val::Agg(agg) = source.bare() else {
                    if source.is_indet() {
                        return Ok(Val::Indet);
                    }
                    return Err(Fault::Data(format!(
                        "QUERY over {}, not an aggregate",
                        source.kind_name()
                    )));
                };
                let saved = cx.frame[*var].clone();
                let mut kept = Vec::new();
                for item in &agg.items {
                    cx.frame[*var] = item.clone();
                    if logical(&self.eval(condition, cx)?)? == Logical::True {
                        kept.push(item.clone());
                    }
                }
                cx.frame[*var] = saved;
                Val::agg(agg.kind, kept)
            }
            Expr::Attribute(base, name) => {
                let base = self.eval(base, cx)?;
                self.attribute(&base, name)?
            }
            Expr::Group(base, group) => {
                let base = self.eval(base, cx)?;
                self.group(&base, group)?
            }
            Expr::Index(base, index, high) => {
                let base = self.eval(base, cx)?;
                let index = self.eval(index, cx)?;
                let high = match high {
                    Some(high) => Some(self.eval(high, cx)?),
                    None => None,
                };
                index_value(&base, &index, high.as_ref())?
            }
            Expr::Unary(op, operand) => {
                let value = self.eval(operand, cx)?;
                match op {
                    UnOp::Not => Val::Logical(logical(&value)?.not()),
                    UnOp::Plus => value,
                    UnOp::Neg => match value.bare() {
                        Val::Int(v) => Val::Int(v.checked_neg().ok_or_else(overflow)?),
                        Val::Real(v) => Val::Real(-v),
                        Val::Indet => Val::Indet,
                        other => {
                            return Err(Fault::Data(format!("negating {}", other.kind_name())))
                        }
                    },
                }
            }
            Expr::Binary(BinOp::And, left, right) => {
                let left = logical(&self.eval(left, cx)?)?;
                if left == Logical::False {
                    return Ok(Val::Logical(Logical::False));
                }
                Val::Logical(left.and(logical(&self.eval(right, cx)?)?))
            }
            Expr::Binary(BinOp::Or, left, right) => {
                let left = logical(&self.eval(left, cx)?)?;
                if left == Logical::True {
                    return Ok(Val::Logical(Logical::True));
                }
                Val::Logical(left.or(logical(&self.eval(right, cx)?)?))
            }
            Expr::Binary(BinOp::Complex, ..) => {
                let mut partials = Vec::new();
                self.partials(expr, cx, &mut partials)?;
                self.construct(partials)?
            }
            Expr::Binary(op, left, right) => {
                let left = self.eval(left, cx)?;
                let right = self.eval(right, cx)?;
                self.binary(*op, &left, &right)?
            }
        })
    }

    /// `base\Group`: the instance when it is a `Group`, else `?`.
    fn group(&self, base: &Val, group: &str) -> R<Val> {
        let Val::Inst(inst) = base.bare() else {
            return Ok(Val::Indet);
        };
        let entity = self.entity_of(inst)?;
        let fits = self
            .tables
            .entity_index(group)
            .is_some_and(|ancestor| self.tables.is_a(entity, ancestor));
        Ok(if fits { base.clone() } else { Val::Indet })
    }

    /// The partial constructors of an `a() || b() || c(...)` chain.
    fn partials(&self, expr: &Expr, cx: &mut Cx, out: &mut Vec<(usize, Vec<Val>)>) -> R<()> {
        match expr {
            Expr::Binary(BinOp::Complex, left, right) => {
                self.partials(left, cx, out)?;
                self.partials(right, cx, out)
            }
            Expr::Call(Callee::Entity(entity), args) => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval(arg, cx)?);
                }
                out.push((*entity, values));
                Ok(())
            }
            _ => Err(Fault::Unsupported(
                "`||` over something other than entity constructors".into(),
            )),
        }
    }
}

/// `base[index]` or `base[index : high]`; `?` out of range.
fn index_value(base: &Val, index: &Val, high: Option<&Val>) -> R<Val> {
    if base.is_indet() || index.is_indet() || high.is_some_and(Val::is_indet) {
        return Ok(Val::Indet);
    }
    let Val::Int(at) = index.bare() else {
        return Err(Fault::Data(format!("an index of {}", index.kind_name())));
    };
    match base.bare() {
        Val::Agg(agg) => {
            let position = |i: i64| usize::try_from(i - agg.lower).ok();
            if let Some(high) = high {
                let Val::Int(to) = high.bare() else {
                    return Err(Fault::Data("a non-integer index range".into()));
                };
                let items: Vec<Val> = (*at..=*to)
                    .filter_map(|i| position(i).and_then(|p| agg.items.get(p)).cloned())
                    .collect();
                return Ok(Val::Agg(Rc::new(Agg::new(agg.kind, items))));
            }
            Ok(position(*at)
                .and_then(|p| agg.items.get(p))
                .cloned()
                .unwrap_or(Val::Indet))
        }
        Val::Binary(_) => Err(Fault::Unsupported("indexing a BINARY".into())),
        Val::Str(text) => {
            let chars: Vec<char> = text.chars().collect();
            let to = match high {
                Some(high) => match high.bare() {
                    Val::Int(to) => *to,
                    _ => return Err(Fault::Data("a non-integer index range".into())),
                },
                None => *at,
            };
            let (Ok(from), Ok(to)) = (usize::try_from(*at - 1), usize::try_from(to)) else {
                return Ok(Val::Indet);
            };
            if from >= to || to > chars.len() {
                return Ok(Val::Indet);
            }
            Ok(Val::Str(chars[from..to].iter().collect::<String>().into()))
        }
        other => Err(Fault::Data(format!("indexing {}", other.kind_name()))),
    }
}

pub(crate) fn overflow() -> Fault {
    Fault::Data("integer overflow".into())
}
