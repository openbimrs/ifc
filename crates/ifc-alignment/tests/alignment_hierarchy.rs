//! The public `IfcAlignment` hierarchy (#238).
//!
//! An alignment reaches its layouts and referents through `IfcRelNests`,
//! its child alignments through `IfcRelAggregates`, and the products it
//! positions through `IfcRelPositions` (IFC4X3 ADD2 concept templates
//! "Alignment Layouts", "Alignment Layout - Reusing Horizontal Layout" and
//! "Object Nesting"). Fixtures cover zero, one and several layouts of each
//! kind, a parent with children, and wrong-type refusals.

mod support;

use ifc_alignment::{
    read_cant_segment, read_horizontal_segment, AlignmentError, AlignmentView, CantLayout,
    ProfileSeam, VerticalLayout, VerticalSeamKind,
};
use ifc_model::EntityId;
use support::{metres, Builder};

#[test]
fn an_alignment_without_layouts_reports_empty_lists() {
    let mut b = Builder::new();
    let bare = b.alignment("bare");
    let model = b.finish();
    let view = AlignmentView::for_model(&model).expect("IFC4X3");

    let hierarchy = view.hierarchy(bare).expect("hierarchy");
    assert_eq!(hierarchy.alignment, bare);
    assert_eq!(hierarchy.parent, None);
    assert!(hierarchy.horizontal.is_empty());
    assert!(hierarchy.vertical.is_empty());
    assert!(hierarchy.cant.is_empty());
    assert!(hierarchy.referents.is_empty());
    assert!(hierarchy.children.is_empty());
    assert!(hierarchy.positioned.is_empty());
    assert_eq!(hierarchy.sole_horizontal(), Ok(None));
    assert_eq!(hierarchy.sole_vertical(), Ok(None));
    assert_eq!(hierarchy.sole_cant(), Ok(None));
    assert_eq!(view.governing_horizontal(bare), Ok(None));
    assert_eq!(view.alignments(), vec![bare]);
}

#[test]
fn one_layout_of_each_kind_is_the_sole_one() {
    let mut b = Builder::new();
    let road = b.alignment("road");
    let (h, v, c) = (b.horizontal(), b.vertical(), b.cant());
    let curve = b.curve();
    let marker = b.marker(curve, 5.0);
    // Layouts and referents may share one nesting relationship.
    b.nest(road, &[h, v, c, marker]);
    let model = b.finish();
    let view = AlignmentView::for_model(&model).expect("IFC4X3");

    let hierarchy = view.hierarchy(road).expect("hierarchy");
    assert_eq!(hierarchy.horizontal, vec![h]);
    assert_eq!(hierarchy.vertical, vec![v]);
    assert_eq!(hierarchy.cant, vec![c]);
    assert_eq!(hierarchy.referents, vec![marker]);
    assert_eq!(hierarchy.sole_horizontal(), Ok(Some(h)));
    assert_eq!(hierarchy.sole_vertical(), Ok(Some(v)));
    assert_eq!(hierarchy.sole_cant(), Ok(Some(c)));
    assert_eq!(view.governing_horizontal(road), Ok(Some(h)));
}

#[test]
fn several_layouts_keep_nesting_order_and_refuse_a_sole_pick() {
    let mut b = Builder::new();
    let road = b.alignment("road");
    let (h1, h2) = (b.horizontal(), b.horizontal());
    let (v1, v2, v3) = (b.vertical(), b.vertical(), b.vertical());
    let (c1, c2) = (b.cant(), b.cant());
    b.nest(road, &[v2, h1, c1]);
    b.nest(road, &[v1, h2, v3, c2]);
    let model = b.finish();
    let view = AlignmentView::for_model(&model).expect("IFC4X3");

    let hierarchy = view.hierarchy(road).expect("hierarchy");
    assert_eq!(hierarchy.horizontal, vec![h1, h2]);
    assert_eq!(hierarchy.vertical, vec![v2, v1, v3]);
    assert_eq!(hierarchy.cant, vec![c1, c2]);
    for refused in [
        hierarchy.sole_horizontal(),
        hierarchy.sole_vertical(),
        hierarchy.sole_cant(),
    ] {
        assert!(
            matches!(
                refused,
                Err(AlignmentError::SemanticViolation { entity: Some(e), .. }) if e == road
            ),
            "{refused:?}"
        );
    }
    assert!(view.governing_horizontal(road).is_err());
}

