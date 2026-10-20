//! The built-in functions (ISO 10303-11 clause 15).
//!
//! `?` in gives `?` out, except where the standard says otherwise:
//! `EXISTS(?)` is FALSE, `TYPEOF(?)` the empty set, `NVL(?, d)` is `d`.
//! `FORMAT`, `HIBOUND`, `LOBOUND` and `ROLESOF` are not implemented (no IFC
//! rule uses them); a rule reaching one is reported as not evaluated.

use std::rc::Rc;

use super::ast::{AggKind, Builtin, Expr, Logical};
use super::eval::Cx;
use super::machine::{Fault, Machine, R};
use super::value::{Agg, Inst, Val};

impl Machine<'_> {
    pub(crate) fn builtin(&self, builtin: Builtin, args: &[Expr], cx: &mut Cx) -> R<Val> {
        if builtin.unsupported() {
            return Err(Fault::Unsupported(
                format!("the built-in {builtin:?}").to_uppercase(),
            ));
        }
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.eval(arg, cx)?);
        }
        let arity = |n: usize| {
            if values.len() == n {
                Ok(())
            } else {
                Err(Fault::Unsupported(format!(
                    "{builtin:?} with {} arguments",
                    values.len()
                )))
            }
        };
        match builtin {
            Builtin::Exists => {
                arity(1)?;
                Ok(Val::logical(!values[0].is_indet()))
            }
            Builtin::Nvl => {
                arity(2)?;
                Ok(if values[0].is_indet() {
                    values[1].clone()
                } else {
                    values[0].clone()
                })
            }
            Builtin::Sizeof => {
                arity(1)?;
                Ok(match values[0].bare() {
                    Val::Agg(agg) => Val::Int(len(agg)),
                    Val::Indet => Val::Indet,
                    other => return Err(Fault::Data(format!("SIZEOF of {}", other.kind_name()))),
                })
            }
            Builtin::Hiindex | Builtin::Loindex => {
                arity(1)?;
                Ok(match values[0].bare() {
                    Val::Agg(agg) if builtin == Builtin::Loindex => Val::Int(agg.lower),
                    Val::Agg(agg) => Val::Int(agg.lower + len(agg) - 1),
                    Val::Indet => Val::Indet,
                    other => {
                        return Err(Fault::Data(format!("{builtin:?} of {}", other.kind_name())))
                    }
                })
            }
            Builtin::Typeof => {
                arity(1)?;
                self.type_of(&values[0])
            }
            Builtin::Usedin => {
                arity(2)?;
                self.used_in(&values[0], &values[1])
            }
            Builtin::Length => {
                arity(1)?;
                Ok(match values[0].bare() {
                    Val::Str(text) => Val::Int(count(text.chars().count())),
                    Val::Indet => Val::Indet,
                    other => return Err(Fault::Data(format!("LENGTH of {}", other.kind_name()))),
                })
            }
            Builtin::Blength => {
                arity(1)?;
                Ok(match values[0].bare() {
                    Val::Binary(hex) => Val::Int(bit_length(hex)?),
                    Val::Indet => Val::Indet,
                    other => return Err(Fault::Data(format!("BLENGTH of {}", other.kind_name()))),
                })
            }
            Builtin::Odd => {
                arity(1)?;
                Ok(match values[0].bare() {
                    Val::Int(v) => Val::logical(v % 2 != 0),
                    Val::Indet => Val::Logical(Logical::Unknown),
                    other => return Err(Fault::Data(format!("ODD of {}", other.kind_name()))),
                })
            }
            Builtin::Value => {
                arity(1)?;
                Ok(match values[0].bare() {
                    Val::Str(text) => text
                        .trim()
                        .parse::<i64>()
                        .map(Val::Int)
                        .or_else(|_| text.trim().parse::<f64>().map(Val::Real))
                        .unwrap_or(Val::Indet),
                    _ => Val::Indet,
                })
            }
            Builtin::ValueIn => {
                arity(2)?;
                let Val::Agg(agg) = values[0].bare() else {
                    return Ok(Val::Logical(Logical::Unknown));
                };
                let mut found = Logical::False;
                for item in &agg.items {
                    found = found.or(self.value_eq(item, &values[1], 0)?);
                }
                Ok(Val::Logical(found))
            }
            Builtin::ValueUnique => {
                arity(1)?;
                let Val::Agg(agg) = values[0].bare() else {
                    return Ok(Val::Logical(Logical::Unknown));
                };
                let mut unique = Logical::True;
                for (i, a) in agg.items.iter().enumerate() {
                    for b in &agg.items[i + 1..] {
                        unique = unique.and(self.value_eq(a, b, 0)?.not());
                    }
                }
                Ok(Val::Logical(unique))
            }
            _ => {
                arity(if builtin == Builtin::Atan { 2 } else { 1 })?;
                math(builtin, &values)
            }
        }
    }

    /// `TYPEOF` (15.25): the qualified names of every type the value is a
    /// member of, SELECTs included.
    fn type_of(&self, value: &Val) -> R<Val> {
        Ok(match value {
            Val::Indet => Val::agg(AggKind::Set, Vec::new()),
            Val::Typed(typed) => self.typeof_type(typed.0),
            Val::Inst(inst) => self.typeof_entity(self.entity_of(inst)?),
            other => {
                let base = match other {
                    Val::Logical(_) => "LOGICAL",
                    Val::Int(_) => "INTEGER",
                    Val::Real(_) => "REAL",
                    Val::Str(_) => "STRING",
                    Val::Binary(_) => "BINARY",
                    Val::Agg(agg) => agg.kind.keyword(),
                    _ => return Ok(Val::agg(AggKind::Set, Vec::new())),
                };
                Val::agg(AggKind::Set, vec![Val::Str(base.into())])
            }
        })
    }

    /// `USEDIN(instance, 'SCHEMA.ENTITY.ATTRIBUTE')` (15.26); `''` for
    /// every referrer.
    fn used_in(&self, target: &Val, role: &Val) -> R<Val> {
        let Val::Inst(inst) = target.bare() else {
            return Ok(Val::agg(AggKind::Bag, Vec::new()));
        };
        let Inst::Model(id) = inst else {
            return Ok(Val::agg(AggKind::Bag, Vec::new()));
        };
        let Val::Str(role) = role.bare() else {
            return Ok(Val::Indet);
        };
        let ids = if role.is_empty() {
            self.all_referrers(*id)
        } else {
            let parts: Vec<&str> = role.split('.').collect();
            let [_, entity, attribute] = parts.as_slice() else {
                return Err(Fault::Data(format!(
                    "USEDIN role '{role}' is not SCHEMA.ENTITY.ATTRIBUTE"
                )));
            };
            self.referrers(
                *id,
                &entity.to_ascii_uppercase(),
                &attribute.to_ascii_uppercase(),
            )?
        };
        Ok(Val::Agg(Rc::new(Agg::new(
            AggKind::Bag,
            ids.into_iter()
                .map(|id| Val::Inst(Inst::Model(id)))
                .collect(),
        ))))
    }
}

