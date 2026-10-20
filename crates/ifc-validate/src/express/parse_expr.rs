//! Reading EXPRESS expressions (ISO 10303-11 clause 12).
//!
//! The precedence is the standard's (Table 12), which differs from most
//! languages: `AND` binds like `*` and `OR` like `+`, both tighter than any
//! comparison, and a unary operator applies to the factor right after it.
//! `NOT a = b` is therefore `(NOT a) = b`. The IFC schemas parenthesise
//! accordingly, and the reader must not "fix" them.

use super::ast::{AggElem, BinOp, Callee, Expr, Logical, UnOp};
use super::error::ExpressError;
use super::lexer::Tok;
use super::parse::{describe, Parser};

/// `expression = simple_expression [ rel_op simple_expression ]`.
pub(crate) fn expression(p: &mut Parser) -> Result<Expr, ExpressError> {
    let left = simple_expression(p)?;
    let op = match p.peek() {
        Tok::Sym("=") => BinOp::Eq,
        Tok::Sym("<>") => BinOp::Ne,
        Tok::Sym("<") => BinOp::Lt,
        Tok::Sym("<=") => BinOp::Le,
        Tok::Sym(">") => BinOp::Gt,
        Tok::Sym(">=") => BinOp::Ge,
        Tok::Sym(":=:") => BinOp::InstEq,
        Tok::Sym(":<>:") => BinOp::InstNe,
        Tok::Word(word, _) if &**word == "IN" => BinOp::In,
        Tok::Word(word, _) if &**word == "LIKE" => BinOp::Like,
        _ => return Ok(left),
    };
    p.bump();
    let right = simple_expression(p)?;
    Ok(Expr::Binary(op, Box::new(left), Box::new(right)))
}

/// `term { (+ | - | OR | XOR) term }`.
pub(crate) fn simple_expression(p: &mut Parser) -> Result<Expr, ExpressError> {
    let mut left = term(p)?;
    loop {
        let op = match p.peek() {
            Tok::Sym("+") => BinOp::Add,
            Tok::Sym("-") => BinOp::Sub,
            Tok::Word(word, _) if &**word == "OR" => BinOp::Or,
            Tok::Word(word, _) if &**word == "XOR" => BinOp::Xor,
            _ => return Ok(left),
        };
        p.bump();
        let right = term(p)?;
        left = Expr::Binary(op, Box::new(left), Box::new(right));
    }
}

/// `factor { (* | / | DIV | MOD | AND | ||) factor }`.
fn term(p: &mut Parser) -> Result<Expr, ExpressError> {
    let mut left = factor(p)?;
    loop {
        let op = match p.peek() {
            Tok::Sym("*") => BinOp::Mul,
            Tok::Sym("/") => BinOp::Div,
            Tok::Sym("||") => BinOp::Complex,
            Tok::Word(word, _) if &**word == "DIV" => BinOp::IntDiv,
            Tok::Word(word, _) if &**word == "MOD" => BinOp::Mod,
            Tok::Word(word, _) if &**word == "AND" => BinOp::And,
            _ => return Ok(left),
        };
        p.bump();
        let right = factor(p)?;
        left = Expr::Binary(op, Box::new(left), Box::new(right));
    }
}

/// `simple_factor [ ** simple_factor ]`.
fn factor(p: &mut Parser) -> Result<Expr, ExpressError> {
    let base = simple_factor(p)?;
    if p.eat_sym("**") {
        let exponent = simple_factor(p)?;
        return Ok(Expr::Binary(BinOp::Pow, Box::new(base), Box::new(exponent)));
    }
    Ok(base)
}

fn simple_factor(p: &mut Parser) -> Result<Expr, ExpressError> {
    let unary = match p.peek() {
        Tok::Sym("-") => Some(UnOp::Neg),
        Tok::Sym("+") => Some(UnOp::Plus),
        Tok::Word(word, _) if &**word == "NOT" => Some(UnOp::Not),
        _ => None,
    };
    if let Some(op) = unary {
        p.bump();
        let operand = simple_factor(p)?;
        return Ok(Expr::Unary(op, Box::new(operand)));
    }
    let primary = primary(p)?;
    qualifiers(p, primary)
}

