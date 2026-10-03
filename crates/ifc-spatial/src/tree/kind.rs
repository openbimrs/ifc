//! Where an entity sits in the spatial hierarchy.
//!
//! # From the declared release, not from the name
//!
//! A spatial container is an `IfcSpatialElement` (IFC4 ADD2 TC1, IFC4X3
//! ADD2) or an `IfcSpatialStructureElement` (IFC2X3 TC1, which has no
//! `IfcSpatialElement`), plus the `IfcProject` at the root. The set differs
//! by release: IFC4 adds `IfcSpatialZone` and `IfcExternalSpatialElement`,
//! IFC4X3 adds `IfcFacility` with `IfcBridge`, `IfcRoad`, `IfcRailway` and
//! `IfcMarineFacility`, and the `IfcFacilityPart` subtypes. None of those
//! names shares a pattern, so membership is the release's own subtype test
//! (`Schema::is_a`) against the bundled table of the release the file
//! declares.

use ifc_model::Model;
use ifc_schema::{for_version, Schema, SchemaVersion};

/// The spatial role of an entity, as far as containment is concerned.
///
/// The five named kinds are those exact entities. Every other spatial
/// element of the release is [`OtherContainer`](Self::OtherContainer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
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
    /// Any other spatial element of the release: `IfcSpatialZone` and
    /// `IfcExternalSpatialElement` (IFC4, IFC4X3), and the IFC4X3
    /// facilities (`IfcFacility`, `IfcBridge`, `IfcRoad`, `IfcRailway`,
    /// `IfcMarineFacility`) and facility parts (`IfcBridgePart`,
    /// `IfcRoadPart`, `IfcRailwayPart`, `IfcMarinePart`,
    /// `IfcFacilityPartCommon`).
    OtherContainer,
    /// Not a spatial container: a wall, a door, a slab, or any entity the
    /// release does not declare as a spatial element.
    Element,
}

impl SpatialKind {
    /// Classify a STEP type name without a release: a container when any
    /// bundled release (IFC2X3, IFC4, IFC4X3) declares it a spatial
    /// element.
    ///
    /// No bundled release declares as a spatial element a name another
    /// declares as something else (`tests/classification.rs` asserts it),
    /// so this never contradicts [`classify_in`](Self::classify_in) for a
    /// name the release declares. Prefer `classify_in` when the release is
    /// known; [`SpatialTree`](crate::SpatialTree) does.
    #[must_use]
    pub fn classify(type_name: &str) -> Self {
        Classifier::any_release().classify(type_name)
    }

    /// Classify a STEP type name against one release's schema table.
    ///
    /// A release whose feature this build leaves out has no table, so every
    /// name classifies as [`Element`](Self::Element); check
    /// `ifc_schema::for_version` first when the build is single-release.
    #[must_use]
    pub fn classify_in(type_name: &str, release: SchemaVersion) -> Self {
        Classifier::for_release(release).classify(type_name)
    }

    /// Whether entities of this kind can contain others.
    #[must_use]
    pub const fn is_container(self) -> bool {
        !matches!(self, Self::Element)
    }
}

/// Releases the spatial classification is verified against.
const VERIFIED: [SchemaVersion; 3] = [
    SchemaVersion::Ifc2x3,
    SchemaVersion::Ifc4,
    SchemaVersion::Ifc4x3,
];

/// The table(s) a classification is answered from.
pub(crate) struct Classifier {
    /// The bound release, if the file declares exactly one bundled release.
    release: Option<SchemaVersion>,
    tables: Vec<&'static Schema>,
}

impl Classifier {
    /// The release `model` declares, when it names exactly one release this
    /// classifier is verified for; otherwise every verified release, with
    /// none bound.
    ///
    /// IFC4X1 and IFC4X2 are bundled by `ifc-schema` but not verified here,
    /// so they bind nothing ([`Self::bound_release`] is `None`) rather than
    /// being read as IFC4 or IFC4X3. A verified release whose table this
    /// build leaves out (its release feature is off, #306) binds nothing the
    /// same way: its empty table would classify every entity as an element.
    pub(crate) fn for_model(model: &Model) -> Self {
        match model.header().schema.as_slice() {
            [token] => match SchemaVersion::from_header_token(token) {
                Some(release) if VERIFIED.contains(&release) && for_version(release).is_ok() => {
                    Self::for_release(release)
                }
                _ => Self::any_release(),
            },
            _ => Self::any_release(),
        }
    }

    fn for_release(release: SchemaVersion) -> Self {
        Self {
            release: Some(release),
            tables: for_version(release).into_iter().collect(),
        }
    }

    fn any_release() -> Self {
        let tables = VERIFIED
            .into_iter()
            .filter_map(|release| for_version(release).ok())
            .collect();
        Self {
            release: None,
            tables,
        }
    }

    /// The release bound, or `None` when every bundled release is asked.
    pub(crate) fn bound_release(&self) -> Option<SchemaVersion> {
        self.release
    }

    pub(crate) fn classify(&self, type_name: &str) -> SpatialKind {
        let upper = type_name.to_ascii_uppercase();
        if !self.tables.iter().any(|table| is_spatial(table, &upper)) {
            return SpatialKind::Element;
        }
        match upper.as_str() {
            "IFCPROJECT" => SpatialKind::Project,
            "IFCSITE" => SpatialKind::Site,
            "IFCBUILDING" => SpatialKind::Building,
            "IFCBUILDINGSTOREY" => SpatialKind::Storey,
            "IFCSPACE" => SpatialKind::Space,
            _ => SpatialKind::OtherContainer,
        }
    }
}

/// Whether `table` declares `upper` as the project or a spatial element.
fn is_spatial(table: &Schema, upper: &str) -> bool {
    // IfcProject is an IfcContext (IFC4, IFC4X3) or an IfcObject (IFC2X3),
    // not a spatial element, but it is the root every tree hangs from.
    if upper == "IFCPROJECT" {
        return table.is_a(upper, "IFCPROJECT");
    }
    // IFC2X3 has no IfcSpatialElement; its spatial root is
    // IfcSpatialStructureElement. Asking for an undeclared ancestor
    // answers false, so the IFC4 root is tried first and the IFC2X3 one
    // only matters where it is the root.
    table.is_a(upper, "IFCSPATIALELEMENT") || table.is_a(upper, "IFCSPATIALSTRUCTUREELEMENT")
}

#[cfg(test)]
mod intermediate_release_tests {
    use super::*;

    /// An IFC4X1 or IFC4X2 header binds no release: the classifier answers
    /// from the verified tables with nothing bound, instead of claiming the
    /// file was read as IFC4 or IFC4X3.
    #[test]
    fn ifc4x1_and_ifc4x2_bind_no_release() {
        for token in ["IFC4X1", "IFC4X2"] {
            let mut model = Model::new();
            model.header_mut().schema = vec![token.to_owned()];
            assert_eq!(
                Classifier::for_model(&model).bound_release(),
                None,
                "{token}"
            );
        }
        let mut model = Model::new();
        model.header_mut().schema = vec!["IFC4X3".to_owned()];
        assert_eq!(
            Classifier::for_model(&model).bound_release(),
            Some(SchemaVersion::Ifc4x3)
        );
    }
}
