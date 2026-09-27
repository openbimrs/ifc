//! Authoring roles and addresses.
//!
//! Both addresses carry a "say something" rule: `IfcPostalAddress.WR1`
//! and `IfcTelecomAddress.MinimumDataProvided` each require at least
//! one contact field. An address with every slot unset parses and
//! reaches nobody, so it is refused rather than written.
//!
//! None of these are `IfcRoot` subtypes: they have no GlobalId and are
//! referenced from the person, organisation or actor that owns them.

use ifc_model::{EntityId, Value};

use crate::author::editor::{build_entity, text, validate_enum, ResourceEditor};
use crate::error::{ResourceError, ResourceResult};

/// Draft for one `IfcActorRole`.
#[derive(Debug, Clone, Copy)]
pub struct ActorRoleDraft<'a> {
    /// `Role`, an `IfcRoleEnum` token.
    pub role: &'a str,
    /// `UserDefinedRole`. Required when `role` is `USERDEFINED`.
    pub user_defined_role: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
}

/// Draft for one `IfcPostalAddress`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PostalAddressDraft<'a> {
    /// `Purpose`, an `IfcAddressTypeEnum` token.
    pub purpose: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `UserDefinedPurpose`. Required when `purpose` is `USERDEFINED`.
    pub user_defined_purpose: Option<&'a str>,
    /// `InternalLocation`: a room or desk within the building.
    pub internal_location: Option<&'a str>,
    /// `PostalBox`.
    pub postal_box: Option<&'a str>,
    /// `Town`.
    pub town: Option<&'a str>,
    /// `Region`.
    pub region: Option<&'a str>,
    /// `PostalCode`.
    pub postal_code: Option<&'a str>,
    /// `Country`.
    pub country: Option<&'a str>,
}

/// Draft for one `IfcTelecomAddress`.
#[derive(Debug, Clone, Copy, Default)]
pub struct TelecomAddressDraft<'a> {
    /// `Purpose`, an `IfcAddressTypeEnum` token.
    pub purpose: Option<&'a str>,
    /// `Description`.
    pub description: Option<&'a str>,
    /// `UserDefinedPurpose`. Required when `purpose` is `USERDEFINED`.
    pub user_defined_purpose: Option<&'a str>,
    /// `PagerNumber`.
    pub pager_number: Option<&'a str>,
    /// `WWWHomePageURL`.
    pub www_home_page_url: Option<&'a str>,
}

/// The `LIST [1:?]` attributes of an `IfcTelecomAddress`.
///
/// Each is optional as a whole but bounded `[1:?]` when present, so an
/// empty slice is written as absent rather than as an empty list.
#[derive(Debug, Clone, Copy, Default)]
pub struct TelecomLists<'a> {
    /// `TelephoneNumbers`.
    pub telephone: &'a [&'a str],
    /// `FacsimileNumbers`.
    pub facsimile: &'a [&'a str],
    /// `ElectronicMailAddresses`.
    pub email: &'a [&'a str],
    /// `MessagingIDs`.
    pub messaging: &'a [&'a str],
}

/// A `LIST [1:?]` written as absent when empty.
fn optional_list(values: &[&str]) -> Option<Value> {
    if values.is_empty() {
        return None;
    }
    Some(Value::List(
        values.iter().map(|v| Value::Text((*v).into())).collect(),
    ))
}

