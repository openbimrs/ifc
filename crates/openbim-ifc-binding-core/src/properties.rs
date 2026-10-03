//! Property sets, quantities and units (feature `properties`, #123).
//!
//! The facade's exact, release-bound resolver (`ifc::properties::
//! exact_properties`) answers for one object: its own sets, then the sets
//! its type object holds, an occurrence property overriding an inherited
//! one of the same set and name. Each set and property crosses as an owned
//! record keyed by entity id, with the set's `GlobalId`. Values keep their
//! declared IFC type in the tagged encoding: a quantity's bare `2.5` read
//! as `IfcLengthMeasure` crosses as `typed IFCLENGTHMEASURE(real 2.5)`, a
//! label as `typed IFCLABEL(text ...)`, and `.U.` stays `unknown`.
//!
//! The release is the one the header declares, IFC2X3, IFC4 or IFC4X3;
//! any other is refused with `unsupported-schema`, and a model read
//! leniently with skipped records with `invalid-model`, since an exact
//! answer cannot be proven from an incomplete file.
//!
//! The records are shaped for the checked edit of part 2 of #123: a
//! property is addressed by its object, set name and property name, and its
//! `value` is exactly the tagged value an edit would write back.

use crate::record::{Field, Record, ToRecord};
use crate::value::Tagged;
use crate::{BindingError, IfcModel};

/// One property set, quantity set or predefined property set applying to
/// an object.
#[derive(Debug, Clone, PartialEq)]
pub struct PropertySet {
    /// The set's entity id.
    pub id: u64,
    /// The set's `GlobalId`.
    pub global_id: Option<String>,
    /// The set's `Name`; a predefined set that states none is named by its
    /// entity (`IfcDoorLiningProperties`).
    pub name: String,
    /// The set's entity type, upper-case: `IFCPROPERTYSET`,
    /// `IFCELEMENTQUANTITY`, or a predefined set such as
    /// `IFCDOORLININGPROPERTIES`.
    pub type_name: String,
    /// `occurrence`: stated on the queried object; `type`: held by a type
    /// object's `HasPropertySets` (inherited, or the queried type's own).
    pub source: String,
    /// For `type`, the type object's id.
    pub source_id: Option<u64>,
    /// The set's properties and quantities, in file order. An inherited
    /// property overridden by the occurrence is left out.
    pub properties: Vec<Property>,
}

/// One property, quantity or predefined-set attribute.
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    /// The property or quantity entity; for a predefined set's attribute,
    /// which is no entity, the set's id.
    pub id: u64,
    /// `Name`, or the attribute name of a predefined set.
    pub name: String,
    /// The entity type of [`Self::id`], upper-case:
    /// `IFCPROPERTYSINGLEVALUE`, `IFCQUANTITYLENGTH`, ...
    pub type_name: String,
    /// The value form: `value` (one value, `null` when unset), `enumerated`,
    /// `list`, `bounded`, `table`, `reference` or `complex`.
    pub kind: String,
    /// The declared IFC value type (`IFCLENGTHMEASURE`), when there is one.
    pub value_type: Option<String>,
    /// The unit entity the property states (`Unit`); `None` when the
    /// project default applies or no unit does. Resolve either with
    /// [`IfcModel::resolve_unit`].
    pub unit: Option<u64>,
    /// `value`: the value, typed. `enumerated`: the selected values as a
    /// `list`. `list`: the values as a `list`. `reference`: the target as a
    /// `ref`, or `null`. `bounded`, `table`, `complex`: `null`, see the
    /// fields below.
    pub value: Tagged,
    /// `enumerated`: the `IfcPropertyEnumeration` the values come from.
    pub enumeration: Option<Enumeration>,
    /// `bounded`: the bounds and set point.
    pub bounds: Option<Bounds>,
    /// `table`: the rows and units.
    pub table: Option<Table>,
    /// `reference`: `UsageName`; `complex`: `UsageName` or `Usage`.
    pub usage: Option<String>,
    /// `complex` quantity: `Discrimination`.
    pub discrimination: Option<String>,
    /// `complex` quantity: `Quality`.
    pub quality: Option<String>,
    /// `complex`: its members, each resolved as a property.
    pub members: Vec<Property>,
}

