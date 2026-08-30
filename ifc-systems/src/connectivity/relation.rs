//! `IfcRelConnectsPorts` and the port connection graph.
//!
//! # Slots
//!
//! ```text
//! IfcRelConnectsPorts   4 = RelatingPort   5 = RelatedPort   6 = RealizingElement
//! ```
//!
//! # Direction is not connectivity
//!
//! `RelatingPort` and `RelatedPort` record which port the exporter wrote
//! first, NOT which way anything flows. Flow is stated separately by each
//! port's `FlowDirection`. Treating the relationship as directed would invent
//! a direction: a pipe connecting an outlet to an inlet is the same physical
//! connection whichever end the exporter happened to name first.
//!
//! The graph is therefore UNDIRECTED, and direction is applied on top from
//! flow directions when a caller asks for it.

use std::collections::{BTreeMap, BTreeSet};

use ifc_model::{EntityId, Model, Value};
use ifc_schema::ifc4;

use crate::error::SystemAnomaly;

mod slot {
    /// `IfcRelConnectsPorts.RelatingPort`.
    pub const RELATING: usize = 4;
    /// `IfcRelConnectsPorts.RelatedPort`.
    pub const RELATED: usize = 5;
    /// `IfcRelConnectsPorts.RealizingElement` -- the pipe/duct that realises
    /// the connection, when the file names one.
    pub const REALIZING: usize = 6;
}

/// One stated port-to-port connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    /// The `IfcRelConnectsPorts` entity.
    pub id: EntityId,
    /// The port named first. NOT an upstream/downstream claim.
    pub relating: EntityId,
    /// The port named second.
    pub related: EntityId,
    /// The element realising this connection, when stated.
    pub realizing: Option<EntityId>,
}

/// An undirected port connection graph.
///
/// Built once and queried many times: a traversal that re-scanned the model
/// for each step would be quadratic on files with thousands of ports.
#[derive(Debug, Default, Clone)]
pub struct ConnectionGraph {
    connections: Vec<Connection>,
    adjacency: BTreeMap<EntityId, BTreeSet<EntityId>>,
}

impl ConnectionGraph {
    /// Read every `IfcRelConnectsPorts` in the file.
    ///
    /// A connection naming a non-port, or an entity not in the file, is
    /// reported and skipped: one malformed relationship must not cost the
    /// caller the rest of the network.
    pub fn build(model: &Model) -> (Self, Vec<SystemAnomaly>) {
        let schema = ifc4();
        let mut anomalies = Vec::new();
        let mut graph = Self::default();

        for &id in model.ids_of_type("IFCRELCONNECTSPORTS") {
            let Some(entity) = model.get(id) else {
                continue;
            };
            let relating = match entity.attributes.get(slot::RELATING) {
                Some(Value::Ref(port)) => *port,
                _ => continue,
            };
            let related = match entity.attributes.get(slot::RELATED) {
                Some(Value::Ref(port)) => *port,
                _ => continue,
            };

            let mut ok = true;
            for port in [relating, related] {
                match model.get(port) {
                    None => {
                        anomalies.push(SystemAnomaly::Dangling {
                            relation: id,
                            missing: port,
                        });
                        ok = false;
                    }
                    Some(e) if !schema.is_a(&e.type_name.to_ascii_uppercase(), "IFCPORT") => {
                        anomalies.push(SystemAnomaly::NotAPort {
                            relation: id,
                            entity: port,
                            type_name: e.type_name.to_ascii_uppercase(),
                        });
                        ok = false;
                    }
                    Some(_) => {}
                }
            }
            if !ok {
                continue;
            }

            let realizing = match entity.attributes.get(slot::REALIZING) {
                Some(Value::Ref(element)) => Some(*element),
                _ => None,
            };
            graph.connections.push(Connection {
                id,
                relating,
                related,
                realizing,
            });
            // Undirected: both directions are inserted, because the schema
            // order records authoring order and not flow.
            graph.adjacency.entry(relating).or_default().insert(related);
            graph.adjacency.entry(related).or_default().insert(relating);
        }
        (graph, anomalies)
    }

