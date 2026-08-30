//! `IfcFlowDirectionEnum` and what it licenses.
//!
//! The enum is a port attribute, but its MEANING is a flow concern: it decides
//! which way an edge may be walked. Keeping the semantics here rather than on
//! the port keeps "what the file says" separate from "what that implies".

use ifc_model::Value;

/// Which way material or energy moves through a port.
///
/// A missing direction is [`FlowDirection::NotDefined`] rather than an assumed
/// default. Guessing `Source` would invent a direction the file never stated
/// and orient edges that have no stated orientation, which is worse than
/// admitting the network is partly undirected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    /// Read the enum from an attribute slot.
    pub(crate) fn parse(value: Option<&Value>) -> Self {
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

    /// Whether material may LEAVE the element through this port.
    ///
    /// `NotDefined` is false in both directions. That is deliberate: an
    /// undirected port cannot orient an edge, and pretending it can would
    /// manufacture flow paths a reviewer cannot trace back to the file.
    pub fn emits(self) -> bool {
        matches!(self, Self::Source | Self::SourceAndSink)
    }

    /// Whether material may ENTER the element through this port.
    pub fn accepts(self) -> bool {
        matches!(self, Self::Sink | Self::SourceAndSink)
    }

    /// Whether the file stated a direction at all.
    pub fn is_stated(self) -> bool {
        !matches!(self, Self::NotDefined)
    }
}