/// An `IfcPropertyEnumeration`.
#[derive(Debug, Clone, PartialEq)]
pub struct Enumeration {
    /// Its entity id.
    pub id: u64,
    /// Its `Name`.
    pub name: String,
    /// The permitted values, typed.
    pub values: Vec<Tagged>,
}

/// The values of an `IfcPropertyBoundedValue`; `null` when unstated.
#[derive(Debug, Clone, PartialEq)]
pub struct Bounds {
    /// `LowerBoundValue`.
    pub lower: Tagged,
    /// `UpperBoundValue`.
    pub upper: Tagged,
    /// `SetPointValue` (IFC4, IFC4X3).
    pub set_point: Tagged,
}

/// An `IfcPropertyTableValue`.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    /// `(defining, defined)` value pairs, typed, in file order.
    pub rows: Vec<(Tagged, Tagged)>,
    /// `Expression`.
    pub expression: Option<String>,
    /// `DefiningUnit`.
    pub defining_unit: Option<u64>,
    /// `DefinedUnit`.
    pub defined_unit: Option<u64>,
    /// `CurveInterpolation` (IFC4, IFC4X3), e.g. `LINEAR`.
    pub interpolation: Option<String>,
}

/// A measure's effective unit, resolved to SI: `si = value * scale +
/// offset`.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedUnit {
    /// The unit entity that applies; `None` for a dimensionless measure.
    pub unit: Option<u64>,
    /// Whether `unit` is the project default rather than the stated one.
    pub from_project: bool,
    /// SI dimensional exponents `[L, M, T, I, Θ, N, J]`.
    pub dimensions: [i32; 7],
    /// Multiplier into the SI base unit.
    pub scale: f64,
    /// Added after scaling; nonzero only for degrees Celsius.
    pub offset: f64,
}

impl IfcModel {
    /// Every property set, quantity set and predefined property set that
    /// applies to `object`, with their properties, resolved exactly.
    ///
    /// Occurrence sets come first, then the sets inherited from the
    /// object's type, each in assignment order. `object` may be a type
    /// object, which reads its own `HasPropertySets`. Refused with
    /// `missing-entity` for an id not in the model, `wrong-entity-type` for
    /// an entity that carries no property sets, `unsupported-schema` for a
    /// release other than IFC2X3, IFC4 or IFC4X3, `invalid-model` when the
    /// file's sets are malformed or ambiguous, and `feature-disabled`
    /// without the `properties` feature.
    pub fn property_sets(&self, object: u64) -> Result<Vec<PropertySet>, BindingError> {
        #[cfg(feature = "properties")]
        {
            read::property_sets(self, object)
        }
        #[cfg(not(feature = "properties"))]
        {
            let _ = object;
            Err(BindingError::FeatureDisabled("properties"))
        }
    }

    /// The effective unit of a value of `measure_type` (`IFCAREAMEASURE`):
    /// `unit` when given (a property's stated unit), otherwise the project
    /// default for that measure, resolved exactly to SI.
    ///
    /// Refused with `invalid-value` for a type that is no measure,
    /// `unsupported` for a measure with no verified SI correspondence (a
    /// monetary one) or an offset unit, `invalid-model` for a missing,
    /// duplicated or malformed unit, and `feature-disabled` without the
    /// `properties` feature.
    pub fn resolve_unit(
        &self,
        measure_type: &str,
        unit: Option<u64>,
    ) -> Result<ResolvedUnit, BindingError> {
        #[cfg(feature = "properties")]
        {
            read::resolve_unit(self, measure_type, unit)
        }
        #[cfg(not(feature = "properties"))]
        {
            let _ = (measure_type, unit);
            Err(BindingError::FeatureDisabled("properties"))
        }
    }
}

impl ToRecord for PropertySet {
    fn to_record(&self) -> Record {
        Record::new(
            "PropertySet",
            vec![
                ("id", Field::Id(self.id)),
                ("global_id", Field::text(self.global_id.clone())),
                ("name", Field::Text(self.name.clone())),
                ("type_name", Field::Text(self.type_name.clone())),
                ("source", Field::Text(self.source.clone())),
                ("source_id", Field::id(self.source_id)),
                ("properties", Field::records(&self.properties)),
            ],
        )
    }
}

