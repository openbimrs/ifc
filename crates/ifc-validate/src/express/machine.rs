//! The evaluation state shared by every rule run over one model.
//!
//! Derived values of file instances, populations and the reverse index are
//! computed once per model and cached here. Work is metered: each
//! expression node and statement is a step, each FUNCTION call or derived
//! attribute a level of depth, and running out of either is a typed
//! [`Fault::Budget`], never a hang or a stack overflow.

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use ifc_model::{EntityId, Model, ReverseIndex};

use super::ast::AggKind;
use super::options::{BudgetExceeded, EvalLimits};
use super::tables::Tables;
use super::value::{Agg, Inst, Val};

/// Why an evaluation stopped without a value.
#[derive(Debug, Clone)]
pub(crate) enum Fault {
    /// The rule needs something this evaluator does not implement.
    Unsupported(String),
    /// A limit was reached.
    Budget(BudgetExceeded),
    /// The file's data has a shape the rule cannot read.
    Data(String),
}

pub(crate) type R<T> = Result<T, Fault>;

/// Shared state for one model.
pub(crate) struct Machine<'a> {
    pub tables: &'a Tables,
    pub model: &'a Model,
    pub limits: EvalLimits,
    reverse: OnceCell<ReverseIndex>,
    pub derived: RefCell<HashMap<(EntityId, usize, usize), Val>>,
    /// Explicit attribute values of file instances, typed, by slot.
    pub explicit: RefCell<HashMap<(EntityId, usize), Val>>,
    /// Derived values being computed, to refuse a value that needs itself.
    pub deriving: RefCell<std::collections::HashSet<(EntityId, usize, usize)>>,
    populations: RefCell<HashMap<usize, Rc<Agg>>>,
    typeof_entity: RefCell<HashMap<usize, Val>>,
    typeof_type: RefCell<HashMap<usize, Val>>,
    entity_types: RefCell<HashMap<std::sync::Arc<str>, Option<usize>>>,
    steps: Cell<u64>,
    pub total_steps: Cell<u64>,
    depth: Cell<usize>,
}

impl<'a> Machine<'a> {
    pub(crate) fn new(tables: &'a Tables, model: &'a Model, limits: EvalLimits) -> Self {
        Self {
            tables,
            model,
            limits,
            reverse: OnceCell::new(),
            derived: RefCell::new(HashMap::new()),
            explicit: RefCell::new(HashMap::new()),
            deriving: RefCell::new(std::collections::HashSet::new()),
            populations: RefCell::new(HashMap::new()),
            typeof_entity: RefCell::new(HashMap::new()),
            typeof_type: RefCell::new(HashMap::new()),
            entity_types: RefCell::new(HashMap::new()),
            steps: Cell::new(0),
            total_steps: Cell::new(0),
            depth: Cell::new(0),
        }
    }

    /// Starts the step count of one rule on one instance.
    pub(crate) fn reset_steps(&self) {
        self.steps.set(0);
        self.depth.set(0);
    }

    /// Counts one step.
    pub(crate) fn step(&self) -> R<()> {
        let steps = self.steps.get() + 1;
        self.steps.set(steps);
        let total = self.total_steps.get() + 1;
        self.total_steps.set(total);
        if steps > self.limits.steps_per_rule {
            return Err(Fault::Budget(BudgetExceeded::Steps {
                limit: self.limits.steps_per_rule,
            }));
        }
        if total > self.limits.total_steps {
            return Err(Fault::Budget(BudgetExceeded::Total {
                limit: self.limits.total_steps,
            }));
        }
        Ok(())
    }

    /// Runs `body` one call level deeper.
    pub(crate) fn nested<T>(&self, body: impl FnOnce() -> R<T>) -> R<T> {
        let depth = self.depth.get() + 1;
        if depth > self.limits.max_depth {
            return Err(Fault::Budget(BudgetExceeded::Depth {
                limit: self.limits.max_depth,
            }));
        }
        self.depth.set(depth);
        let result = body();
        self.depth.set(depth - 1);
        result
    }

    pub(crate) fn reverse(&self) -> &ReverseIndex {
        self.reverse.get_or_init(|| ReverseIndex::build(self.model))
    }

    /// The entity index of a type name as the file spells it.
    pub(crate) fn entity_of_name(&self, type_name: &std::sync::Arc<str>) -> Option<usize> {
        if let Some(found) = self.entity_types.borrow().get(type_name) {
            return *found;
        }
        let found = self.tables.entity_index(type_name);
        self.entity_types
            .borrow_mut()
            .insert(type_name.clone(), found);
        found
    }

    /// The entity index of an instance.
    pub(crate) fn entity_of(&self, inst: &Inst) -> R<usize> {
        match inst {
            Inst::Local(local) => Ok(local.borrow().entity),
            Inst::Model(id) => {
                let entity = self.model.get(*id).ok_or_else(|| {
                    Fault::Data(format!("{id} is referenced but not in the file"))
                })?;
                self.entity_of_name(&entity.type_name).ok_or_else(|| {
                    Fault::Data(format!(
                        "{id} is an {}, which the schema does not declare",
                        entity.type_name
                    ))
                })
            }
        }
    }

    /// Every file instance of entity `index` and its subtypes, by id.
    pub(crate) fn population(&self, index: usize) -> Rc<Agg> {
        if let Some(found) = self.populations.borrow().get(&index) {
            return found.clone();
        }
        let mut ids: Vec<EntityId> = Vec::new();
        for (name, _) in self.model.type_histogram() {
            let Some(entity) = self.tables.entity_index(name) else {
                continue;
            };
            if self.tables.is_a(entity, index) {
                ids.extend(self.model.ids_of_type(name));
            }
        }
        ids.sort_unstable();
        let agg = Rc::new(Agg::new(
            AggKind::Set,
            ids.into_iter()
                .map(|id| Val::Inst(Inst::Model(id)))
                .collect(),
        ));
        self.populations.borrow_mut().insert(index, agg.clone());
        agg
    }

    /// `TYPEOF` of an entity, cached.
    pub(crate) fn typeof_entity(&self, index: usize) -> Val {
        if let Some(found) = self.typeof_entity.borrow().get(&index) {
            return found.clone();
        }
        let value = names(&self.tables.entity_typeof[index]);
        self.typeof_entity.borrow_mut().insert(index, value.clone());
        value
    }

    /// `TYPEOF` of a defined-type value, cached.
    pub(crate) fn typeof_type(&self, index: usize) -> Val {
        if let Some(found) = self.typeof_type.borrow().get(&index) {
            return found.clone();
        }
        let value = names(&self.tables.type_typeof[index]);
        self.typeof_type.borrow_mut().insert(index, value.clone());
        value
    }
}

fn names(names: &[std::sync::Arc<str>]) -> Val {
    Val::agg(
        AggKind::Set,
        names.iter().map(|name| Val::Str(name.clone())).collect(),
    )
}
