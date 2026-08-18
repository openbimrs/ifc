//! `ifc-systems` — distribution systems, ports, and connectivity.
//!
//! # Why this is its own crate
//!
//! IFC4 carries **23 port/system entities**. MEP models are graphs, not just
//! collections of solids: a duct network's meaning is in which port connects to
//! which, and that connectivity supports flow analysis, system-completeness
//! checking, and "what is downstream of this valve?" queries that no amount of
//! geometry answers.
//!
//! # Scope
//!
//! - `IfcDistributionPort`, port nesting, `IfcRelConnectsPorts`
//! - Systems, building systems, and (4x3) built systems; group assignment
//! - Element connectivity and the derived network graph
//! - Flow direction (source/sink/both)
//!
//! # Why the graph is worth building explicitly
//!
//! Connectivity in the file is a set of relationship objects; answering
//! reachability questions over it requires an actual adjacency structure. This
//! crate owns that derivation so consumers do not each rebuild it.