impl ToRecord for Property {
    fn to_record(&self) -> Record {
        Record::new(
            "Property",
            vec![
                ("id", Field::Id(self.id)),
                ("name", Field::Text(self.name.clone())),
                ("type_name", Field::Text(self.type_name.clone())),
                ("kind", Field::Text(self.kind.clone())),
                ("value_type", Field::text(self.value_type.clone())),
                ("unit", Field::id(self.unit)),
                ("value", Field::Value(self.value.clone())),
                ("enumeration", Field::record(self.enumeration.as_ref())),
                ("bounds", Field::record(self.bounds.as_ref())),
                ("table", Field::record(self.table.as_ref())),
                ("usage", Field::text(self.usage.clone())),
                ("discrimination", Field::text(self.discrimination.clone())),
                ("quality", Field::text(self.quality.clone())),
                ("members", Field::records(&self.members)),
            ],
        )
    }
}

impl ToRecord for Enumeration {
    fn to_record(&self) -> Record {
        Record::new(
            "PropertyEnumeration",
            vec![
                ("id", Field::Id(self.id)),
                ("name", Field::Text(self.name.clone())),
                (
                    "values",
                    Field::List(self.values.iter().cloned().map(Field::Value).collect()),
                ),
            ],
        )
    }
}

impl ToRecord for Bounds {
    fn to_record(&self) -> Record {
        Record::new(
            "PropertyBounds",
            vec![
                ("lower", Field::Value(self.lower.clone())),
                ("upper", Field::Value(self.upper.clone())),
                ("set_point", Field::Value(self.set_point.clone())),
            ],
        )
    }
}

impl ToRecord for Table {
    fn to_record(&self) -> Record {
        let rows = self
            .rows
            .iter()
            .map(|(defining, defined)| {
                Field::Record(Record::new(
                    "PropertyTableRow",
                    vec![
                        ("defining", Field::Value(defining.clone())),
                        ("defined", Field::Value(defined.clone())),
                    ],
                ))
            })
            .collect();
        Record::new(
            "PropertyTable",
            vec![
                ("rows", Field::List(rows)),
                ("expression", Field::text(self.expression.clone())),
                ("defining_unit", Field::id(self.defining_unit)),
                ("defined_unit", Field::id(self.defined_unit)),
                ("interpolation", Field::text(self.interpolation.clone())),
            ],
        )
    }
}

impl ToRecord for ResolvedUnit {
    fn to_record(&self) -> Record {
        Record::new(
            "ResolvedUnit",
            vec![
                ("unit", Field::id(self.unit)),
                ("from_project", Field::Bool(self.from_project)),
                (
                    "dimensions",
                    Field::List(
                        self.dimensions
                            .iter()
                            .map(|exponent| Field::Int(i64::from(*exponent)))
                            .collect(),
                    ),
                ),
                ("scale", Field::Real(self.scale)),
                ("offset", Field::Real(self.offset)),
            ],
        )
    }
}

#[cfg(feature = "properties")]
pub(crate) mod read {
    use ifc::properties::{
        exact_properties, exact_unit, ExactComplexMember, ExactLogical, ExactProperty,
        ExactPropertyError, ExactSource, ExactTypedValue, ExactUnitError, ExactValue,
    };
    use ifc::EntityId;

    use super::{Bounds, Enumeration, Property, PropertySet, ResolvedUnit, Table};
    use crate::value::Tagged;
    use crate::{BindingError, IfcModel};

    pub(super) fn property_sets(
        model: &IfcModel,
        object: u64,
    ) -> Result<Vec<PropertySet>, BindingError> {
        if !model.inner.contains(EntityId(object)) {
            return Err(BindingError::MissingEntity(object));
        }
        let entries = exact_properties(&model.inner, EntityId(object)).map_err(property_error)?;
        let mut sets: Vec<PropertySet> = Vec::new();
        for entry in entries {
            let property = &entry.property;
            let (source, source_id) = source(property.source);
            let set_id = property.set_id.0;
            let record = convert(model, &entry.name, property.property_id.0, property)?;
            match sets
                .iter_mut()
                .find(|set| set.id == set_id && set.source_id == source_id)
            {
                Some(set) => set.properties.push(record),
                None => {
                    let identity = model.identity(set_id)?;
                    sets.push(PropertySet {
                        id: set_id,
                        global_id: identity.global_id,
                        name: property.property_set.to_string(),
                        type_name: model.type_of(set_id)?.to_owned(),
                        source: source.to_owned(),
                        source_id,
                        properties: vec![record],
                    });
                }
            }
        }
        Ok(sets)
    }

