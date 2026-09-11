//! The index must be dramatically cheaper than a full parse, or it has no
//! reason to exist. This pins that claim.

use ifc_model::Codec;

mod common;
use common::synthesize;

fn rss_kb() -> u64 {
    let s = std::fs::read_to_string("/proc/self/status").unwrap();
    for l in s.lines() {
        if l.starts_with("VmRSS:") {
            return l.split_whitespace().nth(1).unwrap().parse().unwrap();
        }
    }
    0
}

/// Scanning must cost a fraction of parsing, in both time and memory.
///
/// The budgets are deliberately loose: this asserts an order-of-magnitude
/// property, not a machine-specific number. A CI runner is not a controlled
/// benchmark environment, so tight wall-clock thresholds would flake.
#[test]
fn scanning_is_far_cheaper_than_parsing() {
    let text = synthesize(40_000);
    let bytes = text.as_bytes();

    let base = rss_kb();
    let index = ifc_step::Index::scan(bytes);
    let scan_rss = rss_kb().saturating_sub(base);
    let n = index.len();
    drop(index);

    let base = rss_kb();
    let model = ifc_step::StepCodec.read_bytes(bytes).unwrap();
    let parse_rss = rss_kb().saturating_sub(base);
    assert_eq!(model.ids().count(), n, "index and parser disagree on count");

    assert!(
        scan_rss * 4 < parse_rss,
        "scanning kept {scan_rss} KB against the parsers {parse_rss} KB; the index
         is supposed to be an order of magnitude smaller, so either it started
         retaining decoded data or the model got much cheaper"
    );
    eprintln!("index: {n} entities, scan {scan_rss} KB vs parse {parse_rss} KB");
}
