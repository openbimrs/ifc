//! Binding every identifier in the parsed schema before evaluation.
//!
//! A variable becomes a frame slot, a call names a built-in, a FUNCTION, an
//! entity or a defined type, a bare enumeration item becomes an
//! [`Expr::EnumItem`], and a global rule's `FOR` entity becomes its
//! population. A name that binds to nothing is an [`ExpressError`] here,
//! not a surprise in the middle of a validation run.
//!
//! # One deliberate normalisation
//!
//! A string literal that starts with the schema's own name followed by `.`
//! is upper-cased. `TYPEOF` answers upper-case names (ISO 10303-11 15.25)
//! and EXPRESS compares strings exactly, so
//! `'IFC4X3_ADD2.IfcMappedItem' IN TYPEOF(x)` -- which IFC4X1, IFC4X2 and
//! IFC4X3 ADD2 write in `IfcPresentationLayerWithStyle` -- could never hold
//! and the rule would fail on every file using one. The schema authors'
//! intent is plain, and IfcOpenShell compares these names case-blind too.

use std::collections::{HashMap, HashSet};

use super::ast::{
    Access, Builtin, Callee, Expr, LocalVar, Name, SchemaDecl, Shape, Slot, Stmt, TypeExpr,
};
use super::error::ExpressError;

/// Names declared at schema level.
pub(crate) struct Globals {
    pub entities: HashMap<Name, usize>,
    pub types: HashMap<Name, usize>,
    pub functions: HashMap<Name, usize>,
    enum_types: HashSet<Name>,
    enum_items: HashSet<Name>,
    prefix: String,
}

impl Globals {
    pub(crate) fn new(schema: &SchemaDecl) -> Self {
        let index = |names: Vec<Name>| -> HashMap<Name, usize> {
            names
                .into_iter()
                .enumerate()
                .map(|(i, name)| (name, i))
                .collect()
        };
        let mut enum_types = HashSet::new();
        let mut enum_items = HashSet::new();
        for decl in &schema.types {
            if let TypeExpr::Enumeration(items) = &decl.underlying {
                enum_types.insert(decl.name.clone());
                enum_items.extend(items.iter().cloned());
            }
        }
        Self {
            entities: index(schema.entities.iter().map(|e| e.name.clone()).collect()),
            types: index(schema.types.iter().map(|t| t.name.clone()).collect()),
            functions: index(schema.functions.iter().map(|f| f.name.clone()).collect()),
            enum_types,
            enum_items,
            prefix: format!("{}.", schema.name.to_ascii_uppercase()),
        }
    }
}

/// Records each local's aggregate type from its declared type.
pub(crate) fn local_types(locals: &mut [LocalVar], types: Vec<TypeExpr>) {
    for (local, ty) in locals.iter_mut().zip(types) {
        local.shape = shape(ty);
    }
}

/// The aggregate shape a declared type gives a variable, if any.
pub(crate) fn shape(ty: TypeExpr) -> Option<Shape> {
    match ty {
        TypeExpr::Aggregate { kind, lower, .. } => Some(Shape {
            kind,
            lower: lower
                .filter(|_| kind == super::ast::AggKind::Array)
                .map(|lower| *lower),
        }),
        _ => None,
    }
}

/// Resolves every expression in `schema` in place.
pub(crate) fn schema(schema: &mut SchemaDecl, globals: &Globals) -> Result<(), ExpressError> {
    for decl in &mut schema.types {
        for rule in &mut decl.rules {
            let mut scope = Scope::new(globals, format!("{}.{}", decl.spelling, rule.label));
            scope.expr(&mut rule.expr)?;
            rule.frame = scope.max;
        }
    }
    let attributes: Vec<HashSet<Name>> = (0..schema.entities.len())
        .map(|index| entity_attributes(schema, globals, index))
        .collect();
    for (index, decl) in schema.entities.iter_mut().enumerate() {
        for derived in &mut decl.derived {
            let mut scope = Scope::new(globals, format!("{}.{}", decl.spelling, derived.spelling));
            scope.attributes = Some(&attributes[index]);
            scope.expr(&mut derived.expr)?;
            derived.frame = scope.max;
        }
        for rule in &mut decl.rules {
            let mut scope = Scope::new(globals, format!("{}.{}", decl.spelling, rule.label));
            scope.attributes = Some(&attributes[index]);
            scope.expr(&mut rule.expr)?;
            rule.frame = scope.max;
        }
    }
    for decl in &mut schema.functions {
        let mut scope = Scope::new(globals, decl.spelling.to_string());
        for (param, shape) in decl.params.iter().zip(&decl.param_shapes) {
            let slot = scope.declare(param.clone());
            if let Some(shape) = shape {
                // A parameter's array bounds name other parameters, which
                // the caller has already bound.
                scope.shapes.insert(slot, shape.clone());
            }
        }
        for shape in decl.param_shapes.iter_mut().flatten() {
            if let Some(lower) = &mut shape.lower {
                scope.expr(lower)?;
            }
        }
        scope.locals(&mut decl.locals)?;
        for stmt in &mut decl.body {
            scope.stmt(stmt)?;
        }
        decl.frame = scope.max;
    }
    for decl in &mut schema.rules {
        let mut scope = Scope::new(globals, decl.spelling.to_string());
        for entity in &decl.entities {
            let index = *globals
                .entities
                .get(entity)
                .ok_or_else(|| scope.unresolved(entity))?;
            scope.populations.push((entity.clone(), index));
        }
        scope.locals(&mut decl.locals)?;
        for stmt in &mut decl.body {
            scope.stmt(stmt)?;
        }
        for rule in &mut decl.rules {
            scope.expr(&mut rule.expr)?;
        }
        decl.frame = scope.max;
    }
    Ok(())
}

