//! Openings are subtracted from their host when the caller asks for net
//! geometry (#44), and gross stays the default.
//!
//! An `IfcRelVoidsElement` says an opening body is to be removed from its
//! host's body. The Body representation of a wall is the GROSS wall; the file
//! never authors the net one. So every net quantity -- wall area through a
//! door, clear widths, window-to-floor ratios, clash tests through an opening
//! -- depends on a subtraction this crate has to request from the kernel.
//!
//! # The geometry, and why each number is what it is
//!
//! The host is a 4 x 0.3 x 3 wall (x in [-2, 2], y in [-0.15, 0.15], z in
//! [0, 3]), gross volume 3.6. Its placement is translated to (10, 5, 0) and
//! rotated 30 degrees about Z. Each opening's placement is RELATIVE to the
//! wall's (`PlacementRelTo`), as exporters write them. So an opening that
//! ignored the host chain would sit near the world origin, miss the wall, and
//! leave the gross volume -- a failure the analytic check sees.
//!
//! The flush door is the hard, realistic case: it is exactly as thick as the
//! wall and starts at the wall's base, so three of its faces are coplanar with
//! wall faces. Revit exports doors like this.
#![cfg(feature = "compile-reference-backend")]

use axiolid_core::Tolerance;
use axiolid_mesh::TriMesh;
use ifc_geometry::compile::{compile_product_mesh, compile_product_mesh_net};
use ifc_geometry::error::GeometryError;
use ifc_geometry::openings_of;
use ifc_model::{Codec, EntityId, Model};
use ifc_step::StepCodec;

/// The gross wall: 4 x 0.3 x 3.
const GROSS: f64 = 3.6;

/// One opening body: an axis-aligned box in the WALL's local frame.
#[derive(Clone, Copy)]
struct Opening {
    /// Centre of the opening footprint along the wall (x) and across it (y).
    centre: [f64; 2],
    /// Footprint extent along x and across y.
    size: [f64; 2],
    /// Base height and extrusion height.
    z: [f64; 2],
}

/// A door exactly as thick as the wall, from the floor: 3 coplanar faces.
const FLUSH_DOOR: Opening = Opening {
    centre: [0.5, 0.0],
    size: [1.0, 0.3],
    z: [0.0, 2.1],
};

/// The same door, overhanging the wall on every side it touches.
const OVERHANGING_DOOR: Opening = Opening {
    centre: [0.5, 0.0],
    size: [1.0, 0.5],
    z: [-0.1, 2.2],
};

/// How much of the wall the overhanging door removes: it is clipped at the
/// floor and at the faces, so only 1 x 0.3 x 2.1 of it is inside.
const DOOR_INSIDE: f64 = 1.0 * 0.3 * 2.1;

