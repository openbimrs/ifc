//! #77: every view reads against the release `FILE_SCHEMA` declares.
//!
//! One building -- a layered wall with a layer set usage, a slab with a
//! material list, a column with a single material, and a typed wall that
//! inherits its type's layer set -- is written once per release in that
//! release's own layout and read through the same accessors. IFC2X3 records
//! are the short IFC2X3 TC1 forms; IFC4 and IFC4X3 are the ADD2 forms.

use ifc_material::{
    material_schema, AssignmentSource, DirectionSense, LayerSetDirection, LogicalValue, Material,
    MaterialDefinition, MaterialError, MaterialLayer, MaterialUsageDefinition, MaterialView,
    ResolvedMaterialSelect, SchemaVersion,
};
use ifc_model::codec::Codec;
use ifc_model::{EntityId, Model};
use ifc_step::StepCodec;

fn parse(schema: &str, data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('m.ifc','2026-01-01T00:00:00',(''),(''),'','','');\n\
         FILE_SCHEMA(({schema}));\nENDSEC;\nDATA;\n{data}ENDSEC;\nEND-ISO-10303-21;\n"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

/// Records whose layout is the same in every release.
const SHARED: &str = "\
#1=IFCOWNERHISTORY($,$,$,.NOCHANGE.,$,$,$,0);
#16=IFCMATERIALLIST((#10,#11));
#20=IFCWALL('0wall000000000000000001',#1,'W1',$,$,$,$,$);
#21=IFCSLAB('0slab000000000000000001',#1,'S1',$,$,$,$,$,$);
#22=IFCCOLUMN('0column0000000000000001',#1,'C1',$,$,$,$,$);
#23=IFCWALL('0wall000000000000000002',#1,'W2',$,$,$,$,$);
#30=IFCRELASSOCIATESMATERIAL('0rel0000000000000000001',#1,$,$,(#20),#15);
#31=IFCRELASSOCIATESMATERIAL('0rel0000000000000000002',#1,$,$,(#21),#16);
#32=IFCRELASSOCIATESMATERIAL('0rel0000000000000000003',#1,$,$,(#22),#10);
#33=IFCRELASSOCIATESMATERIAL('0rel0000000000000000004',#1,$,$,(#24),#14);
#34=IFCRELDEFINESBYTYPE('0rel0000000000000000005',#1,$,$,(#23),#24);
";

/// IFC2X3 TC1: `IfcMaterial(Name)`, `IfcMaterialLayer(Material,
/// LayerThickness, IsVentilated)`, `IfcMaterialLayerSet(MaterialLayers,
/// LayerSetName)`, and a four-attribute `IfcMaterialLayerSetUsage`.
const IFC2X3: &str = "\
#10=IFCMATERIAL('Brick');
#11=IFCMATERIAL('Insulation');
#12=IFCMATERIALLAYER(#10,0.24,.F.);
#13=IFCMATERIALLAYER(#11,0.1,.T.);
#14=IFCMATERIALLAYERSET((#12,#13),'Wall 340');
#15=IFCMATERIALLAYERSETUSAGE(#14,.AXIS2.,.POSITIVE.,-0.17);
#24=IFCWALLTYPE('0type000000000000000001',#1,'WT',$,$,$,$,$,$,.STANDARD.);
";

/// IFC4 ADD2 TC1 and IFC4X3 ADD2 share these layouts.
const IFC4: &str = "\
#10=IFCMATERIAL('Brick','Clay brick','Masonry');
#11=IFCMATERIAL('Insulation',$,'Insulation');
#12=IFCMATERIALLAYER(#10,0.24,.F.,'Core',$,'LoadBearing',80);
#13=IFCMATERIALLAYER(#11,0.1,.T.,'Insulation',$,'Insulation',$);
#14=IFCMATERIALLAYERSET((#12,#13),'Wall 340','External wall');
#15=IFCMATERIALLAYERSETUSAGE(#14,.AXIS2.,.POSITIVE.,-0.17,3.);
#24=IFCWALLTYPE('0type000000000000000001',#1,'WT',$,$,$,$,$,$,.STANDARD.);
";

fn fixture(schema: &str) -> Model {
    let own = if schema == "'IFC2X3'" { IFC2X3 } else { IFC4 };
    parse(schema, &format!("{own}{SHARED}"))
}

fn material_of(
    view: MaterialView<'_>,
    object: u64,
) -> (ResolvedMaterialSelect<'_>, AssignmentSource) {
    let resolved = view
        .assigned_material(EntityId(object))
        .expect("resolves")
        .expect("assigned");
    (resolved.material, resolved.source)
}

/// The assertions every release answers identically.
fn assert_common(model: &Model, release: SchemaVersion) {
    let view = MaterialView::new(model);
    assert_eq!(view.schema(), Ok(release));
    assert_eq!(material_schema(model), Ok(release));

    let names: Vec<_> = view.materials().map(|m| m.name().unwrap()).collect();
    assert_eq!(names, ["Brick", "Insulation"]);

    // Wall: layer set usage over the two-layer set.
    let (select, source) = material_of(view, 20);
    assert_eq!(source, AssignmentSource::Occurrence);
    let ResolvedMaterialSelect::Usage(MaterialUsageDefinition::LayerSet(usage)) = select else {
        panic!("{release:?}: expected a layer set usage, got {select:?}");
    };
    assert_eq!(usage.schema(), Ok(release));
    assert_eq!(usage.layer_set_id().unwrap(), EntityId(14));
    assert_eq!(
        usage.layer_set_direction().unwrap(),
        LayerSetDirection::Axis2
    );
    assert_eq!(usage.direction_sense().unwrap(), DirectionSense::Positive);
    assert_eq!(usage.offset_from_reference_line().unwrap(), -0.17);

    let set = view.layer_sets().next().unwrap();
    assert_eq!(set.layer_ids().unwrap(), [EntityId(12), EntityId(13)]);
    assert_eq!(set.name().unwrap(), Some("Wall 340"));
    assert!((view.total_thickness(set).unwrap() - 0.34).abs() < 1e-12);

    let layers: Vec<MaterialLayer<'_>> = view.layers().collect();
    assert_eq!(layers[0].material_id().unwrap(), Some(EntityId(10)));
    assert_eq!(layers[0].thickness().unwrap(), 0.24);
    assert_eq!(
        layers[0].is_ventilated().unwrap(),
        Some(LogicalValue::False)
    );
    assert_eq!(layers[1].is_ventilated().unwrap(), Some(LogicalValue::True));

    // Slab: material list.
    let (select, _) = material_of(view, 21);
    let ResolvedMaterialSelect::List(list) = select else {
        panic!("{release:?}: expected a material list");
    };
    assert_eq!(list.material_ids().unwrap(), [EntityId(10), EntityId(11)]);

    // Column: one material.
    let (select, _) = material_of(view, 22);
    let ResolvedMaterialSelect::Definition(MaterialDefinition::Material(material)) = select else {
        panic!("{release:?}: expected a single material");
    };
    assert_eq!(material.name().unwrap(), "Brick");

    // Typed wall: no direct association, so its type's layer set applies.
    let (select, source) = material_of(view, 23);
    assert_eq!(source, AssignmentSource::Type(EntityId(24)));
    let ResolvedMaterialSelect::Definition(MaterialDefinition::LayerSet(set)) = select else {
        panic!("{release:?}: expected the type's layer set");
    };
    assert_eq!(set.id(), EntityId(14));
}

#[test]
fn ifc2x3_reads_every_view_with_its_own_layout() {
    let model = fixture("'IFC2X3'");
    assert_common(&model, SchemaVersion::Ifc2x3);
}

#[test]
fn ifc4_reads_every_view_with_its_own_layout() {
    let model = fixture("'IFC4'");
    assert_common(&model, SchemaVersion::Ifc4);
    ifc4_attributes(&model);
}

#[test]
fn ifc4x3_reads_every_view_with_its_own_layout() {
    let model = fixture("'IFC4X3_ADD2'");
    assert_common(&model, SchemaVersion::Ifc4x3);
    ifc4_attributes(&model);
}

/// The attributes IFC4 added, present in IFC4 and IFC4X3.
fn ifc4_attributes(model: &Model) {
    let view = MaterialView::new(model);
    let brick = view.materials().next().unwrap();
    assert_eq!(brick.description().unwrap(), Some("Clay brick"));
    assert_eq!(brick.category().unwrap(), Some("Masonry"));
    let core = view.layers().next().unwrap();
    assert_eq!(core.name().unwrap(), Some("Core"));
    assert_eq!(core.description().unwrap(), None);
    assert_eq!(core.category().unwrap(), Some("LoadBearing"));
    assert_eq!(core.priority().unwrap(), Some(80));
    let set = view.layer_sets().next().unwrap();
    assert_eq!(set.description().unwrap(), Some("External wall"));
    let usage = view.layer_set_usages().next().unwrap();
    assert_eq!(usage.reference_extent().unwrap(), Some(3.0));
}

/// Attributes IFC2X3 does not declare are a typed absence, not `None`.
#[test]
fn ifc2x3_reports_attributes_it_lacks_as_absent_by_schema() {
    let model = fixture("'IFC2X3'");
    let view = MaterialView::new(&model);
    let absent =
        |entity: &'static str, id: u64, attribute: &'static str| MaterialError::NotInSchema {
            entity,
            id: EntityId(id),
            attribute,
            schema: SchemaVersion::Ifc2x3,
        };
    let brick = view.materials().next().unwrap();
    assert_eq!(
        brick.description().unwrap_err(),
        absent("IFCMATERIAL", 10, "Description")
    );
    assert_eq!(
        brick.category().unwrap_err(),
        absent("IFCMATERIAL", 10, "Category")
    );
    let layer = view.layers().next().unwrap();
    assert_eq!(
        layer.name().unwrap_err(),
        absent("IFCMATERIALLAYER", 12, "Name")
    );
    assert_eq!(
        layer.description().unwrap_err(),
        absent("IFCMATERIALLAYER", 12, "Description")
    );
    assert_eq!(
        layer.category().unwrap_err(),
        absent("IFCMATERIALLAYER", 12, "Category")
    );
    assert_eq!(
        layer.priority().unwrap_err(),
        absent("IFCMATERIALLAYER", 12, "Priority")
    );
    let set = view.layer_sets().next().unwrap();
    assert_eq!(
        set.description().unwrap_err(),
        absent("IFCMATERIALLAYERSET", 14, "Description")
    );
    let usage = view.layer_set_usages().next().unwrap();
    assert_eq!(
        usage.reference_extent().unwrap_err(),
        absent("IFCMATERIALLAYERSETUSAGE", 15, "ReferenceExtent")
    );
}

/// An IFC2X3 file that carries IFC4-shaped trailing values still never
/// answers from those slots: slot 3 of an IFC2X3 layer is not `Name`.
#[test]
fn ifc2x3_never_reads_an_ifc4_slot() {
    let model = parse(
        "'IFC2X3'",
        "#10=IFCMATERIAL('Brick','Clay brick','Masonry');\n\
         #12=IFCMATERIALLAYER(#10,0.24,.F.,'Core',$,'LoadBearing',80);\n",
    );
    let view = MaterialView::new(&model);
    let brick = view.materials().next().unwrap();
    assert_eq!(brick.name().unwrap(), "Brick");
    assert!(matches!(
        brick.category(),
        Err(MaterialError::NotInSchema { .. })
    ));
    let layer = view.layers().next().unwrap();
    assert_eq!(layer.thickness().unwrap(), 0.24);
    assert!(matches!(
        layer.name(),
        Err(MaterialError::NotInSchema { .. })
    ));
    assert!(matches!(
        layer.priority(),
        Err(MaterialError::NotInSchema { .. })
    ));
}

/// IFC2X3 `LayerThickness` is `IfcPositiveLengthMeasure`; IFC4 made it
/// `IfcNonNegativeLengthMeasure`, so only IFC4 onwards admits zero.
#[test]
fn a_zero_thickness_is_valid_from_ifc4_only() {
    for (schema, valid) in [("'IFC2X3'", false), ("'IFC4'", true), ("'IFC4X3'", true)] {
        let model = parse(
            schema,
            "#10=IFCMATERIAL('Air');\n#12=IFCMATERIALLAYER(#10,0.,$);\n\
             #14=IFCMATERIALLAYERSET((#12),'Gap');\n",
        );
        let view = MaterialView::new(&model);
        let layer = view.layers().next().unwrap();
        assert_eq!(layer.thickness().is_ok(), valid, "{schema}");
        let set = view.layer_sets().next().unwrap();
        assert_eq!(view.total_thickness(set).is_ok(), valid, "{schema}");
    }
}

/// Constituent and profile sets do not exist in IFC2X3; a record of either
/// is refused with a typed error by every path that meets it.
#[test]
fn ifc2x3_refuses_constituent_and_profile_sets() {
    let model = parse(
        "'IFC2X3'",
        "#1=IFCOWNERHISTORY($,$,$,.NOCHANGE.,$,$,$,0);\n\
         #10=IFCMATERIAL('Brick');\n\
         #40=IFCMATERIALCONSTITUENT('Clay',$,#10,1.,$);\n\
         #41=IFCMATERIALCONSTITUENTSET('Mix',$,(#40));\n\
         #42=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,0.2,0.4);\n\
         #43=IFCMATERIALPROFILE('Beam',$,#10,#42,$,$);\n\
         #44=IFCMATERIALPROFILESET('Beam',$,(#43),$);\n\
         #45=IFCMATERIALPROFILESETUSAGE(#44,5,$);\n\
         #20=IFCWALL('0wall000000000000000001',#1,'W1',$,$,$,$,$);\n\
         #21=IFCBEAM('0beam000000000000000001',#1,'B1',$,$,$,$,$);\n\
         #30=IFCRELASSOCIATESMATERIAL('0rel0000000000000000001',#1,$,$,(#20),#41);\n\
         #31=IFCRELASSOCIATESMATERIAL('0rel0000000000000000002',#1,$,$,(#21),#45);\n",
    );
    let view = MaterialView::new(&model);
    let refused = |entity: &'static str, id: u64| MaterialError::EntityNotInSchema {
        entity,
        id: Some(EntityId(id)),
        schema: SchemaVersion::Ifc2x3,
    };
    for (id, entity) in [
        (41, "IFCMATERIALCONSTITUENTSET"),
        (44, "IFCMATERIALPROFILESET"),
        (45, "IFCMATERIALPROFILESETUSAGE"),
        (40, "IFCMATERIALCONSTITUENT"),
        (43, "IFCMATERIALPROFILE"),
    ] {
        assert_eq!(
            view.resolve_material_select(EntityId(id)).unwrap_err(),
            refused(entity, id)
        );
    }
    assert_eq!(
        view.assigned_material(EntityId(20)).unwrap_err(),
        refused("IFCMATERIALCONSTITUENTSET", 41)
    );
    assert_eq!(
        view.assigned_material(EntityId(21)).unwrap_err(),
        refused("IFCMATERIALPROFILESETUSAGE", 45)
    );
    let set = view.constituent_sets().next().unwrap();
    assert_eq!(
        set.name().unwrap_err(),
        refused("IFCMATERIALCONSTITUENTSET", 41)
    );
    assert_eq!(
        view.constituent_fraction_diagnostic(set, 1e-9).unwrap_err(),
        refused("IFCMATERIALCONSTITUENTSET", 41)
    );
    let profiles = view.profile_sets().next().unwrap();
    assert_eq!(
        profiles.profile_ids().unwrap_err(),
        refused("IFCMATERIALPROFILESET", 44)
    );
    // The same records resolve under IFC4.
    let mut ifc4 = model.clone();
    ifc4.header_mut().schema = vec!["IFC4".to_owned()];
    let view = MaterialView::new(&ifc4);
    assert!(view.assigned_material(EntityId(20)).is_ok());
    assert!(view.assigned_material(EntityId(21)).is_ok());
}

