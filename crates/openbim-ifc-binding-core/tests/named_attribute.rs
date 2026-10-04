//! Attributes by name (#326): resolved against the release the header
//! declares, with every refusal's stable code.
//!
//! The cross-release case comes from the normative schemas
//! (`references/ifc-spec/`): `IfcTask.Status` is slot 6 in IFC2X3_TC1 and
//! slot 7 in IFC4 ADD2 TC1 and IFC4X3 ADD2, where `IfcProcess` gained
//! `Identification` and `LongDescription` (IFC2X3's `TaskId` is gone);
//! `IfcGridPlacement.PlacementLocation` is slot 0 in IFC2X3 and IFC4 and
//! slot 1 in IFC4X3, where `PlacementRelTo` moved up into
//! `IfcObjectPlacement`.

#[cfg(feature = "ifc4")]
use openbim_ifc_binding_core::value::Tagged;
#[cfg(feature = "ifc4")]
use openbim_ifc_binding_core::{BindingError, IfcModel};

#[cfg(feature = "ifc4")]
fn file(schema: &str, data: &str) -> IfcModel {
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('ViewDefinition [CoordinationView]'),'2;1');
FILE_NAME('t.ifc','2026-10-03T00:00:00',('a'),('o'),'p','s','');
FILE_SCHEMA(('{schema}'));
ENDSEC;
DATA;
{data}
ENDSEC;
END-ISO-10303-21;
"
    );
    IfcModel::parse(text.as_bytes()).expect("fixture parses")
}

#[cfg(feature = "ifc4")]
fn text(value: &str) -> Tagged {
    Tagged::Text(value.into())
}

#[cfg(feature = "ifc4")]
fn code<T: std::fmt::Debug>(result: Result<T, BindingError>) -> &'static str {
    result.expect_err("refused").code()
}

#[cfg(feature = "ifc4")]
const IFC4: &str = "#1=IFCTASK('0YvctVUKr0kugbFTf53O9L',$,'Pour',$,$,'T1',$,'Planned','Crane',.F.,1,$,.CONSTRUCTION.);
#2=IFCGRIDPLACEMENT(#3,$);
#3=IFCVIRTUALGRIDINTERSECTION((#4,#5),(0.,0.,0.));
#6=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#7=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'W');
#8=IFCALIGNMENT('2YvctVUKr0kugbFTf53O9L',$,'A',$,$,$,$,$);";

/// Slot of `name` as `attribute_names` reports it.
#[cfg(feature = "ifc4")]
fn slot(model: &IfcModel, id: u64, name: &str) -> usize {
    model
        .attribute_names(id)
        .unwrap()
        .into_iter()
        .find(|info| info.name == name)
        .unwrap_or_else(|| panic!("{name} listed"))
        .index
}

#[cfg(all(feature = "ifc2x3", feature = "ifc4", feature = "ifc4x3"))]
mod cross_release {
    use super::*;

    const IFC2X3: &str =
        "#1=IFCTASK('0YvctVUKr0kugbFTf53O9L',$,'Pour',$,$,'T1','Planned','Crane',.F.,1);
#2=IFCGRIDPLACEMENT(#3,$);";
    const IFC4X3: &str = "#1=IFCTASK('0YvctVUKr0kugbFTf53O9L',$,'Pour',$,$,'T1',$,'Planned','Crane',.F.,1,$,.CONSTRUCTION.);
#2=IFCGRIDPLACEMENT($,#3,$);";

    #[test]
    fn each_release_resolves_a_name_to_its_own_slot() {
        // (release, data, Status slot, PlacementLocation slot)
        for (schema, data, status, location) in [
            ("IFC2X3", IFC2X3, 6, 0),
            ("IFC4", IFC4, 7, 0),
            ("IFC4X3_ADD2", IFC4X3, 7, 1),
        ] {
            let mut model = file(schema, data);
            assert_eq!(slot(&model, 1, "Status"), status, "{schema}");
            assert_eq!(slot(&model, 2, "PlacementLocation"), location, "{schema}");
            assert_eq!(
                model.attribute_by_name(1, "Status").unwrap(),
                text("Planned")
            );
            assert_eq!(
                model.attribute_by_name(2, "placementlocation").unwrap(),
                Tagged::Ref(3)
            );
            let previous = model
                .set_attribute_by_name(1, "STATUS", text("Done"))
                .unwrap();
            assert_eq!(previous, text("Planned"), "{schema}");
            assert_eq!(
                model.attribute(1, status).unwrap(),
                text("Done"),
                "{schema}"
            );
            // Its neighbours are untouched.
            assert_eq!(model.attribute(1, status + 1).unwrap(), text("Crane"));
        }
    }

    #[test]
    fn a_name_of_another_release_is_unknown() {
        let ifc2x3 = file("IFC2X3", IFC2X3);
        assert_eq!(ifc2x3.attribute_by_name(1, "TaskId").unwrap(), text("T1"));
        assert_eq!(
            code(ifc2x3.attribute_by_name(1, "Identification")),
            "unknown-attribute"
        );
        let ifc4 = file("IFC4", IFC4);
        assert_eq!(
            ifc4.attribute_by_name(1, "Identification").unwrap(),
            text("T1")
        );
        assert_eq!(
            code(ifc4.attribute_by_name(1, "TaskId")),
            "unknown-attribute"
        );
        assert_eq!(
            code(ifc4.attribute_by_name(2, "PlacementRelTo")),
            "unknown-attribute"
        );
        let ifc4x3 = file("IFC4X3_ADD2", IFC4X3);
        assert_eq!(
            ifc4x3.attribute_by_name(2, "PlacementRelTo").unwrap(),
            Tagged::Null
        );
    }