/// Build a file with the wall `#10` and each opening voiding it.
///
/// `unit` is the length factor written into the file (1 for metres, 1000 for
/// millimetres), so every variant describes the same solids.
fn wall_with(openings: &[Opening], unit: f64) -> String {
    let prefix = if unit == 1.0 { "$" } else { ".MILLI." };
    let l = |v: f64| format!("{:?}", v * unit);
    let (sin, cos) = 30f64.to_radians().sin_cos();
    let mut data = format!(
        "#1=IFCPROJECT('0YvctVUKr0kugbFTf53O9L',$,'P',$,$,$,$,(#2),#3);
#2=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#4,$);
#3=IFCUNITASSIGNMENT((#5));
#4=IFCAXIS2PLACEMENT3D(#6,$,$);
#5=IFCSIUNIT(*,.LENGTHUNIT.,{prefix},.METRE.);
#6=IFCCARTESIANPOINT((0.,0.,0.));
#7=IFCDIRECTION((0.,0.,1.));
#10=IFCWALL('1YvctVUKr0kugbFTf53O9L',$,'Wall',$,$,#11,#12,$,$);
#11=IFCLOCALPLACEMENT($,#13);
#12=IFCPRODUCTDEFINITIONSHAPE($,$,(#16));
#13=IFCAXIS2PLACEMENT3D(#14,#7,#15);
#14=IFCCARTESIANPOINT(({x},{y},0.));
#15=IFCDIRECTION(({cos:?},{sin:?},0.));
#16=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#17));
#17=IFCEXTRUDEDAREASOLID(#18,#4,#7,{height});
#18=IFCRECTANGLEPROFILEDEF(.AREA.,$,#19,{length},{thickness});
#19=IFCAXIS2PLACEMENT2D(#20,$);
#20=IFCCARTESIANPOINT((0.,0.));
",
        x = l(10.0),
        y = l(5.0),
        height = l(3.0),
        length = l(4.0),
        thickness = l(0.3),
    );
    for (index, opening) in openings.iter().enumerate() {
        let base = 100 + 20 * index;
        let [cx, cy] = opening.centre;
        let [sx, sy] = opening.size;
        let [z0, h] = opening.z;
        data.push_str(&format!(
            "#{o}=IFCOPENINGELEMENT('2YvctVUKr0kugbFTf53O{index:02}',$,'Opening',$,$,#{pl},#{sh},$,.OPENING.);
#{pl}=IFCLOCALPLACEMENT(#11,#{ax});
#{ax}=IFCAXIS2PLACEMENT3D(#{pt},$,$);
#{pt}=IFCCARTESIANPOINT(({cx},{cy},{z0}));
#{sh}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}));
#{rep}=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#{solid}));
#{solid}=IFCEXTRUDEDAREASOLID(#{profile},#4,#7,{h});
#{profile}=IFCRECTANGLEPROFILEDEF(.AREA.,$,#19,{sx},{sy});
#{rel}=IFCRELVOIDSELEMENT('3YvctVUKr0kugbFTf53O{index:02}',$,$,$,#10,#{o});
",
            o = base,
            pl = base + 1,
            ax = base + 2,
            pt = base + 3,
            sh = base + 4,
            rep = base + 5,
            solid = base + 6,
            profile = base + 7,
            rel = base + 8,
            cx = l(cx),
            cy = l(cy),
            z0 = l(z0),
            h = l(h),
            sx = l(sx),
            sy = l(sy),
        ));
    }
    file(&data)
}

fn file(data: &str) -> String {
    format!(
        "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
{data}ENDSEC;
END-ISO-10303-21;
"
    )
}

fn parse(text: &str) -> Model {
    StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses")
}

/// Net volume of the wall, and the openings the compiler reports subtracting.
fn net(text: &str) -> (f64, Vec<EntityId>) {
    let model = parse(text);
    let net = compile_product_mesh_net(&model, EntityId(10), Tolerance::MILLIMETRE)
        .unwrap_or_else(|error| panic!("the net wall must compile, got {error}"))
        .expect("the wall has a body");
    (signed_volume(&net.mesh), net.openings)
}

fn gross(text: &str) -> f64 {
    let model = parse(text);
    let mesh = compile_product_mesh(&model, EntityId(10), Tolerance::MILLIMETRE)
        .unwrap_or_else(|error| panic!("the gross wall must compile, got {error}"))
        .expect("the wall has a body");
    signed_volume(&mesh)
}

/// Divergence-theorem volume, summed about the first vertex for conditioning.
fn signed_volume(mesh: &TriMesh) -> f64 {
    assert!(!mesh.indices.is_empty(), "compiled mesh is empty");
    let base = mesh.positions[0];
    mesh.indices
        .chunks_exact(3)
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize] - base);
            a.dot(b.cross(c)) / 6.0
        })
        .sum()
}

fn assert_volume(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < 1e-6,
        "{what}: volume {actual}, expected {expected}"
    );
}

/// The worked example from the issue: net = gross - (door intersect wall).
#[test]
fn a_flush_door_is_subtracted_from_its_wall() {
    let (volume, openings) = net(&wall_with(&[FLUSH_DOOR], 1.0));
    assert_volume(volume, GROSS - DOOR_INSIDE, "flush door");
    assert_eq!(openings, vec![EntityId(100)]);
}

