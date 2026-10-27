//! Selectors and values by path (#464): the facade's `ifc::selector`.
//!
//! [`IfcModel::select`] filters the model with a selector such as
//! `IfcWall, Pset_WallCommon.IsExternal=TRUE` (grammar
//! [`SELECTOR_GRAMMAR_VERSION`]); [`IfcModel::get_value`] reads a value by
//! path (`type.Name`, `Pset_WallCommon.FireRating`) and
//! [`IfcModel::set_value`] writes one, from a plain host value coerced as
//! [`IfcModel::set_attribute_by_name_plain`] coerces it for an attribute,
//! or wrapped in the type of the property already stated for a property.
//!
//! A selector or path that does not parse, and a class or attribute the
//! declared release does not declare, are refused with
//! `invalid-selector`, whose [`BindingError::span`] is the byte range of
//! the offending text in the UTF-8 query. Property facets and paths read
//! with feature `properties`, property writes need `properties-write`;
//! without them they refuse with `feature-disabled`.

use ifc::selector::{
    ParseError, SelectedValue, Selector, SelectorError, SelectorIndex, SetTarget, ValuePath,
};
use ifc::EntityId;

use crate::attribute::refused;
use crate::coerce::Plain;
use crate::value::Tagged;
use crate::{BindingError, IfcModel};

/// The version of the selector grammar the bindings parse.
pub const SELECTOR_GRAMMAR_VERSION: &str = ifc::selector::GRAMMAR_VERSION;

/// A value read by path, for a host: a scalar, a list, or a property set's
/// named values.
#[derive(Debug, Clone, PartialEq)]
pub enum Selected {
    /// A scalar, a reference or `$`; never a `list`.
    Value(Tagged),
    /// Several values.
    List(Vec<Selected>),
    /// A property set, complex property or bounded value: each member's
    /// name and value, in order.
    Properties(Vec<(String, Selected)>),
}

impl Selected {
    fn from_selected(value: SelectedValue) -> Self {
        match value {
            SelectedValue::Null => Self::Value(Tagged::Null),
            SelectedValue::Derived => Self::Value(Tagged::Derived),
            SelectedValue::Bool(b) => Self::Value(Tagged::Bool(b)),
            SelectedValue::Unknown => Self::Value(Tagged::Unknown),
            SelectedValue::Integer(i) => Self::Value(Tagged::Integer(i)),
            SelectedValue::Real(r) => Self::Value(Tagged::Real(r)),
            SelectedValue::Text(text) => Self::Value(Tagged::Text(text)),
            SelectedValue::Binary(digits) => Self::Value(Tagged::Binary(digits)),
            SelectedValue::Enum(item) => Self::Value(Tagged::Enum(item)),
            SelectedValue::Entity(EntityId(id)) => Self::Value(Tagged::Ref(id)),
            SelectedValue::List(items) => {
                Self::List(items.into_iter().map(Self::from_selected).collect())
            }
            SelectedValue::Properties(members) => Self::Properties(
                members
                    .into_iter()
                    .map(|(name, value)| (name, Self::from_selected(value)))
                    .collect(),
            ),
            // `SelectedValue` is non-exhaustive; a new form is reported,
            // never dropped, until it earns a form of its own.
            other => Self::Value(Tagged::Text(format!("{other:?}"))),
        }
    }

    /// The tagged form the C binding writes to its tape: a list as a
    /// `list`, a property set as a `list` of two-item `list`s, each the
    /// member's name (`text`) and its value.
    pub fn to_tagged(&self) -> Tagged {
        match self {
            Self::Value(value) => value.clone(),
            Self::List(items) => Tagged::List(items.iter().map(Self::to_tagged).collect()),
            Self::Properties(members) => Tagged::List(
                members
                    .iter()
                    .map(|(name, value)| {
                        Tagged::List(vec![Tagged::Text(name.clone()), value.to_tagged()])
                    })
                    .collect(),
            ),
        }
    }
}

/// `query` parsed, in the canonical form the grammar writes: spacing,
/// quoting and aliases normalised. Refused with `invalid-selector`.
pub fn canonical_selector(query: &str) -> Result<String, BindingError> {
    Selector::parse(query)
        .map(|selector| selector.to_string())
        .map_err(parse_error)
}