/// Every attribute name of entity `index`, inherited ones included: inside
/// an entity's WHERE and DERIVE clauses a bare name means `SELF.name`.
fn entity_attributes(schema: &SchemaDecl, globals: &Globals, index: usize) -> HashSet<Name> {
    let mut names = HashSet::new();
    let mut stack = vec![index];
    let mut seen = HashSet::new();
    while let Some(current) = stack.pop() {
        if !seen.insert(current) {
            continue;
        }
        let entity = &schema.entities[current];
        names.extend(entity.explicit.iter().map(|a| a.name.clone()));
        names.extend(entity.derived.iter().map(|a| a.name.clone()));
        names.extend(entity.inverses.iter().map(|a| a.name.clone()));
        stack.extend(
            entity
                .supertypes
                .iter()
                .filter_map(|name| globals.entities.get(name).copied()),
        );
    }
    names
}

struct Scope<'g> {
    globals: &'g Globals,
    /// The entity's attribute names, in an entity's WHERE or DERIVE.
    attributes: Option<&'g HashSet<Name>>,
    context: String,
    vars: Vec<(Name, Slot)>,
    next: Slot,
    max: usize,
    populations: Vec<(Name, usize)>,
    shapes: HashMap<Slot, Shape>,
}

impl<'g> Scope<'g> {
    fn new(globals: &'g Globals, context: String) -> Self {
        Self {
            globals,
            attributes: None,
            context,
            vars: Vec::new(),
            next: 0,
            max: 0,
            populations: Vec::new(),
            shapes: HashMap::new(),
        }
    }

    fn unresolved(&self, name: &str) -> ExpressError {
        ExpressError::Unresolved {
            context: self.context.clone(),
            name: name.to_owned(),
        }
    }

    fn declare(&mut self, name: Name) -> Slot {
        let slot = self.next;
        self.next += 1;
        self.max = self.max.max(self.next);
        self.vars.push((name, slot));
        slot
    }

    fn mark(&self) -> (usize, Slot) {
        (self.vars.len(), self.next)
    }

    fn restore(&mut self, (len, next): (usize, Slot)) {
        self.vars.truncate(len);
        self.next = next;
    }

    fn lookup(&self, name: &str) -> Option<Slot> {
        self.vars
            .iter()
            .rev()
            .find(|(declared, _)| &**declared == name)
            .map(|(_, slot)| *slot)
    }

    fn locals(&mut self, locals: &mut [LocalVar]) -> Result<(), ExpressError> {
        for local in locals.iter_mut() {
            local.slot = self.declare(local.name.clone());
        }
        for local in locals.iter_mut() {
            if let Some(init) = &mut local.init {
                self.expr(init)?;
            }
            if let Some(shape) = &mut local.shape {
                if let Some(lower) = &mut shape.lower {
                    self.expr(lower)?;
                }
                self.shapes.insert(local.slot, shape.clone());
            }
        }
        Ok(())
    }

    fn stmt(&mut self, stmt: &mut Stmt) -> Result<(), ExpressError> {
        match stmt {
            Stmt::Null | Stmt::Escape | Stmt::Skip => Ok(()),
            Stmt::Assign(target, value) => {
                target.slot = self
                    .lookup(&target.name)
                    .ok_or_else(|| self.unresolved(&target.name))?;
                for access in &mut target.path {
                    if let Access::Index(index) = access {
                        self.expr(index)?;
                    }
                }
                if target.path.is_empty() {
                    target.shape = self.shapes.get(&target.slot).cloned().map(Box::new);
                }
                self.expr(value)
            }
            Stmt::Case {
                selector,
                arms,
                otherwise,
            } => {
                self.expr(selector)?;
                for (labels, body) in arms {
                    for label in labels {
                        self.expr(label)?;
                    }
                    self.stmt(body)?;
                }
                if let Some(body) = otherwise {
                    self.stmt(body)?;
                }
                Ok(())
            }
            Stmt::Compound(body) => body.iter_mut().try_for_each(|s| self.stmt(s)),
            Stmt::If {
                condition,
                then,
                otherwise,
            } => {
                self.expr(condition)?;
                then.iter_mut().try_for_each(|s| self.stmt(s))?;
                otherwise.iter_mut().try_for_each(|s| self.stmt(s))
            }
            Stmt::Repeat {
                increment,
                while_,
                until,
                body,
            } => {
                let mark = self.mark();
                if let Some(increment) = increment {
                    self.expr(&mut increment.from)?;
                    self.expr(&mut increment.to)?;
                    if let Some(by) = &mut increment.by {
                        self.expr(by)?;
                    }
                    increment.var = self.declare(increment.name.clone());
                }
                if let Some(condition) = while_ {
                    self.expr(condition)?;
                }
                if let Some(condition) = until {
                    self.expr(condition)?;
                }
                body.iter_mut().try_for_each(|s| self.stmt(s))?;
                self.restore(mark);
                Ok(())
            }
            Stmt::Return(value) => value.as_mut().map_or(Ok(()), |v| self.expr(v)),
            Stmt::Call(_, args) => args.iter_mut().try_for_each(|a| self.expr(a)),
        }
    }