/// A window body extruded DOWNWARD from its lintel is subtracted too.
///
/// Solibri and Revit author opening bodies as `ExtrudedDirection = (0,0,-1)`
/// hung from the top of the opening. That is the same box as an upward
/// extrusion from the sill, so the net volume must not depend on which way
/// the file extruded it. `axiolid-construct` 0.3.0 winds a downward
/// extrusion inside-out, and the boolean refuses the inverted tool
/// (axiolid/kernel#166), so this needs the fixed kernel.
#[test]
#[ignore = "needs axiolid-construct with axiolid/kernel#166 (downward extrusion inside-out)"]
fn a_downward_extruded_window_is_subtracted() {
    // A 1 x 0.3 x 1 window whose top sits at z = 2.1: the body is placed at
    // the lintel and extruded 1 m down along -Z.
    let text = replace_line(
        &wall_with(
            &[Opening {
                centre: [0.5, 0.0],
                size: [1.0, 0.3],
                z: [2.1, 1.0],
            }],
            1.0,
        ),
        "#106=",
        "#106=IFCEXTRUDEDAREASOLID(#107,#4,#8,1.);\n#8=IFCDIRECTION((0.,0.,-1.));",
    );
    let (volume, openings) = net(&text);
    assert_volume(volume, GROSS - 1.0 * 0.3 * 1.0, "downward window");
    assert_eq!(openings, vec![EntityId(100)]);
}

/// Only the part of an opening inside the wall is removed.
#[test]
fn an_overhanging_door_removes_only_its_intersection_with_the_wall() {
    let (volume, _) = net(&wall_with(&[OVERHANGING_DOOR], 1.0));
    assert_volume(volume, GROSS - DOOR_INSIDE, "overhanging door");
}

/// Units are converted once: a millimetre file is the same solid.
#[test]
fn a_millimetre_file_nets_to_the_same_volume() {
    let (volume, _) = net(&wall_with(&[FLUSH_DOOR], 1000.0));
    assert_volume(volume, GROSS - DOOR_INSIDE, "millimetre flush door");
}

/// A recess: an opening through part of the thickness only.
///
/// It spans y in [0.05, 0.25], so it cuts 0.1 of the 0.3 wall, over a
/// 1 x 1 footprint: 0.1 removed.
#[test]
fn a_recess_removes_only_the_depth_it_reaches() {
    let recess = Opening {
        centre: [0.0, 0.15],
        size: [1.0, 0.2],
        z: [1.0, 1.0],
    };
    let (volume, _) = net(&wall_with(&[recess], 1.0));
    assert_volume(volume, GROSS - 0.1, "recess");
}

/// Overlapping openings remove their UNION, so the overlap counts once.
///
/// A is x in [-1.5, -0.5], z in [0.5, 1.5]; B is x in [-1, 0], z in [1, 2].
/// Each is 1 x 1 in elevation and they share 0.5 x 0.5, so the removed
/// elevation area is 1.75, through the 0.3 wall.
#[test]
fn overlapping_openings_remove_their_union_once() {
    let a = Opening {
        centre: [-1.0, 0.0],
        size: [1.0, 0.5],
        z: [0.5, 1.0],
    };
    let b = Opening {
        centre: [-0.5, 0.0],
        size: [1.0, 0.5],
        z: [1.0, 1.0],
    };
    let (volume, openings) = net(&wall_with(&[a, b], 1.0));
    assert_volume(volume, GROSS - 1.75 * 0.3, "overlapping openings");
    assert_eq!(openings, vec![EntityId(100), EntityId(120)]);
}

/// Gross stays the default: the existing entry point ignores openings.
#[test]
fn the_gross_entry_point_is_unchanged() {
    assert_volume(gross(&wall_with(&[FLUSH_DOOR], 1.0)), GROSS, "gross wall");
}

/// Relations authored out of id order, and one opening related twice, still
/// report each opening once in ascending id and remove it once.
#[test]
fn duplicate_and_unordered_relations_report_each_opening_once() {
    let text = wall_with(&[FLUSH_DOOR, OVERHANGING_DOOR], 1.0).replacen(
        "DATA;\n",
        "DATA;\n#98=IFCRELVOIDSELEMENT('4YvctVUKr0kugbFTf53O98',$,$,$,#10,#120);\n\
         #99=IFCRELVOIDSELEMENT('4YvctVUKr0kugbFTf53O99',$,$,$,#10,#100);\n",
        1,
    );
    assert!(text.contains("#98=IFCRELVOIDSELEMENT"), "prefix must apply");
    let model = parse(&text);
    assert_eq!(
        openings_of(&model, EntityId(10)),
        vec![EntityId(100), EntityId(120)]
    );
    // Both doors cover the same x range; the overhanging one only reaches
    // further outside the wall, so the pair removes one door's volume.
    let (volume, openings) = net(&text);
    assert_volume(volume, GROSS - DOOR_INSIDE, "duplicated relations");
    assert_eq!(openings, vec![EntityId(100), EntityId(120)]);
}

