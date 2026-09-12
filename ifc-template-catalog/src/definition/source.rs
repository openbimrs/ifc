//! Catalog release identity and provenance.

/// Exact IFC publication edition represented by a catalog.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, bincode::Encode, bincode::Decode,
)]
#[non_exhaustive]
pub enum CatalogEdition {
    /// IFC2X3 TC1.
    Ifc2x3Tc1,
    /// IFC4 ADD2 TC1.
    Ifc4Add2Tc1,
    /// IFC4X3 ADD2.
    Ifc4x3Add2,
}

/// Per-template provenance inside a source publication.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct TemplateSource {
    /// Path of the source file relative to the publication archive root.
    pub relative_path: String,
    /// SHA-256 of the source file's raw bytes.
    pub sha256: String,
}

/// Reproducible source identity for a normalized snapshot.
#[derive(Debug, Clone, PartialEq, Eq, bincode::Encode, bincode::Decode)]
pub struct SourceManifest {
    /// IFC publication edition this snapshot was built from.
    pub edition: CatalogEdition,
    /// Human-readable name of the source publication.
    pub source_label: String,
    /// URL the source publication was retrieved from.
    pub source_url: String,
    /// SHA-256 over sorted relative source paths and bytes.
    pub sha256: String,
    /// Number of property-set templates included in the snapshot.
    pub property_set_count: usize,
    /// Number of quantity-set templates included in the snapshot.
    pub quantity_set_count: usize,
}
