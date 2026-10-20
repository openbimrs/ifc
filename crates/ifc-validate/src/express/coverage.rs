//! What the evaluator can run of one release, found by reading the rules.
//!
//! A declaration is evaluable when neither it nor any FUNCTION it calls,
//! transitively, uses a construct the evaluator refuses: an unimplemented
//! built-in (`FORMAT`, `HIBOUND`, `LOBOUND`, `ROLESOF`), a procedure call,
//! `LIKE`, or `||` over something other than entity constructors. The
//! count is static: a rule can still be stopped at run time by a budget or
//! by data it cannot read, and that is reported per instance.

use std::collections::BTreeSet;

use super::ast::{BinOp, Builtin, Callee, Expr, Stmt};
use super::tables::Tables;

/// Evaluable declarations of one kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct RuleCount {
    /// Declared in the schema.
    pub total: usize,
    /// Of those, evaluable.
    pub evaluable: usize,
}

/// The evaluator's coverage of one release.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Coverage {
    /// The schema name.
    pub release: String,
    /// Entity WHERE rules.
    pub entity_rules: RuleCount,
    /// Defined-type WHERE rules.
    pub type_rules: RuleCount,
    /// Global RULEs (counted once each, whatever their WHERE clauses).
    pub global_rules: RuleCount,
    /// FUNCTIONs.
    pub functions: RuleCount,
    /// DERIVE attributes.
    pub derived: RuleCount,
    /// Every declaration that is not evaluable, with the construct that
    /// stops it, sorted.
    pub not_evaluable: Vec<(String, String)>,
}