/// A relation naming the host as its own opening is refused, not netted to
/// an empty solid.
#[test]
fn a_host_voiding_itself_is_refused() {
    let text = wall_with(&[], 1.0).replacen(
        "DATA;\n",
        "DATA;\n#97=IFCRELVOIDSELEMENT('4YvctVUKr0kugbFTf53O97',$,$,$,#10,#10);\n",
        1,
    );
    let (host, opening, cause) = net_refusal(&text);
    assert_eq!((host, opening), (EntityId(10), EntityId(10)));
    assert!(
        cause.to_string().contains("#97"),
        "must blame the relation: {cause}"
    );
}

/// A host without openings nets to its gross body and reports none.
#[test]
fn a_host_without_openings_nets_to_its_gross_body() {
    let (volume, openings) = net(&wall_with(&[], 1.0));
    assert_volume(volume, GROSS, "no openings");
    assert!(openings.is_empty());
}

/// The relation is readable without the kernel, in id order.
#[test]
fn openings_are_found_through_the_voids_relation() {
    let model = parse(&wall_with(&[FLUSH_DOOR, OVERHANGING_DOOR], 1.0));
    assert_eq!(
        openings_of(&model, EntityId(10)),
        vec![EntityId(100), EntityId(120)]
    );
    assert!(openings_of(&model, EntityId(100)).is_empty());
}

/// The net refusal and the opening it names.
fn net_refusal(text: &str) -> (EntityId, EntityId, GeometryError) {
    let model = parse(text);
    match compile_product_mesh_net(&model, EntityId(10), Tolerance::MILLIMETRE) {
        Err(GeometryError::OpeningNotSubtracted {
            host,
            opening,
            cause,
        }) => (host, opening, *cause),
        Err(other) => panic!("expected OpeningNotSubtracted, got {other:?}"),
        Ok(mesh) => panic!(
            "an opening that cannot be subtracted must refuse, not return {:?}",
            mesh.map(|net| signed_volume(&net.mesh))
        ),
    }
}

/// Replace one line of a fixture, failing loudly if the target is absent.
fn replace_line(text: &str, prefix: &str, replacement: &str) -> String {
    let line = text
        .lines()
        .find(|line| line.starts_with(prefix))
        .unwrap_or_else(|| panic!("fixture has no line starting {prefix}"));
    text.replace(line, replacement)
}

/// An opening whose body cannot be lowered refuses the net wall by name.
///
/// Returning the gross wall here would be the silent failure the issue is
/// about: every net quantity downstream would be wrong with no signal.
#[test]
fn an_unlowerable_opening_body_refuses_naming_the_opening() {
    let text = replace_line(
        &wall_with(&[FLUSH_DOOR, OVERHANGING_DOOR], 1.0),
        "#127=",
        "#127=IFCPROFILEDEF(.AREA.,$);",
    );
    let (host, opening, cause) = net_refusal(&text);
    assert_eq!((host, opening), (EntityId(10), EntityId(120)));
    assert!(cause.is_unsupported(), "cause: {cause:?}");
    // The gross wall is unaffected by a broken opening.
    assert_volume(gross(&text), GROSS, "gross beside a broken opening");
}

/// An opening with no body at all has nothing to subtract: refused.
#[test]
fn an_opening_without_a_body_refuses_naming_the_opening() {
    let text = replace_line(
        &wall_with(&[FLUSH_DOOR], 1.0),
        "#100=",
        "#100=IFCOPENINGELEMENT('2YvctVUKr0kugbFTf53O00',$,'Opening',$,$,#101,$,$,.OPENING.);",
    );
    let (host, opening, _) = net_refusal(&text);
    assert_eq!((host, opening), (EntityId(10), EntityId(100)));
}

/// An opening body that is not a solid is refused by name.
///
/// An open shell bounds no volume, so the reference kernel refuses it as a
/// boolean tool ("brep has no solid", axiolid/kernel#161). What matters here
/// is that the OPENING is blamed and the wall is not returned uncut.
#[test]
fn a_non_solid_opening_body_is_refused_by_name() {
    let shell = "#106=IFCSHELLBASEDSURFACEMODEL((#140));
#140=IFCOPENSHELL((#141));
#141=IFCFACE((#142));
#142=IFCFACEOUTERBOUND(#143,.T.);
#143=IFCPOLYLOOP((#144,#145,#146));
#144=IFCCARTESIANPOINT((0.,-1.,0.));
#145=IFCCARTESIANPOINT((1.,-1.,0.));
#146=IFCCARTESIANPOINT((0.,1.,2.));";
    let text = replace_line(&wall_with(&[FLUSH_DOOR], 1.0), "#106=", shell);
    let (host, opening, cause) = net_refusal(&text);
    assert_eq!((host, opening), (EntityId(10), EntityId(100)));
    assert!(
        matches!(cause, GeometryError::CompilationRefused { entity, .. } if entity == EntityId(100)),
        "cause: {cause:?}"
    );
}