/// "Alignment Layout - Reusing Horizontal Layout": the parent nests the
/// horizontal layout, the children aggregated under it nest their own
/// vertical (and cant) layouts.
#[test]
fn child_alignments_reuse_the_parent_horizontal_layout() {
    let mut b = Builder::new();
    let project = b.project();
    let parent = b.alignment("parent");
    let left = b.alignment("left track");
    let right = b.alignment("right track");
    let h = b.horizontal();
    let (v1, v2, c) = (b.vertical(), b.vertical(), b.cant());
    b.aggregate(project, &[parent]);
    b.nest(parent, &[h]);
    b.aggregate(parent, &[left, right]);
    b.nest(left, &[v1]);
    b.nest(right, &[v2, c]);
    let model = b.finish();
    let view = AlignmentView::for_model(&model).expect("IFC4X3");

    let top = view.hierarchy(parent).expect("parent");
    assert_eq!(top.children, vec![left, right]);
    assert_eq!(
        top.parent, None,
        "aggregated under the project, not an alignment"
    );

    let child = view.hierarchy(right).expect("child");
    assert_eq!(child.parent, Some(parent));
    assert!(child.horizontal.is_empty());
    assert_eq!(child.vertical, vec![v2]);
    assert_eq!(child.cant, vec![c]);
    assert_eq!(view.parent_alignment(left), Ok(Some(parent)));

    assert_eq!(view.governing_horizontal(left), Ok(Some(h)));
    assert_eq!(view.governing_horizontal(right), Ok(Some(h)));
    assert_eq!(view.alignments(), vec![parent, left, right]);
}

#[test]
fn an_aggregation_cycle_is_refused_not_walked_forever() {
    let mut b = Builder::new();
    let (a, c) = (b.alignment("a"), b.alignment("c"));
    b.aggregate(a, &[c]);
    b.aggregate(c, &[a]);
    let model = b.finish();
    let view = AlignmentView::for_model(&model).expect("IFC4X3");
    assert!(matches!(
        view.governing_horizontal(a),
        Err(AlignmentError::SemanticViolation { .. })
    ));
}

#[test]
fn an_alignment_aggregated_twice_is_refused() {
    let mut b = Builder::new();
    let (p1, p2, child) = (b.alignment("p1"), b.alignment("p2"), b.alignment("child"));
    b.aggregate(p1, &[child]);
    b.aggregate(p2, &[child]);
    let model = b.finish();
    let view = AlignmentView::for_model(&model).expect("IFC4X3");
    assert!(matches!(
        view.hierarchy(child),
        Err(AlignmentError::SemanticViolation { entity: Some(e), .. }) if e == child
    ));
}

#[test]
fn referents_keep_nesting_order_and_positioned_products_are_listed() {
    let mut b = Builder::new();
    let road = b.alignment("road");
    let curve = b.curve();
    let (r1, r2, r3) = (
        b.marker(curve, 0.0),
        b.marker(curve, 50.0),
        b.marker(curve, 25.0),
    );
    let pier = b.marker(curve, 75.0);
    b.nest(road, &[r1, r2, r3]);
    b.positions(road, &[pier]);
    let model = b.finish();
    let view = AlignmentView::for_model(&model).expect("IFC4X3");

    let hierarchy = view.hierarchy(road).expect("hierarchy");
    assert_eq!(
        hierarchy.referents,
        vec![r1, r2, r3],
        "nesting order, not distance"
    );
    assert_eq!(hierarchy.positioned, vec![pier]);
}

#[test]
fn the_same_object_nested_twice_is_refused() {
    let mut b = Builder::new();
    let road = b.alignment("road");
    let h = b.horizontal();
    b.nest(road, &[h]);
    b.nest(road, &[h]);
    let model = b.finish();
    let view = AlignmentView::for_model(&model).expect("IFC4X3");
    assert!(matches!(
        view.hierarchy(road),
        Err(AlignmentError::SemanticViolation { entity: Some(e), .. }) if e == h
    ));
}

