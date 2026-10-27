//! The seed a batch derives its fresh `GlobalId`s from: a checked authoring
//! batch, or a property edit (and the selector and sheet writes built on it).
//!
//! A `GlobalId` must be unique across files, not only within one, so the
//! seed must differ between batches and processes. It comes from the same
//! random source as [`new_guid`](crate::new_guid): the operating system's
//! generator natively, and on `wasm32-unknown-unknown` the host's
//! `crypto.getRandomValues` with the `js-random` feature. Without a source
//! the call refuses rather than return a seed that repeats across sessions.
//!
//! A seed names one batch: the `GlobalId`s a batch derives are a function of
//! the seed, so reusing it on a model that holds them is refused.

use crate::name_guid::{random_bytes, GuidError};

/// A fresh seed for one batch: of `apply_authoring` or `apply_append`, or of a
/// property edit (`apply_property_edits` and the selector and sheet writes
/// built on it).
///
/// # Errors
///
/// [`GuidError::NoRandomSource`] on `wasm32-unknown-unknown` without the
/// `js-random` feature; [`GuidError::RandomSourceFailed`] when the
/// operating system (or the JavaScript host, for example Node 18) refuses.
pub fn fresh_seed() -> Result<u128, GuidError> {
    random_bytes().map(u128::from_le_bytes)
}

#[cfg(test)]
mod tests {
    use super::fresh_seed;

    #[test]
    fn seeds_differ() {
        let seeds: std::collections::HashSet<u128> = (0..64)
            .map(|_| fresh_seed().expect("a random source"))
            .collect();
        assert_eq!(seeds.len(), 64);
    }
}
