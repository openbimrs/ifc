//! The generated material workload (#609): walls whose material comes from
//! their own relationship, from a long shared `RelatedObjects`, or from
//! their type; IFC4.
//!
//! Deterministic: the same object count always gives the same bytes, whose
//! FNV-1a is printed with every run so two runs that measured different
//! files cannot be mistaken for one another. Nothing generated is
//! committed; the bench writes the file to `--data-dir`.
//!
//! Ten `IfcMaterial`s `M0..M9`, then per wall `i` (0-based):
//!
//! - `i % 4 == 0`: an own `IfcRelAssociatesMaterial` naming `M(i % 10)`, and
//!   when `i % 20 == 0` a second one naming the same material;
//! - `i % 4 == 2`: listed in the shared `IfcRelAssociatesMaterial` of
//!   material `(i / 4) % 10`, one relationship per material for the whole
//!   model, so its `RelatedObjects` grows with the model;
//! - odd `i`: inherits through the block's `IfcWallType`.
//!
//! Every block of [`BLOCK`] walls shares one `IfcWallType` with an
//! `IfcRelAssociatesMaterial` naming `M(block % 10)` and one
//! `IfcRelDefinesByType` listing the block's odd walls.

use std::fmt::Write;

/// Walls per wall type.
pub(crate) const BLOCK: usize = 100;

/// Materials.
const MATERIALS: usize = 10;

/// One generated scale.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Scale {
    pub(crate) name: &'static str,
    pub(crate) walls: usize,
}

/// The scales, smallest first: a smoke run, then 10^3, 10^4 and 10^5
/// objects, so linearity reads off as a factor of ten per step.
pub(crate) const SCALES: &[Scale] = &[
    Scale {
        name: "mat-smoke",
        walls: 200,
    },
    Scale {
        name: "mat-1k",
        walls: 1_000,
    },
    Scale {
        name: "mat-10k",
        walls: 10_000,
    },
    Scale {
        name: "mat-100k",
        walls: 100_000,
    },
];

/// The scale called `name`.
pub(crate) fn scale(name: &str) -> Option<Scale> {
    SCALES.iter().copied().find(|scale| scale.name == name)
}

/// The number of entities [`generate`] writes for `scale`.
pub(crate) fn entities(scale: Scale) -> usize {
    let walls = scale.walls;
    let blocks = walls.div_ceil(BLOCK);
    let odd_blocks = (0..blocks)
        .filter(|block| (block * BLOCK..((block + 1) * BLOCK).min(walls)).any(|i| i % 2 == 1))
        .count();
    let own = (0..walls).filter(|i| i % 4 == 0).count();
    let second = (0..walls).filter(|i| i % 20 == 0).count();
    let shared = (0..MATERIALS)
        .filter(|&m| (0..walls).any(|i| i % 4 == 2 && (i / 4) % MATERIALS == m))
        .count();
    // Materials, wall types, their associations, walls, own and second
    // associations, shared associations, type relations.
    MATERIALS + 2 * blocks + walls + own + second + shared + odd_blocks
}

/// A 22-character `GlobalId` from the IFC base-64 alphabet, unique per id.
fn guid(out: &mut String, id: usize) {
    const DIGITS: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let mut digits = [b'0'; 22];
    let mut rest = id;
    for digit in digits.iter_mut().rev() {
        *digit = DIGITS[rest % 64];
        rest /= 64;
    }
    out.push_str(std::str::from_utf8(&digits).expect("ASCII"));
}

fn list(ids: &[usize]) -> String {
    let refs: Vec<String> = ids.iter().map(|id| format!("#{id}")).collect();
    refs.join(",")
}

/// The STEP text of `scale`.
pub(crate) fn generate(scale: Scale) -> Vec<u8> {
    let mut out = String::with_capacity(scale.walls * 300);
    out.push_str(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [ReferenceView]'),'2;1');\n\
         FILE_NAME('materials.ifc','2026-10-30T00:00:00',(''),(''),'openbimrs/ifc #609','',''); \n\
         FILE_SCHEMA(('IFC4'));\nENDSEC;\nDATA;\n",
    );
    let mut next = 0;
    let mut id = || {
        next += 1;
        next
    };
    let line = |out: &mut String, id: usize, head: &str, tail: &str| {
        let _ = write!(out, "#{id}={head}('");
        guid(out, id);
        let _ = writeln!(out, "'{tail});");
    };
    let materials: Vec<usize> = (0..MATERIALS).map(|_| id()).collect();
    for (m, material) in materials.iter().enumerate() {
        let _ = writeln!(out, "#{material}=IFCMATERIAL('M{m}',$,$);");
    }
    let mut shared: Vec<Vec<usize>> = vec![Vec::new(); MATERIALS];
    for block in 0..scale.walls.div_ceil(BLOCK) {
        let wall_type = id();
        line(
            &mut out,
            wall_type,
            "IFCWALLTYPE",
            &format!(",$,'Type {block}',$,$,$,$,$,$,.STANDARD."),
        );
        let association = id();
        line(
            &mut out,
            association,
            "IFCRELASSOCIATESMATERIAL",
            &format!(",$,$,$,(#{wall_type}),#{}", materials[block % MATERIALS]),
        );
        let first = block * BLOCK;
        let last = ((block + 1) * BLOCK).min(scale.walls);
        let mut inheriting = Vec::with_capacity(BLOCK / 2);
        for i in first..last {
            let wall = id();
            line(
                &mut out,
                wall,
                "IFCWALL",
                &format!(",$,'Wall {i}',$,$,$,$,$,.STANDARD."),
            );
            if i % 2 == 1 {
                inheriting.push(wall);
            }
            if i % 4 == 2 {
                shared[(i / 4) % MATERIALS].push(wall);
            }
            if i % 4 == 0 {
                for _ in 0..1 + usize::from(i % 20 == 0) {
                    let own = id();
                    line(
                        &mut out,
                        own,
                        "IFCRELASSOCIATESMATERIAL",
                        &format!(",$,$,$,(#{wall}),#{}", materials[i % MATERIALS]),
                    );
                }
            }
        }
        if !inheriting.is_empty() {
            let typing = id();
            line(
                &mut out,
                typing,
                "IFCRELDEFINESBYTYPE",
                &format!(",$,$,$,({}),#{wall_type}", list(&inheriting)),
            );
        }
    }
    for (m, walls) in shared.iter().enumerate() {
        if walls.is_empty() {
            continue;
        }
        let association = id();
        line(
            &mut out,
            association,
            "IFCRELASSOCIATESMATERIAL",
            &format!(",$,$,$,({}),#{}", list(walls), materials[m]),
        );
    }
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    out.into_bytes()
}
