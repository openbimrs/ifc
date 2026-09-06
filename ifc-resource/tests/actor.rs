mod support;

use ifc_resource::{ResourceError, ResourceView};
use ifc_schema::ifc4;

use support::{enumeration, model, named, refs, refs_text, text, GUID_A};

#[test]
fn person_projects_names_and_roles_and_enforces_identifiable_and_name_set_rules() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let role = model.push(named(
        schema,
        "IfcActorRole",
        &[
            ("Role", enumeration("ARCHITECT")),
            ("Description", text("Lead architect")),
        ],
    ));
    let person = model.push(named(
        schema,
        "IfcPerson",
        &[
            ("FamilyName", text("Doe")),
            ("GivenName", text("Jane")),
            ("MiddleNames", refs_text(&["Q"])),
            ("Roles", refs(&[role])),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    let projected = view.person(person).unwrap();
    assert_eq!(projected.family_name().unwrap(), Some("Doe"));
    assert_eq!(projected.given_name().unwrap(), Some("Jane"));
    assert_eq!(projected.middle_names().unwrap(), vec!["Q"]);
    let roles = projected.roles().unwrap();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].role().unwrap(), "ARCHITECT");
    assert_eq!(roles[0].description().unwrap(), Some("Lead architect"));
}

#[test]
fn person_rejects_missing_identity_and_middle_names_without_family_or_given() {
    let schema = ifc4();

    let mut no_identity = model("IFC4");
    let person = no_identity.push(named(schema, "IfcPerson", &[]));
    assert!(matches!(
        ResourceView::for_model(&no_identity)
            .unwrap()
            .person(person),
        Err(ResourceError::SemanticViolation { .. })
    ));

    let mut bad_middle = model("IFC4");
    let person = bad_middle.push(named(
        schema,
        "IfcPerson",
        &[
            ("Identification", text("P-1")),
            ("MiddleNames", refs_text(&["Q"])),
        ],
    ));
    assert!(matches!(
        ResourceView::for_model(&bad_middle).unwrap().person(person),
        Err(ResourceError::SemanticViolation { .. })
    ));
}

#[test]
fn actor_role_enforces_wr1_userdefined_requires_user_defined_role() {
    let schema = ifc4();

    let mut missing = model("IFC4");
    let role = missing.push(named(
        schema,
        "IfcActorRole",
        &[("Role", enumeration("USERDEFINED"))],
    ));
    assert!(matches!(
        ResourceView::for_model(&missing).unwrap().actor_role(role),
        Err(ResourceError::SemanticViolation { .. })
    ));

    let mut present = model("IFC4");
    let role = present.push(named(
        schema,
        "IfcActorRole",
        &[
            ("Role", enumeration("USERDEFINED")),
            ("UserDefinedRole", text("Owner's rep")),
        ],
    ));
    let projected = ResourceView::for_model(&present)
        .unwrap()
        .actor_role(role)
        .unwrap();
    assert_eq!(projected.role().unwrap(), "USERDEFINED");
    assert_eq!(projected.user_defined_role().unwrap(), Some("Owner's rep"));

    let mut unknown = model("IFC4");
    let role = unknown.push(named(
        schema,
        "IfcActorRole",
        &[("Role", enumeration("MAGIC"))],
    ));
    assert!(matches!(
        ResourceView::for_model(&unknown).unwrap().actor_role(role),
        Err(ResourceError::InvalidEnumeration { .. })
    ));
}

#[test]
fn organization_projects_name_roles_and_relationship_membership() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let role = model.push(named(
        schema,
        "IfcActorRole",
        &[("Role", enumeration("OWNER"))],
    ));
    let parent = model.push(named(
        schema,
        "IfcOrganization",
        &[("Name", text("Acme Holding"))],
    ));
    let child = model.push(named(
        schema,
        "IfcOrganization",
        &[
            ("Name", text("Acme Construction")),
            ("Roles", refs(&[role])),
        ],
    ));
    let relation = model.push(named(
        schema,
        "IfcOrganizationRelationship",
        &[
            ("Name", text("Subsidiary")),
            ("RelatingOrganization", ifc_model::Value::Ref(parent)),
            ("RelatedOrganizations", refs(&[child])),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    let org = view.organization(child).unwrap();
    assert_eq!(org.name().unwrap(), "Acme Construction");
    let roles = org.roles().unwrap();
    assert_eq!(roles.len(), 1);
    assert_eq!(roles[0].role().unwrap(), "OWNER");

    let relationship = view.organization_relationship(relation).unwrap();
    assert_eq!(relationship.name().unwrap(), Some("Subsidiary"));
    assert_eq!(relationship.relating_organization().unwrap(), parent);
    assert_eq!(relationship.related_organizations().unwrap(), vec![child]);
}

#[test]
fn person_and_organization_resolves_person_organization_and_roles() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let person = model.push(named(schema, "IfcPerson", &[("FamilyName", text("Doe"))]));
    let organization = model.push(named(schema, "IfcOrganization", &[("Name", text("Acme"))]));
    let role = model.push(named(
        schema,
        "IfcActorRole",
        &[("Role", enumeration("ENGINEER"))],
    ));
    let combo = model.push(named(
        schema,
        "IfcPersonAndOrganization",
        &[
            ("ThePerson", ifc_model::Value::Ref(person)),
            ("TheOrganization", ifc_model::Value::Ref(organization)),
            ("Roles", refs(&[role])),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    let combo = view.person_and_organization(combo).unwrap();
    assert_eq!(combo.person().unwrap(), person);
    assert_eq!(combo.organization().unwrap(), organization);
    assert_eq!(combo.role_ids().unwrap(), vec![role]);
}

#[test]
fn person_and_organization_rejects_wrong_reference_types() {
    let schema = ifc4();
    let mut model = model("IFC4");
    let wall = model.push(named(schema, "IfcWall", &[("GlobalId", text(GUID_A))]));
    let organization = model.push(named(schema, "IfcOrganization", &[("Name", text("Acme"))]));
    let combo = model.push(named(
        schema,
        "IfcPersonAndOrganization",
        &[
            ("ThePerson", ifc_model::Value::Ref(wall)),
            ("TheOrganization", ifc_model::Value::Ref(organization)),
        ],
    ));

    let view = ResourceView::for_model(&model).unwrap();
    assert!(matches!(
        view.person_and_organization(combo).unwrap().person(),
        Err(ResourceError::WrongReferenceType { .. })
    ));
}
