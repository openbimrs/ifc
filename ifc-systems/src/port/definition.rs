//! `IfcPort` and `IfcDistributionPort` definitions.
//!
//! # Two attachment mechanisms, not one
//!
//! A port is attached to its element in one of two ways, and real files use
//! both:
//!
//! ```text
//! IfcRelNests                   4 = RelatingObject (element)  5 = RelatedObjects (ports)
//! IfcRelConnectsPortToElement   4 = RelatingPort              5 = RelatedElement
//! ```
//!
//! `IfcRelNests` is the IFC4 mechanism and `IfcRelConnectsPortToElement` is
//! the IFC2x3 one, retained in IFC4 for compatibility. They are REVERSED with
//! respect to each other: nesting names the element first, the legacy
//! relationship names the port first. Reading one layout for the other
//! silently swaps ports and elements.
//!
//! Supporting only the IFC4 form would drop every port in a file exported by
//! an older tool, which is a large share of what exists.

use ifc_model::{EntityId, Model, Value};
use ifc_schema::ifc4;

use crate::error::SystemAnomaly;

mod slot {
    /// `IfcRelNests.RelatingObject` -- the nesting element.
    pub const NESTS_PARENT: usize = 4;
    /// `IfcRelNests.RelatedObjects` -- the nested ports.
    pub const NESTS_CHILDREN: usize = 5;
    /// `IfcRelConnectsPortToElement.RelatingPort` -- the PORT, not the element.
    pub const PORT_TO_ELEMENT_PORT: usize = 4;
    /// `IfcRelConnectsPortToElement.RelatedElement`.
    pub const PORT_TO_ELEMENT_ELEMENT: usize = 5;
    /// `IfcDistributionPort.FlowDirection`.
    pub const FLOW_DIRECTION: usize = 7;
}

/// Which way material or energy moves through a port.
///
/// `IfcFlowDirectionEnum`. A missing direction is [`FlowDirection::NotDefined`]
/// rather than an assumed default: guessing SOURCE would invent a direction
/// the file never stated, and downstream tracing would follow edges that do
/// not exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowDirection {
    /// Material leaves the element through this port.
    Source,
    /// Material enters the element through this port.
    Sink,
    /// Bidirectional.
    SourceAndSink,
    /// Stated as NOTDEFINED, or not stated at all.
    NotDefined,
}

impl FlowDirection {
    fn parse(value: Option<&Value>) -> Self {
        match value {
            Some(Value::Enum(name)) => match name.to_ascii_uppercase().as_str() {
                "SOURCE" => Self::Source,
                "SINK" => Self::Sink,
                "SOURCEANDSINK" => Self::SourceAndSink,
                _ => Self::NotDefined,
            },
            _ => Self::NotDefined,
        }
    }
}

/// How a port was attached to its element.
///
/// Recorded rather than normalised away: a file mixing both mechanisms is
/// worth knowing about, and a consumer migrating away from the legacy form
/// needs to see which files still use it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attachment {
    /// `IfcRelNests` -- the IFC4 mechanism.
    Nests,
    /// `IfcRelConnectsPortToElement` -- the IFC2x3 mechanism.
    ConnectsPortToElement,
}

/// A port as the file states it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Port {
    /// The `IfcPort` subtype entity.
    pub id: EntityId,
    /// Declared type, upper-cased.
    pub type_name: String,
    /// `Name`, when present.
    pub name: Option<String>,
    /// Flow direction, `NotDefined` when the file omits it.
    pub flow: FlowDirection,
    /// The element this port belongs to, when the file attaches it.
    ///
    /// `IfcPort.ContainedIn` is `SET [0:1]`, so a port has at most one owning
    /// element. A free-floating port is legal and is reported as `None`.
    pub element: Option<EntityId>,
    /// Which relationship attached it.
    pub attachment: Option<Attachment>,
}

/// Ids of every entity whose declared type is an `IfcPort` subtype.
///
/// `IfcPort` is ABSTRACT, so no entity is ever literally an `IFCPORT`: every
/// port in a file is an `IfcDistributionPort`. Selecting by ancestry rather
/// than by the exact-type index is what makes that work, and it keeps working
/// if a later schema adds another subtype.
fn port_ids(model: &Model) -> Vec<EntityId> {
    let schema = ifc4();
    let mut out = Vec::new();
    for (type_name, _) in model.type_histogram() {
        if schema.is_a(type_name, "IFCPORT") {
            out.extend_from_slice(model.ids_of_type(type_name));
        }
    }
    out.sort_unstable();
    out
}

