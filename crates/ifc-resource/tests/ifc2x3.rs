//! #237: the IFC2X3 TC1 resource profile, read through IFC2X3's own table.
//!
//! The fixture is STEP text with a real `FILE_SCHEMA(('IFC2X3'))` header and
//! IFC2X3 attribute layouts; every assertion runs on the parsed model and
//! again after a STEP write/read round trip.

use ifc_model::{Codec, EntityId, Model, Value};
use ifc_resource::{ResourceEditor, ResourceError, ResourceKind, ResourceView};
use ifc_schema::{ifc2x3, ifc4};

fn step(schema: &str, data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('t','2026-10-02T00:00:00',(''),(''),'','','');\n\
         FILE_SCHEMA(({schema}));\nENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

const IFC2X3_DATA: &str = "\
#1=IFCOWNERHISTORY($,$,$,$,$,$,$,0);
#2=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);
#3=IFCMEASUREWITHUNIT(IFCLENGTHMEASURE(12.5),#2);
#4=IFCPERSON('P-7','Doe','Jane',$,$,$,$,$);
#5=IFCORGANIZATION('ORG-1','Acme Formwork',$,$,$);
#6=IFCPERSONANDORGANIZATION(#4,#5,$);
#7=IFCCALENDARDATE(2,10,2026);
#10=IFCLABORRESOURCE('0O2Fr$t4X7Zf8NOew3FLOH',#1,'Carpenters',$,'crew','LAB-01','Formwork',.OCCUPIED.,#3,'Formwork carpentry');
#11=IFCCONSTRUCTIONMATERIALRESOURCE('1O2Fr$t4X7Zf8NOew3FLOH',#1,'Concrete C30/37',$,$,'MAT-01',$,.CONSUMED.,$,(#5,#6),0.25);
#12=IFCSUBCONTRACTRESOURCE('2O2Fr$t4X7Zf8NOew3FLOH',#1,'Rebar subcontract',$,$,$,$,$,$,#5,'Supply and fix rebar');
#13=IFCCREWRESOURCE('3O2Fr$t4X7Zf8NOew3FLOH',#1,'Slab crew',$,$,$,$,$,$);
#14=IFCCONSTRUCTIONEQUIPMENTRESOURCE('2P2Fr$t4X7Zf8NOew3FLOH',#1,'Crane',$,$,$,$,.NOTDEFINED.,$);
#20=IFCRELNESTS('1P2Fr$t4X7Zf8NOew3FLOH',#1,$,$,#13,(#10,#14));
#21=IFCBUILDINGELEMENTPROXY('0P2Fr$t4X7Zf8NOew3FLOH',#1,'Slab',$,$,$,$,$,$);
#22=IFCRELASSIGNSTORESOURCE('3P2Fr$t4X7Zf8NOew3FLOH',#1,$,$,(#21),.PRODUCT.,#11);
#23=IFCINVENTORY('0Q2Fr$t4X7Zf8NOew3FLOH',#1,'Plant',$,$,.ASSETINVENTORY.,#5,(#4),#7,$,$);";

fn ifc2x3_model() -> Model {
    step("'IFC2X3'", IFC2X3_DATA)
}

fn round_trip(model: &Model) -> Model {
    let bytes = ifc_step::StepCodec.write_bytes(model).expect("writes");
    let reread = ifc_step::StepCodec.read_bytes(&bytes).expect("re-reads");
    assert!(reread.diagnostics().is_empty(), "{:?}", reread.diagnostics());
    reread
}

fn not_in_schema(result: Result<impl std::fmt::Debug, ResourceError>, attr: Option<&str>) {
    match result {
        Err(ResourceError::NotInSchema {
            schema, attribute, ..
        }) => {
            assert_eq!(schema, "IFC2X3");
            assert_eq!(attribute.as_deref(), attr);
        }
        other => panic!("expected NotInSchema({attr:?}), got {other:?}"),
    }
}

fn assert_profile(model: &Model) {
    let view = ResourceView::for_model(model).expect("IFC2X3 is read");
    assert!(std::ptr::eq(view.schema(), ifc2x3()));

    // Shared concepts through the shared accessors.
    let labor = view.resource(EntityId(10)).unwrap();
    assert_eq!(labor.kind(), ResourceKind::Labor);
    assert_eq!(labor.name().unwrap(), Some("Carpenters"));
    assert_eq!(labor.identification().unwrap(), Some("LAB-01"));

    // IFC2X3-only attributes through their own accessors.
    assert_eq!(labor.resource_group().unwrap(), Some("Formwork"));
    assert_eq!(labor.resource_consumption().unwrap(), Some("OCCUPIED"));
    assert_eq!(labor.skill_set().unwrap(), Some("Formwork carpentry"));
    let measure_id = labor.base_quantity_measure().unwrap().expect("authored");
    let measure = view.measure_with_unit(measure_id).unwrap();
    assert_eq!(measure.unit_component().unwrap(), EntityId(2));
    match measure.value_component().unwrap() {
        Value::Typed { type_name, value } => {
            assert!(type_name.eq_ignore_ascii_case("IFCLENGTHMEASURE"));
            assert_eq!(**value, Value::Real(12.5));
        }
        other => panic!("typed IfcValue expected, got {other:?}"),
    }

    let material = view.resource(EntityId(11)).unwrap();
    assert_eq!(material.kind(), ResourceKind::Material);
    assert_eq!(material.suppliers().unwrap(), vec![EntityId(5), EntityId(6)]);
    assert_eq!(material.usage_ratio().unwrap(), Some(0.25));
    assert_eq!(material.base_quantity_measure().unwrap(), None);

    let sub = view.resource(EntityId(12)).unwrap();
    assert_eq!(sub.kind(), ResourceKind::Subcontract);
    assert_eq!(sub.sub_contractor().unwrap(), Some(EntityId(5)));
    assert_eq!(sub.job_description().unwrap(), Some("Supply and fix rebar"));

    // What IFC2X3 lacks is a typed refusal, never `None`.
    not_in_schema(labor.long_description(), Some("LongDescription"));
    not_in_schema(labor.predefined_type(), Some("PredefinedType"));
    not_in_schema(labor.usage(), Some("Usage"));
    not_in_schema(labor.base_costs(), Some("BaseCosts"));
    not_in_schema(labor.base_quantity(), Some("BaseQuantity"));
    not_in_schema(labor.suppliers(), Some("Suppliers"));
    not_in_schema(material.skill_set(), Some("SkillSet"));
    not_in_schema(view.resource_type(EntityId(10)), None);
    not_in_schema(view.resource_time(EntityId(10)), None);
    not_in_schema(view.assigned_resource_type(EntityId(10)), None);

    // Composition and allocation read the IFC2X3 relationships.
    assert_eq!(
        view.direct_members(EntityId(13)).unwrap(),
        vec![EntityId(10), EntityId(14)]
    );
    assert_eq!(view.parent_resource(EntityId(14)).unwrap(), Some(EntityId(13)));
    let allocations = view.allocations_for(EntityId(11)).unwrap();
    assert_eq!(allocations.len(), 1);
    assert_eq!(allocations[0].related_objects(), &[EntityId(21)]);
    assert_eq!(allocations[0].related_objects_type(), Some("PRODUCT"));

    // Actors: IFC2X3 `Id` answers `identification()`.
    let person = view.person(EntityId(4)).unwrap();
    assert_eq!(person.identification().unwrap(), Some("P-7"));
    assert_eq!(person.family_name().unwrap(), Some("Doe"));
    let organization = view.organization(EntityId(5)).unwrap();
    assert_eq!(organization.identification().unwrap(), Some("ORG-1"));
    assert_eq!(organization.name().unwrap(), "Acme Formwork");
    let pao = view.person_and_organization(EntityId(6)).unwrap();
    assert_eq!(pao.person().unwrap(), EntityId(4));

    // Inventory: `InventoryType` answers `predefined_type()`; the calendar
    // date has its own accessor.
    let inventory = view.inventory(EntityId(23)).unwrap();
    assert_eq!(inventory.predefined_type().unwrap(), Some("ASSETINVENTORY"));
    assert_eq!(inventory.jurisdiction().unwrap(), Some(EntityId(5)));
    assert_eq!(inventory.responsible_person_ids().unwrap(), vec![EntityId(4)]);
    assert_eq!(inventory.last_update_calendar_date().unwrap(), EntityId(7));
    not_in_schema(inventory.last_update_date(), Some("LastUpdateDate"));
}

#[test]
fn ifc2x3_resources_are_read_through_the_release_table() {
    assert_profile(&ifc2x3_model());
}

#[test]
fn ifc2x3_profile_survives_a_step_round_trip() {
    let model = ifc2x3_model();
    let reread = round_trip(&model);
    assert_eq!(reread.header().schema, vec!["IFC2X3".to_owned()]);
    assert_profile(&reread);
}

#[test]
fn ifc2x3_undeclared_enumeration_member_is_refused() {
    let data = IFC2X3_DATA.replace(".OCCUPIED.", ".BORROWED.");
    let model = step("'IFC2X3'", &data);
    let view = ResourceView::for_model(&model).unwrap();
    assert!(matches!(
        view.resource(EntityId(10)).unwrap().resource_consumption(),
        Err(ResourceError::InvalidEnumeration {
            attribute: "ResourceConsumption",
            ..
        })
    ));
}

#[test]
fn ifc2x3_person_wr1_does_not_count_id() {
    let data = IFC2X3_DATA.replace("'P-7','Doe','Jane'", "'P-7',$,$");
    let model = step("'IFC2X3'", &data);
    let view = ResourceView::for_model(&model).unwrap();
    assert!(matches!(
        view.person(EntityId(4)),
        Err(ResourceError::SemanticViolation { .. })
    ));
}

#[test]
fn ifc2x3_required_inventory_type_unset_is_refused() {
    let data = IFC2X3_DATA.replace(".ASSETINVENTORY.", "$");
    let model = step("'IFC2X3'", &data);
    let view = ResourceView::for_model(&model).unwrap();
    assert!(matches!(
        view.inventory(EntityId(23)).unwrap().predefined_type(),
        Err(ResourceError::InvalidValue {
            attribute: "InventoryType",
            ..
        })
    ));
}

#[test]
fn ifc2x3_material_resource_wr1_and_wr2_are_enforced() {
    let second = IFC2X3_DATA.to_owned()
        + "\n#24=IFCRELASSIGNSTORESOURCE('1Q2Fr$t4X7Zf8NOew3FLOH',#1,$,$,(#21),$,#11);";
    let model = step("'IFC2X3'", &second);
    let view = ResourceView::for_model(&model).unwrap();
    assert!(matches!(
        view.allocations_for(EntityId(11)),
        Err(ResourceError::SemanticViolation { .. })
    ));

    let non_product = IFC2X3_DATA.replace("(#21),.PRODUCT.,#11", "(#21),.NOTDEFINED.,#11");
    let model = step("'IFC2X3'", &non_product);
    let view = ResourceView::for_model(&model).unwrap();
    assert!(matches!(
        view.allocations_for(EntityId(11)),
        Err(ResourceError::SemanticViolation { .. })
    ));

    // An unset RelatedObjectsType leaves WR2 unknown, which EXPRESS does
    // not count as a violation.
    let unset = IFC2X3_DATA.replace("(#21),.PRODUCT.,#11", "(#21),$,#11");
    let model = step("'IFC2X3'", &unset);
    let view = ResourceView::for_model(&model).unwrap();
    assert_eq!(view.allocations_for(EntityId(11)).unwrap().len(), 1);
}

#[test]
fn ifc2x3_authoring_is_refused_and_stages_nothing() {
    let mut model = ifc2x3_model();
    let before = model.iter().count();
    assert!(matches!(
        ResourceEditor::for_model(&mut model),
        Err(ResourceError::UnsupportedSchema { token }) if token == "IFC2X3"
    ));
    assert_eq!(model.iter().count(), before);
}

#[test]
fn ifc2x3_only_accessors_refuse_under_ifc4() {
    let model = step(
        "'IFC4'",
        "#1=IFCLABORRESOURCE('0O2Fr$t4X7Zf8NOew3FLOH',$,'Carpenters',$,$,'LAB-01',$,$,$,$,.NOTDEFINED.);",
    );
    let view = ResourceView::for_model(&model).unwrap();
    assert!(std::ptr::eq(view.schema(), ifc4()));
    let labor = view.resource(EntityId(1)).unwrap();
    assert_eq!(labor.identification().unwrap(), Some("LAB-01"));
    assert_eq!(labor.base_quantity().unwrap(), None);
    for result in [
        labor.resource_group().map(drop),
        labor.resource_consumption().map(drop),
        labor.skill_set().map(drop),
        labor.base_quantity_measure().map(drop),
    ] {
        assert!(
            matches!(result, Err(ResourceError::NotInSchema { ref schema, .. }) if schema == "IFC4"),
            "{result:?}"
        );
    }
}