    #[test]
    fn an_entity_another_release_declares_is_refused_not_guessed() {
        // IFCALIGNMENT is IFC4X1 and later.
        let model = file(
            "IFC2X3",
            "#8=IFCALIGNMENT('2YvctVUKr0kugbFTf53O9L',$,'A',$,$,$,$,$);",
        );
        assert_eq!(code(model.attribute_names(8)), "unsupported-schema");
        assert_eq!(
            code(model.attribute_by_name(8, "Name")),
            "unsupported-schema"
        );
    }
}

#[cfg(feature = "ifc4")]
mod ifc4 {
    use super::*;

    #[test]
    fn names_list_every_explicit_slot_inherited_first_in_schema_spelling() {
        let model = file("IFC4", IFC4);
        let names = model.attribute_names(1).unwrap();
        let listed: Vec<_> = names.iter().map(|info| info.name.as_str()).collect();
        assert_eq!(
            listed,
            [
                "GlobalId",
                "OwnerHistory",
                "Name",
                "Description",
                "ObjectType",
                "Identification",
                "LongDescription",
                "Status",
                "WorkMethod",
                "IsMilestone",
                "Priority",
                "TaskTime",
                "PredefinedType",
            ]
        );
        assert!(names
            .iter()
            .enumerate()
            .all(|(index, info)| info.index == index));
        let global_id = &names[0];
        assert_eq!(global_id.type_name, "IfcGloballyUniqueId");
        assert_eq!(global_id.declared_by, "IfcRoot");
        assert!(!global_id.optional && !global_id.derived && !global_id.aggregate);
        assert!(names[1].optional, "OwnerHistory is OPTIONAL in IFC4");
        // INVERSE attributes hold no slot.
        assert!(model
            .attribute_names(7)
            .unwrap()
            .iter()
            .all(|info| info.name != "IsDefinedBy"));
        assert_eq!(
            code(model.attribute_by_name(7, "IsDefinedBy")),
            "unknown-attribute"
        );
    }

    #[test]
    fn a_derived_slot_is_listed_read_as_stored_and_refused_on_write() {
        let mut model = file("IFC4", IFC4);
        let dimensions = &model.attribute_names(6).unwrap()[0];
        assert_eq!(dimensions.name, "Dimensions");
        assert!(dimensions.derived);
        assert_eq!(
            model.attribute_by_name(6, "Dimensions").unwrap(),
            Tagged::Derived
        );
        let before = model.write().unwrap();
        for value in [Tagged::Null, Tagged::Derived, Tagged::Ref(1)] {
            let refused = model.set_attribute_by_name(6, "dimensions", value);
            assert_eq!(code(refused), "derived-attribute");
        }
        assert_eq!(
            model.write().unwrap(),
            before,
            "a refused write changes nothing"
        );
    }

    #[test]
    fn a_short_record_reads_null_and_a_write_pads_it() {
        let mut model = file("IFC4", IFC4);
        assert_eq!(model.attribute_by_name(7, "Tag").unwrap(), Tagged::Null);
        let previous = model.set_attribute_by_name(7, "Tag", text("T-7")).unwrap();
        assert_eq!(previous, Tagged::Null);
        assert_eq!(model.attribute(7, 7).unwrap(), text("T-7"));
        assert_eq!(model.attribute(7, 6).unwrap(), Tagged::Null);
    }

    #[test]
    fn every_refusal_has_its_code_and_writes_nothing() {
        let mut model = file("IFC4", IFC4);
        let before = model.write().unwrap();
        assert_eq!(code(model.attribute_names(99)), "missing-entity");
        assert_eq!(code(model.attribute_by_name(99, "Name")), "missing-entity");
        assert_eq!(
            code(model.set_attribute_by_name(99, "Name", Tagged::Null)),
            "missing-entity"
        );
        assert_eq!(
            code(model.attribute_by_name(7, "Nmae")),
            "unknown-attribute"
        );
        assert_eq!(
            code(model.set_attribute_by_name(7, "Nmae", Tagged::Null)),
            "unknown-attribute"
        );
        let invalid = Tagged::Enum("not an identifier".into());
        assert_eq!(
            code(model.set_attribute_by_name(7, "Name", invalid)),
            "invalid-value"
        );
        // IFCALIGNMENT is IFC4X1 and later: not an IFC4 entity.
        assert_eq!(code(model.attribute_names(8)), "unsupported-schema");
        assert_eq!(
            code(model.set_attribute_by_name(8, "Name", Tagged::Null)),
            "unsupported-schema"
        );
        assert_eq!(model.write().unwrap(), before);
    }

    #[test]
    fn an_unknown_header_release_is_unsupported() {
        let model = file("IFC9", "#7=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'W');");
        assert_eq!(code(model.attribute_names(7)), "unsupported-schema");
        assert_eq!(
            code(model.attribute_by_name(7, "Name")),
            "unsupported-schema"
        );
    }
}

/// A release this build leaves out is refused, never read against another
/// release's tables (#306).
#[cfg(all(feature = "ifc4", not(feature = "ifc2x3")))]
#[test]
fn a_release_left_out_of_the_build_is_unsupported() {
    let model = file("IFC2X3", "#7=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'W');");
    assert_eq!(
        code(model.attribute_by_name(7, "Name")),
        "unsupported-schema"
    );
}
