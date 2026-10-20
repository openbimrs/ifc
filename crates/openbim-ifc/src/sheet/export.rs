//! [`export`]: entities by value paths as a [`Sheet`].

use std::fmt;

use ifc_model::EntityId;

use super::{KeyColumn, Sheet, SheetOptions};
use crate::selector::{value_text, SelectedValue, SelectorError, SelectorIndex, ValuePath};

/// Why a table could not be exported.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ExportError {
    /// A value path could not be read for an entity.
    Value {
        /// The entity.
        entity: EntityId,
        /// The path as written.
        path: String,
        /// Why.
        error: Box<SelectorError>,
    },
    /// An entity has no key in the key column: no `GlobalId` (it is no
    /// `IfcRoot`, or states none). Export it keyed by `id`.
    NoKey {
        /// The entity.
        entity: EntityId,
        /// The key column.
        key: KeyColumn,
    },
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value {
                entity,
                path,
                error,
            } => write!(f, "#{} {path}: {error}", entity.0),
            Self::NoKey { entity, key } => write!(
                f,
                "#{} has no {}; key the table by id instead",
                entity.0,
                key.header()
            ),
        }
    }
}

impl std::error::Error for ExportError {}

/// `entities` as a sheet: the key column first (`GlobalId` unless
/// `options` names `id`), then one column per path, headed by the path's
/// canonical text; each cell is [`value_text`] of the value the path
/// reads, `NULL` as `options.null`.
///
/// A path equal to the key column is not repeated.
///
/// # Errors
///
/// [`ExportError::Value`] for the first value that cannot be read (an
/// ambiguous relationship, a property the resolver refuses);
/// [`ExportError::NoKey`] for an entity without a key.
pub fn export(
    index: &SelectorIndex<'_>,
    entities: &[EntityId],
    columns: &[ValuePath],
    options: &SheetOptions,
) -> Result<Sheet, ExportError> {
    let key = options.key.unwrap_or_default();
    let key_path = key.path();
    let columns: Vec<&ValuePath> = columns.iter().filter(|path| !key.is(path)).collect();
    let mut header = vec![key.header().to_owned()];
    header.extend(columns.iter().map(ToString::to_string));
    let mut rows = Vec::with_capacity(entities.len());
    for &entity in entities {
        let read = |path: &ValuePath| {
            index
                .value(entity, path)
                .map_err(|error| ExportError::Value {
                    entity,
                    path: path.to_string(),
                    error: Box::new(error),
                })
        };
        // An entity that declares no `GlobalId` reads the key as a property
        // set, which a build without `properties` refuses: it has no key.
        let key_value = match index.value(entity, &key_path) {
            Err(SelectorError::FeatureDisabled(_)) => SelectedValue::Null,
            other => other.map_err(|error| ExportError::Value {
                entity,
                path: key_path.to_string(),
                error: Box::new(error),
            })?,
        };
        let key_text = match key_value {
            SelectedValue::Text(text) if !text.is_empty() => text,
            SelectedValue::Integer(id) => id.to_string(),
            _ => return Err(ExportError::NoKey { entity, key }),
        };
        let mut row = Vec::with_capacity(columns.len() + 1);
        row.push(key_text);
        for path in &columns {
            row.push(value_text(&read(path)?, &options.null));
        }
        rows.push(row);
    }
    Ok(Sheet { header, rows })
}
