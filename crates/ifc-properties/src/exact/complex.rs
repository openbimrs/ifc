//! Complex properties and quantities as present composites (#208).
//!
//! ```text
//! IfcComplexProperty          2 = UsageName        3 = HasProperties
//! IfcPhysicalComplexQuantity  2 = HasQuantities    3 = Discrimination
//!                             4 = OPT Quality      5 = OPT Usage
//! ```
//!
//! The positions hold in IFC2X3 TC1, IFC4 ADD2 TC1 and IFC4X3 ADD2, and are
//! read by name from the bound release's table all the same. A complex has
//! no value of its own: it resolves to [`ExactValue::Complex`], whose
//! members resolve as members of a set do, nested complexes included.
//!
//! Nesting is bounded as the permissive reader bounds it (`nesting.rs`):
//! a member that re-enters the path, a complex nested deeper than
//! [`MAX_COMPLEX_DEPTH`] and more than [`MAX_COMPLEX_MEMBERS`] nested
//! member references in one read are refused, never cut short. Member
//! names must be unique where the release says so (`WR22` of
//! `IfcComplexProperty` in every release, `UniqueQuantityNames` of
//! `IfcPhysicalComplexQuantity` in IFC4 and IFC4X3).

use std::collections::BTreeSet;
use std::sync::Arc;

use ifc_model::{Entity, EntityId, Model, Value};
use ifc_schema::SchemaVersion;

use crate::nesting::{MAX_COMPLEX_DEPTH, MAX_COMPLEX_MEMBERS};

use super::quantity::{simple_quantity_value, slot};
use super::refs::{nonempty_refs_at, text_at};
use super::release::Release;
use super::set::simple_property_value;
use super::value::ResolvedValue;
use super::{ExactComplexMember, ExactComplexValue, ExactPropertyError, ExactValue};

/// Whether `entity` is an `IfcComplexProperty` or an
/// `IfcPhysicalComplexQuantity` in the bound release.
pub(super) fn is_complex(release: Release, entity: &Entity) -> bool {
    let type_name = entity.type_name.as_ref();
    release.schema.is_a(type_name, "IFCCOMPLEXPROPERTY")
        || release.schema.is_a(type_name, "IFCPHYSICALCOMPLEXQUANTITY")
}

/// The composite value of complex property or quantity `id`, whose arity
/// the caller confirmed.
///
/// # Errors
///
/// A malformed member list, name or label; a missing, foreign, mis-sized
/// or unnamed member, or one of the wrong family; a repeated member name
/// the release forbids; a cycle, too deep a nesting or an exhausted
/// budget; or any error of a member's value.
pub(super) fn complex_value(
    model: &Model,
    release: Release,
    id: EntityId,
    entity: &Entity,
) -> Result<ResolvedValue, ExactPropertyError> {
    let mut walk = Walk {
        model,
        release,
        path: Vec::new(),
        followed: 0,
    };
    Ok(ResolvedValue {
        value: ExactValue::Complex(walk.complex(id, entity)?),
        value_type: None,
        unit_id: None,
    })
}

/// The two complex families and how each names its parts.
#[derive(Clone, Copy)]
enum Family {
    Properties,
    Quantities,
}

impl Family {
    fn of(release: Release, entity: &Entity) -> Self {
        if release
            .schema
            .is_a(entity.type_name.as_ref(), "IFCCOMPLEXPROPERTY")
        {
            Self::Properties
        } else {
            Self::Quantities
        }
    }

    fn entity(self) -> &'static str {
        match self {
            Self::Properties => "IFCCOMPLEXPROPERTY",
            Self::Quantities => "IFCPHYSICALCOMPLEXQUANTITY",
        }
    }

    fn members(self) -> &'static str {
        match self {
            Self::Properties => "HasProperties",
            Self::Quantities => "HasQuantities",
        }
    }

    /// The supertype every member must be.
    fn member_type(self) -> &'static str {
        match self {
            Self::Properties => "IFCPROPERTY",
            Self::Quantities => "IFCPHYSICALQUANTITY",
        }
    }

    /// The release's label of the unique-member-name rule, if it has one.
    ///
    /// Only the releases exact resolution is verified for are named; any
    /// other is refused rather than given a neighbour's label.
    fn unique_names_rule(
        self,
        version: SchemaVersion,
    ) -> Result<Option<&'static str>, ExactPropertyError> {
        match (self, version) {
            (
                Self::Properties,
                SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3,
            ) => Ok(Some("WR22")),
            (Self::Quantities, SchemaVersion::Ifc2x3) => Ok(None),
            (Self::Quantities, SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3) => {
                Ok(Some("UniqueQuantityNames"))
            }
            (_, other) => Err(ExactPropertyError::UnsupportedSchema {
                schema: format!("{other:?}"),
            }),
        }
    }
}

