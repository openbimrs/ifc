//! Reading FUNCTIONs, global RULEs and their statements (ISO 10303-11
//! clauses 9.5, 9.6 and 13).

use super::ast::{
    Access, FunctionDecl, Increment, LocalVar, Name, RuleDecl, Stmt, Target, TypeExpr,
};
use super::error::ExpressError;
use super::lexer::Tok;
use super::parse::{describe, domain_rules, type_expr, Parser};
use super::parse_expr::expression;

/// A local variable before resolution: its type is kept for an `ARRAY`'s
/// lower bound.
pub(crate) struct ParsedLocal {
    pub var: LocalVar,
    pub ty: TypeExpr,
}

/// `FUNCTION name (params) : type; [LOCAL ...] statements END_FUNCTION;`
pub(crate) fn function(p: &mut Parser) -> Result<FunctionDecl, ExpressError> {
    p.expect_word("FUNCTION")?;
    let (name, spelling) = p.ident()?;
    let mut params = Vec::new();
    let mut param_shapes = Vec::new();
    if p.eat_sym("(") {
        loop {
            p.eat_word("VAR");
            let mut names = vec![p.ident()?.0];
            while p.eat_sym(",") {
                names.push(p.ident()?.0);
            }
            p.expect_sym(":")?;
            let shape = super::resolve::shape(type_expr(p)?);
            param_shapes.extend(names.iter().map(|_| shape.clone()));
            params.extend(names);
            if !p.eat_sym(";") {
                break;
            }
        }
        p.expect_sym(")")?;
    }
    p.expect_sym(":")?;
    type_expr(p)?;
    p.expect_sym(";")?;
    let locals = algorithm_head(p)?;
    let body = statements(p, &["END_FUNCTION"])?;
    p.expect_word("END_FUNCTION")?;
    p.expect_sym(";")?;
    let (locals, types) = split(locals);
    let mut decl = FunctionDecl {
        name,
        spelling,
        params,
        param_shapes,
        locals,
        body,
        frame: 0,
    };
    super::resolve::local_types(&mut decl.locals, types);
    Ok(decl)
}

/// `RULE name FOR (entities); [LOCAL ...] statements WHERE ... END_RULE;`
pub(crate) fn rule(p: &mut Parser) -> Result<RuleDecl, ExpressError> {
    p.expect_word("RULE")?;
    let (_, spelling) = p.ident()?;
    p.expect_word("FOR")?;
    p.expect_sym("(")?;
    let mut entities = vec![p.ident()?.0];
    while p.eat_sym(",") {
        entities.push(p.ident()?.0);
    }
    p.expect_sym(")")?;
    p.expect_sym(";")?;
    let locals = algorithm_head(p)?;
    let body = statements(p, &["WHERE", "END_RULE"])?;
    let rules = if p.eat_word("WHERE") {
        domain_rules(p, &["END_RULE"])?
    } else {
        Vec::new()
    };
    p.expect_word("END_RULE")?;
    p.expect_sym(";")?;
    let (locals, types) = split(locals);
    let mut decl = RuleDecl {
        spelling,
        entities,
        locals,
        body,
        rules,
        frame: 0,
    };
    super::resolve::local_types(&mut decl.locals, types);
    Ok(decl)
}

fn split(locals: Vec<ParsedLocal>) -> (Vec<LocalVar>, Vec<TypeExpr>) {
    locals
        .into_iter()
        .map(|local| (local.var, local.ty))
        .unzip()
}

/// `[CONSTANT ... END_CONSTANT;] [LOCAL ... END_LOCAL;]`.
fn algorithm_head(p: &mut Parser) -> Result<Vec<ParsedLocal>, ExpressError> {
    let mut locals = Vec::new();
    loop {
        if p.is_word("CONSTANT") || p.is_word("FUNCTION") || p.is_word("PROCEDURE") {
            return Err(p.error(format!(
                "{} inside an algorithm is not read by this evaluator",
                describe(p.peek())
            )));
        }
        if !p.eat_word("LOCAL") {
            return Ok(locals);
        }
        while !p.eat_word("END_LOCAL") {
            let mut names = vec![p.ident()?.0];
            while p.eat_sym(",") {
                names.push(p.ident()?.0);
            }
            p.expect_sym(":")?;
            let ty = type_expr(p)?;
            let init = if p.eat_sym(":=") {
                Some(expression(p)?)
            } else {
                None
            };
            p.expect_sym(";")?;
            for name in names {
                locals.push(ParsedLocal {
                    var: LocalVar {
                        name,
                        slot: 0,
                        init: init.clone(),
                        shape: None,
                    },
                    ty: ty.clone(),
                });
            }
        }
        p.expect_sym(";")?;
    }
}

/// Statements up to (not including) one of the keywords `ends`.
fn statements(p: &mut Parser, ends: &[&str]) -> Result<Vec<Stmt>, ExpressError> {
    let mut out = Vec::new();
    while !ends.iter().any(|end| p.is_word(end)) {
        if matches!(p.peek(), Tok::End) {
            return Err(p.error("unexpected end of source in a statement list"));
        }
        out.push(statement(p)?);
    }
    Ok(out)
}

