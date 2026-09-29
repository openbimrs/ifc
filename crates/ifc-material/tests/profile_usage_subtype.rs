//! `IfcMaterialProfileSetUsageTapering` is a profile-set usage (#136).
//!
//! IFC4 and IFC4X3 declare `IfcMaterialProfileSetUsageTapering` as
//! `SUBTYPE OF (IfcMaterialProfileSetUsage)`, keeping `ForProfileSet`,
//! `CardinalPoint` and `ReferenceExtent` at slots 0..2 and adding
//! `ForProfileEndSet` and `CardinalEndPoint`. A caller asking for every
//! profile-set usage must therefore get the tapering one too, with its taper
//! reachable.

use ifc_material::{MaterialProfileSetUsage, MaterialView};
use ifc_model::{Entity, EntityId, Model, Value};

fn r(id: u64) -> Value {
    Value::Ref(EntityId(id))
}

/// A plain usage #7 and a tapering usage #5 (ids out of type order, so the
/// iteration order is proved to be by id), both over profile set #4.
fn model(schema: &str) -> Model {
    let mut model = Model::new();
    model.header_mut().schema = vec![schema.to_owned()];
    model.insert(
        EntityId(4),
        Entity::new(
            "IFCMATERIALPROFILESET",
            vec![Value::Null, Value::Null, Value::List(vec![]), Value::Null],
        ),
    );
    model.insert(
        EntityId(5),
        Entity::new(
            "IFCMATERIALPROFILESETUSAGETAPERING",
            vec![
                r(4),
                Value::Integer(5),
                Value::Real(2.0),
                r(4),
                Value::Integer(8),
            ],
        ),
    );
    model.insert(
        EntityId(7),
        Entity::new(
            "IFCMATERIALPROFILESETUSAGE",
            vec![r(4), Value::Integer(1), Value::Null],
        ),
    );
    model
}

#[test]
fn profile_set_usages_include_the_tapering_subtype_with_its_taper() {
    for schema in ["IFC4", "IFC4X3_ADD2"] {
        let model = model(schema);
        let view = MaterialView::new(&model);
        let usages: Vec<_> = view.profile_set_usages().collect();
        let ids: Vec<_> = usages.iter().map(|usage| usage.id()).collect();
        assert_eq!(ids, [EntityId(5), EntityId(7)], "{schema}");

        let tapering = usages[0];
        assert_eq!(tapering.profile_set_id().unwrap(), EntityId(4));
        assert_eq!(tapering.cardinal_point().unwrap().unwrap().get(), 5);
        assert_eq!(tapering.reference_extent().unwrap(), Some(2.0));
        let taper = tapering
            .tapering()
            .expect("the subtype's taper is reachable");
        assert_eq!(taper.end_profile_set_id().unwrap(), EntityId(4));
        assert_eq!(taper.cardinal_end_point().unwrap().unwrap().get(), 8);

        assert!(usages[1].tapering().is_none(), "a plain usage has no taper");

        // The subtype-only iterator is unchanged.
        let only: Vec<_> = view
            .tapering_profile_set_usages()
            .map(|usage| usage.id())
            .collect();
        assert_eq!(only, [EntityId(5)]);
    }
}

#[test]
fn the_supertype_projection_accepts_the_subtype_and_nothing_else() {
    let model = model("IFC4");
    let view = MaterialView::new(&model);
    for id in [5, 7] {
        let usage = MaterialProfileSetUsage::try_from_view(view, EntityId(id))
            .unwrap_or_else(|error| panic!("#{id}: {error}"));
        assert_eq!(usage.profile_set_id().unwrap(), EntityId(4));
    }
    assert!(MaterialProfileSetUsage::try_from_view(view, EntityId(4)).is_err());
}