    fn expr(&mut self, expr: &mut Expr) -> Result<(), ExpressError> {
        match expr {
            Expr::Int(_)
            | Expr::Real(_)
            | Expr::Bits(_)
            | Expr::Logical(_)
            | Expr::Indeterminate
            | Expr::SelfRef
            | Expr::Pi
            | Expr::ConstE
            | Expr::Var(_)
            | Expr::EnumItem(_)
            | Expr::Population(_) => Ok(()),
            Expr::Str(text) => {
                if text.to_ascii_uppercase().starts_with(&self.globals.prefix) {
                    *text = text.to_ascii_uppercase().into();
                }
                Ok(())
            }
            Expr::Name(name) => {
                *expr = self.name(&name.clone())?;
                Ok(())
            }
            Expr::Attribute(base, item) => {
                if let Expr::Name(name) = &**base {
                    if self.lookup(name).is_none() && self.globals.enum_types.contains(name) {
                        *expr = Expr::EnumItem(item.clone());
                        return Ok(());
                    }
                }
                self.expr(base)
            }
            Expr::Group(base, _) => self.expr(base),
            Expr::Index(base, index, high) => {
                self.expr(base)?;
                self.expr(index)?;
                high.as_mut().map_or(Ok(()), |h| self.expr(h))
            }
            Expr::Unary(_, operand) => self.expr(operand),
            Expr::Binary(_, left, right) => {
                self.expr(left)?;
                self.expr(right)
            }
            Expr::Aggregate(elems) => {
                for elem in elems {
                    self.expr(&mut elem.value)?;
                    if let Some(repeat) = &mut elem.repeat {
                        self.expr(repeat)?;
                    }
                }
                Ok(())
            }
            Expr::Interval {
                low, item, high, ..
            } => {
                self.expr(low)?;
                self.expr(item)?;
                self.expr(high)
            }
            Expr::Query {
                source, condition, ..
            } => {
                self.expr(source)?;
                self.expr(condition)
            }
            Expr::Call(callee, args) => {
                if let Some(resolved) = self.call(callee, args)? {
                    *expr = resolved;
                }
                Ok(())
            }
        }
    }

    /// Resolves a call; a QUERY comes back as its own expression.
    fn call(
        &mut self,
        callee: &mut Callee,
        args: &mut Vec<Expr>,
    ) -> Result<Option<Expr>, ExpressError> {
        let Callee::Unresolved(name) = callee else {
            return Ok(None);
        };
        let name = name.clone();
        if &*name == "QUERY" && args.len() == 3 {
            let mut condition = args.pop().expect("three arguments");
            let mut source = args.pop().expect("three arguments");
            let Some(Expr::Name(var)) = args.pop() else {
                return Err(self.unresolved("QUERY variable"));
            };
            self.expr(&mut source)?;
            let mark = self.mark();
            let slot = self.declare(var);
            self.expr(&mut condition)?;
            self.restore(mark);
            return Ok(Some(Expr::Query {
                var: slot,
                source: Box::new(source),
                condition: Box::new(condition),
            }));
        }
        for arg in args.iter_mut() {
            self.expr(arg)?;
        }
        *callee = if let Some(builtin) = Builtin::from_name(&name) {
            Callee::Builtin(builtin)
        } else if let Some(index) = self.globals.functions.get(&name) {
            Callee::Function(*index)
        } else if let Some(index) = self.globals.entities.get(&name) {
            Callee::Entity(*index)
        } else if let Some(index) = self.globals.types.get(&name) {
            Callee::Type(*index)
        } else {
            return Err(self.unresolved(&name));
        };
        Ok(None)
    }

    fn name(&self, name: &Name) -> Result<Expr, ExpressError> {
        if let Some(slot) = self.lookup(name) {
            return Ok(Expr::Var(slot));
        }
        if self.attributes.is_some_and(|names| names.contains(name)) {
            return Ok(Expr::Attribute(Box::new(Expr::SelfRef), name.clone()));
        }
        if let Some((_, index)) = self.populations.iter().find(|(n, _)| n == name) {
            return Ok(Expr::Population(*index));
        }
        if self.globals.enum_items.contains(name) {
            return Ok(Expr::EnumItem(name.clone()));
        }
        Err(self.unresolved(name))
    }
}
