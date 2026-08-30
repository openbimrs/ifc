//! Deterministic upstream/downstream queries, without geometry.
//!
//! # Why this needs flow, and why it is separate from `NetworkGraph`
//!
//! `ConnectionGraph` and `NetworkGraph` are UNDIRECTED: they answer "what is
//! joined to what". That cannot answer "what does this pump feed", because
//! reachability alone walks backwards through the supply as happily as
//! forwards.
//!
//! Direction comes from `IfcDistributionPort.FlowDirection`, which is stated
//! per port, not per connection. The orientation rules are:
//!
//! - Leaving an element through a `SINK` port is not flow. A sink is where
//!   material ENTERS the element, so material moves INTO it there.
//! - Arriving at a `SOURCE` port is not flow, for the mirror reason.
//! - `SOURCEANDSINK` is bidirectional by definition, so both are allowed.
//! - `NOTDEFINED`, or an absent attribute, states nothing. Those edges are
//!   traversable in BOTH directions, and every query that used one says so.
//!
//! That last rule is the important one. Silently treating unstated direction
//! as "no flow" would make queries on the very common under-specified file
//! return empty and look authoritative. Treating it as bidirectional keeps
//! the answer complete, and `used_undirected` tells the caller the result
//! rests on an assumption the file did not make.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ifc_model::EntityId;

use crate::connectivity::ConnectionGraph;
use crate::flow::FlowDirection;
use crate::port::Port;

/// Which way a query walks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Follow flow: what this element feeds.
    Downstream,
    /// Walk against flow: what feeds this element.
    Upstream,
}

/// The answer to a directed query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowQuery {
    /// Elements reached, ascending by id. Excludes the starting element.
    ///
    /// Ascending rather than discovery order: BFS order depends on adjacency
    /// insertion, so two files stating the same network could disagree. A
    /// caller wanting distance should ask for it explicitly.
    pub elements: Vec<EntityId>,
    /// Whether any traversed edge had unstated direction.
    ///
    /// `true` means the result depends on treating an unstated port as
    /// bidirectional. The elements are still correct as a superset, but the
    /// file did not actually license all of them.
    pub used_undirected: bool,
}

/// A network oriented by flow direction.
///
/// Built from a connection graph plus ports, because direction lives on the
/// port and connectivity lives on the relationship. Neither alone is enough.
pub struct FlowNetwork {
    /// port -> owning element.
    owner: BTreeMap<EntityId, EntityId>,
    /// element -> its ports.
    owned: BTreeMap<EntityId, Vec<EntityId>>,
    /// port -> its stated direction.
    direction: BTreeMap<EntityId, FlowDirection>,
    /// Undirected port-to-port adjacency, from the connection graph.
    adjacency: BTreeMap<EntityId, BTreeSet<EntityId>>,
}

impl FlowNetwork {
    /// Orient a connection graph using the ports' flow directions.
    pub fn build(graph: &ConnectionGraph, ports: &[Port]) -> Self {
        let mut owner = BTreeMap::new();
        let mut owned: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
        let mut direction = BTreeMap::new();
        for port in ports {
            direction.insert(port.id, port.flow);
            if let Some(element) = port.element {
                owner.insert(port.id, element);
                owned.entry(element).or_default().push(port.id);
            }
        }
        let mut adjacency: BTreeMap<EntityId, BTreeSet<EntityId>> = BTreeMap::new();
        for connection in graph.connections() {
            // relating/related is authoring order, not flow: both directions
            // go in, and orientation comes solely from port FlowDirection.
            adjacency
                .entry(connection.relating)
                .or_default()
                .insert(connection.related);
            adjacency
                .entry(connection.related)
                .or_default()
                .insert(connection.relating);
        }
        Self {
            owner,
            owned,
            direction,
            adjacency,
        }
    }

    fn stated(&self, port: EntityId) -> FlowDirection {
        self.direction
            .get(&port)
            .copied()
            .unwrap_or(FlowDirection::NotDefined)
    }

    /// May material leave an element through this port, walking `direction`?
    ///
    /// Returns `(allowed, stated)`. `stated` is false when the port declares
    /// nothing, which the caller records so the answer stays honest.
    fn may_exit(&self, port: EntityId, direction: Direction) -> (bool, bool) {
        match (self.stated(port), direction) {
            // Exiting through a source is flow; through a sink it is not.
            (FlowDirection::Source, Direction::Downstream) => (true, true),
            (FlowDirection::Sink, Direction::Downstream) => (false, true),
            // Walking upstream inverts the test.
            (FlowDirection::Sink, Direction::Upstream) => (true, true),
            (FlowDirection::Source, Direction::Upstream) => (false, true),
            (FlowDirection::SourceAndSink, _) => (true, true),
            // Unstated: allowed, but flagged.
            (FlowDirection::NotDefined, _) => (true, false),
        }
    }

    /// May material enter an element through this port, walking `direction`?
    fn may_enter(&self, port: EntityId, direction: Direction) -> (bool, bool) {
        // Entering is the mirror of exiting, so reuse the rule inverted
        // rather than restating it and risking the two drifting apart.
        match (self.stated(port), direction) {
            (FlowDirection::Sink, Direction::Downstream) => (true, true),
            (FlowDirection::Source, Direction::Downstream) => (false, true),
            (FlowDirection::Source, Direction::Upstream) => (true, true),
            (FlowDirection::Sink, Direction::Upstream) => (false, true),
            (FlowDirection::SourceAndSink, _) => (true, true),
            (FlowDirection::NotDefined, _) => (true, false),
        }
    }

    /// Elements reachable from `element` following (or opposing) flow.
    ///
    /// Cycle-safe: a ring main revisits elements, and the visited set is what
    /// makes this terminate. Ring mains are normal topology, not corruption.
    pub fn query(&self, element: EntityId, direction: Direction) -> FlowQuery {
        let mut seen_elements = BTreeSet::new();
        let mut queue = VecDeque::new();
        let mut used_undirected = false;
        seen_elements.insert(element);
        queue.push_back(element);

        while let Some(current) = queue.pop_front() {
            let Some(ports) = self.owned.get(&current) else {
                continue;
            };
            for &exit in ports {
                let (can_exit, exit_stated) = self.may_exit(exit, direction);
                if !can_exit {
                    continue;
                }
                for next_port in self.adjacency.get(&exit).into_iter().flatten() {
                    let (can_enter, enter_stated) = self.may_enter(*next_port, direction);
                    if !can_enter {
                        continue;
                    }
                    let Some(&next) = self.owner.get(next_port) else {
                        continue;
                    };
                    if next == current {
                        continue;
                    }
                    if seen_elements.insert(next) {
                        if !exit_stated || !enter_stated {
                            used_undirected = true;
                        }
                        queue.push_back(next);
                    }
                }
            }
        }

        seen_elements.remove(&element);
        FlowQuery {
            elements: seen_elements.into_iter().collect(),
            used_undirected,
        }
    }

    /// What this element feeds.
    pub fn downstream_of(&self, element: EntityId) -> FlowQuery {
        self.query(element, Direction::Downstream)
    }

    /// What feeds this element.
    pub fn upstream_of(&self, element: EntityId) -> FlowQuery {
        self.query(element, Direction::Upstream)
    }
}
