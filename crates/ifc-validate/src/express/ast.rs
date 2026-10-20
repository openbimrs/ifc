//! The parsed form of the EXPRESS declarations rules are evaluated from.
//!
//! Only what evaluation needs is kept: types, attributes, DERIVE and
//! INVERSE clauses, WHERE rules, FUNCTIONs and RULEs. Supertype
//! constraints and UNIQUE clauses are skipped: `structure` checks UNIQUE
//! from the bundled tables.
//!
//! Every name is upper-cased (`Name`); a declaration also keeps its
//! spelling for messages. After [`super::resolve`] has run, no
//! [`Expr::Name`] or [`Callee::Unresolved`] remains.

use std::sync::Arc;

/// An upper-cased identifier.
pub(crate) type Name = Arc<str>;

/// A variable slot in the frame of the expression's evaluation context.
pub(crate) type Slot = usize;

/// Kinds of aggregate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AggKind {
    List,
    Set,
    Bag,
    Array,
}

impl AggKind {
    pub(crate) fn keyword(self) -> &'static str {
        match self {
            Self::List => "LIST",
            Self::Set => "SET",
            Self::Bag => "BAG",
            Self::Array => "ARRAY",
        }
    }
}

/// The simple types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Simple {
    Integer,
    Real,
    Number,
    Boolean,
    Logical,
    String,
    Binary,
}

impl Simple {
    pub(crate) fn keyword(self) -> &'static str {
        match self {
            Self::Integer => "INTEGER",
            Self::Real => "REAL",
            Self::Number => "NUMBER",
            Self::Boolean => "BOOLEAN",
            Self::Logical => "LOGICAL",
            Self::String => "STRING",
            Self::Binary => "BINARY",
        }
    }
}

/// A type as written in a declaration.
#[derive(Debug, Clone)]
pub(crate) enum TypeExpr {
    Simple(Simple),
    Named(Name),
    Aggregate {
        kind: AggKind,
        lower: Option<Box<Expr>>,
        elem: Box<TypeExpr>,
    },
    Enumeration(Vec<Name>),
    Select(Vec<Name>),
    /// `GENERIC`, `GENERIC_ENTITY`, `AGGREGATE OF ...`: formal parameters.
    Generic,
}

/// Three-valued logic (ISO 10303-11 8.1.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Logical {
    False,
    Unknown,
    True,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnOp {
    Not,
    Neg,
    Plus,
}

/// Binary operators, by ISO 10303-11 Table 12 precedence group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BinOp {
    Pow,
    Mul,
    Div,
    IntDiv,
    Mod,
    And,
    Complex,
    Add,
    Sub,
    Or,
    Xor,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    InstEq,
    InstNe,
    In,
    Like,
}

/// The built-in functions and procedures (ISO 10303-11 clauses 15, 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Builtin {
    Abs,
    Acos,
    Asin,
    Atan,
    Blength,
    Cos,
    Exists,
    Exp,
    Format,
    Hibound,
    Hiindex,
    Length,
    Lobound,
    Loindex,
    Log,
    Log2,
    Log10,
    Nvl,
    Odd,
    Rolesof,
    Sin,
    Sizeof,
    Sqrt,
    Tan,
    Typeof,
    Usedin,
    Value,
    ValueIn,
    ValueUnique,
    Insert,
    Remove,
}

impl Builtin {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "ABS" => Self::Abs,
            "ACOS" => Self::Acos,
            "ASIN" => Self::Asin,
            "ATAN" => Self::Atan,
            "BLENGTH" => Self::Blength,
            "COS" => Self::Cos,
            "EXISTS" => Self::Exists,
            "EXP" => Self::Exp,
            "FORMAT" => Self::Format,
            "HIBOUND" => Self::Hibound,
            "HIINDEX" => Self::Hiindex,
            "LENGTH" => Self::Length,
            "LOBOUND" => Self::Lobound,
            "LOINDEX" => Self::Loindex,
            "LOG" => Self::Log,
            "LOG2" => Self::Log2,
            "LOG10" => Self::Log10,
            "NVL" => Self::Nvl,
            "ODD" => Self::Odd,
            "ROLESOF" => Self::Rolesof,
            "SIN" => Self::Sin,
            "SIZEOF" => Self::Sizeof,
            "SQRT" => Self::Sqrt,
            "TAN" => Self::Tan,
            "TYPEOF" => Self::Typeof,
            "USEDIN" => Self::Usedin,
            "VALUE" => Self::Value,
            "VALUE_IN" => Self::ValueIn,
            "VALUE_UNIQUE" => Self::ValueUnique,
            "INSERT" => Self::Insert,
            "REMOVE" => Self::Remove,
            _ => return None,
        })
    }

    /// Built-ins this evaluator does not implement; a rule reaching one is
    /// reported as not evaluated, by name.
    pub(crate) fn unsupported(self) -> bool {
        matches!(
            self,
            Self::Format | Self::Hibound | Self::Lobound | Self::Rolesof
        )
    }
}

/// What a call names.
#[derive(Debug, Clone)]
pub(crate) enum Callee {
    Unresolved(Name),
    Builtin(Builtin),
    /// A schema FUNCTION, by index into the declarations.
    Function(usize),
    /// A partial entity constructor, by entity index.
    Entity(usize),
    /// A defined-type constructor, by type index.
    Type(usize),
}

/// One element of an aggregate initializer: a value, optionally repeated.
#[derive(Debug, Clone)]
pub(crate) struct AggElem {
    pub value: Expr,
    pub repeat: Option<Expr>,
}

