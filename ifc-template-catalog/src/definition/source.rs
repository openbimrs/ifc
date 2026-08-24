//! Catalog release identity and provenance.

/// Exact IFC publication edition represented by a catalog.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, bincode::Encode, bincode::Decode,
)]
#[non_exhaustive]
pub enum CatalogEdition {
    Ifc2x3Tc1,
    Ifc4Add2Tc1,
    Ifc4x3Add2,
}

/// Per-template provenance inside a source publication.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct TemplateSource {
    pub relative_path: String,
    pub sha256: String,
}

/// Reproducible source identity for a normalized snapshot.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct SourceManifest {
    pub edition: CatalogEdition,
    pub source_label: String,
    pub source_url: String,
    /// SHA-256 over sorted relative source paths and bytes.
    pub sha256: String,
    pub property_set_count: usize,
    pub quantity_set_count: usize,
}
