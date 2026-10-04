//! The generated property workload (#352): walls with occurrence property
//! sets, quantity sets and type-inherited property sets, IFC4.
//!
//! Deterministic: the same object count always gives the same bytes, whose
//! FNV-1a is printed with every run so two runs that measured different
//! files cannot be mistaken for one another. Nothing generated is
//! committed; the bench writes the file to `--data-dir`.
//!
//! Per wall `i` (1-based):
//!
//! - an `IfcWall`;
//! - an own `Pset_WallCommon` with `LoadBearing`, and on every tenth wall
//!   also `IsExternal = .T.`, overriding the type's value;
//! - an `IfcRelDefinesByProperties` relating the wall to it;
//! - on every second wall, a `Qto_WallBaseQuantities` with one
//!   `IfcQuantityLength` and its own `IfcRelDefinesByProperties`.
//!
//! Every block of [`BLOCK`] walls shares one `IfcWallType` whose
//! `HasPropertySets` holds a `Pset_WallCommon` with `FireRating` and
//! `IsExternal = .F.`, bound to the block by one `IfcRelDefinesByType`.
//! Every wall therefore inherits `FireRating`, and nine in ten inherit
//! `IsExternal`. About 5.7 entities per wall.

use std::fmt::Write;

/// Walls per wall type.
pub(crate) const BLOCK: usize = 100;

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
        name: "props-smoke",
        walls: 200,
    },
    Scale {
        name: "props-1k",
        walls: 1_000,
    },
    Scale {
        name: "props-10k",
        walls: 10_000,
    },
    Scale {
        name: "props-100k",
        walls: 100_000,
    },
];

/// The scale called `name`.
pub(crate) fn scale(name: &str) -> Option<Scale> {
    SCALES.iter().copied().find(|scale| scale.name == name)
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

/// The STEP text of `scale`.
pub(crate) fn generate(scale: Scale) -> Vec<u8> {
    let mut out = String::with_capacity(scale.walls * 600);
    out.push_str(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION(('ViewDefinition [ReferenceView]'),'2;1');\n\
         FILE_NAME('properties.ifc','2026-10-04T00:00:00',(''),(''),'openbimrs/ifc #352','',''); \n\
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
    for block in 0..scale.walls.div_ceil(BLOCK) {
        let fire = id();
        let external = id();
        let set = id();
        let wall_type = id();
        let _ = writeln!(
            out,
            "#{fire}=IFCPROPERTYSINGLEVALUE('FireRating',$,IFCLABEL('REI{}'),$);",
            60 + 30 * (block % 3)
        );
        let _ = writeln!(
            out,
            "#{external}=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.F.),$);"
        );
        line(
            &mut out,
            set,
            "IFCPROPERTYSET",
            &format!(",$,'Pset_WallCommon',$,(#{fire},#{external})"),
        );
        line(
            &mut out,
            wall_type,
            "IFCWALLTYPE",
            &format!(",$,'Type {block}',$,$,(#{set}),$,$,$,.STANDARD."),
        );
        let first = block * BLOCK + 1;
        let last = ((block + 1) * BLOCK).min(scale.walls);
        let mut walls = Vec::with_capacity(BLOCK);
        for wall in first..=last {
            let wall_id = id();
            walls.push(wall_id);
            line(
                &mut out,
                wall_id,
                "IFCWALL",
                &format!(",$,'Wall {wall}',$,$,$,$,$,.STANDARD."),
            );
            let bearing = id();
            let _ = writeln!(
                out,
                "#{bearing}=IFCPROPERTYSINGLEVALUE('LoadBearing',$,IFCBOOLEAN(.{}.),$);",
                if wall % 3 == 0 { 'F' } else { 'T' }
            );
            let mut members = format!("#{bearing}");
            if wall % 10 == 0 {
                let external = id();
                let _ = writeln!(
                    out,
                    "#{external}=IFCPROPERTYSINGLEVALUE('IsExternal',$,IFCBOOLEAN(.T.),$);"
                );
                let _ = write!(members, ",#{external}");
            }
            let own = id();
            line(
                &mut out,
                own,
                "IFCPROPERTYSET",
                &format!(",$,'Pset_WallCommon',$,({members})"),
            );
            let relation = id();
            line(
                &mut out,
                relation,
                "IFCRELDEFINESBYPROPERTIES",
                &format!(",$,$,$,(#{wall_id}),#{own}"),
            );
            if wall % 2 == 0 {
                let length = id();
                let quantities = id();
                let relation = id();
                let _ = writeln!(
                    out,
                    "#{length}=IFCQUANTITYLENGTH('Length',$,$,{}.{},$);",
                    1 + wall % 9,
                    wall % 100
                );
                line(
                    &mut out,
                    quantities,
                    "IFCELEMENTQUANTITY",
                    &format!(",$,'Qto_WallBaseQuantities',$,$,(#{length})"),
                );
                line(
                    &mut out,
                    relation,
                    "IFCRELDEFINESBYPROPERTIES",
                    &format!(",$,$,$,(#{wall_id}),#{quantities}"),
                );
            }
        }
        let typing = id();
        let related: Vec<String> = walls.iter().map(|wall| format!("#{wall}")).collect();
        line(
            &mut out,
            typing,
            "IFCRELDEFINESBYTYPE",
            &format!(",$,$,$,({}),#{wall_type}", related.join(",")),
        );
    }
    out.push_str("ENDSEC;\nEND-ISO-10303-21;\n");
    out.into_bytes()
}