/// Replace the opening's single extrusion by `items`, a comma-separated list
/// of representation items, appending `extra` entity lines.
fn opening_items(text: &str, items: &str, extra: &str) -> String {
    let with_items = replace_line(
        text,
        "#105=",
        &format!("#105=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',({items}));\n{extra}"),
    );
    assert_ne!(with_items, text, "the opening item substitution must apply");
    with_items
}

/// An opening authored as TWO items subtracts both.
///
/// A multi-item Body lowers to a `Collection`, which the neutral graph does
/// not accept as a boolean operand. Real exporters write openings like this
/// (a door leaf plus a frame recess), and before the split every one of them
/// was refused.
#[test]
fn a_two_item_opening_subtracts_both_items() {
    let text = opening_items(
        &wall_with(&[FLUSH_DOOR], 1.0),
        "#106,#150",
        "#150=IFCEXTRUDEDAREASOLID(#151,#152,#7,1.);
#151=IFCRECTANGLEPROFILEDEF(.AREA.,$,#19,0.5,0.3);
#152=IFCAXIS2PLACEMENT3D(#153,$,$);
#153=IFCCARTESIANPOINT((-1.5,0.,0.));",
    );
    // The door (1 x 0.3 x 2.1) plus a 0.5 x 0.3 x 1 block well clear of it.
    let (volume, openings) = net(&text);
    assert_volume(
        volume,
        GROSS - DOOR_INSIDE - 0.5 * 0.3 * 1.0,
        "two-item opening",
    );
    assert_eq!(openings, vec![EntityId(100)]);
}

/// An opening whose Body is a curve bounds no volume and is refused by name.
///
/// Caught while splitting the body, before any boolean is built, so the cause
/// says what is wrong with the geometry instead of a graph type mismatch.
#[test]
fn an_opening_body_that_bounds_no_volume_is_refused() {
    let text = replace_line(
        &wall_with(&[FLUSH_DOOR], 1.0),
        "#106=",
        "#106=IFCPOLYLINE((#170,#171));
#170=IFCCARTESIANPOINT((0.,0.,0.));
#171=IFCCARTESIANPOINT((1.,0.,0.));",
    );
    let (host, opening, cause) = net_refusal(&text);
    assert_eq!((host, opening), (EntityId(10), EntityId(100)));
    assert!(
        cause.to_string().contains("bounds no volume"),
        "cause: {cause}"
    );
}

/// A MAPPED opening whose map holds two solids subtracts both, placed.
///
/// A mapped item lowers to an `Instance` of the map's `Collection`, which the
/// graph rejects as an operand. The split pushes the instance down onto each
/// part. The mapping target moves the whole map 1.5 m along the wall, so an
/// implementation that dropped the instance transform would cut the wrong
/// place: the block would then sit fully inside the wall and remove more.
#[test]
fn a_mapped_multi_solid_opening_is_split_and_placed() {
    let text = opening_items(
        &wall_with(&[FLUSH_DOOR], 1.0),
        "#160",
        "#160=IFCMAPPEDITEM(#161,#162);
#161=IFCREPRESENTATIONMAP(#4,#163);
#162=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#164,$,$);
#163=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#106,#150));
#164=IFCCARTESIANPOINT((-1.5,0.,0.));
#150=IFCEXTRUDEDAREASOLID(#151,#152,#7,1.);
#151=IFCRECTANGLEPROFILEDEF(.AREA.,$,#19,0.5,0.3);
#152=IFCAXIS2PLACEMENT3D(#153,$,$);
#153=IFCCARTESIANPOINT((-1.,0.,0.));",
    );
    // In wall x, after the opening placement (+0.5) and the mapping target
    // (-1.5): the door spans [-1.5, -0.5], fully inside; the block spans
    // [-2.25, -1.75], of which only [-2, -1.75] (0.25 long) is inside the
    // 4 m wall. Dropping the target would put both fully inside instead.
    let (volume, openings) = net(&text);
    assert_volume(
        volume,
        GROSS - DOOR_INSIDE - 0.25 * 0.3 * 1.0,
        "mapped two-solid opening",
    );
    assert_eq!(openings, vec![EntityId(100)]);
}