fn text(model: &Model, id: EntityId, slot: usize) -> Option<String> {
    match model.get(id)?.attributes.get(slot)? {
        Value::Text(t) => Some(t.to_string()),
        _ => None,
    }
}

/// Every port in the file, with its owning element resolved.
///
/// Both attachment mechanisms are read. `IfcRelNests` is applied first
/// because it is the IFC4 form, so when an exporter writes both and they
/// disagree the modern one wins and the conflict is reported.
pub fn ports(model: &Model) -> (Vec<Port>, Vec<SystemAnomaly>) {
    let schema = ifc4();
    let mut anomalies = Vec::new();
    let mut owner: std::collections::BTreeMap<EntityId, (EntityId, Attachment)> =
        std::collections::BTreeMap::new();

    let mut attach =
        |port: EntityId, element: EntityId, how: Attachment, anomalies: &mut Vec<SystemAnomaly>| {
            match owner.get(&port) {
                Some((kept, _)) if *kept != element => {
                    anomalies.push(SystemAnomaly::PortAttachedTwice {
                        port,
                        kept: *kept,
                        rejected: element,
                    });
                }
                Some(_) => {}
                None => {
                    owner.insert(port, (element, how));
                }
            }
        };

    // IFC4: the element nests its ports.
    for &relation in model.ids_of_type("IFCRELNESTS") {
        let Some(entity) = model.get(relation) else {
            continue;
        };
        let Some(Value::Ref(parent)) = entity.attributes.get(slot::NESTS_PARENT) else {
            continue;
        };
        for child in refs(entity.attributes.get(slot::NESTS_CHILDREN)) {
            let Some(child_entity) = model.get(child) else {
                anomalies.push(SystemAnomaly::Dangling {
                    relation,
                    missing: child,
                });
                continue;
            };
            // IfcRelNests nests anything, not just ports: a distribution
            // element nests its ports, but an element type nests its
            // components too. Only ports are this module's concern.
            if !schema.is_a(&child_entity.type_name.to_ascii_uppercase(), "IFCPORT") {
                continue;
            }
            attach(child, *parent, Attachment::Nests, &mut anomalies);
        }
    }

    // IFC2x3 compatibility form, still emitted by real exporters.
    for &relation in model.ids_of_type("IFCRELCONNECTSPORTTOELEMENT") {
        let Some(entity) = model.get(relation) else {
            continue;
        };
        let port = match entity.attributes.get(slot::PORT_TO_ELEMENT_PORT) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        let element = match entity.attributes.get(slot::PORT_TO_ELEMENT_ELEMENT) {
            Some(Value::Ref(id)) => *id,
            _ => continue,
        };
        if model.get(port).is_none() {
            anomalies.push(SystemAnomaly::Dangling {
                relation,
                missing: port,
            });
            continue;
        }
        if model.get(element).is_none() {
            anomalies.push(SystemAnomaly::Dangling {
                relation,
                missing: element,
            });
            continue;
        }
        attach(
            port,
            element,
            Attachment::ConnectsPortToElement,
            &mut anomalies,
        );
    }

    let mut ports = Vec::new();
    for id in port_ids(model) {
        let Some(entity) = model.get(id) else {
            continue;
        };
        let attached = owner.get(&id);
        ports.push(Port {
            id,
            type_name: entity.type_name.to_ascii_uppercase(),
            name: text(model, id, 2),
            flow: FlowDirection::parse(entity.attributes.get(slot::FLOW_DIRECTION)),
            element: attached.map(|(e, _)| *e),
            attachment: attached.map(|(_, how)| *how),
        });
    }
    (ports, anomalies)
}

fn refs(value: Option<&Value>) -> Vec<EntityId> {
    match value {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::Ref(id) => Some(*id),
                _ => None,
            })
            .collect(),
        Some(Value::Ref(id)) => vec![*id],
        _ => Vec::new(),
    }
}
