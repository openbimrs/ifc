//! Where an entity sits in the spatial hierarchy.

/// The spatial role of an entity, as far as containment is concerned.
///
/// This is a *structural* classification, not a schema subtype check: it asks
/// "can this contain things, and roughly at what level", which is what tree
/// assembly needs. A full subtype test needs `ifc-schema` and belongs in a
/// validation pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpatialKind {
    /// `IfcProject` -- the root. A conformant file has exactly one.
    Project,
    /// `IfcSite`.
    Site,
    /// `IfcBuilding`.
    Building,
    /// `IfcBuildingStorey`.
    Storey,
    /// `IfcSpace`.
    Space,
    /// Any other spatial container, including IFC4x3 facility types such as
    /// `IfcBridge` and `IfcRoad`, which are recognised by suffix so this crate
    /// does not need a per-version type list.
    OtherContainer,
    /// A physical element: wall, door, slab. Contained, never a container.
    Element,
}

impl SpatialKind {
    /// Classify a STEP type name.
    ///
    /// Unknown types are [`Element`](Self::Element): an entity that no
    /// relationship names as a container is a leaf, and treating it as one
    /// keeps unfamiliar IFC4x3 or vendor types in the tree rather than
    /// dropping them.
    #[must_use]
    pub fn classify(type_name: &str) -> Self {
        let upper = type_name.to_ascii_uppercase();
        match upper.as_str() {
            "IFCPROJECT" => Self::Project,
            "IFCSITE" => Self::Site,
            "IFCBUILDING" => Self::Building,
            "IFCBUILDINGSTOREY" => Self::Storey,
            "IFCSPACE" => Self::Space,
            // IFC4x3 added facilities (IfcBridge, IfcRoad, IfcRailway) and
            // their parts. Matching the spatial-element suffix keeps them
            // classified without pinning this crate to one schema version.
            _ if upper.starts_with("IFCSPATIAL") || upper == "IFCFACILITY" => Self::OtherContainer,
            _ => Self::Element,
        }
    }

    /// Whether entities of this kind can contain others.
    #[must_use]
    pub const fn is_container(self) -> bool {
        !matches!(self, Self::Element)
    }
}
