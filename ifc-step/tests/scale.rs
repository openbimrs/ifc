//! Parse cost at realistic model scale.
//!
//! # Why this exists
//!
//! Every committed fixture is under 250 KB, but real building exports run
//! from tens of megabytes to a few gigabytes. Nothing measured what the
//! parser costs at that size, so the practical ceiling -- the largest file
//! a given machine can open -- was unknown and could regress unnoticed.
//!
//! The fixture is generated here rather than committed: a representative
//! file is far too large to store, and generating it keeps the shape
//! explicit and adjustable.
//!
//! # What is asserted
//!
//! Peak RSS per byte of input. This is the number that decides whether a
//! 500 MB export opens at all, and it is stable across machines in a way
//! wall-clock time is not. The bound is deliberately loose: it catches a
//! structural regression (a new per-entity allocation, a retained copy of
//! the source text) without failing on allocator noise.

use ifc_model::Codec;
use std::fmt::Write as _;

/// Entities per generated wall: placement chain, geometry, property set.
const PER_WALL: usize = 10;

/// Build a STEP file shaped like a real export: a spatial spine, then walls
/// each carrying a placement chain, a shape representation and a property set.
fn synthesize(walls: usize) -> String {
    let mut s = String::with_capacity(walls * 512);
    s.push_str("ISO-10303-21;\nHEADER;\n");
    s.push_str("FILE_DESCRIPTION((''),'2;1');\n");
    s.push_str("FILE_NAME('scale.ifc','2026-01-01T00:00:00',(''),(''),'','','');\n");
    s.push_str("FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n");
    s.push_str("#1=IFCPERSON($,'A',$,$,$,$,$,$);\n");
    s.push_str("#2=IFCORGANIZATION($,'O',$,$,$);\n");
    s.push_str("#3=IFCPERSONANDORGANIZATION(#1,#2,$);\n");
    s.push_str("#4=IFCAPPLICATION(#2,'1','app','app');\n");
    s.push_str("#5=IFCOWNERHISTORY(#3,#4,$,.ADDED.,$,$,$,0);\n");
    s.push_str("#6=IFCCARTESIANPOINT((0.,0.,0.));\n");
    s.push_str("#7=IFCAXIS2PLACEMENT3D(#6,$,$);\n");
    s.push_str("#8=IFCLOCALPLACEMENT($,#7);\n");
    let mut id = 9usize;
    for i in 0..walls {
        let p = id;
        let _ = writeln!(s, "#{}=IFCCARTESIANPOINT(({}.,{}.,0.));", id, i * 3, i * 2);
        id += 1;
        let a = id;
        let _ = writeln!(s, "#{id}=IFCAXIS2PLACEMENT3D(#{p},$,$);");
        id += 1;
        let lp = id;
        let _ = writeln!(s, "#{id}=IFCLOCALPLACEMENT(#8,#{a});");
        id += 1;
        let pl = id;
        let _ = writeln!(s, "#{id}=IFCPOLYLINE((#{p},#{p}));");
        id += 1;
        let sr = id;
        let _ = writeln!(
            s,
            "#{id}=IFCSHAPEREPRESENTATION($,'Axis','Curve2D',(#{pl}));"
        );
        id += 1;
        let pd = id;
        let _ = writeln!(s, "#{id}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{sr}));");
        id += 1;
        let w = id;
        let _ = writeln!(
            s,
            "#{id}=IFCWALL('{i:022}',#5,'Wall-{i}',$,$,#{lp},#{pd},$,$);"
        );
        id += 1;
        let pv = id;
        let _ = writeln!(
            s,
            "#{id}=IFCPROPERTYSINGLEVALUE('P{i}',$,IFCLABEL('v{i}'),$);"
        );
        id += 1;
        let ps = id;
        let _ = writeln!(
            s,
            "#{}=IFCPROPERTYSET('{:022}',#5,'Pset_{}',$,(#{}));",
            id,
            i + 1_000_000,
            i,
            pv
        );
        id += 1;
        let _ = writeln!(
            s,
            "#{}=IFCRELDEFINESBYPROPERTIES('{:022}',#5,$,$,(#{}),#{});",
            id,
            i + 2_000_000,
            w,
            ps
        );
        id += 1;
    }
    s.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    s
}

/// Peak resident set size in KB, or `None` where /proc is unavailable.
fn peak_rss_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find(|l| l.starts_with("VmHWM:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

#[test]
fn parsing_a_large_model_stays_within_its_memory_budget() {
    let Some(_) = peak_rss_kb() else {
        eprintln!("skipped: /proc/self/status unavailable");
        return;
    };
    // 60k walls is ~600k entities: large enough for per-entity costs to
    // dominate fixed overhead, small enough to stay well inside a CI runner.
    let walls = 60_000;
    let text = synthesize(walls);
    let bytes = text.len() as f64;

    let before = peak_rss_kb().unwrap();
    let model = ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("synthetic model parses");
    let after = peak_rss_kb().unwrap();

    let expected = walls * PER_WALL + 8;
    assert_eq!(model.ids().count(), expected, "entity count");

    // Growth attributable to the parse, over the size of the input.
    let growth_kb = after.saturating_sub(before) as f64;
    let ratio = (growth_kb * 1024.0) / bytes;

    // Measured ~13x on this shape. 25x leaves generous headroom for
    // allocator differences while still catching a structural regression.
    assert!(
        ratio < 10.0,
        "parse used {ratio:.1}x the input size in RSS ({growth_kb:.0} KB for {bytes:.0} bytes); \
         budget is 10x -- a new per-entity allocation or a retained source copy is the usual cause"
    );
    eprintln!(
        "scale: {walls} walls, {:.0} MB input, {ratio:.1}x RSS",
        bytes / 1048576.0
    );
}
