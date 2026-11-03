//! Descriptions for a purpose other than Body, and tessellated bounds (#651).
//!
//! The opening of an IFC4 Reference View carries its void as a `Reference`
//! representation; its bounds are read from the vertices the faces address,
//! never from the whole point list. Models are built here from quoted records;
//! expected values are derived at each test.

use ifc_geometry::{
    body_description, body_description_for, units, BodyDescription, BodyKind, GeometryError,
    RepresentationPurpose,
};
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

use super::step::{close, metres, PRODUCT};

/// An opening at (10, 20, 30) mm, local X along +Y, whose only representation
/// is `Reference` over `items`; the product is `#30`. Lengths are millimetres.
///
/// ```text
/// #15 'Reference' sub-context   #20 IfcShapeRepresentation 'Reference'
/// #30 IfcOpeningElement         frame: origin (0.01, 0.02, 0.03) m,
///                               X = +Y, Y = -X, Z = +Z
/// ```
fn opening(schema: &str, entities: &str, items: &str) -> Model {
    let text = format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('{schema}'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.);
#3=IFCUNITASSIGNMENT((#1,#2));
#4=IFCPROJECT('0',$,'p',$,$,$,$,(#10),#3);
#5=IFCCARTESIANPOINT((0.,0.,0.));
#6=IFCAXIS2PLACEMENT3D(#5,$,$);
#10=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#6,$);
#15=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Reference','Model',*,*,*,*,#10,$,.MODEL_VIEW.,$);
#11=IFCLOCALPLACEMENT($,#12);
#12=IFCAXIS2PLACEMENT3D(#13,$,#14);
#13=IFCCARTESIANPOINT((10.,20.,30.));
#14=IFCDIRECTION((0.,1.,0.));
#20=IFCSHAPEREPRESENTATION(#15,'Reference','Tessellation',({items}));
#21=IFCPRODUCTDEFINITIONSHAPE($,$,(#20));
#30=IFCOPENINGELEMENT('1',$,'o',$,$,#11,#21,$,.OPENING.);
{entities}
ENDSEC;
END-ISO-10303-21;
"
    );
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("test model parses")
}

/// A box's faces over a nine-point list whose first point is unreferenced.
///
/// ```text
/// #100  points: (9000,9000,9000) unreferenced, then the box corners
///       (1500,0,500)..(2500,200,2500) mm
/// #107  IfcPolygonalFaceSet, PnIndex (2..9), six quad faces #101..#106
/// ```
const BOX: &str = "#100=IFCCARTESIANPOINTLIST3D(((9000.,9000.,9000.),(1500.,0.,500.),(2500.,0.,500.),(2500.,200.,500.),(1500.,200.,500.),(1500.,0.,2500.),(2500.,0.,2500.),(2500.,200.,2500.),(1500.,200.,2500.)),$);
     #101=IFCINDEXEDPOLYGONALFACE((1,4,3,2));
     #102=IFCINDEXEDPOLYGONALFACE((5,6,7,8));
     #103=IFCINDEXEDPOLYGONALFACE((1,2,6,5));
     #104=IFCINDEXEDPOLYGONALFACE((2,3,7,6));
     #105=IFCINDEXEDPOLYGONALFACE((3,4,8,7));
     #106=IFCINDEXEDPOLYGONALFACE((4,1,5,8));
     #107=IFCPOLYGONALFACESET(#100,.T.,(#101,#102,#103,#104,#105,#106),(2,3,4,5,6,7,8,9));";

fn describe_for(
    model: &Model,
    product: EntityId,
    purpose: RepresentationPurpose,
) -> Result<Option<BodyDescription>, GeometryError> {
    body_description_for(
        model,
        &units::resolve(model).expect("project units"),
        product,
        purpose,
    )
}

fn reference(model: &Model) -> Result<Option<BodyDescription>, GeometryError> {
    describe_for(model, PRODUCT, RepresentationPurpose::Reference)
}

fn bounds_of(body: &BodyDescription) -> ([f64; 3], [f64; 3]) {
    let bounds = body
        .sole_item()
        .expect("one item")
        .bounds
        .expect("a tessellated item has bounds");
    (bounds.min, bounds.max)
}

