//! The seed a batch derives its fresh `GlobalId`s from.
//!
//! A `GlobalId` must be unique across files, not only within one, so the
//! seed must differ between batches and processes. Native targets draw it
//! from the standard library's per-process random hash keys (seeded from
//! the operating system), the clock and a counter. `wasm32-unknown-unknown`
//! has none of those without a global build flag, so there the seed is only
//! a counter, and a browser host passes entropy of its own instead (the
//! JavaScript binding draws it from `Math.random`).

use std::sync::atomic::{AtomicU64, Ordering};

use crate::name_guid::avalanche;

/// A fresh seed for [`apply_authoring`](super::apply_authoring).
///
/// On `wasm32-unknown-unknown` it carries no entropy (see the module
/// documentation); pass a seed drawn by the host there.
pub fn fresh_seed() -> u128 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    avalanche(entropy(count) ^ u128::from(count))
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
fn entropy(count: u64) -> u128 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let draw = |salt: u64| {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(salt);
        hasher.write_u64(count);
        hasher.write_u128(nanos);
        hasher.finish()
    };
    (u128::from(draw(1)) << 64) | u128::from(draw(2))
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn entropy(count: u64) -> u128 {
    u128::from(count) << 64
}

#[cfg(test)]
mod tests {
    use super::fresh_seed;

    #[test]
    fn seeds_differ() {
        let seeds: std::collections::HashSet<u128> = (0..64).map(|_| fresh_seed()).collect();
        assert_eq!(seeds.len(), 64);
    }
}