#[test]
fn layout_segments_resolve_each_family_in_nesting_order() {
    let mut b = Builder::new();
    let (h, v, c) = (b.horizontal(), b.vertical(), b.cant());
    let (hs1, hs2) = (b.horizontal_segment(10.0), b.horizontal_segment(20.0));
    let vs = b.vertical_segment(0.0, 30.0, 100.0, (0.01, 0.01), None, "CONSTANTGRADIENT");
    let cs = b.cant_segment(0.0, 30.0);
    b.nest(h, &[hs2, hs1]);
    b.nest(v, &[vs]);
    b.nest(c, &[cs]);
    let model = b.finish();
    let view = AlignmentView::for_model(&model).expect("IFC4X3");

    let horizontal = view.layout_segments(h).expect("horizontal");
    assert_eq!(
        horizontal,
        vec![
            Builder::parameters_of(&model, hs2),
            Builder::parameters_of(&model, hs1)
        ]
    );
    let first = read_horizontal_segment(&model, horizontal[0], metres()).expect("reads");
    assert_eq!(first.segment_length, 20.0);
    assert_eq!(
        view.layout_segments(v),
        Ok(vec![Builder::parameters_of(&model, vs)])
    );
    let cant = view.layout_segments(c).expect("cant");
    assert!(read_cant_segment(&model, cant[0], metres()).is_ok());
}

#[test]
fn a_wrong_type_is_refused_by_every_entry_point() {
    let mut b = Builder::new();
    let road = b.alignment("road");
    let (h, c) = (b.horizontal(), b.cant());
    b.nest(road, &[h, c]);
    let model = b.finish();
    let view = AlignmentView::for_model(&model).expect("IFC4X3");

    let wrong = |result: Result<(), AlignmentError>, entity: EntityId| {
        assert!(
            matches!(&result, Err(AlignmentError::WrongType { entity: e, .. }) if *e == entity),
            "{entity}: {result:?}",
        );
    };
    wrong(view.hierarchy(h).map(|_| ()), h);
    wrong(view.parent_alignment(h).map(|_| ()), h);
    wrong(view.governing_horizontal(c).map(|_| ()), c);
    wrong(view.layout_segments(road).map(|_| ()), road);
    wrong(VerticalLayout::resolve(&model, c, metres()).map(|_| ()), c);
    wrong(VerticalLayout::resolve(&model, h, metres()).map(|_| ()), h);
    wrong(CantLayout::resolve(&model, h, metres()).map(|_| ()), h);
    assert!(matches!(
        view.hierarchy(EntityId(9_999)),
        Err(AlignmentError::MissingEntity { .. })
    ));
}

#[test]
fn a_release_other_than_ifc4x3_is_refused_before_any_traversal() {
    for token in ["IFC2X3", "IFC4", "IFC4X1", "IFC4X2"] {
        let mut b = Builder::with_schema(token);
        let v = b.vertical();
        let model = b.finish();
        assert!(
            matches!(
                AlignmentView::for_model(&model),
                Err(AlignmentError::UnsupportedSchema { .. })
            ),
            "{token}"
        );
        assert!(
            matches!(
                VerticalLayout::resolve(&model, v, metres()),
                Err(AlignmentError::UnsupportedSchema { .. })
            ),
            "{token}"
        );
    }
}

#[test]
fn a_vertical_layout_resolves_like_a_cant_layout() {
    let mut b = Builder::new();
    let v = b.vertical();
    // A grade, a parabolic crest, and the zero-length closing segment the
    // concept template requires; a circular arc is read even though no
    // exact elevation law exists for it yet.
    let grade = b.vertical_segment(100.0, 50.0, 10.0, (0.02, 0.02), None, "CONSTANTGRADIENT");
    let crest = b.vertical_segment(
        150.0,
        100.0,
        11.0,
        (0.02, -0.01),
        Some(3_000.0),
        "PARABOLICARC",
    );
    let arc = b.vertical_segment(
        250.0,
        40.0,
        11.5,
        (-0.01, 0.0),
        Some(4_000.0),
        "CIRCULARARC",
    );
    let end = b.vertical_segment(290.0, 0.0, 11.3, (0.0, 0.0), None, "CONSTANTGRADIENT");
    b.nest(v, &[grade, crest, arc, end]);
    let model = b.finish();

    let layout = VerticalLayout::resolve(&model, v, metres()).expect("resolves");
    assert_eq!(layout.entity, v);
    assert_eq!(layout.segments().len(), 4);
    assert_eq!(
        layout.segments()[1].entity,
        Builder::parameters_of(&model, crest)
    );
    assert_eq!(layout.start_dist_along(), 100.0);
    assert_eq!(layout.length(), 190.0);
    // Two seams between segments of positive length; the closing segment
    // has none. Every grade continues.
    assert_eq!(layout.seams().len(), 2);
    assert!(layout
        .seams()
        .iter()
        .all(|seam| seam.kind == VerticalSeamKind::Tangential));
    layout.require_tangential().expect("tangent");
}

