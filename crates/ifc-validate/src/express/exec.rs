//! Calls and statements (ISO 10303-11 clauses 9.5 and 13).
//!
//! A FUNCTION runs in its own frame: parameters, then locals initialised in
//! declaration order, then REPEAT and QUERY variables. Control leaves a
//! statement list normally or by RETURN, ESCAPE (out of the innermost
//! REPEAT) or SKIP (to its next iteration).
//!
//! An IF whose condition is UNKNOWN or `?` takes its ELSE branch; a REPEAT
//! whose bounds are `?` does not run; a WHILE or UNTIL control stops or
//! continues only on TRUE, as clause 13 prescribes.

use std::rc::Rc;

use super::ast::{Access, AggKind, Callee, Expr, Logical, Shape, Stmt, Target};
use super::eval::{logical, Cx};
use super::machine::{Fault, Machine, R};
use super::value::{Agg, Inst, Val};

/// How a statement ended.
pub(crate) enum Flow {
    Normal,
    Return(Val),
    Escape,
    Skip,
}

/// One evaluated step of an assignment target.
enum Step {
    Attribute(super::ast::Name),
    Index(i64),
}

impl Machine<'_> {
    pub(crate) fn call(&self, callee: &Callee, args: &[Expr], cx: &mut Cx) -> R<Val> {
        match callee {
            Callee::Builtin(builtin) => self.builtin(*builtin, args, cx),
            Callee::Entity(entity) => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval(arg, cx)?);
                }
                self.construct(vec![(*entity, values)])
            }
            Callee::Type(index) => {
                let [arg] = args else {
                    return Err(Fault::Unsupported(
                        "a type constructor with other than one argument".into(),
                    ));
                };
                let value = self.eval(arg, cx)?;
                Ok(Val::Typed(Rc::new((*index, value.bare().clone()))))
            }
            Callee::Function(index) => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval(arg, cx)?);
                }
                self.nested(|| self.function(*index, values))
            }
            Callee::Unresolved(name) => {
                Err(Fault::Unsupported(format!("the unresolved call {name}")))
            }
        }
    }

    fn function(&self, index: usize, args: Vec<Val>) -> R<Val> {
        let decl = &self.tables.schema.functions[index];
        if args.len() != decl.params.len() {
            return Err(Fault::Unsupported(format!(
                "{} called with {} arguments for {} parameters",
                decl.spelling,
                args.len(),
                decl.params.len()
            )));
        }
        let mut cx = Cx::new(Val::Indet, decl.frame);
        for (slot, value) in args.into_iter().enumerate() {
            cx.frame[slot] = value;
        }
        for (slot, shape) in decl.param_shapes.iter().enumerate() {
            if let Some(shape) = shape {
                let value = std::mem::replace(&mut cx.frame[slot], Val::Indet);
                cx.frame[slot] = self.shaped(value, shape, &mut cx)?;
            }
        }
        self.init_locals(&decl.locals, &mut cx)?;
        match self.block(&decl.body, &mut cx)? {
            Flow::Return(value) => Ok(value),
            _ => Ok(Val::Indet),
        }
    }

    pub(crate) fn init_locals(&self, locals: &[super::ast::LocalVar], cx: &mut Cx) -> R<()> {
        for local in locals {
            if let Some(init) = &local.init {
                let mut value = self.eval(init, cx)?;
                if let Some(shape) = &local.shape {
                    value = self.shaped(value, shape, cx)?;
                }
                cx.frame[local.slot] = value;
            }
        }
        Ok(())
    }

    /// An aggregate assigned to a variable of aggregate type: it takes the
    /// variable's kind, a SET drops repeated members, and an `ARRAY` takes
    /// its declared lower index.
    fn shaped(&self, value: Val, shape: &Shape, cx: &mut Cx) -> R<Val> {
        let Val::Agg(agg) = &value else {
            return Ok(value);
        };
        let lower = match (&shape.lower, shape.kind) {
            (Some(lower), AggKind::Array) => match self.eval(lower, cx)?.bare() {
                Val::Int(lower) => *lower,
                _ => return Ok(Val::Indet),
            },
            _ => 1,
        };
        if agg.kind == shape.kind && agg.lower == lower {
            return Ok(value);
        }
        let items = if shape.kind == AggKind::Set && agg.kind != AggKind::Set {
            self.dedupe(&agg.items)?
        } else {
            agg.items.clone()
        };
        Ok(Val::Agg(Rc::new(Agg {
            kind: shape.kind,
            lower,
            items,
        })))
    }

    pub(crate) fn block(&self, body: &[Stmt], cx: &mut Cx) -> R<Flow> {
        for stmt in body {
            match self.stmt(stmt, cx)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    fn stmt(&self, stmt: &Stmt, cx: &mut Cx) -> R<Flow> {
        self.step()?;
        match stmt {
            Stmt::Null => Ok(Flow::Normal),
            Stmt::Escape => Ok(Flow::Escape),
            Stmt::Skip => Ok(Flow::Skip),
            Stmt::Return(value) => Ok(Flow::Return(match value {
                Some(value) => self.eval(value, cx)?,
                None => Val::Indet,
            })),
            Stmt::Compound(body) => self.block(body, cx),
            Stmt::Assign(target, value) => {
                let value = self.eval(value, cx)?;
                self.assign(target, value, cx)?;
                Ok(Flow::Normal)
            }
            Stmt::If {
                condition,
                then,
                otherwise,
            } => {
                if logical(&self.eval(condition, cx)?)? == Logical::True {
                    self.block(then, cx)
                } else {
                    self.block(otherwise, cx)
                }
            }
            Stmt::Case {
                selector,
                arms,
                otherwise,
            } => {
                let selector = self.eval(selector, cx)?;
                if !selector.is_indet() {
                    for (labels, body) in arms {
                        for label in labels {
                            let label = self.eval(label, cx)?;
                            if self.value_eq(&selector, &label, 0)? == Logical::True {
                                return self.stmt(body, cx);
                            }
                        }
                    }
                }
                match otherwise {
                    Some(body) => self.stmt(body, cx),
                    None => Ok(Flow::Normal),
                }
            }
            Stmt::Repeat {
                increment,
                while_,
                until,
                body,
            } => self.repeat(
                increment.as_ref(),
                while_.as_ref(),
                until.as_ref(),
                body,
                cx,
            ),
            Stmt::Call(name, _) => Err(Fault::Unsupported(format!("the procedure {name}"))),
        }
    }

    fn repeat(
        &self,
        increment: Option<&super::ast::Increment>,
        while_: Option<&Expr>,
        until: Option<&Expr>,
        body: &[Stmt],
        cx: &mut Cx,
    ) -> R<Flow> {
        let mut counter = None;
        if let Some(increment) = increment {
            let from = self.eval(&increment.from, cx)?;
            let to = self.eval(&increment.to, cx)?;
            let by = match &increment.by {
                Some(by) => self.eval(by, cx)?,
                None => Val::Int(1),
            };
            let (Val::Int(from), Val::Int(to), Val::Int(by)) = (from.bare(), to.bare(), by.bare())
            else {
                return Ok(Flow::Normal);
            };
            if *by == 0 {
                return Err(Fault::Data("a REPEAT increment of zero".into()));
            }
            counter = Some((increment.var, *from, *to, *by));
        }
        let mut current = counter.map(|(_, from, _, _)| from);
        loop {
            self.step()?;
            if let (Some((var, _, to, by)), Some(value)) = (counter, current) {
                if (by > 0 && value > to) || (by < 0 && value < to) {
                    break;
                }
                cx.frame[var] = Val::Int(value);
            }
            if let Some(condition) = while_ {
                if logical(&self.eval(condition, cx)?)? != Logical::True {
                    break;
                }
            }
            match self.block(body, cx)? {
                Flow::Return(value) => return Ok(Flow::Return(value)),
                Flow::Escape => break,
                Flow::Normal | Flow::Skip => {}
            }
            if let Some(condition) = until {
                if logical(&self.eval(condition, cx)?)? == Logical::True {
                    break;
                }
            }
            if let (Some((_, _, _, by)), Some(value)) = (counter, current) {
                current = Some(value.checked_add(by).ok_or_else(super::eval::overflow)?);
            }
        }
        Ok(Flow::Normal)
    }

    fn assign(&self, target: &Target, value: Val, cx: &mut Cx) -> R<()> {
        let mut steps = Vec::with_capacity(target.path.len());
        for access in &target.path {
            match access {
                Access::Attribute(name) => steps.push(Step::Attribute(name.clone())),
                Access::Group => {}
                Access::Index(index) => match self.eval(index, cx)?.bare() {
                    Val::Int(at) => steps.push(Step::Index(*at)),
                    _ => return Err(Fault::Data("assignment at a non-integer index".into())),
                },
            }
        }
        let value = match &target.shape {
            Some(shape) => self.shaped(value, shape, cx)?,
            None => value,
        };
        assign_into(&mut cx.frame[target.slot], &steps, value)
    }
}

fn assign_into(target: &mut Val, steps: &[Step], value: Val) -> R<()> {
    let Some((first, rest)) = steps.split_first() else {
        *target = value;
        return Ok(());
    };
    match (first, target) {
        (Step::Index(at), Val::Agg(agg)) => {
            let agg = Rc::make_mut(agg);
            let position = usize::try_from(*at - agg.lower)
                .ok()
                .filter(|&p| p < agg.items.len())
                .ok_or_else(|| Fault::Data(format!("assignment at index {at}, out of bounds")))?;
            assign_into(&mut agg.items[position], rest, value)
        }
        (Step::Index(_), Val::Typed(typed)) => {
            assign_into(&mut Rc::make_mut(typed).1, steps, value)
        }
        (Step::Attribute(name), Val::Inst(Inst::Local(local))) => {
            let mut local = local.borrow_mut();
            let slot = local.attrs.entry(name.clone()).or_insert(Val::Indet);
            assign_into(slot, rest, value)
        }
        (Step::Attribute(_), Val::Inst(Inst::Model(_))) => Err(Fault::Unsupported(
            "assignment to an attribute of a file instance".into(),
        )),
        (_, other) => Err(Fault::Data(format!(
            "assignment into {}",
            other.kind_name()
        ))),
    }
}