/// The Reference representation is described; the Body call finds nothing.
/// The bounds skip the unreferenced first point and sit in the
/// representation frame, which the description reports.
#[test]
fn a_reference_representation_is_described_with_referenced_bounds() {
    let model = opening("IFC4", BOX, "#107");
    let units = units::resolve(&model).expect("project units");
    assert!(body_description(&model, &units, PRODUCT).unwrap().is_none());

    let body = reference(&model).unwrap().expect("a reference");
    assert_eq!(body.representation, EntityId(20));
    assert_eq!(body.purpose, RepresentationPurpose::Reference);
    assert!(close(body.frame.origin, [0.01, 0.02, 0.03]));
    let item = body.sole_item().expect("one item");
    assert_eq!(item.item, EntityId(107));
    assert_eq!(item.kind, BodyKind::Tessellated);
    assert_eq!(item.item_world, body.frame);
    let (min, max) = bounds_of(&body);
    assert!(close(min, [1.5, 0.0, 0.5]), "{min:?}");
    assert!(close(max, [2.5, 0.2, 2.5]), "{max:?}");
    // X = +Y, Y = -X: the minimum corner lands at the frame origin plus
    // 1.5 m along +Y and 0.5 m along +Z.
    assert!(close(body.frame.apply(min), [0.01, 1.52, 0.53]));
}

/// Body is Body: same answer through either entry point, and no bounds for
/// a swept solid.
#[test]
fn body_description_is_the_body_purpose_of_body_description_for() {
    let model = metres(
        "#100=IFCRECTANGLEPROFILEDEF(.AREA.,$,$,2.,1.);
         #101=IFCCARTESIANPOINT((0.,0.,0.));
         #102=IFCAXIS2PLACEMENT3D(#101,$,$);
         #103=IFCDIRECTION((0.,0.,1.));
         #104=IFCEXTRUDEDAREASOLID(#100,#102,#103,3.);",
        "#104",
    );
    let units = units::resolve(&model).expect("project units");
    let plain = body_description(&model, &units, PRODUCT).unwrap();
    let body = describe_for(&model, PRODUCT, RepresentationPurpose::Body).unwrap();
    assert_eq!(plain, body);
    let body = body.expect("a body");
    assert_eq!(body.purpose, RepresentationPurpose::Body);
    assert!(body.sole_item().unwrap().bounds.is_none());
}

/// A missing purpose is `None`; no other representation stands in.
#[test]
fn a_missing_purpose_never_falls_back_to_another() {
    let opening = opening("IFC4", BOX, "#107");
    assert!(describe_for(&opening, PRODUCT, RepresentationPurpose::Axis)
        .unwrap()
        .is_none());
    assert!(describe_for(&opening, PRODUCT, RepresentationPurpose::Body)
        .unwrap()
        .is_none());
    let beam = metres("", "");
    assert!(reference(&beam).unwrap().is_none());
}

/// Bounds follow the mapping: `frame o target o origin` applied to the
/// item's own coordinates, so a scale of 2 and a shift of 100 mm give
/// `2 * p + 0.1 m`.
#[test]
fn a_mapped_tessellation_reports_bounds_through_its_mapping() {
    let model = opening(
        "IFC4",
        &format!(
            "{BOX}
             #110=IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Reference','Model',*,*,*,*,#10,$,.MODEL_VIEW.,$);
             #111=IFCSHAPEREPRESENTATION(#110,'Reference','Tessellation',(#107));
             #112=IFCCARTESIANPOINT((0.,0.,0.));
             #113=IFCAXIS2PLACEMENT3D(#112,$,$);
             #114=IFCREPRESENTATIONMAP(#113,#111);
             #115=IFCCARTESIANPOINT((100.,0.,0.));
             #116=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#115,2.,$);
             #117=IFCMAPPEDITEM(#114,#116);"
        ),
        "#117",
    );
    let body = reference(&model).unwrap().expect("a reference");
    let item = body.sole_item().expect("one item");
    assert_eq!(item.mapped_by, [EntityId(117)]);
    let (min, max) = bounds_of(&body);
    assert!(close(min, [3.1, 0.0, 1.0]), "{min:?}");
    assert!(close(max, [5.1, 0.4, 5.0]), "{max:?}");
    // The bounds and `item_world` agree on where the item's own corner is.
    assert!(close(
        body.frame.apply(min),
        item.item_world.apply([1.5, 0.0, 0.5])
    ));
}