/// A gap and an empty layout are refused. A grade break is legal IFC4.3
/// (#259): it resolves and is reported, and only an explicit tangency
/// request refuses it.
#[test]
fn a_vertical_layout_refuses_gaps_and_emptiness_and_reports_grade_breaks() {
    let mut b = Builder::new();
    let (gappy, kinked, empty) = (b.vertical(), b.vertical(), b.vertical());
    let g1 = b.vertical_segment(0.0, 50.0, 10.0, (0.02, 0.02), None, "CONSTANTGRADIENT");
    let g2 = b.vertical_segment(60.0, 50.0, 11.0, (0.02, 0.02), None, "CONSTANTGRADIENT");
    b.nest(gappy, &[g1, g2]);
    let k1 = b.vertical_segment(0.0, 50.0, 10.0, (0.02, 0.02), None, "CONSTANTGRADIENT");
    let k2 = b.vertical_segment(50.0, 50.0, 11.0, (0.03, 0.03), None, "CONSTANTGRADIENT");
    b.nest(kinked, &[k1, k2]);
    let model = b.finish();

    assert!(matches!(
        VerticalLayout::resolve(&model, gappy, metres()),
        Err(AlignmentError::InvalidSegment { entity, .. })
            if entity == Builder::parameters_of(&model, g2)
    ));
    let kinked = VerticalLayout::resolve(&model, kinked, metres()).expect("grade break");
    let (k1, k2) = (
        Builder::parameters_of(&model, k1),
        Builder::parameters_of(&model, k2),
    );
    assert_eq!(kinked.seams().len(), 1);
    let seam = &kinked.seams()[0];
    assert_eq!((seam.previous, seam.next), (k1, k2));
    assert_eq!(seam.kind, VerticalSeamKind::GradeBreak);
    assert_eq!(
        (
            seam.distance_along,
            seam.incoming_gradient,
            seam.outgoing_gradient
        ),
        (50.0, 0.02, 0.03)
    );
    assert_eq!(
        kinked.require_tangential(),
        Err(AlignmentError::ProfileDiscontinuity {
            entity: k2,
            previous: k1,
            seam: ProfileSeam::Gradient,
            expected: 0.02,
            actual: 0.03,
        })
    );
    assert!(matches!(
        VerticalLayout::resolve(&model, empty, metres()),
        Err(AlignmentError::SemanticViolation { entity: Some(e), .. }) if e == empty
    ));
}

/// A broken relationship elsewhere in the file does not refuse an
/// unrelated alignment, but does refuse the alignment it belongs to.
#[test]
fn a_dangling_nest_only_refuses_its_own_alignment() {
    let mut b = Builder::new();
    let (sound, broken) = (b.alignment("sound"), b.alignment("broken"));
    let h = b.horizontal();
    b.nest(sound, &[h]);
    let mut model = b.finish();
    let missing = EntityId(9_999);
    let next = model.next_id();
    model.insert(
        next,
        ifc_model::Entity::new(
            "IFCRELNESTS",
            vec![
                ifc_model::Value::Text("dangling-nest-000000001".into()),
                ifc_model::Value::Null,
                ifc_model::Value::Null,
                ifc_model::Value::Null,
                ifc_model::Value::Ref(broken),
                ifc_model::Value::List(vec![ifc_model::Value::Ref(missing)]),
            ],
        ),
    );
    let view = AlignmentView::for_model(&model).expect("IFC4X3");
    assert_eq!(view.hierarchy(sound).expect("sound").horizontal, vec![h]);
    assert!(matches!(
        view.hierarchy(broken),
        Err(AlignmentError::DanglingReference { target, .. }) if target == missing
    ));
}
