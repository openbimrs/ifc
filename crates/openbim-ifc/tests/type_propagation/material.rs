//! Deriving a material set usage from the type's set.

use super::*;

// --- material usage ---

pub(super) fn usage_of(f: &Fixture, object: EntityId) -> EntityId {
    let associations = f.relationships("IFCRELASSOCIATESMATERIAL", "RelatedObjects", object);
    assert_eq!(associations.len(), 1, "{}: one material", f.name());
    f.reference(associations[0], "RelatingMaterial")
}

#[test]
fn a_wall_type_layer_set_gives_each_wall_a_layer_set_usage() {
    each_release(|mut f| {
        let first = f.wall(vec![]);
        let second = f.wall(vec![]);
        let op = f.assign(
            f.base.wall_type,
            &[first, second],
            AssignTypeOptions::new().material_usage(true),
        );
        f.apply(vec![op]);
        let usages: Vec<EntityId> = [first, second].iter().map(|w| usage_of(&f, *w)).collect();
        assert_ne!(usages[0], usages[1], "{}: one usage each", f.name());
        for usage in usages {
            assert_eq!(f.get(usage).type_name.as_ref(), "IFCMATERIALLAYERSETUSAGE");
            assert_eq!(f.reference(usage, "ForLayerSet"), f.base.wall_set);
            assert_eq!(f.attr(usage, "LayerSetDirection"), enumeration("AXIS2"));
            assert_eq!(f.attr(usage, "DirectionSense"), enumeration("POSITIVE"));
            assert_eq!(
                f.attr(usage, "OffsetFromReferenceLine")
                    .unwrap_typed()
                    .as_f64(),
                Some(0.0)
            );
        }
    });
}

#[test]
fn a_slab_takes_axis3_and_an_undocumented_class_needs_a_direction() {
    each_release(|mut f| {
        let mut b = Batch::default();
        let set = layer_set(&mut b, "Screed", 0.05);
        let slab_type = b.push(AuthorOp::TypeObject {
            type_name: "IfcSlabType".into(),
            attributes: named(vec![
                ("Name", text("S")),
                ("PredefinedType", enumeration("FLOOR")),
            ]),
            owner_history: Some(f.base.owner),
        });
        associate(&mut b, f.base.owner, &[slab_type], set);
        let covering_type = b.push(AuthorOp::TypeObject {
            type_name: "IfcCoveringType".into(),
            attributes: named(vec![
                ("Name", text("C")),
                ("PredefinedType", enumeration("FLOORING")),
            ]),
            owner_history: Some(f.base.owner),
        });
        associate(&mut b, f.base.owner, &[covering_type], set);
        let slab = b.push(f.product("IfcSlab", vec![("Name", text("Slab"))]));
        let covering = b.push(f.product("IfcCovering", vec![("Name", text("Covering"))]));
        let ids = f.apply(b.ops);
        let real =
            |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
        let (slab_type, covering_type, slab, covering) = (
            real(slab_type),
            real(covering_type),
            real(slab),
            real(covering),
        );

        let usage = AssignTypeOptions::new().material_usage(true);
        let op = f.assign(slab_type, &[slab], usage);
        f.apply(vec![op]);
        assert_eq!(
            f.attr(usage_of(&f, slab), "LayerSetDirection"),
            enumeration("AXIS3")
        );

        let op = f.assign(covering_type, &[covering], usage);
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(failure, AuthoringFailure::MaterialUsageUndetermined { object, .. } if object == covering),
            "{}: {failure:?}",
            f.name()
        );
        let op = f.assign(
            covering_type,
            &[covering],
            usage.layer_set_direction(Some(LayerAxis::Axis3)),
        );
        f.apply(vec![op]);
        assert_eq!(
            f.attr(usage_of(&f, covering), "LayerSetDirection"),
            enumeration("AXIS3")
        );
    });
}

#[test]
fn an_object_with_its_own_material_is_refused() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let mut b = Batch::default();
        let own = b.create("IfcMaterial", vec![("Name", text("Brick"))]);
        associate(&mut b, f.base.owner, &[wall], own);
        f.apply(b.ops);
        let op = f.assign(
            f.base.wall_type,
            &[wall],
            AssignTypeOptions::new().material_usage(true),
        );
        let failure = f.refuse(vec![op]);
        assert!(
            matches!(failure, AuthoringFailure::AlreadyRelated { id, relationship: "IFCRELASSOCIATESMATERIAL", .. } if id == wall),
            "{}: {failure:?}",
            f.name()
        );
    });
}

