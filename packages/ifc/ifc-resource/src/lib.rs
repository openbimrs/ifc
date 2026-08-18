//! `ifc-resource` — construction resources.
//!
//! # Why this is its own crate
//!
//! IFC4 has **21 resource entities**. Resources are the third leg of
//! construction planning: `ifc-schedule` says *when*, `ifc-cost` says *how
//! much*, and this says *with what* — the crews, plant, and materials a task
//! consumes. 4D/5D tools need all three, but plenty of consumers need only one.
//!
//! # Scope
//!
//! - Labour, equipment, material, product, crew, subcontract resources
//! - Resource types and their quantities
//! - Assignment of resources to processes
//! - Resource time (work, usage, utilisation)
