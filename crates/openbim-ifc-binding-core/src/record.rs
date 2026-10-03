//! The host-independent shape of a domain record (#123).
//!
//! A domain view borrows the model and cannot cross a language boundary,
//! so each domain operation returns an owned snapshot: typed Rust structs
//! for native callers, and the same data as a [`Record`] for the hosts. A
//! record is a named list of named fields in a fixed order, so every host
//! converts every domain with one function and the three cannot disagree
//! about a field:
//!
//! - JavaScript: a plain object; field names become camelCase, ids and
//!   [`Field::Int`] are `bigint`, counts and reals `number`, an absent
//!   value `undefined`.
//! - Python: a frozen dataclass named [`Record::name`], field names as
//!   written here (snake_case), ids `int`, absent `None`.
//! - C: one value tape, through [`Record::to_tagged`]: each record is a
//!   `LIST` of its field values in declaration order.
//!
//! IFC values travel as [`Field::Value`], in the lossless tagged encoding
//! (ADR 0013), never folded into a host number.

use crate::value::Tagged;

/// A named, ordered list of fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// The record type, e.g. `PropertySet`; the Python dataclass name and
    /// the TypeScript interface name.
    pub name: &'static str,
    /// Fields in declaration order: the order of the C tape.
    pub fields: Vec<(&'static str, Field)>,
}

/// One field value.
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    /// Absent: an optional field with no value.
    Null,
    /// An entity id (`bigint` in JavaScript, `REF` on the C tape).
    Id(u64),
    /// A boolean.
    Bool(bool),
    /// A non-negative count or index (`number` in JavaScript, `INTEGER` on
    /// the C tape).
    Count(usize),
    /// A 64-bit integer that is data, not a count (`bigint` in JavaScript).
    Int(i64),
    /// A real number that is a resolved parameter, not an IFC value.
    Real(f64),
    /// Text.
    Text(String),
    /// An IFC value in the tagged encoding.
    Value(Tagged),
    /// A list of fields.
    List(Vec<Field>),
    /// A nested record.
    Record(Record),
}

impl Record {
    /// A record of type `name` with `fields`.
    pub fn new(name: &'static str, fields: Vec<(&'static str, Field)>) -> Self {
        Self { name, fields }
    }

    /// The field named `name`, if the record has one.
    pub fn get(&self, name: &str) -> Option<&Field> {
        self.fields
            .iter()
            .find(|(field, _)| *field == name)
            .map(|(_, value)| value)
    }

    /// The C binding's tape form: a `LIST` of the field values in
    /// declaration order, nested records and lists as nested `LIST`s.
    pub fn to_tagged(&self) -> Tagged {
        Tagged::List(
            self.fields
                .iter()
                .map(|(_, field)| field.to_tagged())
                .collect(),
        )
    }
}

impl Field {
    /// An optional field.
    pub fn optional<T>(value: Option<T>, map: impl FnOnce(T) -> Field) -> Self {
        value.map_or(Field::Null, map)
    }

    /// Optional text.
    pub fn text(value: Option<impl Into<String>>) -> Self {
        Self::optional(value, |text| Field::Text(text.into()))
    }

    /// An optional id.
    pub fn id(value: Option<u64>) -> Self {
        Self::optional(value, Field::Id)
    }

    /// A list of ids.
    pub fn ids(values: impl IntoIterator<Item = u64>) -> Self {
        Field::List(values.into_iter().map(Field::Id).collect())
    }

    /// A list of records.
    pub fn records<T: ToRecord>(values: &[T]) -> Self {
        Field::List(
            values
                .iter()
                .map(|value| Field::Record(value.to_record()))
                .collect(),
        )
    }

    /// An optional record.
    pub fn record<T: ToRecord>(value: Option<&T>) -> Self {
        Self::optional(value, |value| Field::Record(value.to_record()))
    }

    /// The tape form of this field (see [`Record::to_tagged`]).
    pub fn to_tagged(&self) -> Tagged {
        match self {
            Field::Null => Tagged::Null,
            Field::Id(id) => Tagged::Ref(*id),
            Field::Bool(value) => Tagged::Bool(*value),
            // A count is far below i64::MAX.
            Field::Count(count) => Tagged::Integer(i64::try_from(*count).unwrap_or(i64::MAX)),
            Field::Int(value) => Tagged::Integer(*value),
            Field::Real(value) => Tagged::Real(*value),
            Field::Text(text) => Tagged::Text(text.clone()),
            Field::Value(value) => value.clone(),
            Field::List(items) => Tagged::List(items.iter().map(Field::to_tagged).collect()),
            Field::Record(record) => record.to_tagged(),
        }
    }
}

/// A snapshot that every host carries as a [`Record`].
pub trait ToRecord {
    /// This snapshot as a record.
    fn to_record(&self) -> Record;
}

/// Convert a list of snapshots, the shape every list-returning domain
/// operation hands to a host.
pub fn to_records<T: ToRecord>(values: &[T]) -> Vec<Record> {
    values.iter().map(ToRecord::to_record).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_crosses_as_a_list_of_its_field_values_in_order() {
        let record = Record::new(
            "Example",
            vec![
                ("id", Field::Id(7)),
                ("name", Field::text(None::<String>)),
                ("count", Field::Count(2)),
                (
                    "value",
                    Field::Value(Tagged::Typed {
                        type_name: "IFCLABEL".into(),
                        value: Box::new(Tagged::Text("x".into())),
                    }),
                ),
                ("ids", Field::ids([1, 2])),
            ],
        );
        assert_eq!(record.get("count"), Some(&Field::Count(2)));
        assert_eq!(
            record.to_tagged(),
            Tagged::List(vec![
                Tagged::Ref(7),
                Tagged::Null,
                Tagged::Integer(2),
                Tagged::Typed {
                    type_name: "IFCLABEL".into(),
                    value: Box::new(Tagged::Text("x".into())),
                },
                Tagged::List(vec![Tagged::Ref(1), Tagged::Ref(2)]),
            ])
        );
    }
}
