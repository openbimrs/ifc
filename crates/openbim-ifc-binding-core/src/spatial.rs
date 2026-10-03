//! The spatial tree and containment (feature `spatial`, #123).
//!
//! The facade's `SpatialTree` reads the aggregation, containment and
//! spatial-reference relationships once; this snapshot carries every
//! container with its parent, sub-containers and the elements placed in
//! it, so a host builds its project/site/building/storey/space outline
//! from one call. Containers are classified against the release the header
//! declares (IFC2X3, IFC4, IFC4X3); `release` is `None` for a bundled
//! release the tree is not verified for (IFC4X1, IFC4X2), whose containers
//! are those any verified release declares. A header naming no bundled
//! release is refused with `unsupported-schema`.

use crate::record::{Field, Record, ToRecord};
use crate::{BindingError, IfcModel};

/// The containment tree of a model.
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialTree {
    /// The release containers were classified against (`IFC4_ADD2_TC1`), or `None`
    /// when the header's release is not one the tree is verified for.
    pub release: Option<String>,
    /// Containers no relationship places under a parent; a conformant file
    /// has one, the `IfcProject`.
    pub roots: Vec<u64>,
    /// Every spatial container, by entity id.
    pub nodes: Vec<SpatialNode>,
    /// Containers other than the most general root that no aggregation
    /// places under a parent: a detached branch.
    pub orphans: Vec<u64>,
    /// `(relationship, target)` pairs naming an entity the file lacks.
    pub dangling: Vec<(u64, u64)>,
    /// Statements the tree could not honour, in the order they were met.
    pub anomalies: Vec<SpatialAnomaly>,
}

/// One spatial container.
#[derive(Debug, Clone, PartialEq)]
pub struct SpatialNode {
    /// The container's entity id.
    pub id: u64,
    /// Its `GlobalId`.
    pub global_id: Option<String>,
    /// Its `Name`.
    pub name: Option<String>,
    /// Its entity type, upper-case.
    pub type_name: String,
    /// `project`, `site`, `building`, `storey`, `space` or `other` (a
    /// spatial zone, an IFC4X3 facility or facility part, ...).
    pub kind: String,
    /// The container it is aggregated into.
    pub parent: Option<u64>,
    /// Sub-containers, in file order.
    pub children: Vec<u64>,
    /// Elements contained directly, in file order.
    pub elements: Vec<u64>,
    /// Elements referenced (`IfcRelReferencedInSpatialStructure`), not
    /// contained.
    pub referenced: Vec<u64>,
}

/// A second parent or a non-container structure the tree rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatialAnomaly {
    /// `contained-twice`, `aggregated-twice`, `contained-in-non-container`
    /// or `referenced-in-non-container`.
    pub kind: String,
    /// The relationship whose statement was dropped.
    pub relation: u64,
    /// The element, child or structure the statement is about.
    pub subject: u64,
    /// For a second parent: the parent kept.
    pub kept: Option<u64>,
}

impl IfcModel {
    /// The spatial containment tree.
    ///
    /// Refused with `unsupported-schema` when the header names no bundled
    /// release, and with `feature-disabled` without the `spatial` feature.
    pub fn spatial_tree(&self) -> Result<SpatialTree, BindingError> {
        #[cfg(feature = "spatial")]
        {
            read(self)
        }
        #[cfg(not(feature = "spatial"))]
        {
            Err(BindingError::FeatureDisabled("spatial"))
        }
    }
}

