//! Entity descriptors used by the IFC schema registry.

use crate::attribute::{Aggregation, Attribute};

/// One `INVERSE` attribute: `Name : SET [0:1] OF Entity FOR Attribute;`.
///
/// Inverse attributes occupy no Part 21 slot; they state how many instances
/// of `entity` may point at this one through `for_attribute`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct InverseAttribute {
    /// Declared name, unqualified.
    pub name: String,
    /// The supertype named when a subtype redeclares an inherited inverse
    /// (`SELF\X.Name : ...`); `None` for a new inverse attribute.
    pub redeclares: Option<String>,
    /// The entity whose attribute points back at this one.
    pub entity: String,
    /// The attribute named after `FOR`, as written.
    pub for_attribute: String,
    /// `SET` or `BAG` with its bounds; `None` for a single-valued inverse.
    pub aggregation: Option<Aggregation>,
}

impl InverseAttribute {
    /// Creates a single-valued inverse attribute.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        entity: impl Into<String>,
        for_attribute: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            redeclares: None,
            entity: entity.into(),
            for_attribute: for_attribute.into(),
            aggregation: None,
        }
    }

    /// Marks the inverse a redeclaration of `supertype`'s.
    #[must_use]
    pub fn redeclaring(mut self, supertype: impl Into<String>) -> Self {
        self.redeclares = Some(supertype.into());
        self
    }

    /// Makes the inverse an aggregate (`SET` or `BAG`).
    #[must_use]
    pub fn with_aggregation(mut self, aggregation: Aggregation) -> Self {
        self.aggregation = Some(aggregation);
        self
    }
}

/// One `UNIQUE` rule: the named attributes are unique, jointly, across
/// every instance of the declaring entity and its subtypes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UniqueRule {
    /// Rule label, e.g. `UR1`; `None` when the rule is unlabelled.
    pub label: Option<String>,
    /// Attribute names as written; a qualified name (`SELF\X.Y`) is kept.
    pub attributes: Vec<String>,
}

impl UniqueRule {
    /// Creates a labelled rule over `attributes`.
    #[must_use]
    pub fn new(label: impl Into<String>, attributes: Vec<String>) -> Self {
        Self {
            label: Some(label.into()),
            attributes,
        }
    }

    /// Creates an unlabelled rule over `attributes`.
    #[must_use]
    pub const fn unlabelled(attributes: Vec<String>) -> Self {
        Self {
            label: None,
            attributes,
        }
    }
}

/// One `WHERE` rule: a named constraint an instance must satisfy.
///
/// The bundled tables carry the **label** only; [`Self::expression`] is empty
/// for them (the expressions are CC BY-ND schema text, see
/// `data/NOTICE.md`). A schema parsed from EXPRESS source with the `express`
/// feature keeps the whitespace-normalised expression.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct WhereRule {
    /// Rule label as declared, e.g. `CurveIs3D`.
    pub label: String,
    /// Constraint expression as written, or empty when not recorded.
    pub expression: String,
}

impl WhereRule {
    /// Creates a rule from its label and expression text.
    #[must_use]
    pub fn new(label: impl Into<String>, expression: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            expression: expression.into(),
        }
    }
}

/// One structural entity declaration.
///
/// `#[non_exhaustive]`: build one with [`EntityDef::new`] and the builder
/// methods. Declaration facts not yet recorded are added as new fields
/// without breaking readers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct EntityDef {
    /// Declared entity name.
    pub name: String,
    /// Direct supertypes in `SUBTYPE OF` order; empty for a root entity.
    ///
    /// IFC is single-inheritance, so this holds at most one name for every
    /// bundled schema; [`Self::supertype`] reads it.
    pub supertypes: Vec<String>,
    /// Whether the declaration includes `ABSTRACT`.
    pub abstract_: bool,
    /// Explicit attributes declared by this entity, excluding derived and
    /// inverse declarations and explicit redeclarations of inherited ones.
    pub attributes: Vec<Attribute>,
    /// Names of attributes this entity declares in its `DERIVE` block,
    /// unqualified (`SELF\Entity.` stripped), in declaration order.
    ///
    /// A redeclared inherited attribute keeps its position but is written
    /// `*` in a Part 21 record. New derived attributes appear here too; they
    /// occupy no positional slot.
    pub derived: Vec<String>,
    /// `WHERE` rules declared by this entity (not inherited ones), in
    /// declaration order.
    pub where_rules: Vec<WhereRule>,
    /// `INVERSE` attributes declared by this entity, in declaration order.
    /// Empty in a table written before they were recorded (artifact format
    /// 1 or 2).
    pub inverses: Vec<InverseAttribute>,
    /// `UNIQUE` rules declared by this entity (not inherited ones), in
    /// declaration order. Empty in a table written before they were
    /// recorded (artifact format 1 or 2).
    pub unique_rules: Vec<UniqueRule>,
}

impl EntityDef {
    /// Creates an empty concrete entity declaration.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            supertypes: Vec::new(),
            abstract_: false,
            attributes: Vec::new(),
            derived: Vec::new(),
            where_rules: Vec::new(),
            inverses: Vec::new(),
            unique_rules: Vec::new(),
        }
    }

    /// Appends an `INVERSE` attribute in declaration order.
    #[must_use]
    pub fn with_inverse(mut self, inverse: InverseAttribute) -> Self {
        self.inverses.push(inverse);
        self
    }

    /// Appends a `UNIQUE` rule in declaration order.
    #[must_use]
    pub fn with_unique_rule(mut self, rule: UniqueRule) -> Self {
        self.unique_rules.push(rule);
        self
    }

    /// Appends a direct supertype, after any already declared.
    #[must_use]
    pub fn with_supertype(mut self, supertype: impl Into<String>) -> Self {
        self.supertypes.push(supertype.into());
        self
    }

    /// Marks the declaration `ABSTRACT`.
    #[must_use]
    pub const fn abstract_entity(mut self) -> Self {
        self.abstract_ = true;
        self
    }

    /// Appends an explicit attribute in declaration order.
    #[must_use]
    pub fn with_attribute(mut self, attribute: Attribute) -> Self {
        self.attributes.push(attribute);
        self
    }

    /// Declares an attribute name as derived, as a `DERIVE` block would.
    #[must_use]
    pub fn with_derived(mut self, name: impl Into<String>) -> Self {
        self.derived.push(name.into());
        self
    }

    /// Appends a `WHERE` rule in declaration order.
    #[must_use]
    pub fn with_where_rule(mut self, rule: WhereRule) -> Self {
        self.where_rules.push(rule);
        self
    }

    /// The first direct supertype, if any.
    #[must_use]
    pub fn supertype(&self) -> Option<&str> {
        self.supertypes.first().map(String::as_str)
    }

    /// Whether `name` is declared derived by this entity.
    ///
    /// Comparison is ASCII case-insensitive.
    #[must_use]
    pub fn is_derived(&self, name: &str) -> bool {
        self.derived
            .iter()
            .any(|declared| declared.eq_ignore_ascii_case(name))
    }
}