pub(crate) fn measure(tables: &Tables) -> Coverage {
    let schema = &tables.schema;
    // Direct refusals per function, then closed over calls.
    let direct: Vec<BTreeSet<String>> = schema
        .functions
        .iter()
        .map(|f| {
            let mut found = Found::default();
            f.locals
                .iter()
                .filter_map(|l| l.init.as_ref())
                .for_each(|e| found.expr(e));
            f.body.iter().for_each(|s| found.stmt(s));
            found.refused
        })
        .collect();
    let calls: Vec<BTreeSet<usize>> = schema
        .functions
        .iter()
        .map(|f| {
            let mut found = Found::default();
            f.body.iter().for_each(|s| found.stmt(s));
            f.locals
                .iter()
                .filter_map(|l| l.init.as_ref())
                .for_each(|e| found.expr(e));
            found.calls
        })
        .collect();
    // A function is refused for its own first refusal, or for the first
    // refusal of a function it calls; only `None` turns `Some`, so this
    // settles.
    let mut closed: Vec<Option<String>> = direct.iter().map(|d| d.iter().next().cloned()).collect();
    loop {
        let mut changed = false;
        for (index, callees) in calls.iter().enumerate() {
            if closed[index].is_some() {
                continue;
            }
            if let Some((callee, why)) = callees
                .iter()
                .find_map(|&c| closed[c].as_ref().map(|why| (c, why.clone())))
            {
                closed[index] = Some(format!("{why} (via {})", schema.functions[callee].spelling));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut coverage = Coverage {
        release: schema.name.to_string(),
        ..Coverage::default()
    };
    let mut judge = |name: String, exprs: &[&Expr], stmts: &[Stmt], count: &mut RuleCount| {
        let mut found = Found::default();
        exprs.iter().for_each(|e| found.expr(e));
        stmts.iter().for_each(|s| found.stmt(s));
        let mut refused = found.refused;
        for callee in found.calls {
            if let Some(why) = &closed[callee] {
                refused.insert(format!("{why} (via {})", schema.functions[callee].spelling));
            }
        }
        count.total += 1;
        match refused.into_iter().next() {
            None => count.evaluable += 1,
            Some(why) => coverage.not_evaluable.push((name, why)),
        }
    };
    let (mut entity_rules, mut type_rules, mut global_rules, mut derived) = Default::default();
    for entity in &schema.entities {
        for rule in &entity.rules {
            let name = format!("{}.{}", entity.spelling, rule.label);
            judge(name, &[&rule.expr], &[], &mut entity_rules);
        }
        for attr in &entity.derived {
            let name = format!("{}.{}", entity.spelling, attr.spelling);
            judge(name, &[&attr.expr], &[], &mut derived);
        }
    }
    for decl in &schema.types {
        for rule in &decl.rules {
            let name = format!("{}.{}", decl.spelling, rule.label);
            judge(name, &[&rule.expr], &[], &mut type_rules);
        }
    }
    for rule in &schema.rules {
        let mut exprs: Vec<&Expr> = rule.rules.iter().map(|r| &r.expr).collect();
        exprs.extend(rule.locals.iter().filter_map(|l| l.init.as_ref()));
        judge(
            format!("global.{}", rule.spelling),
            &exprs,
            &rule.body,
            &mut global_rules,
        );
    }
    let mut functions = RuleCount::default();
    for (index, function) in schema.functions.iter().enumerate() {
        functions.total += 1;
        match &closed[index] {
            None => functions.evaluable += 1,
            Some(why) => coverage
                .not_evaluable
                .push((function.spelling.to_string(), why.clone())),
        }
    }
    coverage.entity_rules = entity_rules;
    coverage.type_rules = type_rules;
    coverage.global_rules = global_rules;
    coverage.derived = derived;
    coverage.functions = functions;
    coverage.not_evaluable.sort();
    coverage
}

/// The refused constructs and called functions found in some code.
#[derive(Default)]
struct Found {
    refused: BTreeSet<String>,
    calls: BTreeSet<usize>,
}

impl Found {
    fn stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Null | Stmt::Escape | Stmt::Skip => {}
            Stmt::Assign(target, value) => {
                for access in &target.path {
                    if let super::ast::Access::Index(index) = access {
                        self.expr(index);
                    }
                }
                self.expr(value);
            }
            Stmt::Case {
                selector,
                arms,
                otherwise,
            } => {
                self.expr(selector);
                for (labels, body) in arms {
                    labels.iter().for_each(|l| self.expr(l));
                    self.stmt(body);
                }
                if let Some(body) = otherwise {
                    self.stmt(body);
                }
            }
            Stmt::Compound(body) => body.iter().for_each(|s| self.stmt(s)),
            Stmt::If {
                condition,
                then,
                otherwise,
            } => {
                self.expr(condition);
                then.iter().chain(otherwise).for_each(|s| self.stmt(s));
            }
            Stmt::Repeat {
                increment,
                while_,
                until,
                body,
            } => {
                if let Some(increment) = increment {
                    self.expr(&increment.from);
                    self.expr(&increment.to);
                    if let Some(by) = &increment.by {
                        self.expr(by);
                    }
                }
                while_.iter().chain(until).for_each(|e| self.expr(e));
                body.iter().for_each(|s| self.stmt(s));
            }
            Stmt::Return(value) => value.iter().for_each(|e| self.expr(e)),
            Stmt::Call(name, args) => {
                self.refused.insert(format!("the procedure {name}"));
                args.iter().for_each(|e| self.expr(e));
            }
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Call(callee, args) => {
                match callee {
                    Callee::Builtin(builtin) if builtin.unsupported() => {
                        self.refused
                            .insert(format!("the built-in {}", builtin_name(*builtin)));
                    }
                    Callee::Function(index) => {
                        self.calls.insert(*index);
                    }
                    Callee::Type(_) if args.len() != 1 => {
                        self.refused
                            .insert("a type constructor with other than one argument".into());
                    }
                    Callee::Unresolved(name) => {
                        self.refused.insert(format!("the unresolved call {name}"));
                    }
                    _ => {}
                }
                args.iter().for_each(|e| self.expr(e));
            }
            Expr::Binary(BinOp::Like, left, right) => {
                self.refused.insert("the LIKE operator".into());
                self.expr(left);
                self.expr(right);
            }
            Expr::Binary(BinOp::Complex, left, right) => {
                for side in [left, right] {
                    if !matches!(
                        **side,
                        Expr::Call(Callee::Entity(_), _) | Expr::Binary(BinOp::Complex, ..)
                    ) {
                        self.refused
                            .insert("`||` over something other than entity constructors".into());
                    }
                    self.expr(side);
                }
            }
            Expr::Binary(_, left, right) => {
                self.expr(left);
                self.expr(right);
            }
            Expr::Unary(_, operand) => self.expr(operand),
            Expr::Attribute(base, _) | Expr::Group(base, _) => self.expr(base),
            Expr::Index(base, index, high) => {
                self.expr(base);
                self.expr(index);
                if let Some(high) = high {
                    self.expr(high);
                }
            }
            Expr::Aggregate(elems) => {
                for elem in elems {
                    self.expr(&elem.value);
                    if let Some(repeat) = &elem.repeat {
                        self.expr(repeat);
                    }
                }
            }
            Expr::Interval {
                low, item, high, ..
            } => {
                self.expr(low);
                self.expr(item);
                self.expr(high);
            }
            Expr::Query {
                source, condition, ..
            } => {
                self.expr(source);
                self.expr(condition);
            }
            Expr::Name(name) => {
                self.refused.insert(format!("the unresolved name {name}"));
            }
            _ => {}
        }
    }
}

fn builtin_name(builtin: Builtin) -> String {
    format!("{builtin:?}").to_ascii_uppercase()
}