/// An expression.
#[derive(Debug, Clone)]
pub(crate) enum Expr {
    Int(i64),
    Real(f64),
    Str(Arc<str>),
    Bits(Arc<str>),
    Logical(Logical),
    Indeterminate,
    SelfRef,
    Pi,
    ConstE,
    /// An identifier before resolution.
    Name(Name),
    Var(Slot),
    EnumItem(Name),
    /// A global rule's `FOR` entity: every instance of it.
    Population(usize),
    Call(Callee, Vec<Expr>),
    Aggregate(Vec<AggElem>),
    Interval {
        low: Box<Expr>,
        low_strict: bool,
        item: Box<Expr>,
        high_strict: bool,
        high: Box<Expr>,
    },
    Query {
        var: Slot,
        source: Box<Expr>,
        condition: Box<Expr>,
    },
    Attribute(Box<Expr>, Name),
    Index(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    Group(Box<Expr>, Name),
    Unary(UnOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
}

/// One step of an assignment target after its variable.
#[derive(Debug, Clone)]
pub(crate) enum Access {
    Attribute(Name),
    Index(Expr),
    Group,
}

/// The left-hand side of `:=`.
#[derive(Debug, Clone)]
pub(crate) struct Target {
    pub name: Name,
    pub slot: Slot,
    pub path: Vec<Access>,
    /// The variable's aggregate type, for a whole-variable assignment.
    pub shape: Option<Box<Shape>>,
}

/// A REPEAT's increment control.
#[derive(Debug, Clone)]
pub(crate) struct Increment {
    pub name: Name,
    pub var: Slot,
    pub from: Expr,
    pub to: Expr,
    pub by: Option<Expr>,
}

/// A statement.
#[derive(Debug, Clone)]
pub(crate) enum Stmt {
    Null,
    Assign(Target, Expr),
    Case {
        selector: Expr,
        arms: Vec<(Vec<Expr>, Stmt)>,
        otherwise: Option<Box<Stmt>>,
    },
    Compound(Vec<Stmt>),
    Escape,
    Skip,
    If {
        condition: Expr,
        then: Vec<Stmt>,
        otherwise: Vec<Stmt>,
    },
    Repeat {
        increment: Option<Increment>,
        while_: Option<Expr>,
        until: Option<Expr>,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    /// `INSERT(...)` / `REMOVE(...)`, or a call this evaluator refuses.
    Call(Name, Vec<Expr>),
}

/// A labelled WHERE rule.
#[derive(Debug, Clone)]
pub(crate) struct DomainRule {
    pub label: Arc<str>,
    pub expr: Expr,
    /// Slots the expression needs (QUERY variables).
    pub frame: usize,
}

/// A defined, enumeration or select type.
#[derive(Debug, Clone)]
pub(crate) struct TypeDecl {
    pub name: Name,
    pub spelling: Arc<str>,
    pub underlying: TypeExpr,
    pub rules: Vec<DomainRule>,
}

/// An explicit attribute.
#[derive(Debug, Clone)]
pub(crate) struct Explicit {
    pub name: Name,
    pub spelling: Arc<str>,
    pub ty: TypeExpr,
    /// `SELF\Super.Name`: a redeclaration, which adds no slot.
    pub redeclares: Option<Name>,
}

/// A DERIVE attribute.
#[derive(Debug, Clone)]
pub(crate) struct Derived {
    pub name: Name,
    pub spelling: Arc<str>,
    pub expr: Expr,
    pub frame: usize,
}

/// An INVERSE attribute.
#[derive(Debug, Clone)]
pub(crate) struct Inverse {
    pub name: Name,
    pub entity: Name,
    pub for_attribute: Name,
    /// `SET` / `BAG` with its bounds, or `None` for a single entity.
    pub aggregate: Option<(AggKind, u64, Option<u64>)>,
}

/// An entity declaration.
#[derive(Debug, Clone)]
pub(crate) struct EntityDecl {
    pub name: Name,
    pub spelling: Arc<str>,
    pub supertypes: Vec<Name>,
    pub explicit: Vec<Explicit>,
    pub derived: Vec<Derived>,
    pub inverses: Vec<Inverse>,
    pub rules: Vec<DomainRule>,
}

/// A local variable of a function or rule.
#[derive(Debug, Clone)]
pub(crate) struct LocalVar {
    pub name: Name,
    pub slot: Slot,
    pub init: Option<Expr>,
    /// The variable's aggregate type: an aggregate assigned to it takes
    /// this kind (and an `ARRAY`'s lower index).
    pub shape: Option<Shape>,
}

/// The declared aggregate type of a variable or parameter: `[]` assigned
/// to a `SET` variable is a set, and indexes an `ARRAY [0:n]` from 0.
#[derive(Debug, Clone)]
pub(crate) struct Shape {
    pub kind: AggKind,
    /// An `ARRAY`'s lower index.
    pub lower: Option<Expr>,
}

/// A FUNCTION.
#[derive(Debug, Clone)]
pub(crate) struct FunctionDecl {
    pub name: Name,
    pub spelling: Arc<str>,
    pub params: Vec<Name>,
    /// Each parameter's aggregate type, parallel to `params`.
    pub param_shapes: Vec<Option<Shape>>,
    pub locals: Vec<LocalVar>,
    pub body: Vec<Stmt>,
    pub frame: usize,
}

/// A global RULE.
#[derive(Debug, Clone)]
pub(crate) struct RuleDecl {
    pub spelling: Arc<str>,
    pub entities: Vec<Name>,
    pub locals: Vec<LocalVar>,
    pub body: Vec<Stmt>,
    pub rules: Vec<DomainRule>,
    pub frame: usize,
}

/// One parsed schema.
#[derive(Debug, Clone, Default)]
pub(crate) struct SchemaDecl {
    pub name: Arc<str>,
    pub types: Vec<TypeDecl>,
    pub entities: Vec<EntityDecl>,
    pub functions: Vec<FunctionDecl>,
    pub rules: Vec<RuleDecl>,
}
