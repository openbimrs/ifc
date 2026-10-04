//! The synthetic workloads: `benchmarks/generate-fixture.awk`, ported.
//!
//! The output is byte-identical to `awk -v n=<walls> -f
//! benchmarks/generate-fixture.awk` for every scale below, and every
//! generation is checked against the committed SHA-256 of that awk output,
//! so the two generators cannot drift apart unnoticed and the bytes measured
//! are the bytes named. Nothing generated is committed.
//!
//! The model is uniform: an owner-history spine, then per wall a placement
//! chain, an axis polyline representation, a wall and a property set bound
//! by an `IfcRelDefinesByProperties` -- ten entities per wall.

use sha2::{Digest, Sha256};
use std::fmt::Write;

/// One synthetic scale.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Scale {
    pub(crate) name: &'static str,
    pub(crate) walls: usize,
    /// SHA-256 of `awk -v n=<walls> -f benchmarks/generate-fixture.awk`.
    pub(crate) sha256: &'static str,
}

/// The scales, smallest first.
///
/// - `tiny` (48 entities): the CI smoke run.
/// - `small` (~1 MB, 18,008 entities): the decoded model (~6 MB) fits the
///   last-level cache of the baseline machine.
/// - `crossover` (~4.4 MB, 80,008 entities): just past the 4 MiB input at
///   which the lazy read starts validating on several threads, and a decoded
///   model (~28 MB) past the 16 MiB last-level cache.
/// - `large` (~105 MB, 1,800,008 entities): far beyond every cache.
pub(crate) const SCALES: &[Scale] = &[
    Scale {
        name: "tiny",
        walls: 4,
        sha256: "4ae4ff726f9f83b6982e5be48a125ea549a345ff57bb4f910f1cca9a6098f206",
    },
    Scale {
        name: "small",
        walls: 1_800,
        sha256: "45057a95785845c418dd6d5fc89f261248977e7c24a2cbcfec1493f0b1dd8f0a",
    },
    Scale {
        name: "crossover",
        walls: 8_000,
        sha256: "0de2d9d8d80768c864b55ac2802026acd349b888d1265e8388cfdf505331fb86",
    },
    Scale {
        name: "large",
        walls: 180_000,
        sha256: "09409504da7bffec10b5ec490c19704459fa7ba8d9832267d6b6aa5c0ff3c9c2",
    },
];

/// The scale called `name`.
pub(crate) fn scale(name: &str) -> Option<Scale> {
    SCALES.iter().copied().find(|scale| scale.name == name)
}

/// Lower-case hex SHA-256 of `bytes`.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Generates the file for `scale` and checks it against the committed hash.
///
/// # Panics
///
/// When the bytes differ from the awk generator's: the measured workload
/// would no longer be the one the baseline names.
pub(crate) fn generate(scale: Scale) -> Vec<u8> {
    let text = render(scale.walls);
    let found = sha256_hex(text.as_bytes());
    assert_eq!(
        found, scale.sha256,
        "synthetic scale {} no longer matches generate-fixture.awk",
        scale.name
    );
    text.into_bytes()
}

/// The awk program's output for `n` walls.
fn render(n: usize) -> String {
    let mut s = String::with_capacity(600 * n + 1024);
    s.push_str(concat!(
        "ISO-10303-21;\n",
        "HEADER;\n",
        "FILE_DESCRIPTION((''),'2;1');\n",
        "FILE_NAME('bench.ifc','2026-01-01T00:00:00',(''),(''),'','','');\n",
        "FILE_SCHEMA(('IFC4'));\n",
        "ENDSEC;\n",
        "DATA;\n",
        "#1=IFCPERSON($,'A',$,$,$,$,$,$);\n",
        "#2=IFCORGANIZATION($,'O',$,$,$);\n",
        "#3=IFCPERSONANDORGANIZATION(#1,#2,$);\n",
        "#4=IFCAPPLICATION(#2,'1','app','app');\n",
        "#5=IFCOWNERHISTORY(#3,#4,$,.ADDED.,$,$,$,0);\n",
        "#6=IFCCARTESIANPOINT((0.,0.,0.));\n",
        "#7=IFCAXIS2PLACEMENT3D(#6,$,$);\n",
        "#8=IFCLOCALPLACEMENT($,#7);\n",
    ));
    let mut id = 9;
    for i in 0..n {
        let (pt, ax, lp, pl, sr, pd, w, pv, ps, rel) = (
            id,
            id + 1,
            id + 2,
            id + 3,
            id + 4,
            id + 5,
            id + 6,
            id + 7,
            id + 8,
            id + 9,
        );
        // Writing to a String cannot fail.
        let _ = write!(
            s,
            "#{pt}=IFCCARTESIANPOINT(({x}.,{y}.,0.));\n\
             #{ax}=IFCAXIS2PLACEMENT3D(#{pt},$,$);\n\
             #{lp}=IFCLOCALPLACEMENT(#8,#{ax});\n\
             #{pl}=IFCPOLYLINE((#{pt},#{pt}));\n\
             #{sr}=IFCSHAPEREPRESENTATION($,'Axis','Curve2D',(#{pl}));\n\
             #{pd}=IFCPRODUCTDEFINITIONSHAPE($,$,(#{sr}));\n\
             #{w}=IFCWALL('{i:022}',#5,'W{i}',$,$,#{lp},#{pd},$,$);\n\
             #{pv}=IFCPROPERTYSINGLEVALUE('P',$,IFCLABEL('v{i}'),$);\n\
             #{ps}=IFCPROPERTYSET('{g1:022}',#5,'PS',$,(#{pv}));\n\
             #{rel}=IFCRELDEFINESBYPROPERTIES('{g2:022}',#5,$,$,(#{w}),#{ps});\n",
            x = i * 3,
            y = i * 2,
            g1 = i + 1_000_000,
            g2 = i + 2_000_000,
        );
        id += 10;
    }
    s.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    s
}
