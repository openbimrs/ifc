//! Native tests of the domain views every host binds (#123): property
//! sets and units, the spatial tree, classification, materials, systems,
//! cost and georeferencing. The hosts' own suites only check their
//! conversion of these records.
//!
//! Each domain has a feature-on test and a feature-off test; the gate runs
//! this file with every domain left out (`--no-default-features --features
//! ifc4`), where each operation must refuse with `feature-disabled`.

use openbim_ifc_binding_core::{BindingError, IfcModel};

fn fixture(path: &str) -> IfcModel {
    let path = format!("{}/../../test/fixtures/{path}", env!("CARGO_MANIFEST_DIR"));
    IfcModel::open(std::path::Path::new(&path)).expect("fixture reads")
}

/// A wall classified directly and through its type, with a layer set
/// usage on the type: no shipped fixture states classification or layered
/// materials, so it is written here.
#[cfg_attr(
    not(any(feature = "classification", feature = "material")),
    allow(dead_code)
)]
const CLASSIFIED: &str = "ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('IFC4'));ENDSEC;
DATA;
#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'Project',$,$,$,$,$,$);
#2=IFCWALLTYPE('1YvctVUKr0kugbFTf53O9L',$,'WT',$,$,$,$,$,$,.SOLIDWALL.);
#3=IFCWALL('2YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,$,$,$,.STANDARD.);
#4=IFCRELDEFINESBYTYPE('3YvctVUKr0kugbFTf53O9L',$,$,$,(#3),#2);
#10=IFCCLASSIFICATION('CSI',$,$,'Uniclass 2015',$,$,$);
#11=IFCCLASSIFICATIONREFERENCE($,'Ss_25',$,#10,$,$);
#12=IFCCLASSIFICATIONREFERENCE('https://example.org/Ss_25_10','Ss_25_10','Wall systems',#11,$,$);
#13=IFCRELASSOCIATESCLASSIFICATION('0ZvctVUKr0kugbFTf53O9L',$,$,$,(#3),#12);
#14=IFCCLASSIFICATIONREFERENCE($,'EF_25',$,#10,$,$);
#15=IFCRELASSOCIATESCLASSIFICATION('1ZvctVUKr0kugbFTf53O9L',$,$,$,(#2),#14);
#20=IFCMATERIAL('Concrete',$,'Concrete');
#21=IFCMATERIAL('Insulation',$,$);
#22=IFCMATERIALLAYER(#20,0.2,.F.,'Core',$,$,$);
#23=IFCMATERIALLAYER(#21,0.1,.U.,$,$,$,$);
#24=IFCMATERIALLAYERSET((#22,#23),'WT-300',$);
#25=IFCMATERIALLAYERSETUSAGE(#24,.AXIS2.,.POSITIVE.,-0.15,$);
#26=IFCRELASSOCIATESMATERIAL('2ZvctVUKr0kugbFTf53O9L',$,$,$,(#2),#25);
ENDSEC;
END-ISO-10303-21;
";

/// `text` with its `FILE_SCHEMA` replaced by `token`.
#[allow(dead_code)]
fn declaring(text: &str, token: &str) -> IfcModel {
    let text = text.replacen(
        "FILE_SCHEMA(('IFC4'))",
        &format!("FILE_SCHEMA(('{token}'))"),
        1,
    );
    IfcModel::parse(text.as_bytes()).expect("parses")
}

// --- properties ------------------------------------------------------------

#[cfg(feature = "properties")]
mod properties {
    use super::*;
    use openbim_ifc_binding_core::value::Tagged;

    fn wall_a() -> Vec<openbim_ifc_binding_core::properties::PropertySet> {
        fixture("synthetic-properties/synthetic_properties.ifc")
            .property_sets(30)
            .expect("wall A resolves")
    }

