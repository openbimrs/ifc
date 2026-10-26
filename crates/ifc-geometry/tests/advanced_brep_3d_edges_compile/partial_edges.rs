//! B-spline and polyline edge curves trimmed at their vertices (#525,
//! axiolid/kernel#333), on the split-rim cylinder's seam.

use super::*;

/// The seam of `split_rim_cylinder` from `A = (1, 0, 0)` up to
/// `T = (1, 0, 2)` as `curve` (an `IFCPOLYLINE` or
/// `IFCBSPLINECURVEWITHKNOTS` record whose `{points}` stands for its point
/// list) through `points`. The points run upward along the curve, or
/// downward with `against`, the edge then authored `SameSense = .F.`; the
/// edge runs from `A` to `T` either way.
fn seam(curve: &str, points: &[[f64; 3]], against: bool) -> String {
    let mut ordered = points.to_vec();
    if against {
        ordered.reverse();
    }
    let mut records = String::new();
    let mut refs = Vec::new();
    for (i, [x, y, z]) in ordered.iter().enumerate() {
        let id = 90 + i;
        records.push_str(&format!("#{id}=IFCCARTESIANPOINT(({x:?},{y:?},{z:?}));\n"));
        refs.push(format!("#{id}"));
    }
    let curve = curve.replace("{points}", &format!("({})", refs.join(",")));
    let sense = if against { ".F." } else { ".T." };
    format!("{records}#43={curve};\n#54=IFCEDGECURVE(#34,#36,#43,{sense});\n")
}

/// Seam curves that cover the seam from `A = (1, 0, 0)` to `T = (1, 0, 2)`
/// and run past both vertices, listed upward (`seam` reverses them):
///
/// - a quadratic B-spline on the line `x = 1, y = 0` from `z = -1` to `3`
///   whose middle control point at `z = 0.2` makes its speed uneven, so its
///   vertices' parameters are not proportional to their heights;
/// - a linear B-spline and a polyline that turn at the vertices, their
///   tails leaving the cylinder: `(2, 0, -1)`, `A`, `T`, `(1, 1, 3)`;
/// - a straight polyline past both vertices, and one with a collinear
///   corner between them at `z = 0.5`.
///
/// The last field says whether the kernel's bound meets the 1 mm request.
/// A seam trimmed to one straight span does. The quadratic B-spline and the
/// polyline with a corner between the vertices are bounded piecewise: their
/// wall is chorded to just under the request and the trim terms of the
/// seam's pieces are added to it, so the certified bound, `1.59e-3` m,
/// exceeds the request while the wall measured lies within it. That is the
/// kernel's answer, as for the pillow, and is pinned as such.
const PARTIAL_SEAMS: [(&str, &str, &[[f64; 3]], bool); 5] = [
    (
        "quadratic B-spline",
        "IFCBSPLINECURVEWITHKNOTS(2,{points},.UNSPECIFIED.,.F.,.F.,(3,3),(0.,1.),.UNSPECIFIED.)",
        &[[1.0, 0.0, -1.0], [1.0, 0.0, 0.2], [1.0, 0.0, 3.0]],
        false,
    ),
    (
        "linear B-spline turning at the vertices",
        "IFCBSPLINECURVEWITHKNOTS(1,{points},.POLYLINE_FORM.,.F.,.F.,(2,1,1,2),(0.,1.,2.,3.),.UNSPECIFIED.)",
        &[
            [2.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 2.0],
            [1.0, 1.0, 3.0],
        ],
        true,
    ),
    (
        "polyline turning at the vertices",
        "IFCPOLYLINE({points})",
        &[
            [2.0, 0.0, -1.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 2.0],
            [1.0, 1.0, 3.0],
        ],
        true,
    ),
    (
        "straight polyline",
        "IFCPOLYLINE({points})",
        &[[1.0, 0.0, -1.0], [1.0, 0.0, 3.0]],
        true,
    ),
    (
        "polyline with a corner between the vertices",
        "IFCPOLYLINE({points})",
        &[[1.0, 0.0, -1.0], [1.0, 0.0, 0.5], [1.0, 0.0, 3.0]],
        false,
    ),
];