    /// Every stated connection, in file order.
    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }

    /// Ports directly connected to `port`, ascending by id.
    pub fn neighbours(&self, port: EntityId) -> Vec<EntityId> {
        self.adjacency
            .get(&port)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Every port reachable from `start`, including `start` itself.
    ///
    /// Breadth-first with a visited set, so a network containing a LOOP -- a
    /// ring main, a recirculating circuit -- terminates instead of running
    /// forever. Loops are normal in real distribution systems, not corrupt
    /// data, so this must not be a refusal.
    pub fn reachable_from(&self, start: EntityId) -> Vec<EntityId> {
        let mut seen = BTreeSet::new();
        let mut queue = std::collections::VecDeque::new();
        seen.insert(start);
        queue.push_back(start);
        while let Some(port) = queue.pop_front() {
            for next in self.adjacency.get(&port).into_iter().flatten() {
                if seen.insert(*next) {
                    queue.push_back(*next);
                }
            }
        }
        seen.into_iter().collect()
    }

    /// Connected components of the graph, each sorted, components ascending.
    ///
    /// A distribution system that splits into two components is usually an
    /// authoring error -- a missing connection -- and is worth surfacing.
    pub fn components(&self) -> Vec<Vec<EntityId>> {
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for &port in self.adjacency.keys() {
            if seen.contains(&port) {
                continue;
            }
            let component = self.reachable_from(port);
            seen.extend(component.iter().copied());
            out.push(component);
        }
        out
    }
}

/// A network view that also steps THROUGH elements, not just between them.
///
/// `IfcRelConnectsPorts` joins one element's port to another's. It never
/// joins an element's OWN ports to each other: the fact that fluid entering
/// a pipe's inlet leaves by its outlet is implied by the element, not stated
/// by any relationship.
///
/// So the raw connection graph of a real chain
///
/// ```text
/// [seg0] out --- in [seg1] out --- in [fitting]
/// ```
///
/// has NO path from seg0's inlet to seg1 at all: every connection is an
/// isolated pair. Answering "what is downstream of this pipe" needs both
/// kinds of edge -- across connections AND through elements.
///
/// This is the distinction between the two, made explicit rather than
/// silently folded into [`ConnectionGraph`].
#[derive(Debug, Default, Clone)]
pub struct NetworkGraph {
    adjacency: BTreeMap<EntityId, BTreeSet<EntityId>>,
}

impl NetworkGraph {
    /// Combine stated connections with through-element port pairing.
    ///
    /// `ports` supplies each port's owning element, which is what makes the
    /// through-element edges knowable.
    pub fn build(graph: &ConnectionGraph, ports: &[crate::port::Port]) -> Self {
        let mut adjacency: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();

        // Stated port-to-port connections.
        for connection in &graph.connections {
            adjacency
                .entry(connection.relating)
                .or_default()
                .insert(connection.related);
            adjacency
                .entry(connection.related)
                .or_default()
                .insert(connection.relating);
        }

        // Through-element edges: every pair of ports on the same element.
        let mut by_element: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
        for port in ports {
            if let Some(element) = port.element {
                by_element.entry(element).or_default().push(port.id);
            }
        }
        for members in by_element.values() {
            for (i, &a) in members.iter().enumerate() {
                for &b in &members[i + 1..] {
                    adjacency.entry(a).or_default().insert(b);
                    adjacency.entry(b).or_default().insert(a);
                }
            }
        }

        Self { adjacency }
    }

    /// Every port reachable from `start`, including `start`.
    ///
    /// Cycle-safe: ring mains are normal, so a visited set is required, not
    /// an optimisation.
    pub fn reachable_from(&self, start: EntityId) -> Vec<EntityId> {
        let mut seen = BTreeSet::new();
        let mut queue = std::collections::VecDeque::new();
        seen.insert(start);
        queue.push_back(start);
        while let Some(port) = queue.pop_front() {
            for next in self.adjacency.get(&port).into_iter().flatten() {
                if seen.insert(*next) {
                    queue.push_back(*next);
                }
            }
        }
        seen.into_iter().collect()
    }

    /// Connected components, each sorted, components ascending.
    pub fn components(&self) -> Vec<Vec<EntityId>> {
        let mut seen = BTreeSet::new();
        let mut out = Vec::new();
        for &port in self.adjacency.keys() {
            if seen.contains(&port) {
                continue;
            }
            let component = self.reachable_from(port);
            seen.extend(component.iter().copied());
            out.push(component);
        }
        out
    }
}