    pub(super) fn resolve_unit(
        model: &IfcModel,
        measure_type: &str,
        unit: Option<u64>,
    ) -> Result<ResolvedUnit, BindingError> {
        let resolved =
            exact_unit(&model.inner, measure_type, unit.map(EntityId)).map_err(unit_error)?;
        Ok(ResolvedUnit {
            unit: resolved.unit.map(|EntityId(id)| id),
            from_project: resolved.from_project,
            dimensions: resolved.dimensions,
            scale: resolved.scale,
            offset: resolved.offset,
        })
    }

    fn source(source: ExactSource) -> (&'static str, Option<u64>) {
        match source {
            ExactSource::Occurrence => ("occurrence", None),
            ExactSource::Type(id) => ("type", Some(id.0)),
            ExactSource::Material(id) => ("material", Some(id.0)),
            // A provenance added after this binding still names its holder
            // nowhere; say so rather than guess.
            _ => ("other", None),
        }
    }

    fn convert(
        model: &IfcModel,
        name: &str,
        id: u64,
        property: &ExactProperty,
    ) -> Result<Property, BindingError> {
        member(
            model,
            name,
            id,
            property.value_type.as_deref(),
            property.unit_id.map(|EntityId(id)| id),
            &property.value,
        )
    }

    fn member(
        model: &IfcModel,
        name: &str,
        id: u64,
        value_type: Option<&str>,
        unit: Option<u64>,
        value: &ExactValue,
    ) -> Result<Property, BindingError> {
        let mut property = Property {
            id,
            name: name.to_owned(),
            type_name: model.type_of(id)?.to_owned(),
            kind: "value".to_owned(),
            value_type: value_type.map(str::to_owned),
            unit,
            value: Tagged::Null,
            enumeration: None,
            bounds: None,
            table: None,
            usage: None,
            discrimination: None,
            quality: None,
            members: Vec::new(),
        };
        match value {
            ExactValue::Enumerated(enumerated) => {
                property.kind = "enumerated".to_owned();
                property.value = Tagged::List(typed_all(&enumerated.values)?);
                property.enumeration = enumerated
                    .enumeration
                    .as_ref()
                    .map(|e| -> Result<_, BindingError> {
                        Ok(Enumeration {
                            id: e.id.0,
                            name: e.name.to_string(),
                            values: typed_all(&e.values)?,
                        })
                    })
                    .transpose()?;
            }
            ExactValue::List(values) => {
                property.kind = "list".to_owned();
                property.value = Tagged::List(typed_all(values)?);
            }
            ExactValue::Bounded(bounded) => {
                property.kind = "bounded".to_owned();
                let bound = |value: &Option<ExactTypedValue>| {
                    value.as_ref().map_or(Ok(Tagged::Null), typed)
                };
                property.bounds = Some(Bounds {
                    lower: bound(&bounded.lower)?,
                    upper: bound(&bounded.upper)?,
                    set_point: bound(&bounded.set_point)?,
                });
            }
            ExactValue::Table(table) => {
                property.kind = "table".to_owned();
                property.table = Some(Table {
                    rows: table
                        .rows
                        .iter()
                        .map(|row| Ok((typed(&row.defining)?, typed(&row.defined)?)))
                        .collect::<Result<_, BindingError>>()?,
                    expression: table.expression.as_deref().map(str::to_owned),
                    defining_unit: table.defining_unit.map(|EntityId(id)| id),
                    defined_unit: table.defined_unit.map(|EntityId(id)| id),
                    interpolation: table.interpolation.as_deref().map(str::to_owned),
                });
            }
            ExactValue::Reference(reference) => {
                property.kind = "reference".to_owned();
                property.usage = reference.usage_name.as_deref().map(str::to_owned);
                property.value = reference
                    .target
                    .as_ref()
                    .map_or(Tagged::Null, |target| Tagged::Ref(target.id.0));
            }
            ExactValue::Complex(complex) => {
                property.kind = "complex".to_owned();
                property.usage = complex.usage.as_deref().map(str::to_owned);
                property.discrimination = complex.discrimination.as_deref().map(str::to_owned);
                property.quality = complex.quality.as_deref().map(str::to_owned);
                property.members = complex
                    .members
                    .iter()
                    .map(|m: &ExactComplexMember| {
                        member(
                            model,
                            &m.name,
                            m.id.0,
                            m.value_type.as_deref(),
                            m.unit_id.map(|EntityId(id)| id),
                            &m.value,
                        )
                    })
                    .collect::<Result<_, _>>()?;
            }
            scalar => {
                let payload = scalar_value(scalar)?;
                property.value = match (value_type, payload) {
                    // An unset value, an enumeration constant and an entity
                    // are never wrapped: `value_type` still names their
                    // declared type.
                    (_, payload @ (Tagged::Null | Tagged::Enum(_) | Tagged::Ref(_))) => payload,
                    (Some(value_type), payload) => Tagged::Typed {
                        type_name: value_type.to_owned(),
                        value: Box::new(payload),
                    },
                    (None, payload) => payload,
                };
            }
        }
        Ok(property)
    }