    #[test]
    fn occurrence_sets_come_first_and_override_the_type_by_property() {
        let sets = wall_a();
        let summary: Vec<(&str, &str, Option<u64>)> = sets
            .iter()
            .map(|set| (set.name.as_str(), set.source.as_str(), set.source_id))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("Pset_WallCommon", "occurrence", None),
                ("Qto_WallBaseQuantities", "occurrence", None),
                ("Pset_WallCommon", "type", Some(29)),
            ]
        );
        let own = &sets[0];
        assert_eq!(own.id, 38);
        assert_eq!(own.global_id.as_deref(), Some("0Mz9fPfwfBYBYTYsJjiw_L"));
        assert_eq!(own.type_name, "IFCPROPERTYSET");
        assert_eq!(own.properties[0].name, "IsExternal");
        assert_eq!(
            own.properties[0].value,
            Tagged::Typed {
                type_name: "IFCBOOLEAN".into(),
                value: Box::new(Tagged::Bool(false)),
            }
        );
        // The type's IsExternal is overridden; its FireRating is inherited.
        let inherited = &sets[2];
        let names: Vec<&str> = inherited
            .properties
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(names, vec!["FireRating"]);
        assert_eq!(
            inherited.properties[0].value,
            Tagged::Typed {
                type_name: "IFCLABEL".into(),
                value: Box::new(Tagged::Text("F30".into())),
            }
        );
    }

    #[test]
    fn quantities_keep_their_declared_type_and_stated_unit() {
        let sets = wall_a();
        let quantities = &sets[1];
        assert_eq!(quantities.type_name, "IFCELEMENTQUANTITY");
        let width = &quantities.properties[0];
        assert_eq!(
            (width.name.as_str(), width.type_name.as_str(), width.unit),
            ("Width", "IFCQUANTITYLENGTH", Some(7))
        );
        assert_eq!(
            width.value,
            Tagged::Typed {
                type_name: "IFCLENGTHMEASURE".into(),
                value: Box::new(Tagged::Real(200.0)),
            }
        );
        let layers = quantities.properties.last().unwrap();
        assert_eq!(layers.kind, "complex");
        assert_eq!(layers.discrimination.as_deref(), Some("layer"));
        assert_eq!(layers.members.len(), 2);

        // The stated millimetre, not the metre project default.
        let model = fixture("synthetic-properties/synthetic_properties.ifc");
        let unit = model.resolve_unit("IFCLENGTHMEASURE", width.unit).unwrap();
        assert_eq!((unit.unit, unit.from_project), (Some(7), false));
        assert!((unit.scale - 0.001).abs() < 1e-15);
        let default = model.resolve_unit("IFCLENGTHMEASURE", None).unwrap();
        assert_eq!(
            (default.unit, default.from_project, default.scale),
            (Some(6), true, 1.0)
        );
        assert!(matches!(
            model.resolve_unit("IFCLABEL", None),
            Err(BindingError::InvalidValue(_))
        ));
    }

    #[test]
    fn every_value_family_crosses_whole() {
        let model = fixture("synthetic-properties/synthetic_properties.ifc");
        let sets = model.property_sets(31).unwrap();
        let families = sets.iter().find(|set| set.name == "Pset_Families").unwrap();
        let by_name = |name: &str| {
            families
                .properties
                .iter()
                .find(|p| p.name == name)
                .unwrap_or_else(|| panic!("{name}"))
        };
        let colour = by_name("Colour");
        assert_eq!(colour.kind, "enumerated");
        let enumeration = colour.enumeration.as_ref().unwrap();
        assert_eq!((enumeration.id, enumeration.name.as_str()), (40, "Colours"));
        assert_eq!(enumeration.values.len(), 2);
        let range = by_name("Range").bounds.as_ref().unwrap();
        let length = |v: f64| Tagged::Typed {
            type_name: "IFCLENGTHMEASURE".into(),
            value: Box::new(Tagged::Real(v)),
        };
        assert_eq!(
            (&range.lower, &range.upper, &range.set_point),
            (&length(2.0), &length(10.0), &length(6.0))
        );
        assert_eq!(by_name("Layers").kind, "list");
        let curve = by_name("Curve").table.as_ref().unwrap();
        assert_eq!(curve.rows.len(), 2);
        assert_eq!(curve.interpolation.as_deref(), Some("LINEAR"));
        let reference = by_name("Material");
        assert_eq!(
            (
                reference.kind.as_str(),
                &reference.value,
                reference.usage.as_deref()
            ),
            ("reference", &Tagged::Ref(46), Some("Reference"))
        );
        let assembly = by_name("Assembly");
        assert_eq!(assembly.kind, "complex");
        assert_eq!(assembly.usage.as_deref(), Some("Layered"));
        assert_eq!(assembly.members[1].name, "Finish");
    }

    #[test]
    fn a_property_set_query_refuses_what_it_cannot_answer() {
        let model = fixture("synthetic-properties/synthetic_properties.ifc");
        assert_eq!(
            model.property_sets(9999),
            Err(BindingError::MissingEntity(9999))
        );
        // A cartesian point carries no property sets.
        assert!(matches!(
            model.property_sets(19),
            Err(BindingError::WrongEntityType(_))
        ));
        let text = std::fs::read_to_string(format!(
            "{}/../../test/fixtures/synthetic-properties/synthetic_properties.ifc",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let ifc4x1 = declaring(&text, "IFC4X1");
        let refused = ifc4x1.property_sets(30).unwrap_err();
        assert_eq!(refused.code(), "unsupported-schema", "{refused}");
    }

    /// One answer of `property_sets_many`, as the per-object call's result.
    fn answer(
        record: &openbim_ifc_binding_core::properties::ObjectPropertySets,
    ) -> Result<Vec<openbim_ifc_binding_core::properties::PropertySet>, (String, String)> {
        match &record.refusal {
            None => Ok(record.sets.clone()),
            Some(refusal) => {
                assert!(record.sets.is_empty());
                Err((refusal.code.clone(), refusal.message.clone()))
            }
        }
    }

    fn per_object(
        model: &IfcModel,
        id: u64,
    ) -> Result<Vec<openbim_ifc_binding_core::properties::PropertySet>, (String, String)> {
        model
            .property_sets(id)
            .map_err(|error| (error.code().to_owned(), error.to_string()))
    }

    /// The batch (#358) answers every id exactly as the per-object call
    /// does, refusals included, in the order asked.
    #[test]
    fn many_objects_answer_as_one_object_each() {
        let model = fixture("synthetic-properties/synthetic_properties.ifc");
        let mut ids = model.ids();
        ids.push(9999);
        ids.reverse();
        let many = model.property_sets_many(Some(&ids)).unwrap();
        assert_eq!(many.len(), ids.len());
        let mut refused = 0;
        for (record, &id) in many.iter().zip(&ids) {
            assert_eq!(record.object, id);
            assert_eq!(answer(record), per_object(&model, id), "#{id}");
            refused += usize::from(record.refusal.is_some());
        }
        assert!(refused > 1, "points, units and the missing id are refused");
        assert!(
            many.iter().any(|r| r.sets.len() > 1),
            "some object has sets"
        );
        assert_eq!(model.property_sets_many(Some(&[])).unwrap(), vec![]);
    }

    /// With no ids: every object definition, objects and type objects, in
    /// file order, and none of them refused.
    #[test]
    fn every_object_definition_is_the_default_selection() {
        let model = fixture("synthetic-properties/synthetic_properties.ifc");
        let every = model.property_sets_many(None).unwrap();
        let expected = model
            .ids_of_type_including_subtypes("IfcObjectDefinition")
            .unwrap();
        assert!(expected.contains(&30));
        assert_eq!(every.iter().map(|r| r.object).collect::<Vec<_>>(), expected);
        for record in &every {
            assert_eq!(answer(record), per_object(&model, record.object));
            assert!(record.refusal.is_none(), "{record:?}");
        }
    }

    /// The same in IFC2X3 and IFC4X3: occurrence and type sets alike.
    #[cfg(all(feature = "ifc2x3", feature = "ifc4x3"))]
    #[test]
    fn many_objects_answer_alike_in_every_release() {
        let wall = |schema: &str, wall: &str, wall_type: &str| {
            format!(
                "ISO-10303-21;
HEADER;FILE_DESCRIPTION((''),'2;1');FILE_NAME('','',(''),(''),'','','');FILE_SCHEMA(('{schema}'));ENDSEC;
DATA;
#1={wall};
#2={wall_type};
#3=IFCRELDEFINESBYTYPE('3YvctVUKr0kugbFTf53O9L',$,$,$,(#1),#2);
#4=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);
#5=IFCPROPERTYSET('4YvctVUKr0kugbFTf53O9L',$,'Pset_WallCommon',$,(#4));
#6=IFCRELDEFINESBYPROPERTIES('5YvctVUKr0kugbFTf53O9L',$,$,$,(#1),#5);
#7=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('F30'),$);
#8=IFCPROPERTYSET('6YvctVUKr0kugbFTf53O9L',$,'Pset_WallCommon',$,(#7));
ENDSEC;
END-ISO-10303-21;
"
            )
        };
        for (schema, wall_entity, type_entity) in [
            (
                "IFC2X3",
                "IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'W',$,$,$,$,$)",
                "IFCWALLTYPE('2YvctVUKr0kugbFTf53O9L',$,'T',$,$,(#8),$,$,$,.STANDARD.)",
            ),
            (
                "IFC4X3_ADD2",
                "IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'W',$,$,$,$,$,.STANDARD.)",
                "IFCWALLTYPE('2YvctVUKr0kugbFTf53O9L',$,'T',$,$,(#8),$,$,$,.STANDARD.)",
            ),
        ] {
            let text = wall(schema, wall_entity, type_entity);
            let model = IfcModel::parse(text.as_bytes()).expect("parses");
            let ids = model.ids();
            let many = model.property_sets_many(Some(&ids)).unwrap();
            for (record, &id) in many.iter().zip(&ids) {
                assert_eq!(answer(record), per_object(&model, id), "{schema} #{id}");
            }
            let wall_sets = &many[0].sets;
            assert_eq!(wall_sets.len(), 2, "{schema}: own set and the type's");
            assert_eq!(wall_sets[1].source, "type");
        }
    }

    /// A model refused as a whole is refused once, with the per-object code.
    #[test]
    fn a_refused_model_refuses_the_batch() {
        let text = std::fs::read_to_string(format!(
            "{}/../../test/fixtures/synthetic-properties/synthetic_properties.ifc",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let ifc4x1 = declaring(&text, "IFC4X1");
        assert_eq!(
            ifc4x1.property_sets_many(Some(&[30])).unwrap_err().code(),
            ifc4x1.property_sets(30).unwrap_err().code()
        );
        assert_eq!(
            ifc4x1.property_sets_many(None).unwrap_err().code(),
            "unsupported-schema"
        );
    }

    /// No index survives a call, so a batch after an edit sees the edit.
    #[cfg(feature = "properties-write")]
    #[test]
    fn a_batch_after_an_edit_sees_the_edit() {
        use openbim_ifc_binding_core::property_edit::PropertyEdit;
        let mut model = fixture("synthetic-properties/synthetic_properties.ifc");
        let before = model.property_sets_many(Some(&[30])).unwrap();
        model
            .set_properties(vec![PropertyEdit::set(
                30,
                "ACME_Batch",
                "Mark",
                Tagged::Typed {
                    type_name: "IFCLABEL".into(),
                    value: Box::new(Tagged::Text("B-1".into())),
                },
            )])
            .unwrap();
        let after = model.property_sets_many(Some(&[30])).unwrap();
        assert_eq!(after[0].sets.len(), before[0].sets.len() + 1);
        assert_eq!(answer(&after[0]), per_object(&model, 30));
        assert!(after[0].sets.iter().any(|set| set.name == "ACME_Batch"));
    }
}

#[cfg(not(feature = "properties"))]
#[test]
fn properties_without_the_feature_refuse() {
    let model = IfcModel::empty();
    assert_eq!(
        model.property_sets(1),
        Err(BindingError::FeatureDisabled("properties"))
    );
    assert_eq!(
        model.property_sets_many(None),
        Err(BindingError::FeatureDisabled("properties"))
    );
    assert_eq!(
        model.resolve_unit("IFCLENGTHMEASURE", None),
        Err(BindingError::FeatureDisabled("properties"))
    );
}

// --- spatial ---------------------------------------------------------------

#[cfg(feature = "spatial")]
mod spatial {
    use super::*;

    #[test]
    fn the_tree_runs_from_project_to_storey_elements() {
        let tree = fixture("synthetic-properties/synthetic_properties.ifc")
            .spatial_tree()
            .unwrap();
        assert_eq!(tree.release.as_deref(), Some("IFC4_ADD2_TC1"));
        assert_eq!(tree.roots, vec![22]);
        let kinds: Vec<(u64, &str, Option<u64>)> = tree
            .nodes
            .iter()
            .map(|node| (node.id, node.kind.as_str(), node.parent))
            .collect();
        assert_eq!(
            kinds,
            vec![
                (22, "project", None),
                (23, "site", Some(22)),
                (24, "building", Some(23)),
                (25, "storey", Some(24)),
            ]
        );
        let storey = &tree.nodes[3];
        assert_eq!(storey.elements, vec![30, 31]);
        assert_eq!(storey.name.as_deref(), Some("Level 0"));
        assert_eq!(storey.global_id.as_deref(), Some("1B3Fknknr1IB15rEwzoEkq"));
        assert!(tree.orphans.is_empty() && tree.anomalies.is_empty());
    }

    // The file declares IFC4X1, which a build must bundle to read it (#306).
    #[cfg(feature = "ifc4x1")]
    #[test]
    fn a_release_the_tree_is_not_verified_for_binds_none() {
        let text = std::fs::read_to_string(format!(
            "{}/../../test/fixtures/synthetic-properties/synthetic_properties.ifc",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let tree = declaring(&text, "IFC4X1").spatial_tree().unwrap();
        assert_eq!(tree.release, None);
        assert_eq!(tree.nodes.len(), 4);
        assert_eq!(
            declaring(&text, "IFC9").spatial_tree().unwrap_err().code(),
            "unsupported-schema"
        );
    }
}

#[cfg(not(feature = "spatial"))]
#[test]
fn spatial_without_the_feature_refuses() {
    assert_eq!(
        IfcModel::empty().spatial_tree(),
        Err(BindingError::FeatureDisabled("spatial"))
    );
}

// --- classification ----------------------------------------------------------

#[cfg(feature = "classification")]
mod classification {
    use super::*;

    #[test]
    fn a_wall_has_its_own_and_its_types_classifications() {
        let model = IfcModel::parse(CLASSIFIED.as_bytes()).unwrap();
        let classes = model.classifications(3).unwrap();
        assert_eq!(classes.len(), 2);
        let own = &classes[0];
        assert_eq!(
            (own.relationship, own.source.as_str(), own.kind.as_str()),
            (13, "occurrence", "reference")
        );
        assert_eq!(own.global_id.as_deref(), Some("0ZvctVUKr0kugbFTf53O9L"));
        assert_eq!(own.identification.as_deref(), Some("Ss_25_10"));
        assert_eq!(own.name.as_deref(), Some("Wall systems"));
        assert_eq!(
            own.location.as_deref(),
            Some("https://example.org/Ss_25_10")
        );
        assert_eq!(own.parents, vec![11]);
        let system = own.system.as_ref().unwrap();
        assert_eq!(
            (system.id, system.name.as_str(), system.source.as_deref()),
            (10, "Uniclass 2015", Some("CSI"))
        );
        let inherited = &classes[1];
        assert_eq!(
            (
                inherited.source.as_str(),
                inherited.type_object,
                inherited.target
            ),
            ("type", Some(2), 14)
        );
        assert_eq!(inherited.identification.as_deref(), Some("EF_25"));
    }

    #[test]
    fn classification_refuses_an_unverified_release_and_a_missing_object() {
        let model = IfcModel::parse(CLASSIFIED.as_bytes()).unwrap();
        assert_eq!(
            model.classifications(99),
            Err(BindingError::MissingEntity(99))
        );
        let refused = declaring(CLASSIFIED, "IFC4X2")
            .classifications(3)
            .unwrap_err();
        assert_eq!(refused.code(), "unsupported-schema", "{refused}");
    }
}

#[cfg(not(feature = "classification"))]
#[test]
fn classification_without_the_feature_refuses() {
    assert_eq!(
        IfcModel::empty().classifications(1),
        Err(BindingError::FeatureDisabled("classification"))
    );
}

// --- material --------------------------------------------------------------

#[cfg(feature = "material")]
mod material {
    use super::*;
    use openbim_ifc_binding_core::value::Tagged;

    #[test]
    fn a_layer_set_usage_is_inherited_from_the_type() {
        let model = IfcModel::parse(CLASSIFIED.as_bytes()).unwrap();
        let assigned = model.material(3).unwrap().expect("inherited");
        assert_eq!(
            (
                assigned.source.as_str(),
                assigned.type_object,
                assigned.kind.as_str()
            ),
            ("type", Some(2), "layer-set-usage")
        );
        assert_eq!((assigned.target, assigned.set), (25, Some(24)));
        assert_eq!(assigned.name.as_deref(), Some("WT-300"));
        assert_eq!(assigned.layers.len(), 2);
        let core = &assigned.layers[0];
        assert_eq!(core.thickness, 0.2);
        assert_eq!(core.is_ventilated, Tagged::Bool(false));
        assert_eq!(core.name.as_deref(), Some("Core"));
        let concrete = core.material.as_ref().unwrap();
        assert_eq!(
            (
                concrete.id,
                concrete.name.as_str(),
                concrete.category.as_deref()
            ),
            (20, "Concrete", Some("Concrete"))
        );
        assert_eq!(assigned.layers[1].is_ventilated, Tagged::Unknown);
        let usage = assigned.usage.as_ref().unwrap();
        assert_eq!(usage.layer_set_direction.as_deref(), Some("AXIS2"));
        assert_eq!(usage.direction_sense.as_deref(), Some("POSITIVE"));
        assert_eq!(usage.offset_from_reference_line, Some(-0.15));
        // The type itself carries the same association directly.
        assert_eq!(model.material(2).unwrap().unwrap().source, "occurrence");
        // An object with no association has none.
        assert_eq!(model.material(1).unwrap(), None);
    }

    #[test]
    fn material_refuses_an_unverified_release() {
        let refused = declaring(CLASSIFIED, "IFC4X1").material(3).unwrap_err();
        assert_eq!(refused.code(), "unsupported-schema", "{refused}");
    }
}

#[cfg(not(feature = "material"))]
#[test]
fn material_without_the_feature_refuses() {
    assert_eq!(
        IfcModel::empty().material(1),
        Err(BindingError::FeatureDisabled("material"))
    );
}

// --- systems ---------------------------------------------------------------

#[cfg(feature = "systems")]
mod systems {
    use super::*;

    #[test]
    fn systems_carry_members_served_buildings_and_anomalies() {
        let systems = fixture("synthetic-systems/synthetic_systems.ifc")
            .systems()
            .unwrap();
        let heating = systems.systems.iter().find(|s| s.id == 14).unwrap();
        assert_eq!(heating.type_name, "IFCDISTRIBUTIONSYSTEM");
        assert_eq!(heating.name.as_deref(), Some("Heating"));
        assert_eq!(heating.long_name.as_deref(), Some("Hot water heating"));
        assert_eq!(heating.predefined_type.as_deref(), Some("HEATING"));
        assert_eq!(heating.members, vec![18, 19, 21, 22, 61]);
        assert_eq!(heating.serviced_buildings, vec![12]);
        assert_eq!(heating.global_id.as_deref(), Some("3muCihWk98IQm7MKef4ifT"));
        // IfcZone is an IfcSystem in IFC4.
        assert!(systems.systems.iter().any(|s| s.type_name == "IFCZONE"));
        // The inventory is a group but no system: its membership is
        // reported, not dropped silently.
        let anomaly = &systems.anomalies[0];
        assert_eq!(
            (anomaly.kind.as_str(), anomaly.subject, anomaly.other),
            ("not-a-system", 25, Some(17)),
            "{:?}",
            systems.anomalies
        );
    }
}

#[cfg(not(feature = "systems"))]
#[test]
fn systems_without_the_feature_refuse() {
    assert_eq!(
        IfcModel::empty().systems(),
        Err(BindingError::FeatureDisabled("systems"))
    );
}

// --- cost ------------------------------------------------------------------

#[cfg(feature = "cost")]
mod cost {
    use super::*;
    use openbim_ifc_binding_core::value::Tagged;

    #[test]
    fn schedules_items_and_value_trees_cross_whole() {
        let cost = fixture("synthetic-cost-schedule/synthetic_cost_schedule.ifc")
            .cost()
            .unwrap();
        let budget = &cost.schedules[0];
        assert_eq!(
            (
                budget.id,
                budget.name.as_deref(),
                budget.identification.as_deref()
            ),
            (31, Some("Budget"), Some("CS-1"))
        );
        assert_eq!(budget.predefined_type.as_deref(), Some("BUDGET"));
        assert_eq!(budget.items, vec![38, 42, 44]);

        let item = |id: u64| cost.items.iter().find(|item| item.id == id).unwrap();
        let substructure = item(38);
        assert_eq!(substructure.children, vec![39, 40]);
        assert_eq!(item(39).parent, Some(38));
        assert_eq!(item(40).quantities, vec![25]);
        let rate = &item(43).values[0];
        assert_eq!(
            rate.applied_value,
            Tagged::Typed {
                type_name: "IFCMONETARYMEASURE".into(),
                value: Box::new(Tagged::Real(45.5)),
            }
        );
        let basis = rate.unit_basis.as_ref().unwrap();
        assert_eq!(basis.id, 32);
        assert_eq!(
            basis.value,
            Tagged::Typed {
                type_name: "IFCVOLUMEMEASURE".into(),
                value: Box::new(Tagged::Real(1.0)),
            }
        );
        let setup = &item(44).values[0];
        assert_eq!(setup.applied_value, Tagged::Null);
        assert_eq!(setup.operator.as_deref(), Some("ADD"));
        let parts: Vec<u64> = setup.components.iter().map(|c| c.id).collect();
        assert_eq!(parts, vec![34, 35]);
    }

    #[test]
    fn a_cyclic_value_tree_is_refused() {
        let text = std::fs::read_to_string(format!(
            "{}/../../test/fixtures/synthetic-cost-schedule/synthetic_cost_schedule.ifc",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
        .replace(
            "#34=IFCCOSTVALUE('Labour',$,IFCMONETARYMEASURE(320.),$,$,$,'Labour',$,$,$);",
            "#34=IFCCOSTVALUE('Labour',$,IFCMONETARYMEASURE(320.),$,$,$,'Labour',$,.ADD.,(#36));",
        );
        let refused = IfcModel::parse(text.as_bytes())
            .unwrap()
            .cost()
            .unwrap_err();
        assert_eq!(refused.code(), "budget-exceeded", "{refused}");
        assert_eq!(
            declaring(&text, "IFC4X1").cost().unwrap_err().code(),
            "unsupported-schema"
        );
    }
}

#[cfg(not(feature = "cost"))]
#[test]
fn cost_without_the_feature_refuses() {
    assert_eq!(
        IfcModel::empty().cost(),
        Err(BindingError::FeatureDisabled("cost"))
    );
}

// --- georeferencing ----------------------------------------------------------

#[cfg(feature = "georef")]
mod georef {
    use super::*;

    // The fixture declares IFC4X3, which a build must bundle to read it (#306).
    #[cfg(feature = "ifc4x3")]
    #[test]
    fn a_map_conversion_crosses_with_its_crs_and_units() {
        let maps = fixture("synthetic-surfaces/synthetic_conic_offset_bounded.ifc")
            .georeferencing()
            .unwrap();
        assert_eq!(maps.len(), 1);
        let map = &maps[0];
        assert_eq!(
            (
                map.operation,
                map.kind.as_str(),
                map.source,
                map.source_kind.as_str()
            ),
            (51, "map-conversion", 7, "context")
        );
        assert_eq!(map.target_crs.name.as_deref(), Some("EPSG:25832"));
        assert_eq!(map.target_crs.map_zone.as_deref(), Some("32N"));
        assert_eq!((map.eastings, map.northings), (1.0, 2.0));
        assert_eq!(map.x_axis, (1.0, 0.0));
        assert_eq!(map.project_unit.metres_per_unit, 1.0);
        assert!(map.map_unit_declared);
        assert_eq!(map.translation, [1.0, 2.0, 0.01]);
    }

    #[test]
    fn a_release_without_georeferencing_is_refused_and_none_is_empty() {
        let refused = declaring(CLASSIFIED, "IFC2X3")
            .georeferencing()
            .unwrap_err();
        assert_eq!(refused.code(), "unsupported-schema", "{refused}");
        let none = IfcModel::parse(CLASSIFIED.as_bytes()).unwrap();
        assert_eq!(none.georeferencing(), Ok(Vec::new()));
    }
}

#[cfg(not(feature = "georef"))]
#[test]
fn georef_without_the_feature_refuses() {
    assert_eq!(
        IfcModel::empty().georeferencing(),
        Err(BindingError::FeatureDisabled("georef"))
    );
}
