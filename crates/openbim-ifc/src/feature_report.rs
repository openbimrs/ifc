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
    #[cfg(feature = "schema-api")]
    features.push("schema-api");
    #[cfg(feature = "ifc2x3")]
    features.push("ifc2x3");
    #[cfg(feature = "ifc4")]
    features.push("ifc4");
    #[cfg(feature = "ifc4x1")]
    features.push("ifc4x1");
    #[cfg(feature = "ifc4x2")]
    features.push("ifc4x2");
    #[cfg(feature = "ifc4x3")]
    features.push("ifc4x3");
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
    #[cfg(feature = "approval")]
    features.push("approval");
    #[cfg(feature = "control")]
    features.push("control");
    #[cfg(feature = "tabular")]
    features.push("tabular");
    #[cfg(feature = "constraint")]
    features.push("constraint");
    #[cfg(feature = "element-type")]
    features.push("element-type");
    #[cfg(feature = "occurrence")]
    features.push("occurrence");
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
    #[cfg(feature = "author")]
    features.push("author");
    #[cfg(feature = "spatial")]
    features.push("spatial");
    #[cfg(feature = "geometry-select")]
    features.push("geometry-select");
    #[cfg(feature = "geometry")]
    features.push("geometry");
    #[cfg(feature = "georef")]
    features.push("georef");
    #[cfg(feature = "alignment")]
    features.push("alignment");
    features
}