/// A triangulated set reaches its points through `PnIndex`; a point no
/// triangle addresses stays out of the box.
#[test]
fn a_triangulated_set_bounds_only_the_indexed_vertices() {
    let model = opening(
        "IFC4",
        "#100=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(1000.,0.,0.),(0.,1000.,0.),(0.,0.,1000.),(7000.,7000.,7000.)),$);
         #101=IFCTRIANGULATEDFACESET(#100,$,$,((1,3,2),(1,2,4),(1,4,3),(2,3,4)),(1,2,3,4));",
        "#101",
    );
    let body = reference(&model).unwrap().expect("a reference");
    let (min, max) = bounds_of(&body);
    assert!(close(min, [0.0, 0.0, 0.0]), "{min:?}");
    assert!(close(max, [1.0, 1.0, 1.0]), "{max:?}");
}

/// Without `PnIndex` the faces index `Coordinates` directly, and a stray
/// point no face addresses stays out of the box.
#[test]
fn a_polygonal_set_without_pn_index_bounds_the_addressed_points() {
    let model = opening(
        "IFC4",
        "#100=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(1000.,0.,0.),(1000.,2000.,0.),(0.,2000.,0.),(9000.,9000.,9000.)),$);
         #101=IFCINDEXEDPOLYGONALFACE((1,2,3,4));
         #102=IFCPOLYGONALFACESET(#100,.F.,(#101),$);",
        "#102",
    );
    let body = reference(&model).unwrap().expect("a reference");
    let (min, max) = bounds_of(&body);
    assert!(close(min, [0.0, 0.0, 0.0]), "{min:?}");
    assert!(close(max, [1.0, 2.0, 0.0]), "{max:?}");
}

/// A face with voids is read through its inner loops too: the hole's
/// vertices are addressed, so they count.
#[test]
fn a_face_with_voids_is_bounded() {
    let model = opening(
        "IFC4",
        "#100=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(4000.,0.,0.),(4000.,4000.,0.),(0.,4000.,0.),(1000.,1000.,0.),(2000.,1000.,0.),(2000.,2000.,0.)),$);
         #101=IFCINDEXEDPOLYGONALFACEWITHVOIDS((1,2,3,4),((5,6,7)));
         #102=IFCPOLYGONALFACESET(#100,.F.,(#101),$);",
        "#102",
    );
    let body = reference(&model).unwrap().expect("a reference");
    let (min, max) = bounds_of(&body);
    assert!(close(min, [0.0, 0.0, 0.0]), "{min:?}");
    assert!(close(max, [4.0, 4.0, 0.0]), "{max:?}");
}

#[test]
fn a_non_finite_vertex_is_refused() {
    let error = refusal(
        "#100=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(1.E999,0.,0.),(0.,1.,0.)),$);
         #101=IFCTRIANGULATEDFACESET(#100,$,$,((1,2,3)),$);",
        "#101",
    );
    let GeometryError::Degenerate { detail, .. } = &error else {
        panic!("expected Degenerate, got {error:?}");
    };
    assert_eq!(error.entity(), Some(EntityId(101)));
    assert!(detail.contains("not finite"), "{detail}");
}

fn refusal(entities: &str, items: &str) -> GeometryError {
    reference(&opening("IFC4", entities, items)).expect_err("refused")
}

#[test]
fn a_coord_index_past_coordinates_is_refused() {
    let error = refusal(
        "#100=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(1.,0.,0.),(0.,1.,0.)),$);
         #101=IFCTRIANGULATEDFACESET(#100,$,$,((1,2,9)),$);",
        "#101",
    );
    let GeometryError::Degenerate { detail, .. } = &error else {
        panic!("expected Degenerate, got {error:?}");
    };
    assert_eq!(error.entity(), Some(EntityId(101)));
    assert!(
        detail.contains("CoordIndex addresses vertex 9 but Coordinates has 3"),
        "{detail}"
    );
}

#[test]
fn an_inner_loop_past_pn_index_is_refused() {
    let error = refusal(
        "#100=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(4.,0.,0.),(4.,4.,0.),(0.,4.,0.)),$);
         #101=IFCINDEXEDPOLYGONALFACEWITHVOIDS((1,2,3,4),((1,2,9)));
         #102=IFCPOLYGONALFACESET(#100,.F.,(#101),(1,2,3,4));",
        "#102",
    );
    let GeometryError::Degenerate { detail, .. } = &error else {
        panic!("expected Degenerate, got {error:?}");
    };
    assert_eq!(error.entity(), Some(EntityId(102)));
    assert!(
        detail.contains("CoordIndex 9 is past the end of PnIndex (4 entries)"),
        "{detail}"
    );
}

