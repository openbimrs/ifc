#![cfg(feature = "lowering")]
//! Requires the `lowering` feature: this suite exercises the neutral DAG.
//! Profile families: steel sections, ellipse, trapezium, and the nesting
//! composite/derived/mirrored forms.

use axiolid_profile::{Profile, SectionProfile};
use ifc_geometry::lower::profile::lower_profile;
use ifc_geometry::lower::Tolerance;
use ifc_geometry::units;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;
use std::path::PathBuf;

fn model() -> Model {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../test/fixtures/synthetic-surfaces/synthetic_profile_families.ifc");
    StepCodec.read_path(&path).expect("fixture parses")
}

fn profile_named(model: &Model, type_name: &str) -> Profile {
    // EXACT type match: ids_of_type is subtype-inclusive, so asking for
    // IFCISHAPEPROFILEDEF also returns the asymmetric subtype and the
    // assertions would silently read the wrong entity.
    let id = *model
        .ids_of_type(type_name)
        .iter()
        .find(|id| {
            model
                .get(**id)
                .is_some_and(|e| e.type_name.eq_ignore_ascii_case(type_name))
        })
        .unwrap_or_else(|| panic!("fixture has a {type_name}"));
    let scale = units::resolve(model);
    lower_profile(model, id, &scale, &Tolerance::building_scale())
        .unwrap_or_else(|e| panic!("{type_name} must lower: {e}"))
}

/// The I-section keeps its fillet radius and flange edge radius.
///
/// These are exactly the values the kernel could not hold before: an I-beam
/// lowered without them has the right outline and the wrong area, so its
/// section modulus and steel weight are both wrong.
#[test]
fn an_i_section_keeps_its_fillet_and_edge_radius() {
    let Profile::Section(SectionProfile::I {
        depth,
        width,
        web_thickness,
        fillet_radius,
        flange_edge_radius,
        flange_slope,
        ..
    }) = profile_named(&model(), "IFCISHAPEPROFILEDEF")
    else {
        panic!("an I profile must lower to SectionProfile::I");
    };
    // The fixture is an HEA300-like section authored in millimetres.
    assert!((depth - 0.290).abs() < 1e-9, "depth in metres, got {depth}");
    assert!((width - 0.300).abs() < 1e-9, "width in metres, got {width}");
    assert!(
        (web_thickness - 0.0085).abs() < 1e-9,
        "web thickness in metres, got {web_thickness}"
    );
    assert_eq!(
        fillet_radius,
        Some(0.027),
        "the fillet radius must survive lowering"
    );
    assert_eq!(
        flange_edge_radius,
        Some(0.003),
        "the flange edge radius must survive lowering"
    );
    // The slope is an ANGLE: the file declares 2 degrees, and the length
    // factor must not touch it.
    let slope = flange_slope.expect("the flange slope must survive lowering");
    assert!(
        (slope - 2.0_f64.to_radians()).abs() < 1e-9,
        "a slope is converted as an angle, not a length, got {slope}"
    );
}

/// An asymmetric I keeps BOTH flange widths.
///
/// The old kernel had one `width`, so this section could only lower by
/// discarding a flange. The fixture's top flange is deliberately narrower.
#[test]
fn an_asymmetric_i_keeps_both_flange_widths() {
    let Profile::Section(SectionProfile::AsymmetricI {
        bottom_flange_width,
        top_flange_width,
        top_flange_thickness,
        ..
    }) = profile_named(&model(), "IFCASYMMETRICISHAPEPROFILEDEF")
    else {
        panic!("an asymmetric I must lower to SectionProfile::AsymmetricI");
    };
    assert!(
        (bottom_flange_width - 0.300).abs() < 1e-9,
        "bottom flange, got {bottom_flange_width}"
    );
    assert!(
        (top_flange_width - 0.200).abs() < 1e-9,
        "top flange, got {top_flange_width}"
    );
    assert_ne!(
        bottom_flange_width, top_flange_width,
        "the asymmetry is the point: collapsing to one width is the bug"
    );
    assert_eq!(
        top_flange_thickness,
        Some(0.012),
        "an explicit top flange thickness must be preserved"
    );
}

