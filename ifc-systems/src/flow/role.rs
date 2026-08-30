//! Element flow roles, and consistency between a role and its ports.
//!
//! IFC states an element's kind (`IfcFlowSegment`, `IfcFlowTerminal`, ...) and
//! its ports' directions independently. Nothing forces them to agree, so a
//! file can say "terminal" while wiring it as a through-segment. Those
//! disagreements are reported, never corrected: the file is the record, and
//! silently repairing it would hide an authoring fault.

use ifc_model::{EntityId, Model};
use ifc_schema::ifc4;

use crate::port::Port;

/// What kind of flow element this is, by schema ancestry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementRole {
    /// `IfcFlowSegment` -- pipe, duct, cable. Carries flow between two points.
    Segment,
    /// `IfcFlowFitting` -- elbow, tee, reducer. Joins segments.
    Fitting,
    /// `IfcFlowTerminal` -- radiator, diffuser, outlet. An endpoint.
    Terminal,
    /// `IfcFlowController` -- valve, damper, switch.
    Controller,
    /// `IfcFlowMovingDevice` -- pump, fan.
    MovingDevice,
    /// `IfcFlowStorageDevice` -- tank, cylinder.
    StorageDevice,
    /// `IfcFlowTreatmentDevice` -- filter, interceptor.
    TreatmentDevice,
    /// `IfcEnergyConversionDevice` -- boiler, chiller, heat exchanger.
    EnergyConversionDevice,
    /// A distribution element with no more specific flow role.
    Other,
}

impl ElementRole {
    /// Classify an element by walking its supertype chain.
    ///
    /// Ancestry, not exact type: a file states `IfcPipeSegment`, never
    /// `IfcFlowSegment`, because the concrete subtypes are what exporters
    /// write. An exact-type match finds nothing in a real file.
    pub fn of(model: &Model, element: EntityId) -> Option<Self> {
        let entity = model.get(element)?;
        let name = entity.type_name.to_ascii_uppercase();
        let schema = ifc4();
        // Order matters: the first match wins, so the most specific roles are
        // tested before the catch-all distribution element.
        for (ancestor, role) in [
            ("IFCFLOWSEGMENT", Self::Segment),
            ("IFCFLOWFITTING", Self::Fitting),
            ("IFCFLOWTERMINAL", Self::Terminal),
            ("IFCFLOWCONTROLLER", Self::Controller),
            ("IFCFLOWMOVINGDEVICE", Self::MovingDevice),
            ("IFCFLOWSTORAGEDEVICE", Self::StorageDevice),
            ("IFCFLOWTREATMENTDEVICE", Self::TreatmentDevice),
            ("IFCENERGYCONVERSIONDEVICE", Self::EnergyConversionDevice),
        ] {
            if schema.is_a(&name, ancestor) {
                return Some(role);
            }
        }
        if schema.is_a(&name, "IFCDISTRIBUTIONELEMENT") {
            return Some(Self::Other);
        }
        None
    }

    /// Whether this role is expected to terminate a run rather than pass through.
    ///
    /// Advisory only. A terminal with two ports is unusual, not illegal, and
    /// this is used to describe a file rather than to reject one.
    pub fn is_endpoint(self) -> bool {
        matches!(self, Self::Terminal)
    }
}

/// A disagreement between an element's role and how its ports are directed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoleInconsistency {
    /// An element that should pass flow has no way in, or no way out.
    ///
    /// A segment with two SOURCE ports cannot receive anything: whatever the
    /// exporter intended, nothing can flow through it as stated.
    NoPath {
        /// The element.
        element: EntityId,
        /// Its role.
        role: ElementRole,
        /// Whether any port accepts flow.
        has_inlet: bool,
        /// Whether any port emits flow.
        has_outlet: bool,
    },
    /// An element carries ports whose direction the file never stated.
    ///
    /// Not an error: `FlowDirection` is OPTIONAL. It is reported because an
    /// undirected port cannot orient an edge, so downstream queries will stop
    /// at it and a caller deserves to know why.
    UndirectedPorts {
        /// The element.
        element: EntityId,
        /// How many of its ports have no stated direction.
        count: usize,
    },
}

/// Check every element that owns ports for role/direction disagreement.
///
/// Ports are grouped by their owning element, so an element whose ports were
/// never attached is not reported: absent data is not a contradiction.
pub fn role_inconsistencies(model: &Model, ports: &[Port]) -> Vec<RoleInconsistency> {
    let mut by_element: std::collections::BTreeMap<EntityId, Vec<&Port>> =
        std::collections::BTreeMap::new();
    for port in ports {
        if let Some(element) = port.element {
            by_element.entry(element).or_default().push(port);
        }
    }

    let mut out = Vec::new();
    for (element, owned) in by_element {
        let Some(role) = ElementRole::of(model, element) else {
            continue;
        };

        let undirected = owned.iter().filter(|p| !p.flow.is_stated()).count();
        if undirected > 0 {
            out.push(RoleInconsistency::UndirectedPorts {
                element,
                count: undirected,
            });
        }

        // A terminal is an endpoint by definition, so a single-direction
        // terminal is correct and must not be reported.
        if role.is_endpoint() || owned.len() < 2 {
            continue;
        }
        let has_inlet = owned.iter().any(|p| p.flow.accepts());
        let has_outlet = owned.iter().any(|p| p.flow.emits());
        // Only report when the file DID state directions: an entirely
        // undirected element is already covered above, and reporting it twice
        // would make the undirected case look like a contradiction.
        let any_stated = owned.iter().any(|p| p.flow.is_stated());
        if any_stated && !(has_inlet && has_outlet) {
            out.push(RoleInconsistency::NoPath {
                element,
                role,
                has_inlet,
                has_outlet,
            });
        }
    }
    out
}