/// IFC4X3 renamed `IfcMaterialRelationship.Expression` to
/// `MaterialExpression` at the same position.
#[test]
fn ifc4x3_reads_the_renamed_material_expression() {
    for schema in ["'IFC4'", "'IFC4X3_ADD2'"] {
        let model = parse(
            schema,
            "#10=IFCMATERIAL('C30/37',$,$);\n#11=IFCMATERIAL('Cement',$,$);\n\
             #50=IFCMATERIALRELATIONSHIP('Mix',$,#10,(#11),'CEM I');\n",
        );
        let relationship = MaterialView::new(&model)
            .material_relationships()
            .next()
            .unwrap();
        assert_eq!(
            relationship.expression().unwrap(),
            Some("CEM I"),
            "{schema}"
        );
    }
}

/// IFC4X3 dropped `IfcDoorStyle`, so it is no type object there; IFC4 and
/// IFC2X3 still inherit a door style's material.
#[test]
fn the_type_fallback_uses_the_release_type_objects() {
    for (schema, inherits) in [("'IFC2X3'", true), ("'IFC4'", true), ("'IFC4X3'", false)] {
        let model = parse(
            schema,
            "#1=IFCOWNERHISTORY($,$,$,.NOCHANGE.,$,$,$,0);\n#10=IFCMATERIAL('Oak');\n\
             #20=IFCDOOR('0door000000000000000001',#1,'D1',$,$,$,$,$,$,$);\n\
             #24=IFCDOORSTYLE('0type000000000000000001',#1,'DS',$,$,$,$,$,.SINGLE_SWING_LEFT.,.WOOD.,.F.,.F.);\n\
             #33=IFCRELASSOCIATESMATERIAL('0rel0000000000000000004',#1,$,$,(#24),#10);\n\
             #34=IFCRELDEFINESBYTYPE('0rel0000000000000000005',#1,$,$,(#20),#24);\n",
        );
        let resolved = MaterialView::new(&model).assigned_material(EntityId(20));
        if inherits {
            let resolved = resolved.unwrap().unwrap();
            assert_eq!(
                resolved.source,
                AssignmentSource::Type(EntityId(24)),
                "{schema}"
            );
        } else {
            assert!(
                matches!(resolved, Err(MaterialError::ReferenceType { .. })),
                "{schema}: {resolved:?}"
            );
        }
    }
}