/// An L-section keeps its fillet and edge radius, and both leg dimensions.
///
/// `Width` is OPTIONAL in IFC and `Option` in the kernel; the fixture supplies
/// it, so lowering must carry the declared value rather than defaulting.
#[test]
fn an_l_section_keeps_both_legs_and_its_radii() {
    let Profile::Section(SectionProfile::L {
        depth,
        width,
        thickness,
        fillet_radius,
        edge_radius,
        ..
    }) = profile_named(&model(), "IFCLSHAPEPROFILEDEF")
    else {
        panic!("an L profile must lower to SectionProfile::L");
    };
    assert!((depth - 0.150).abs() < 1e-9, "depth, got {depth}");
    assert_eq!(width, Some(0.100), "the declared width must be carried");
    assert!(
        (thickness - 0.010).abs() < 1e-9,
        "thickness, got {thickness}"
    );
    assert_eq!(fillet_radius, Some(0.012), "fillet radius must survive");
    assert_eq!(edge_radius, Some(0.006), "edge radius must survive");
}

/// Every remaining parameterized family lowers with its own dimensions.
///
/// One test over the table rather than six near-identical ones: what matters
/// is that each family reaches its OWN variant with the values the file
/// declares, not that each has its own test function.
#[test]
fn each_parameterized_family_reaches_its_own_variant() {
    let m = model();
    match profile_named(&m, "IFCTSHAPEPROFILEDEF") {
        Profile::Section(SectionProfile::T {
            depth,
            flange_width,
            web_edge_radius,
            ..
        }) => {
            assert!((depth - 0.200).abs() < 1e-9);
            assert!((flange_width - 0.200).abs() < 1e-9);
            assert_eq!(web_edge_radius, Some(0.003), "T keeps its web edge radius");
        }
        other => panic!("T lowered as {other:?}"),
    }
    match profile_named(&m, "IFCUSHAPEPROFILEDEF") {
        Profile::Section(SectionProfile::U {
            flange_width,
            edge_radius,
            ..
        }) => {
            assert!((flange_width - 0.075).abs() < 1e-9);
            assert_eq!(edge_radius, Some(0.006));
        }
        other => panic!("U lowered as {other:?}"),
    }
    match profile_named(&m, "IFCCSHAPEPROFILEDEF") {
        Profile::Section(SectionProfile::C { girth, .. }) => {
            assert!((girth - 0.020).abs() < 1e-9, "C keeps its girth");
        }
        other => panic!("C lowered as {other:?}"),
    }
    match profile_named(&m, "IFCZSHAPEPROFILEDEF") {
        Profile::Section(SectionProfile::Z { flange_width, .. }) => {
            assert!((flange_width - 0.075).abs() < 1e-9);
        }
        other => panic!("Z lowered as {other:?}"),
    }
}

/// The trapezium keeps a NEGATIVE top offset.
///
/// `TopXOffset` is a signed IfcLengthMeasure. Clamping it to non-negative,
/// or reading it with a positive-only helper, silently mirrors the section.
#[test]
fn a_trapezium_keeps_its_negative_top_offset() {
    let Profile::Section(SectionProfile::Trapezium {
        bottom_x,
        top_x,
        top_offset,
        ..
    }) = profile_named(&model(), "IFCTRAPEZIUMPROFILEDEF")
    else {
        panic!("a trapezium must lower to SectionProfile::Trapezium");
    };
    assert!((bottom_x - 0.300).abs() < 1e-9);
    assert!((top_x - 0.180).abs() < 1e-9);
    assert!(
        (top_offset + 0.040).abs() < 1e-9,
        "the offset is signed and negative here, got {top_offset}"
    );
}

/// The ellipse keeps distinct semi-axes.
#[test]
fn an_ellipse_keeps_distinct_semi_axes() {
    let Profile::Ellipse(e) = profile_named(&model(), "IFCELLIPSEPROFILEDEF") else {
        panic!("an ellipse must lower to Profile::Ellipse");
    };
    assert!((e.semi_axis_x - 0.200).abs() < 1e-9);
    assert!((e.semi_axis_y - 0.120).abs() < 1e-9);
    assert_ne!(
        e.semi_axis_x, e.semi_axis_y,
        "collapsing an ellipse to a circle is the failure this catches"
    );
}

