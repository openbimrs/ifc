//! `IfcRelConnectsPorts` and network traversal.
//!
//! The graph walk that answers 'what is downstream of this valve'. Cycles are
//! legal here (ring mains), so traversal must handle them by design.
//!
//! Not yet implemented -- see `docs/ROADMAP.md`.