/// The bits of a binary in its Part 21 form: a leading digit counting the
/// unused high bits (0-3) of the first hex digit, then the hex digits.
fn bit_length(hex: &str) -> R<i64> {
    let mut digits = hex.chars();
    let unused = digits
        .next()
        .and_then(|first| first.to_digit(10))
        .filter(|unused| *unused <= 3)
        .ok_or_else(|| Fault::Data(format!("\"{hex}\" is not a Part 21 binary")))?;
    let nibbles = count(digits.count());
    Ok(4 * nibbles - i64::from(unused))
}

/// An EXPRESS `%0101` literal in Part 21 form, so a literal and a file
/// value compare and measure alike.
pub(crate) fn binary_literal(bits: &str) -> String {
    let unused = (4 - bits.len() % 4) % 4;
    let padded = format!("{}{bits}", "0".repeat(unused));
    let mut out = unused.to_string();
    for nibble in padded.as_bytes().chunks(4) {
        let value = nibble
            .iter()
            .fold(0u32, |acc, bit| acc * 2 + u32::from(*bit == b'1'));
        out.push(char::from_digit(value, 16).map_or('0', |c| c.to_ascii_uppercase()));
    }
    out
}

fn len(agg: &Agg) -> i64 {
    count(agg.items.len())
}

fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[allow(clippy::cast_precision_loss)]
fn math(builtin: Builtin, values: &[Val]) -> R<Val> {
    if values.iter().any(Val::is_indet) {
        return Ok(Val::Indet);
    }
    let number = |value: &Val| match value.bare() {
        Val::Int(v) => Ok(*v as f64),
        Val::Real(v) => Ok(*v),
        other => Err(Fault::Data(format!("{builtin:?} of {}", other.kind_name()))),
    };
    if builtin == Builtin::Abs {
        return Ok(match values[0].bare() {
            Val::Int(v) => Val::Int(v.checked_abs().ok_or_else(super::eval::overflow)?),
            _ => Val::Real(number(&values[0])?.abs()),
        });
    }
    let x = number(&values[0])?;
    let result = match builtin {
        Builtin::Sqrt if x < 0.0 => return Ok(Val::Indet),
        Builtin::Sqrt => x.sqrt(),
        Builtin::Sin => x.sin(),
        Builtin::Cos => x.cos(),
        Builtin::Tan => x.tan(),
        Builtin::Asin => x.asin(),
        Builtin::Acos => x.acos(),
        Builtin::Atan => x.atan2(number(&values[1])?),
        Builtin::Exp => x.exp(),
        Builtin::Log if x <= 0.0 => return Ok(Val::Indet),
        Builtin::Log => x.ln(),
        Builtin::Log2 if x <= 0.0 => return Ok(Val::Indet),
        Builtin::Log2 => x.log2(),
        Builtin::Log10 if x <= 0.0 => return Ok(Val::Indet),
        Builtin::Log10 => x.log10(),
        other => return Err(Fault::Unsupported(format!("the built-in {other:?}"))),
    };
    Ok(if result.is_nan() {
        Val::Indet
    } else {
        Val::Real(result)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ISO 10303-21 binary form: `"092"` is the eight bits `10010010`.
    #[test]
    fn binary_length_reads_the_unused_bit_count() {
        assert_eq!(bit_length("092").ok(), Some(8));
        assert_eq!(bit_length("3F").ok(), Some(1));
        assert_eq!(bit_length("0").ok(), Some(0));
        assert!(bit_length("X1").is_err());
        assert_eq!(binary_literal("1"), "31");
        assert_eq!(binary_literal("10010010"), "092");
        assert_eq!(bit_length(&binary_literal("101")).ok(), Some(3));
    }
}