/// The largest distance inside the cylinder of radius `r` about the Z axis
/// of the wall triangles' points on a barycentric grid: a measured lower
/// bound of the wall's deviation, independent of the kernel's report.
fn sampled_wall_deviation(mesh: &TriMesh, r: f64) -> f64 {
    let radius = |q: axiolid_core::Vec3| (q.x * q.x + q.y * q.y).sqrt();
    let mut deviation: f64 = 0.0;
    for t in mesh.indices.as_chunks::<3>().0 {
        let p = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
        let on_wall = p.iter().all(|&q| (radius(q) - r).abs() <= 1e-9)
            && (p[1] - p[0]).cross(p[2] - p[0]).normalize().z.abs() < 0.5;
        if !on_wall {
            continue;
        }
        for i in 0..=20_u32 {
            for j in 0..=(20 - i) {
                let (a, b) = (f64::from(i) / 20.0, f64::from(j) / 20.0);
                let q = p[0] * (1.0 - a - b) + p[1] * a + p[2] * b;
                deviation = deviation.max(r - radius(q));
            }
        }
    }
    deviation
}

/// A B-spline or polyline seam covering part of its curve, authored along
/// and against it, is trimmed at its vertices (axiolid/kernel#333; #525):
/// the split-rim cylinder closes at `pi r^2 h`, within the cylinder's
/// extent (the tails past the vertices are not meshed). Its report is
/// certified, with no unbounded contribution; the wall measured lies within
/// the bound and the request; the bound meets the request where
/// `PARTIAL_SEAMS` says so and stays within 1.6 times it otherwise.
#[test]
fn a_bspline_or_polyline_edge_covering_part_of_its_curve_compiles() {
    let (r, h) = (1.0_f64, 2.0_f64);
    let exact = PI * r * r * h;
    let area = 2.0 * PI * r * h + 2.0 * PI * r * r;
    for schema in ["IFC4", "IFC4X3"] {
        for (what, curve, points, meets) in PARTIAL_SEAMS {
            for against in [false, true] {
                let what = format!("{schema}, {what}, against {against}");
                let model = split_rim_cylinder(schema, [false; 4], &seam(curve, points, against));
                let mesh = solid(&model, "SPLIT");
                let volume = signed_volume(&mesh);
                assert!(
                    volume <= exact && exact - volume <= area * 1e-3,
                    "{what}: volume {volume}, exact {exact}"
                );
                assert_bounds(&what, &mesh, [-r, -r, 0.0], [r, r, h], 1e-3);
                let report = report(&model, "SPLIT");
                let bound = report.bound.expect("certified");
                assert_eq!(report.unbounded().count(), 0, "{what}: {report:?}");
                assert_eq!(report.meets_requested(), meets, "{what}: {report:?}");
                assert!(bound <= 1.6 * report.requested, "{what}: {report:?}");
                let measured = sampled_wall_deviation(&mesh, r);
                assert!(
                    measured <= bound && measured <= report.requested,
                    "{what}: measured {measured} against {report:?}"
                );
            }
        }
    }
}

/// A B-spline or polyline seam a centimetre beside its vertices (at
/// `y = 0.01`, ten times the 1 mm tolerance) has no span between them: the
/// kernel refuses it by name as a vertex off its curve rather than mesh a
/// guessed span.
#[test]
fn a_bspline_or_polyline_edge_off_its_vertices_is_refused_by_name() {
    let off: &[[f64; 3]] = &[[1.0, 0.01, -1.0], [1.0, 0.01, 3.0]];
    for curve in [
        "IFCBSPLINECURVEWITHKNOTS(1,{points},.POLYLINE_FORM.,.F.,.F.,(2,2),(0.,1.),.UNSPECIFIED.)",
        "IFCPOLYLINE({points})",
    ] {
        for schema in ["IFC4", "IFC4X3"] {
            for against in [false, true] {
                let what = format!("{schema}, {curve}, against {against}");
                let model = split_rim_cylinder(schema, [false; 4], &seam(curve, off, against));
                let id = product(&model, "SPLIT");
                let error = compile_product_mesh(&model, id, Tolerance::MILLIMETRE)
                    .expect_err("the seam misses its vertices");
                match error {
                    GeometryError::CompilationRefused { entity, reason } => {
                        assert_eq!(entity, id);
                        assert!(
                            reason.contains(
                                "vertex of an edge without a p-curve is not on its curve"
                            ),
                            "{what}: {reason}"
                        );
                    }
                    other => panic!("{what}: expected CompilationRefused, got {other:?}"),
                }
            }
        }
    }
}