impl ResourceEditor<'_> {
    /// Stage an `IfcActorRole`.
    ///
    /// # Errors
    ///
    /// Refuses a token outside `IfcRoleEnum` and `USERDEFINED`
    /// without a `user_defined_role` (WR1).
    pub fn create_actor_role(&mut self, draft: ActorRoleDraft<'_>) -> ResourceResult<EntityId> {
        const ENTITY: &str = "IfcActorRole";
        validate_enum(self.schema, ENTITY, "Role", draft.role)?;
        if draft.role.eq_ignore_ascii_case("USERDEFINED")
            && draft
                .user_defined_role
                .is_none_or(|text| text.trim().is_empty())
        {
            return Err(ResourceError::SemanticViolation {
                entity: None,
                rule: "WR1",
            });
        }
        let entity = build_entity(
            self.schema,
            ENTITY,
            &[
                ("Role", Some(Value::Enum(draft.role.into()))),
                ("UserDefinedRole", draft.user_defined_role.map(text)),
                ("Description", draft.description.map(text)),
            ],
        )?;
        self.commit_create(entity)
    }

    /// Stage an `IfcPostalAddress`.
    ///
    /// `address_lines` is `LIST [1:?]` when present.
    ///
    /// # Errors
    ///
    /// Refuses a `purpose` outside `IfcAddressTypeEnum`, `USERDEFINED`
    /// without a `user_defined_purpose`, and an address with no
    /// locating field at all (WR1).
    pub fn create_postal_address(
        &mut self,
        draft: PostalAddressDraft<'_>,
        address_lines: &[&str],
    ) -> ResourceResult<EntityId> {
        const ENTITY: &str = "IfcPostalAddress";
        self.validate_address_purpose(ENTITY, draft.purpose, draft.user_defined_purpose)?;
        // WR1: an address that locates nothing reaches nobody.
        let locates = !address_lines.is_empty()
            || draft.internal_location.is_some()
            || draft.postal_box.is_some()
            || draft.postal_code.is_some()
            || draft.town.is_some()
            || draft.region.is_some()
            || draft.country.is_some();
        if !locates {
            return Err(ResourceError::SemanticViolation {
                entity: None,
                rule: "WR1",
            });
        }
        let entity = build_entity(
            self.schema,
            ENTITY,
            &[
                ("Purpose", draft.purpose.map(|p| Value::Enum(p.into()))),
                ("Description", draft.description.map(text)),
                ("UserDefinedPurpose", draft.user_defined_purpose.map(text)),
                ("InternalLocation", draft.internal_location.map(text)),
                ("AddressLines", optional_list(address_lines)),
                ("PostalBox", draft.postal_box.map(text)),
                ("Town", draft.town.map(text)),
                ("Region", draft.region.map(text)),
                ("PostalCode", draft.postal_code.map(text)),
                ("Country", draft.country.map(text)),
            ],
        )?;
        self.commit_create(entity)
    }

    /// Stage an `IfcTelecomAddress`.
    ///
    /// # Errors
    ///
    /// Refuses a `purpose` outside `IfcAddressTypeEnum`, `USERDEFINED`
    /// without a `user_defined_purpose`, and an address carrying no
    /// contact route at all (MinimumDataProvided).
    pub fn create_telecom_address(
        &mut self,
        draft: TelecomAddressDraft<'_>,
        lists: TelecomLists<'_>,
    ) -> ResourceResult<EntityId> {
        const ENTITY: &str = "IfcTelecomAddress";
        self.validate_address_purpose(ENTITY, draft.purpose, draft.user_defined_purpose)?;
        // MinimumDataProvided: at least one way to reach someone.
        let reachable = !lists.telephone.is_empty()
            || !lists.facsimile.is_empty()
            || !lists.email.is_empty()
            || !lists.messaging.is_empty()
            || draft.pager_number.is_some()
            || draft.www_home_page_url.is_some();
        if !reachable {
            return Err(ResourceError::SemanticViolation {
                entity: None,
                rule: "MinimumDataProvided",
            });
        }
        let entity = build_entity(
            self.schema,
            ENTITY,
            &[
                ("Purpose", draft.purpose.map(|p| Value::Enum(p.into()))),
                ("Description", draft.description.map(text)),
                ("UserDefinedPurpose", draft.user_defined_purpose.map(text)),
                ("TelephoneNumbers", optional_list(lists.telephone)),
                ("FacsimileNumbers", optional_list(lists.facsimile)),
                ("PagerNumber", draft.pager_number.map(text)),
                ("ElectronicMailAddresses", optional_list(lists.email)),
                ("WWWHomePageURL", draft.www_home_page_url.map(text)),
                ("MessagingIDs", optional_list(lists.messaging)),
            ],
        )?;
        self.commit_create(entity)
    }

    /// Both addresses share `Purpose` and its USERDEFINED rule.
    fn validate_address_purpose(
        &self,
        entity: &'static str,
        purpose: Option<&str>,
        user_defined: Option<&str>,
    ) -> ResourceResult<()> {
        let Some(token) = purpose else {
            return Ok(());
        };
        validate_enum(self.schema, entity, "Purpose", token)?;
        if token.eq_ignore_ascii_case("USERDEFINED")
            && user_defined.is_none_or(|text| text.trim().is_empty())
        {
            return Err(ResourceError::SemanticViolation {
                entity: None,
                rule: "USERDEFINED_REQUIRES_USER_DEFINED_PURPOSE",
            });
        }
        Ok(())
    }

    /// Stage an `IfcOrganizationRelationship`.
    ///
    /// # Errors
    ///
    /// Refuses a reference that is not an `IfcOrganization`, an empty
    /// related set (`SET [1:?]`), and an organisation related to itself.
    pub fn create_organization_relationship(
        &mut self,
        name: Option<&str>,
        description: Option<&str>,
        relating: EntityId,
        related: &[EntityId],
    ) -> ResourceResult<EntityId> {
        const ENTITY: &str = "IfcOrganizationRelationship";
        self.check_reference(
            relating,
            "RelatingOrganization",
            "IfcOrganization",
            relating,
        )?;
        if related.is_empty() {
            return Err(ResourceError::InvalidDraft {
                entity_type: ENTITY,
                attribute: "RelatedOrganizations",
                expected: "at least one organisation, per SET [1:?]",
            });
        }
        for organization in related {
            self.check_reference(
                *organization,
                "RelatedOrganizations",
                "IfcOrganization",
                *organization,
            )?;
            // A relationship from an organisation to itself states
            // nothing and makes the graph cyclic at depth one.
            if *organization == relating {
                return Err(ResourceError::SemanticViolation {
                    entity: None,
                    rule: "RELATING_ORGANIZATION_IS_NOT_RELATED",
                });
            }
        }
        let entity = build_entity(
            self.schema,
            ENTITY,
            &[
                ("Name", name.map(text)),
                ("Description", description.map(text)),
                ("RelatingOrganization", Some(Value::Ref(relating))),
                (
                    "RelatedOrganizations",
                    Some(Value::List(
                        related.iter().copied().map(Value::Ref).collect(),
                    )),
                ),
            ],
        )?;
        self.commit_create(entity)
    }
}