    /// A typed member of a list, bound, table or enumeration: a scalar by
    /// the resolver's contract, refused with `unsupported` otherwise rather
    /// than replaced.
    fn typed(value: &ExactTypedValue) -> Result<Tagged, BindingError> {
        Ok(Tagged::Typed {
            type_name: value.value_type.to_string(),
            value: Box::new(scalar_value(&value.value)?),
        })
    }

    fn typed_all(values: &[ExactTypedValue]) -> Result<Vec<Tagged>, BindingError> {
        values.iter().map(typed).collect()
    }

    /// The tagged payload of a scalar exact value.
    fn scalar_value(value: &ExactValue) -> Result<Tagged, BindingError> {
        Ok(match value {
            ExactValue::Null => Tagged::Null,
            ExactValue::Bool(value) => Tagged::Bool(*value),
            ExactValue::Logical(ExactLogical::True) => Tagged::Bool(true),
            ExactValue::Logical(ExactLogical::False) => Tagged::Bool(false),
            ExactValue::Logical(ExactLogical::Unknown) => Tagged::Unknown,
            ExactValue::Binary(digits) => Tagged::Binary(digits.to_string()),
            ExactValue::Integer(value) => Tagged::Integer(*value),
            ExactValue::Real(value) => Tagged::Real(*value),
            ExactValue::Text(text) => Tagged::Text(text.to_string()),
            ExactValue::Enum(token) => Tagged::Enum(token.to_string()),
            ExactValue::Entity(entity) => Tagged::Ref(entity.id.0),
            other => {
                return Err(BindingError::Unsupported(format!(
                    "a property value form this binding does not carry: {other:?}"
                )))
            }
        })
    }

    pub(crate) fn property_error(error: ExactPropertyError) -> BindingError {
        use ExactPropertyError as E;
        let detail = error.to_string();
        match error {
            E::MissingSchema => BindingError::UnsupportedSchema(String::new()),
            E::UnsupportedSchema { schema } => BindingError::UnsupportedSchema(schema),
            E::MultipleSchemas { .. } => BindingError::UnsupportedSchema(detail),
            E::MissingReference { .. } => BindingError::MissingReference(detail),
            E::InvalidQueryObject { .. } => BindingError::WrongEntityType(detail),
            E::ComplexCycle { .. } | E::ComplexTooDeep { .. } | E::ComplexBudgetExceeded { .. } => {
                BindingError::BudgetExceeded(detail)
            }
            E::UnsupportedDefinition { .. }
            | E::UnsupportedProperty { .. }
            | E::UnsupportedRelationship { .. }
            | E::UnsupportedUnit { .. } => BindingError::Unsupported(detail),
            _ => BindingError::InvalidModel(detail),
        }
    }

    pub(crate) fn unit_error(error: ExactUnitError) -> BindingError {
        use ExactUnitError as E;
        let detail = error.to_string();
        match error {
            E::Structure(error) => property_error(error),
            E::MeasureNotInSchema { .. } | E::NotAMeasure { .. } => {
                BindingError::InvalidValue(detail)
            }
            E::UnmappedMeasureType { .. }
            | E::UnsupportedUnit { .. }
            | E::UnsupportedOffset { .. } => BindingError::Unsupported(detail),
            E::CyclicConversion { .. } | E::ConversionChainTooDeep { .. } => {
                BindingError::BudgetExceeded(detail)
            }
            _ => BindingError::InvalidModel(detail),
        }
    }
}