impl IfcModel {
    /// The entities `query` selects, in ascending id order.
    ///
    /// Refused with `invalid-selector` for a query that does not parse or
    /// names a class or attribute the declared release does not declare,
    /// `unsupported-schema`, `invalid-model` for an ambiguous or malformed
    /// relationship or a duplicated `GlobalId` an instance facet names,
    /// `missing-reference`, and the property refusals of
    /// [`Self::property_sets`].
    pub fn select(&self, query: &str) -> Result<Vec<u64>, BindingError> {
        let selector = Selector::parse(query).map_err(parse_error)?;
        let schema = self.declared_schema()?;
        SelectorIndex::new(&self.inner, schema)
            .select(&selector)
            .map(raw)
            .map_err(selector_error)
    }

    /// The entities `query` selects from `base` alone, in ascending id
    /// order. Refused as [`Self::select`], and with `missing-entity` for an
    /// id of `base` the model lacks.
    pub fn select_from(&self, query: &str, base: &[u64]) -> Result<Vec<u64>, BindingError> {
        let selector = Selector::parse(query).map_err(parse_error)?;
        let schema = self.declared_schema()?;
        let base: Vec<EntityId> = base.iter().copied().map(EntityId).collect();
        SelectorIndex::new(&self.inner, schema)
            .select_from(&selector, &base)
            .map(raw)
            .map_err(selector_error)
    }

    /// The value `path` reads from entity `id`. Refused with
    /// `invalid-selector` for a path that does not parse, `missing-entity`,
    /// and as [`Self::select`].
    pub fn get_value(&self, id: u64, path: &str) -> Result<Selected, BindingError> {
        let path = ValuePath::parse(path).map_err(parse_error)?;
        let schema = self.declared_schema()?;
        SelectorIndex::new(&self.inner, schema)
            .value(EntityId(id), &path)
            .map(Selected::from_selected)
            .map_err(selector_error)
    }

    /// Write `value` at `path` of entity `id`, as one transaction: an
    /// attribute (`Name`, `type.Name`, `materials.Category`) from a plain
    /// value coerced against its declared type, or a property
    /// (`Pset_WallCommon.FireRating`, `type.Pset_WallCommon.FireRating`)
    /// from a typed value, or a plain one the property already states the
    /// type of. Every check runs first: a refusal changes nothing.
    ///
    /// Refused with `invalid-value` for a path that names no single value
    /// (a pattern, `count`, a prefix reaching no entity), `unknown-attribute`,
    /// `derived-attribute`, `type-mismatch` (also for a plain value no
    /// stated property types), `ambiguous-value`, and the property edit's
    /// refusals. Also `unsupported` when the build or host has no random
    /// source for the seed (as [`crate::relations::new_guid`]); that is
    /// checked first, before `feature-disabled` for a property write.
    pub fn set_value(&mut self, id: u64, path: &str, value: Plain) -> Result<(), BindingError> {
        let seed = crate::relations::fresh_seed()?;
        self.tracked(|this| this.set_value_seeded(id, path, value, seed))
    }

    /// [`Self::set_value`]
    /// with the seed the `GlobalId`s of new sets and relationships derive
    /// from: a host with entropy of its own (the browser) passes it. The
    /// seed names the batch: a fixed seed reproduces its `GlobalId`s, also
    /// after a write/read cycle; reusing it where the model holds one it
    /// derives is refused (`invalid-value`).
    pub fn set_value_seeded(
        &mut self,
        id: u64,
        path: &str,
        value: Plain,
        seed: u128,
    ) -> Result<(), BindingError> {
        let path = ValuePath::parse(path).map_err(parse_error)?;
        let schema = self.declared_schema()?;
        let targets = SelectorIndex::new(&self.inner, schema)
            .set_targets(EntityId(id), &path)
            .map_err(selector_error)?;
        match targets.first() {
            Some(SetTarget::Attribute { entity, name }) => {
                let value = self.coerce_attribute(entity.0, name, value)?.into_value()?;
                let transaction = SelectorIndex::new(&self.inner, schema)
                    .stage_attributes(&targets, &value)
                    .map_err(selector_error)?;
                self.tracked(|this| transaction.commit(&mut this.inner))
                    .map(|_| ())
                    .map_err(|conflicts| BindingError::InvalidModel(format!("{conflicts:?}")))
            }
            Some(_) => self.tracked(|this| {
                write_properties(this, &targets, plain_tagged(value)?.into_value()?, seed)
            }),
            None => Err(BindingError::InvalidValue(format!(
                "{path} names no value to write"
            ))),
        }
    }
}