/// One root read: the complexes from the root down, and the nested
/// member references followed so far.
struct Walk<'m> {
    model: &'m Model,
    release: Release,
    path: Vec<EntityId>,
    followed: usize,
}

impl<'m> Walk<'m> {
    fn complex(
        &mut self,
        id: EntityId,
        entity: &Entity,
    ) -> Result<ExactComplexValue, ExactPropertyError> {
        if self.path.len() >= MAX_COMPLEX_DEPTH {
            return Err(ExactPropertyError::ComplexTooDeep {
                complex: id,
                limit: MAX_COMPLEX_DEPTH,
            });
        }
        let family = Family::of(self.release, entity);
        let attribute = |name| {
            entity
                .attributes
                .get(slot(self.release, family.entity(), name))
        };
        let member_ids = nonempty_refs_at(id, attribute(family.members()), family.members())?;
        let (usage, discrimination, quality) = match family {
            Family::Properties => (
                Some(text_at(id, attribute("UsageName"), "UsageName")?.into()),
                None,
                None,
            ),
            Family::Quantities => (
                optional_text(id, attribute("Usage"), "Usage")?,
                Some(text_at(id, attribute("Discrimination"), "Discrimination")?.into()),
                optional_text(id, attribute("Quality"), "Quality")?,
            ),
        };
        self.path.push(id);
        let mut names = BTreeSet::new();
        let mut members = Vec::with_capacity(member_ids.len());
        for member_id in member_ids {
            self.followed += 1;
            if self.followed > MAX_COMPLEX_MEMBERS {
                return Err(ExactPropertyError::ComplexBudgetExceeded {
                    complex: id,
                    limit: MAX_COMPLEX_MEMBERS,
                });
            }
            if self.path.contains(&member_id) {
                return Err(ExactPropertyError::ComplexCycle {
                    complex: id,
                    member: member_id,
                });
            }
            let (member, name) = self.member(id, family, member_id)?;
            if !names.insert(name) {
                if let Some(rule) = family.unique_names_rule(self.release.version)? {
                    return Err(ExactPropertyError::InconsistentValues { entity: id, rule });
                }
            }
            let resolved = if is_complex(self.release, member) {
                ResolvedValue {
                    value: ExactValue::Complex(self.complex(member_id, member)?),
                    value_type: None,
                    unit_id: None,
                }
            } else {
                match family {
                    Family::Properties => {
                        simple_property_value(self.model, self.release, member_id, member)?
                    }
                    Family::Quantities => {
                        simple_quantity_value(self.model, self.release, member_id, member)?
                    }
                }
            };
            members.push(ExactComplexMember {
                name: name.into(),
                id: member_id,
                value_type: resolved.value_type,
                unit_id: resolved.unit_id,
                value: resolved.value,
            });
        }
        self.path.pop();
        Ok(ExactComplexValue {
            usage,
            discrimination,
            quality,
            members,
        })
    }

    /// Member `member_id` of complex `id`, checked as a set member is, with
    /// its `Name`.
    fn member(
        &self,
        id: EntityId,
        family: Family,
        member_id: EntityId,
    ) -> Result<(&'m Entity, &'m str), ExactPropertyError> {
        let schema = self.release.schema;
        let model: &'m Model = self.model;
        let member = model
            .get(member_id)
            .ok_or(ExactPropertyError::MissingReference {
                from: id,
                to: member_id,
            })?;
        if schema.entity(member.type_name.as_ref()).is_none() {
            return Err(self
                .release
                .not_in_schema(member_id, member.type_name.clone()));
        }
        if !schema.is_a(member.type_name.as_ref(), family.member_type()) {
            return Err(ExactPropertyError::UnsupportedProperty {
                entity: member_id,
                type_name: member.type_name.clone(),
            });
        }
        self.release.require_exact_slots(member_id, member)?;
        let name = text_at(member_id, member.attributes.first(), "Name")?;
        Ok((member, name))
    }
}

/// An `OPTIONAL IfcLabel`: `$`, or text.
fn optional_text(
    id: EntityId,
    value: Option<&Value>,
    attribute: &'static str,
) -> Result<Option<Arc<str>>, ExactPropertyError> {
    match value {
        Some(Value::Null) => Ok(None),
        value => text_at(id, value, attribute).map(|text| Some(text.into())),
    }
}
