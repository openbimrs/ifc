//! One positional attribute slot on an IFC entity, and aggregate shapes.
//!
//! This crate owns the types rather than re-exporting the EXPRESS
//! extractor's: the bundled tables are decoded straight into them, so a
//! release of the parser never changes this crate's public API.

/// The four EXPRESS aggregation types (ISO 10303-11 §8.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AggregateKind {
    /// `LIST`: ordered, duplicates allowed unless `UNIQUE`.
    List,
    /// `SET`: unordered, no duplicates.
    Set,
    /// `BAG`: unordered, duplicates allowed.
    Bag,
    /// `ARRAY`: fixed-size, indexed by its bounds.
    Array,
}

/// One bound of an aggregation, as declared.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Bound {
    /// An integer literal.
    Integer(u64),
    /// `?`: no upper limit.
    Unbounded,
    /// Any other bound, a reference to another attribute such as
    /// `SELF\IfcBSplineCurve.UpperIndexOnControlPoints`, kept as written
    /// with whitespace normalised. It is not evaluated.
    Expression(String),
}

impl Bound {
    /// The bound as an integer, when it is a literal.
    #[must_use]
    pub const fn as_integer(&self) -> Option<u64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }
}

/// One aggregation level of a declared type, e.g. `LIST [1:3] OF ...`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Aggregation {
    /// Which aggregation type.
    pub kind: AggregateKind,
    /// Lower bound. `0` when a `LIST`, `SET` or `BAG` omits its bounds,
    /// which ISO 10303-11 defines as `[0:?]`.
    pub lower: Bound,
    /// Upper bound; [`Bound::Unbounded`] for `?` or omitted bounds.
    pub upper: Bound,
    /// Whether the elements are declared `UNIQUE` (`OF UNIQUE ...`). A `SET`
    /// is unique by definition whether or not this is set.
    pub unique: bool,
    /// Whether an `ARRAY`'s elements are declared `OPTIONAL`.
    pub optional_elements: bool,
}

impl Aggregation {
    /// Creates an aggregation level with explicit bounds.
    #[must_use]
    pub const fn new(kind: AggregateKind, lower: Bound, upper: Bound) -> Self {
        Self {
            kind,
            lower,
            upper,
            unique: false,
            optional_elements: false,
        }
    }

    /// Marks the elements `UNIQUE`.
    #[must_use]
    pub const fn unique(mut self) -> Self {
        self.unique = true;
        self
    }

    /// Marks an `ARRAY`'s elements `OPTIONAL`.
    #[must_use]
    pub const fn optional_elements(mut self) -> Self {
        self.optional_elements = true;
        self
    }

    /// Whether duplicate elements are forbidden: `UNIQUE`, or a `SET`.
    #[must_use]
    pub const fn forbids_duplicates(&self) -> bool {
        self.unique || matches!(self.kind, AggregateKind::Set)
    }
}

/// One explicit positional attribute declared by an entity.
///
/// `#[non_exhaustive]`: build one with [`Attribute::new`] and the builder
/// methods. Further facts about a declaration are added as new fields
/// without breaking existing readers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Attribute {
    /// Declared attribute name.
    pub name: String,
    /// Declared scalar type, or the innermost element type of an aggregate:
    /// `IfcLengthMeasure` for `LIST [1:?] OF LIST [3:3] OF IfcLengthMeasure`.
    pub type_name: String,
    /// Whether `OPTIONAL` was present.
    pub optional: bool,
    /// Whether a `LIST`, `SET`, `ARRAY`, or `BAG` wrapper was present.
    pub aggregate: bool,
    /// Aggregation levels with their bounds, outermost first; empty for a
    /// scalar. A table written before bounds were recorded (artifact format
    /// 1 or 2) has `aggregate` set and this empty.
    pub aggregation: Vec<Aggregation>,
}

impl Attribute {
    /// Creates a required scalar attribute.
    #[must_use]
    pub fn new(name: impl Into<String>, type_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_name: type_name.into(),
            optional: false,
            aggregate: false,
            aggregation: Vec::new(),
        }
    }

    /// Marks the attribute as optional.
    #[must_use]
    pub const fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    /// Marks the attribute as an aggregate without recording its shape.
    ///
    /// Prefer [`Self::with_aggregation`], which records kind and bounds.
    #[must_use]
    pub const fn aggregate(mut self) -> Self {
        self.aggregate = true;
        self
    }

    /// Wraps the type in one more aggregation level, inside any already
    /// added: call it outermost first. Also sets [`Self::aggregate`].
    #[must_use]
    pub fn with_aggregation(mut self, aggregation: Aggregation) -> Self {
        self.aggregation.push(aggregation);
        self.aggregate = true;
        self
    }
}