#[cfg(feature = "spatial")]
fn read(model: &IfcModel) -> Result<SpatialTree, BindingError> {
    use ifc::spatial::SpatialAnomaly as Anomaly;
    use ifc::SpatialKind;

    model.declared_schema()?;
    let tree = ifc::SpatialTree::build(&model.inner);
    let ids = |ids: &[ifc::EntityId]| ids.iter().map(|id| id.0).collect::<Vec<_>>();
    let mut nodes = Vec::new();
    for node in tree.containers() {
        let identity = model.identity(node.id.0)?;
        let kind = match node.kind {
            SpatialKind::Project => "project",
            SpatialKind::Site => "site",
            SpatialKind::Building => "building",
            SpatialKind::Storey => "storey",
            SpatialKind::Space => "space",
            _ => "other",
        };
        nodes.push(SpatialNode {
            id: node.id.0,
            global_id: identity.global_id,
            name: identity.name,
            type_name: model.type_of(node.id.0)?.to_owned(),
            kind: kind.to_owned(),
            parent: node.parent.map(|id| id.0),
            children: ids(&node.children),
            elements: ids(&node.elements),
            referenced: ids(tree.referenced_elements(node.id)),
        });
    }
    let anomalies = tree
        .anomalies()
        .iter()
        .map(|anomaly| {
            let (kind, relation, subject, kept) = match *anomaly {
                Anomaly::ContainedTwice {
                    element,
                    kept,
                    relation,
                    ..
                } => ("contained-twice", relation, element, Some(kept)),
                Anomaly::AggregatedTwice {
                    child,
                    kept,
                    relation,
                    ..
                } => ("aggregated-twice", relation, child, Some(kept)),
                Anomaly::ContainedInNonContainer {
                    relation,
                    structure,
                } => ("contained-in-non-container", relation, structure, None),
                Anomaly::ReferencedInNonContainer {
                    relation,
                    structure,
                } => ("referenced-in-non-container", relation, structure, None),
                // An anomaly added after this binding: still reported.
                _ => ("other", ifc::EntityId(0), ifc::EntityId(0), None),
            };
            SpatialAnomaly {
                kind: kind.to_owned(),
                relation: relation.0,
                subject: subject.0,
                kept: kept.map(|id| id.0),
            }
        })
        .collect();
    Ok(SpatialTree {
        release: tree
            .release()
            .map(|release| release.release_id().to_owned()),
        roots: ids(tree.roots()),
        nodes,
        orphans: ids(tree.orphans()),
        dangling: tree
            .dangling()
            .iter()
            .map(|(relation, target)| (relation.0, target.0))
            .collect(),
        anomalies,
    })
}

impl ToRecord for SpatialTree {
    fn to_record(&self) -> Record {
        Record::new(
            "SpatialTree",
            vec![
                ("release", Field::text(self.release.clone())),
                ("roots", Field::ids(self.roots.iter().copied())),
                ("nodes", Field::records(&self.nodes)),
                ("orphans", Field::ids(self.orphans.iter().copied())),
                (
                    "dangling",
                    Field::List(
                        self.dangling
                            .iter()
                            .map(|&(relation, target)| {
                                Field::Record(Record::new(
                                    "SpatialDanglingReference",
                                    vec![
                                        ("relation", Field::Id(relation)),
                                        ("target", Field::Id(target)),
                                    ],
                                ))
                            })
                            .collect(),
                    ),
                ),
                ("anomalies", Field::records(&self.anomalies)),
            ],
        )
    }
}

impl ToRecord for SpatialNode {
    fn to_record(&self) -> Record {
        Record::new(
            "SpatialNode",
            vec![
                ("id", Field::Id(self.id)),
                ("global_id", Field::text(self.global_id.clone())),
                ("name", Field::text(self.name.clone())),
                ("type_name", Field::Text(self.type_name.clone())),
                ("kind", Field::Text(self.kind.clone())),
                ("parent", Field::id(self.parent)),
                ("children", Field::ids(self.children.iter().copied())),
                ("elements", Field::ids(self.elements.iter().copied())),
                ("referenced", Field::ids(self.referenced.iter().copied())),
            ],
        )
    }
}

impl ToRecord for SpatialAnomaly {
    fn to_record(&self) -> Record {
        Record::new(
            "SpatialAnomaly",
            vec![
                ("kind", Field::Text(self.kind.clone())),
                ("relation", Field::Id(self.relation)),
                ("subject", Field::Id(self.subject)),
                ("kept", Field::id(self.kept)),
            ],
        )
    }
}