#[test]
fn a_type_with_several_materials_or_a_usage_is_refused() {
    each_release(|mut f| {
        let wall = f.wall(vec![]);
        let mut b = Batch::default();
        let extra = b.create("IfcMaterial", vec![("Name", text("Extra"))]);
        associate(&mut b, f.base.owner, &[f.base.wall_type], extra);
        let usage_on_type = b.create(
            "IfcMaterialLayerSetUsage",
            vec![
                ("ForLayerSet", Value::Ref(f.base.other_wall_set)),
                ("LayerSetDirection", enumeration("AXIS2")),
                ("DirectionSense", enumeration("POSITIVE")),
                ("OffsetFromReferenceLine", Value::Real(0.0)),
            ],
        );
        let odd_type = wall_type(
            &mut b,
            f.base.owner,
            "Odd",
            f.base.other_wall_map,
            f.kinds.0,
        );
        associate(&mut b, f.base.owner, &[odd_type], usage_on_type);
        let ids = f.apply(b.ops);
        let odd_type = ids[usize::try_from(odd_type.0 - HANDLE_BASE).unwrap()].unwrap();
        for type_object in [f.base.wall_type, odd_type] {
            let op = f.assign(
                type_object,
                &[wall],
                AssignTypeOptions::new().material_usage(true),
            );
            let failure = f.refuse(vec![op]);
            assert!(
                matches!(failure, AuthoringFailure::MaterialUsageUndetermined { object, .. } if object == type_object),
                "{}: {failure:?}",
                f.name()
            );
        }
    });
}

#[test]
fn a_profile_set_gives_a_profile_set_usage() {
    for (release, token) in RELEASES.into_iter().skip(1) {
        let mut f = fixture(release, token);
        let mut b = Batch::default();
        let steel = b.create("IfcMaterial", vec![("Name", text("Steel"))]);
        let centre = b.create(
            "IfcCartesianPoint",
            vec![("Coordinates", reals(&[0.0, 0.0]))],
        );
        let position = b.create(
            "IfcAxis2Placement2D",
            vec![("Location", Value::Ref(centre))],
        );
        let profile = b.create(
            "IfcRectangleProfileDef",
            vec![
                ("ProfileType", enumeration("AREA")),
                ("Position", Value::Ref(position)),
                ("XDim", Value::Real(0.2)),
                ("YDim", Value::Real(0.4)),
            ],
        );
        let material_profile = b.create(
            "IfcMaterialProfile",
            vec![
                ("Material", Value::Ref(steel)),
                ("Profile", Value::Ref(profile)),
            ],
        );
        let set = b.create(
            "IfcMaterialProfileSet",
            vec![
                ("Name", text("B-200x400")),
                ("MaterialProfiles", refs(&[material_profile])),
            ],
        );
        let beam_type = b.push(AuthorOp::TypeObject {
            type_name: "IfcBeamType".into(),
            attributes: named(vec![
                ("Name", text("B")),
                ("PredefinedType", enumeration("BEAM")),
            ]),
            owner_history: Some(f.base.owner),
        });
        associate(&mut b, f.base.owner, &[beam_type], set);
        let beam = b.push(f.product("IfcBeam", vec![("Name", text("Beam"))]));
        let ids = f.apply(b.ops);
        let real =
            |handle: EntityId| ids[usize::try_from(handle.0 - HANDLE_BASE).unwrap()].unwrap();
        let (beam_type, beam, set) = (real(beam_type), real(beam), real(set));
        let op = f.assign(
            beam_type,
            &[beam],
            AssignTypeOptions::new().material_usage(true),
        );
        f.apply(vec![op]);
        let usage = usage_of(&f, beam);
        assert_eq!(
            f.get(usage).type_name.as_ref(),
            "IFCMATERIALPROFILESETUSAGE"
        );
        assert_eq!(f.reference(usage, "ForProfileSet"), set);
        assert_eq!(f.attr(usage, "CardinalPoint"), Value::Null);
    }
}

#[test]
fn a_single_material_applies_through_the_type() {
    each_release(|mut f| {
        let mut b = Batch::default();
        let wood = b.create("IfcMaterial", vec![("Name", text("Wood"))]);
        associate(&mut b, f.base.owner, &[f.base.door_type], wood);
        let door = b.push(f.product("IfcDoor", vec![("Name", text("Door"))]));
        let ids = f.apply(b.ops);
        let door = ids[usize::try_from(door.0 - HANDLE_BASE).unwrap()].unwrap();
        let op = f.assign(f.base.door_type, &[door], AssignTypeOptions::propagate());
        f.apply(vec![op]);
        assert!(f
            .relationships("IFCRELASSOCIATESMATERIAL", "RelatedObjects", door)
            .is_empty());
        assert_mapped(&f, door, f.base.door_map);
    });
}
