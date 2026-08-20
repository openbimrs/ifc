/// The feature set this build was compiled with, for diagnostics.
///
/// A support question about a thin build is much easier to answer when the
/// binary can state what it contains.
// Same rationale as `codecs`: the pushes are feature-gated, not a literal.
#[allow(clippy::vec_init_then_push)]
pub fn compiled_features() -> Vec<&'static str> {
    // `mut` is unused when no optional feature is enabled; the allow keeps the
    // no-feature build warning-clean without special-casing the body.
    #[allow(unused_mut)]
    let mut features = Vec::new();
    #[cfg(feature = "step")]
    features.push("step");
    #[cfg(feature = "ifcxml")]
    features.push("ifcxml");
    #[cfg(feature = "schema")]
    features.push("schema");
    #[cfg(feature = "properties")]
    features.push("properties");
    #[cfg(feature = "property-catalog")]
    features.push("property-catalog");
    #[cfg(feature = "material-templates")]
    features.push("material-templates");
    #[cfg(feature = "cost")]
    features.push("cost");
    #[cfg(feature = "schedule")]
    features.push("schedule");
    #[cfg(feature = "material")]
    features.push("material");
    #[cfg(feature = "classification")]
    features.push("classification");
    #[cfg(feature = "structural")]
    features.push("structural");
    #[cfg(feature = "resource")]
    features.push("resource");
    #[cfg(feature = "systems")]
    features.push("systems");
    #[cfg(feature = "style")]
    features.push("style");
    #[cfg(feature = "validate")]
    features.push("validate");
    #[cfg(feature = "geometry")]
    features.push("geometry");
    #[cfg(feature = "georef")]
    features.push("georef");
    #[cfg(feature = "alignment")]
    features.push("alignment");
    features
}