/// Several or unknown declarations bind no release, and every read says so.
#[test]
fn an_ambiguous_or_unknown_header_fails_closed() {
    for (schema, expected) in [
        (
            "'IFC2X3','IFC4'",
            MaterialError::MultipleSchemas { schemas: 2 },
        ),
        (
            "'IFC5'",
            MaterialError::UnsupportedSchema {
                schema: "IFC5".to_owned(),
            },
        ),
    ] {
        let model = parse(
            schema,
            "#1=IFCOWNERHISTORY($,$,$,.NOCHANGE.,$,$,$,0);\n#10=IFCMATERIAL('Brick');\n\
             #20=IFCWALL('0wall000000000000000001',#1,'W1',$,$,$,$,$);\n\
             #30=IFCRELASSOCIATESMATERIAL('0rel0000000000000000001',#1,$,$,(#20),#10);\n",
        );
        let view = MaterialView::new(&model);
        assert_eq!(view.schema().unwrap_err(), expected, "{schema}");
        assert_eq!(material_schema(&model).unwrap_err(), expected);
        assert_eq!(
            view.materials().next().unwrap().name().unwrap_err(),
            expected
        );
        assert_eq!(view.assigned_material(EntityId(20)).unwrap_err(), expected);
    }
}

/// `try_new` has no header and keeps the 0.2.0 IFC4 reading;
/// `try_from_view` binds the model's release.
#[test]
fn a_standalone_projection_reads_ifc4_and_a_view_lookup_binds() {
    let model = fixture("'IFC2X3'");
    let entity = model.get(EntityId(10)).unwrap();
    let standalone = Material::try_new(EntityId(10), entity).unwrap();
    assert_eq!(standalone.schema(), Ok(SchemaVersion::Ifc4));
    assert_eq!(standalone.category().unwrap(), None);
    let bound = Material::try_from_view(MaterialView::new(&model), EntityId(10)).unwrap();
    assert_eq!(bound.schema(), Ok(SchemaVersion::Ifc2x3));
    assert!(matches!(
        bound.category(),
        Err(MaterialError::NotInSchema { .. })
    ));
    assert_eq!(
        Material::try_from_view(MaterialView::new(&model), EntityId(999)).unwrap_err(),
        MaterialError::UnknownEntity { id: EntityId(999) }
    );
    assert!(matches!(
        Material::try_from_view(MaterialView::new(&model), EntityId(12)),
        Err(MaterialError::WrongEntityType { .. })
    ));
}
