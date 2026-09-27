//! Where in a file a finding applies.

use std::fmt;

use ifc_model::EntityId;

/// Where in the file a finding applies.
///
/// Deliberately not a string: a caller that wants to jump to the offending
/// attribute needs the entity and slot as data, and formatting is a
/// presentation choice made at the edge.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Path {
    /// The file as a whole, e.g. a header or global-rule finding.
    File,
    /// One entity instance.
    Entity(EntityId),
    /// One attribute slot of one entity.
    Attribute {
        /// The entity carrying the attribute.
        entity: EntityId,
        /// Zero-based slot index in Part 21 positional order.
        index: usize,
        /// The attribute's schema name, when the schema declares one.
        name: Option<String>,
    },
}

impl fmt::Display for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::File => formatter.write_str("<file>"),
            Self::Entity(id) => write!(formatter, "{id}"),
            Self::Attribute {
                entity,
                index,
                name,
            } => match name {
                Some(name) => write!(formatter, "{entity}.{name}"),
                None => write!(formatter, "{entity}[{index}]"),
            },
        }
    }
}

/// Total order over paths: file first, then by entity, then by slot.
///
/// Used by report sorting; kept beside `Path` so a new variant cannot be
/// added without confronting its ordering.
#[must_use]
pub(crate) fn path_key(path: &Path) -> (u64, usize) {
    match path {
        Path::File => (0, 0),
        Path::Entity(id) => (id.0, 0),
        Path::Attribute { entity, index, .. } => (entity.0, *index + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_attribute_path_names_the_slot_when_the_schema_does() {
        let named = Path::Attribute {
            entity: EntityId(7),
            index: 2,
            name: Some("Name".into()),
        };
        assert_eq!(named.to_string(), "#7.Name");
        let anonymous = Path::Attribute {
            entity: EntityId(7),
            index: 2,
            name: None,
        };
        assert_eq!(anonymous.to_string(), "#7[2]");
    }

    /// The file itself sorts before any entity.
    #[test]
    fn the_file_path_sorts_first() {
        assert!(path_key(&Path::File) < path_key(&Path::Entity(EntityId(1))));
    }
}
