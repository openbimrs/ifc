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

/// Entities per generated wall: placement chain, geometry, property set.
const PER_WALL: usize = 10;

/// Build a STEP file shaped like a real export: a spatial spine, then walls
/// each carrying a placement chain, a shape representation and a property set.
mod common;
use common::synthesize;

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
