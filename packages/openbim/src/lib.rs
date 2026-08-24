//! `openbim` — a facade over the openBIM standards.
//!
//! Enable only what you need:
//!
//! ```toml
//! openbim = { version = "0.1", features = ["ids"] }
//! ```
//!
//! # Why the standards are separate crates
//!
//! Cargo features are **additive across the whole dependency graph**. If every
//! standard were a feature of one crate, then any dependency anywhere enabling
//! `icdd` would make *everyone* compile an RDF stack — including a consumer
//! that only wanted to read a `.ids` file.
//!
//! Separate packages make that structurally impossible: a crate not named in
//! your feature set is never built. This facade exists so the convenience of
//! one dependency line is still available, and it re-exports only. See
//! `docs/adr/0015`.
//!
//! # Available features
//!
//! | Feature | Crate | Standard |
//! | --- | --- | --- |
//! | `dt` | `openbim-dt` | ISO 23387 data templates |
//! | `ids` | `openbim-ids` | buildingSMART IDS |
//! | `bcf` | `openbim-bcf` | BCF (BIM Collaboration Format) |
//! | `icdd` | `openbim-icdd` | ISO 21597 ICDD |
//! | `idm` | `openbim-idm` | ISO 29481-3 idmXML |
//! | `loin` | `openbim-loin` | ISO 7817-3 / EN 17412-3 LOIN |
//! | `full` | all of the above | |
//!
//! `loin` implies `dt`, because the LOIN schema imports ISO 23387.
//!
//! No feature is on by default: depending on `openbim` must never cost more
//! than the shared vocabulary in [`core`].
//!
//! # Status
//!
//! **Scaffold.** [`core`] carries real types; the standard crates are name
//! reservations with no parsing yet. Nothing here silently pretends to
//! validate a model.

#![forbid(unsafe_code)]

/// Vocabulary shared by every standard: outcomes, element references,
/// version detection. Always available.
pub use openbim_core as core;

#[cfg(feature = "dt")]
pub use openbim_dt as dt;

#[cfg(feature = "ids")]
pub use openbim_ids as ids;

#[cfg(feature = "bcf")]
pub use openbim_bcf as bcf;

#[cfg(feature = "icdd")]
pub use openbim_icdd as icdd;

#[cfg(feature = "idm")]
pub use openbim_idm as idm;

#[cfg(feature = "loin")]
pub use openbim_loin as loin;

#[cfg(test)]
mod tests {
    /// The shared vocabulary is reachable with no features enabled.
    #[test]
    fn core_is_always_available() {
        assert!(crate::core::Outcome::Failed.is_applicable());
        assert!(!crate::core::Outcome::NotApplicable.is_applicable());
    }

    /// `loin` must imply `dt` — the LOIN schema imports ISO 23387, so a build
    /// with LOIN but no data templates would be incoherent.
    #[test]
    #[cfg(feature = "loin")]
    fn loin_implies_dt() {
        assert!(crate::loin::is_known_namespace(crate::loin::NAMESPACE_2024));
        assert!(!crate::dt::NAMESPACE.is_empty());
    }
}