/// `.name`, `\Group` and `[index]` / `[low : high]` after a factor.
fn qualifiers(p: &mut Parser, mut expr: Expr) -> Result<Expr, ExpressError> {
    loop {
        if p.eat_sym(".") {
            let (name, _) = p.ident()?;
            expr = Expr::Attribute(Box::new(expr), name);
        } else if p.eat_sym("\\") {
            let (name, _) = p.ident()?;
            expr = Expr::Group(Box::new(expr), name);
        } else if p.is_sym("[") {
            p.bump();
            let index = expression(p)?;
            let high = if p.eat_sym(":") {
                Some(Box::new(expression(p)?))
            } else {
                None
            };
            p.expect_sym("]")?;
            expr = Expr::Index(Box::new(expr), Box::new(index), high);
        } else {
            return Ok(expr);
        }
    }
}

fn primary(p: &mut Parser) -> Result<Expr, ExpressError> {
    match p.bump() {
        Tok::Int(value) => Ok(Expr::Int(value)),
        Tok::Real(value) => Ok(Expr::Real(value)),
        Tok::Str(text) => Ok(Expr::Str(text.as_ref().into())),
        Tok::Binary(bits) => Ok(Expr::Bits(bits.as_ref().into())),
        Tok::Sym("?") => Ok(Expr::Indeterminate),
        Tok::Sym("(") => {
            let inner = expression(p)?;
            p.expect_sym(")")?;
            Ok(inner)
        }
        Tok::Sym("[") => aggregate(p),
        Tok::Sym("{") => interval(p),
        Tok::Word(word, _) => match &*word {
            "SELF" => Ok(Expr::SelfRef),
            "TRUE" => Ok(Expr::Logical(Logical::True)),
            "FALSE" => Ok(Expr::Logical(Logical::False)),
            "UNKNOWN" => Ok(Expr::Logical(Logical::Unknown)),
            "PI" => Ok(Expr::Pi),
            "CONST_E" => Ok(Expr::ConstE),
            "QUERY" => query(p),
            _ if p.is_sym("(") => {
                p.bump();
                let mut args = Vec::new();
                if !p.eat_sym(")") {
                    loop {
                        args.push(expression(p)?);
                        if !p.eat_sym(",") {
                            break;
                        }
                    }
                    p.expect_sym(")")?;
                }
                Ok(Expr::Call(Callee::Unresolved(word), args))
            }
            _ => Ok(Expr::Name(word)),
        },
        other => Err(p.error(format!(
            "expected an expression, found {}",
            describe(&other)
        ))),
    }
}

/// `[ e1, e2 : n, ... ]` after its `[`.
fn aggregate(p: &mut Parser) -> Result<Expr, ExpressError> {
    let mut elems = Vec::new();
    if p.eat_sym("]") {
        return Ok(Expr::Aggregate(elems));
    }
    loop {
        let value = expression(p)?;
        let repeat = if p.eat_sym(":") {
            Some(expression(p)?)
        } else {
            None
        };
        elems.push(AggElem { value, repeat });
        if !p.eat_sym(",") {
            break;
        }
    }
    p.expect_sym("]")?;
    Ok(Expr::Aggregate(elems))
}

/// `{ low (<|<=) item (<|<=) high }` after its `{`.
fn interval(p: &mut Parser) -> Result<Expr, ExpressError> {
    let low = simple_expression(p)?;
    let low_strict = interval_op(p)?;
    let item = simple_expression(p)?;
    let high_strict = interval_op(p)?;
    let high = simple_expression(p)?;
    p.expect_sym("}")?;
    Ok(Expr::Interval {
        low: Box::new(low),
        low_strict,
        item: Box::new(item),
        high_strict,
        high: Box::new(high),
    })
}

fn interval_op(p: &mut Parser) -> Result<bool, ExpressError> {
    if p.eat_sym("<") {
        Ok(true)
    } else if p.eat_sym("<=") {
        Ok(false)
    } else {
        Err(p.error("expected `<` or `<=` in an interval"))
    }
}

/// `QUERY ( var <* source | condition )` after its keyword.
fn query(p: &mut Parser) -> Result<Expr, ExpressError> {
    p.expect_sym("(")?;
    let (name, _) = p.ident()?;
    p.expect_sym("<*")?;
    let source = simple_expression(p)?;
    p.expect_sym("|")?;
    let condition = expression(p)?;
    p.expect_sym(")")?;
    // The variable is a name until resolution gives it a slot; the
    // condition refers to it by that name.
    Ok(Expr::Call(
        Callee::Unresolved("QUERY".into()),
        vec![Expr::Name(name), source, condition],
    ))
}