/// Nested mapped items compose their transforms in the right order.
///
/// The outer map rotates its contents +90 degrees about Z. The inner map,
/// inside it, translates two 0.2 x 0.2 x 1 blocks 1 m along ITS y.
///
/// Correct order (outer after inner): the rotation turns that y move into
/// -1 m along the wall's x, so both blocks land inside the wall at wall
/// x = -0.5, clear of the door, and each removes 0.04 m3.
///
/// Swapped order: the 1 m move is applied last, in WORLD space. The wall is
/// rotated 30 degrees, so that puts the blocks 0.87 m across the wall --
/// outside its 0.3 m thickness -- and they remove nothing.
#[test]
fn nested_mapped_transforms_compose_outer_after_inner() {
    let text = opening_items(
        &wall_with(&[FLUSH_DOOR], 1.0),
        "#106,#180",
        "#180=IFCMAPPEDITEM(#181,#182);
#181=IFCREPRESENTATIONMAP(#4,#183);
#182=IFCCARTESIANTRANSFORMATIONOPERATOR3D(#184,#185,#6,$,#7);
#183=IFCSHAPEREPRESENTATION(#2,'Body','MappedRepresentation',(#190));
#184=IFCDIRECTION((0.,1.,0.));
#185=IFCDIRECTION((-1.,0.,0.));
#190=IFCMAPPEDITEM(#191,#192);
#191=IFCREPRESENTATIONMAP(#4,#193);
#192=IFCCARTESIANTRANSFORMATIONOPERATOR3D($,$,#194,$,$);
#193=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#150,#155));
#194=IFCCARTESIANPOINT((0.,1.,0.));
#150=IFCEXTRUDEDAREASOLID(#151,#4,#7,1.);
#151=IFCRECTANGLEPROFILEDEF(.AREA.,$,#19,0.2,0.2);
#155=IFCEXTRUDEDAREASOLID(#151,#156,#7,1.);
#156=IFCAXIS2PLACEMENT3D(#157,$,$);
#157=IFCCARTESIANPOINT((0.,0.,1.5));",
    );
    let (volume, openings) = net(&text);
    assert_volume(
        volume,
        GROSS - DOOR_INSIDE - 2.0 * 0.2 * 0.2 * 1.0,
        "nested mapped opening",
    );
    assert_eq!(openings, vec![EntityId(100)]);
}

/// A host authored as TWO items has the opening removed from each.
///
/// The wall is split into two halves that meet at x = 0, and the door
/// straddles the seam, so each half loses exactly half of it.
/// `(A u B) - O` must equal `(A - O) u (B - O)`.
#[test]
fn a_two_item_host_has_the_opening_removed_from_each_part() {
    let straddling = Opening {
        centre: [0.0, 0.0],
        size: [1.0, 0.3],
        z: [0.0, 2.1],
    };
    let base = wall_with(&[straddling], 1.0);
    // Left half x in [-2, 0] and right half x in [0, 2], same section.
    let text = replace_line(
        &base,
        "#16=",
        "#16=IFCSHAPEREPRESENTATION(#2,'Body','SweptSolid',(#30,#40));
#30=IFCEXTRUDEDAREASOLID(#31,#32,#7,3.);
#31=IFCRECTANGLEPROFILEDEF(.AREA.,$,#19,2.,0.3);
#32=IFCAXIS2PLACEMENT3D(#33,$,$);
#33=IFCCARTESIANPOINT((-1.,0.,0.));
#40=IFCEXTRUDEDAREASOLID(#31,#42,#7,3.);
#42=IFCAXIS2PLACEMENT3D(#43,$,$);
#43=IFCCARTESIANPOINT((1.,0.,0.));",
    );
    assert_volume(gross(&text), GROSS, "two-item gross wall");
    let (volume, openings) = net(&text);
    assert_volume(volume, GROSS - DOOR_INSIDE, "two-item host");
    assert_eq!(openings, vec![EntityId(100)]);
}