fn statement(p: &mut Parser) -> Result<Stmt, ExpressError> {
    if p.eat_sym(";") {
        return Ok(Stmt::Null);
    }
    let word = match p.peek() {
        Tok::Word(upper, _) => upper.clone(),
        other => return Err(p.error(format!("expected a statement, found {}", describe(other)))),
    };
    match &*word {
        "BEGIN" => {
            p.bump();
            let body = statements(p, &["END"])?;
            p.expect_word("END")?;
            p.expect_sym(";")?;
            Ok(Stmt::Compound(body))
        }
        "IF" => {
            p.bump();
            let condition = expression(p)?;
            p.expect_word("THEN")?;
            let then = statements(p, &["ELSE", "END_IF"])?;
            let otherwise = if p.eat_word("ELSE") {
                statements(p, &["END_IF"])?
            } else {
                Vec::new()
            };
            p.expect_word("END_IF")?;
            p.expect_sym(";")?;
            Ok(Stmt::If {
                condition,
                then,
                otherwise,
            })
        }
        "CASE" => case(p),
        "REPEAT" => repeat(p),
        "RETURN" => {
            p.bump();
            let value = if p.eat_sym("(") {
                let value = expression(p)?;
                p.expect_sym(")")?;
                Some(value)
            } else {
                None
            };
            p.expect_sym(";")?;
            Ok(Stmt::Return(value))
        }
        "ESCAPE" | "SKIP" => {
            p.bump();
            p.expect_sym(";")?;
            Ok(if &*word == "ESCAPE" {
                Stmt::Escape
            } else {
                Stmt::Skip
            })
        }
        "ALIAS" => Err(p.error("ALIAS statements are not read by this evaluator")),
        _ => assignment_or_call(p),
    }
}

fn case(p: &mut Parser) -> Result<Stmt, ExpressError> {
    p.expect_word("CASE")?;
    let selector = expression(p)?;
    p.expect_word("OF")?;
    let mut arms = Vec::new();
    let mut otherwise = None;
    loop {
        if p.eat_word("END_CASE") {
            break;
        }
        if p.eat_word("OTHERWISE") {
            p.expect_sym(":")?;
            otherwise = Some(Box::new(statement(p)?));
            continue;
        }
        let mut labels = vec![expression(p)?];
        while p.eat_sym(",") {
            labels.push(expression(p)?);
        }
        p.expect_sym(":")?;
        arms.push((labels, statement(p)?));
    }
    p.expect_sym(";")?;
    Ok(Stmt::Case {
        selector,
        arms,
        otherwise,
    })
}

fn repeat(p: &mut Parser) -> Result<Stmt, ExpressError> {
    p.expect_word("REPEAT")?;
    let mut increment = None;
    if matches!(p.peek(), Tok::Word(..)) && matches!(p.peek_at(1), Tok::Sym(":=")) {
        let (name, _) = p.ident()?;
        p.bump();
        let from = super::parse_expr::simple_expression(p)?;
        p.expect_word("TO")?;
        let to = super::parse_expr::simple_expression(p)?;
        let by = if p.eat_word("BY") {
            Some(super::parse_expr::simple_expression(p)?)
        } else {
            None
        };
        increment = Some(Increment {
            name,
            var: 0,
            from,
            to,
            by,
        });
    }
    let while_ = if p.eat_word("WHILE") {
        Some(expression(p)?)
    } else {
        None
    };
    let until = if p.eat_word("UNTIL") {
        Some(expression(p)?)
    } else {
        None
    };
    p.expect_sym(";")?;
    let body = statements(p, &["END_REPEAT"])?;
    p.expect_word("END_REPEAT")?;
    p.expect_sym(";")?;
    Ok(Stmt::Repeat {
        increment,
        while_,
        until,
        body,
    })
}

/// `name { .a | [i] | \G } := expression;` or `name(args);`.
fn assignment_or_call(p: &mut Parser) -> Result<Stmt, ExpressError> {
    let (name, _) = p.ident()?;
    if p.eat_sym("(") {
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
        p.expect_sym(";")?;
        return Ok(Stmt::Call(name, args));
    }
    let mut path = Vec::new();
    loop {
        if p.eat_sym(".") {
            path.push(Access::Attribute(p.ident()?.0));
        } else if p.eat_sym("\\") {
            p.ident()?;
            path.push(Access::Group);
        } else if p.eat_sym("[") {
            path.push(Access::Index(expression(p)?));
            p.expect_sym("]")?;
        } else {
            break;
        }
    }
    p.expect_sym(":=")?;
    let value = expression(p)?;
    p.expect_sym(";")?;
    Ok(Stmt::Assign(
        Target {
            name: Name::clone(&name),
            slot: 0,
            path,
            shape: None,
        },
        value,
    ))
}