/// A composite keeps every member, in order.
///
/// Order is the only identity a composite member has: reordering silently
/// changes which section sits where in a built-up column.
#[test]
fn a_composite_keeps_its_members_in_order() {
    let Profile::Composite(members) = profile_named(&model(), "IFCCOMPOSITEPROFILEDEF") else {
        panic!("a composite must lower to Profile::Composite");
    };
    assert_eq!(members.len(), 2, "the fixture composes two members");
    assert!(
        matches!(members[0], Profile::Rectangle(_)),
        "first member must be the rectangle, got {:?}",
        members[0]
    );
    assert!(
        matches!(members[1], Profile::Circle(_)),
        "second member must be the circle, got {:?}",
        members[1]
    );
}

/// A derived profile keeps its parent AND its operator transform.
#[test]
fn a_derived_profile_keeps_its_basis_and_transform() {
    let Profile::Derived { basis, transform } = profile_named(&model(), "IFCDERIVEDPROFILEDEF")
    else {
        panic!("a derived profile must lower to Profile::Derived");
    };
    assert!(
        matches!(*basis, Profile::Section(SectionProfile::I { .. })),
        "the basis must be the I section it references"
    );
    // The fixture scales by 2 and translates; an identity transform would
    // mean the operator was read but discarded.
    assert!(
        transform.matrix2.determinant().abs() > 1.0,
        "the operator scale must be applied, got {:?}",
        transform.matrix2
    );
}

/// A mirrored profile mirrors, even though its Operator is DERIVED.
///
/// `IfcMirroredProfileDef` inherits ParentProfile and Operator, but the
/// schema marks Operator DERIVED: the file carries a `*` placeholder and no
/// operator entity exists to read. Lowering it through the parent path would
/// produce an UNMIRRORED copy that looks correct in isolation.
#[test]
fn a_mirrored_profile_actually_mirrors() {
    let Profile::Derived { basis, transform } = profile_named(&model(), "IFCMIRROREDPROFILEDEF")
    else {
        panic!("a mirrored profile must lower to Profile::Derived");
    };
    assert!(
        matches!(*basis, Profile::Section(SectionProfile::I { .. })),
        "the basis must be the I section it references"
    );
    assert!(
        transform.matrix2.determinant() < 0.0,
        "a mirror REVERSES orientation: determinant must be negative, got {}",
        transform.matrix2.determinant()
    );
}

/// A self-referencing derived profile is refused, not stack-overflowed.
///
/// Nothing in IFC forbids a profile whose parent is itself. Without a depth
/// budget the recursive lowerer would exhaust the stack, which is a crash a
/// consumer cannot catch, rather than a typed error it can report.
#[test]
fn a_profile_reference_cycle_is_refused_rather_than_overflowing() {
    use ifc_model::{Entity, Value};

    let mut m = Model::new();
    let a = EntityId(1);
    let b = EntityId(2);
    // Two derived profiles, each naming the other as its parent.
    m.insert(
        a,
        Entity::new(
            "IFCDERIVEDPROFILEDEF",
            vec![
                Value::Enum("AREA".into()),
                Value::Null,
                Value::Ref(b),
                Value::Null,
                Value::Null,
            ],
        ),
    );
    m.insert(
        b,
        Entity::new(
            "IFCDERIVEDPROFILEDEF",
            vec![
                Value::Enum("AREA".into()),
                Value::Null,
                Value::Ref(a),
                Value::Null,
                Value::Null,
            ],
        ),
    );

    let scale = units::resolve(&m);
    let error = lower_profile(&m, a, &scale, &Tolerance::building_scale())
        .expect_err("a profile cycle must be refused");
    let text = error.to_string();
    assert!(
        text.contains("nesting depth") || text.contains("depth"),
        "the error must name the depth budget, got: {text}"
    );
}
