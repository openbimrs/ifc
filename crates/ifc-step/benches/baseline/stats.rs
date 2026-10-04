//! Sampling and order statistics.
//!
//! Every sample times one call. Its input is prepared before the clock
//! starts and its output is dropped after it stops, so neither setup nor
//! deallocation is measured. Warm-up samples run the same code and are
//! discarded. Both input and output pass through [`black_box`], and the
//! caller's `check` asserts every output, so the work cannot be optimised
//! away or silently change.

use std::hint::black_box;
use std::time::Instant;

/// How many samples to take.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Plan {
    pub(crate) warmup: usize,
    pub(crate) samples: usize,
}

/// Times `routine` over `plan.samples` fresh inputs from `setup`, after
/// `plan.warmup` discarded runs. Returns milliseconds per sample.
pub(crate) fn time<I, O>(
    plan: Plan,
    mut setup: impl FnMut() -> I,
    mut routine: impl FnMut(I) -> O,
    mut check: impl FnMut(&O),
) -> Vec<f64> {
    let mut out = Vec::with_capacity(plan.samples);
    for round in 0..plan.warmup + plan.samples {
        let input = black_box(setup());
        let start = Instant::now();
        let output = black_box(routine(input));
        let elapsed = start.elapsed();
        check(&output);
        drop(output);
        if round >= plan.warmup {
            out.push(elapsed.as_secs_f64() * 1e3);
        }
    }
    out
}

/// Median, quartiles, median absolute deviation and range of a sample.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Summary {
    pub(crate) median: f64,
    pub(crate) p25: f64,
    pub(crate) p75: f64,
    pub(crate) mad: f64,
    pub(crate) min: f64,
    pub(crate) max: f64,
}

/// Linear interpolation between closest ranks, as `bench-parse.mjs` (#303).
fn quantile(sorted: &[f64], q: f64) -> f64 {
    let pos = (sorted.len() - 1) as f64 * q;
    let (lo, hi) = (pos.floor() as usize, pos.ceil() as usize);
    sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
}

impl Summary {
    /// Summarises a non-empty sample.
    pub(crate) fn of(samples: &[f64]) -> Self {
        assert!(!samples.is_empty(), "a summary needs at least one sample");
        let mut sorted = samples.to_vec();
        sorted.sort_by(f64::total_cmp);
        let median = quantile(&sorted, 0.5);
        let mut deviations: Vec<f64> = sorted.iter().map(|x| (x - median).abs()).collect();
        deviations.sort_by(f64::total_cmp);
        Self {
            median,
            p25: quantile(&sorted, 0.25),
            p75: quantile(&sorted, 0.75),
            mad: quantile(&deviations, 0.5),
            min: sorted[0],
            max: sorted[sorted.len() - 1],
        }
    }
}