#[cfg(feature = "properties-write")]
fn write_properties(
    model: &mut IfcModel,
    targets: &[SetTarget],
    value: ifc::Value,
    seed: u128,
) -> Result<(), BindingError> {
    let schema = model.declared_schema()?;
    let edits = SelectorIndex::new(&model.inner, schema)
        .property_edits(targets, &value)
        .map_err(selector_error)?;
    ifc::apply_property_edits(&mut model.inner, &edits, seed)
        .map(|_| ())
        .map_err(crate::property_edit::write::edit_error)
}

#[cfg(not(feature = "properties-write"))]
fn write_properties(
    _model: &mut IfcModel,
    _targets: &[SetTarget],
    _value: ifc::Value,
    _seed: u128,
) -> Result<(), BindingError> {
    Err(BindingError::FeatureDisabled("properties-write"))
}

/// A plain host value as the tagged value a property edit takes: a typed
/// value as given, a bare one to be wrapped in the stated type.
fn plain_tagged(value: Plain) -> Result<Tagged, BindingError> {
    Ok(match value {
        Plain::Null => Tagged::Null,
        Plain::Bool(b) => Tagged::Bool(b),
        Plain::Integer(i) => Tagged::Integer(i),
        Plain::Real(r) => Tagged::Real(r),
        Plain::Text(text) => Tagged::Text(text),
        Plain::Ref(id) => Tagged::Ref(id),
        Plain::List(items) => Tagged::List(
            items
                .into_iter()
                .map(plain_tagged)
                .collect::<Result<_, _>>()?,
        ),
        Plain::Exact(tagged) => tagged,
    })
}

fn raw(ids: Vec<EntityId>) -> Vec<u64> {
    ids.into_iter().map(|EntityId(id)| id).collect()
}

pub(crate) fn parse_error(error: ParseError) -> BindingError {
    BindingError::InvalidSelector {
        detail: error.kind.to_string(),
        span: (error.span.start, error.span.end),
    }
}

pub(crate) fn selector_error(error: SelectorError) -> BindingError {
    let detail = error.to_string();
    match error {
        SelectorError::UnknownClass { span, .. } | SelectorError::UnknownAttribute { span, .. } => {
            BindingError::InvalidSelector {
                detail,
                span: (span.start, span.end),
            }
        }
        SelectorError::MissingEntity(EntityId(id)) => BindingError::MissingEntity(id),
        SelectorError::MissingReference { .. } => BindingError::MissingReference(detail),
        SelectorError::FeatureDisabled(feature) => BindingError::FeatureDisabled(feature),
        SelectorError::UnsupportedValue { .. } => BindingError::Unsupported(detail),
        SelectorError::NotSettable { .. } | SelectorError::ConflictingWrite { .. } => {
            BindingError::InvalidValue(detail)
        }
        SelectorError::Attribute(error) => refused(error),
        SelectorError::TypeMismatch { .. }
        | SelectorError::UntypedValue { .. }
        | SelectorError::InvalidText { .. } => BindingError::TypeMismatch(detail),
        #[cfg(feature = "properties")]
        SelectorError::Property { error, .. } => crate::properties::read::property_error(error),
        #[cfg(feature = "properties-write")]
        SelectorError::PropertyEdit(error) => crate::property_edit::write::edit_error(error),
        // Ambiguous and malformed relationships, a duplicated GlobalId, a
        // conflict, and any refusal added after this binding.
        _ => BindingError::InvalidModel(detail),
    }
}