#[test]
fn an_index_of_zero_is_refused() {
    let error = refusal(
        "#100=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(1.,0.,0.),(0.,1.,0.)),$);
         #101=IFCTRIANGULATEDFACESET(#100,$,$,((0,2,3)),$);",
        "#101",
    );
    assert!(
        matches!(error, GeometryError::Degenerate { .. }),
        "{error:?}"
    );
}

#[test]
fn a_dangling_or_two_dimensional_point_list_is_refused() {
    let error = refusal("#101=IFCTRIANGULATEDFACESET(#999,$,$,((1,2,3)),$);", "#101");
    assert!(
        matches!(
            error,
            GeometryError::MissingEntity {
                missing: EntityId(999),
                ..
            }
        ),
        "{error:?}"
    );
    let error = refusal(
        "#100=IFCCARTESIANPOINTLIST2D(((0.,0.),(1.,0.),(0.,1.)),$);
         #101=IFCTRIANGULATEDFACESET(#100,$,$,((1,2,3)),$);",
        "#101",
    );
    assert!(
        matches!(error, GeometryError::WrongEntityType { .. }),
        "{error:?}"
    );
}

#[test]
fn a_face_set_that_references_no_vertex_is_refused() {
    let error = refusal(
        "#100=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(1.,0.,0.),(0.,1.,0.)),$);
         #101=IFCTRIANGULATEDFACESET(#100,$,$,(),$);",
        "#101",
    );
    let GeometryError::Degenerate { detail, .. } = &error else {
        panic!("expected Degenerate, got {error:?}");
    };
    assert!(detail.contains("references no vertex"), "{detail}");
}

/// A TIN void or hole flag is refused as lowering refuses it; breakline
/// codes are bounded like any triangulated set.
#[test]
fn an_irregular_network_is_admitted_as_lowering_admits_it() {
    let tin = |flags: &str| {
        format!(
            "#100=IFCCARTESIANPOINTLIST3D(((0.,0.,0.),(1000.,0.,0.),(0.,1000.,0.),(1000.,1000.,500.)),$);
             #101=IFCTRIANGULATEDIRREGULARNETWORK(#100,$,$,((1,2,3),(2,4,3)),$,{flags});"
        )
    };
    let refused = reference(&opening("IFC4X3_ADD2", &tin("(0,-2)"), "#101"))
        .expect_err("a void flag is refused");
    assert!(refused.is_unsupported(), "{refused:?}");
    assert_eq!(refused.entity(), Some(EntityId(101)));

    let body = reference(&opening("IFC4X3_ADD2", &tin("(0,0)"), "#101"))
        .unwrap()
        .expect("a reference");
    let (min, max) = bounds_of(&body);
    assert!(close(min, [0.0, 0.0, 0.0]), "{min:?}");
    assert!(close(max, [1.0, 1.0, 0.5]), "{max:?}");
}

#[test]
fn the_reference_purpose_round_trips_its_name() {
    let purpose = RepresentationPurpose::from_name("Reference").expect("a purpose");
    assert_eq!(purpose, RepresentationPurpose::Reference);
    assert_eq!(purpose.name(), "Reference");
    assert_eq!(purpose.identifier(), Some("Reference"));
}

/// The Reference View fixture: opening O1 carries only a Reference void, in
/// the host's frame (the identity here), and lowering reads the same
/// representation.
#[test]
fn the_reference_view_fixture_opening_is_described() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../test/fixtures/synthetic-lowering/reference_view_openings.ifc");
    let model = StepCodec.read_path(&path).expect("fixture parses");
    let opening = EntityId(47);
    let units = units::resolve(&model).expect("project units");
    assert!(body_description(&model, &units, opening).unwrap().is_none());

    let body = describe_for(&model, opening, RepresentationPurpose::Reference)
        .unwrap()
        .expect("a reference");
    assert_eq!(body.representation, EntityId(42));
    assert!(body.frame.is_identity(1e-12));
    assert_eq!(body.sole_item().unwrap().item, EntityId(41));
    let (min, max) = bounds_of(&body);
    assert!(close(min, [1.5, 0.0, 0.5]), "{min:?}");
    assert!(close(max, [2.5, 0.2, 2.5]), "{max:?}");

    #[cfg(feature = "lowering")]
    {
        use ifc_geometry::lower::{lower_product_representation, LoweringSession};
        let mut session = LoweringSession::new(&model, &units);
        let root =
            lower_product_representation(&mut session, opening, RepresentationPurpose::Reference)
                .expect("lowers");
        assert!(root.is_some());
    }
}
