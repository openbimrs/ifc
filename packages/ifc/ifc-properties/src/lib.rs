//! `ifc-properties` — property sets, quantities, and units.
//!
//! # Why this is a separate crate from `ifc-model`
//!
//! Properties are where most real IFC work happens — COBie handover, LOD
//! audits, quantity takeoff, classification checks — and **none of it needs
//! geometry**. Keeping it separate means a consumer doing a property audit
//! compiles no mesh code at all. That is a concrete advantage over
//! IfcOpenShell, where the geometry engine is not optional in practice.
//!
//! # Scope
//!
//! - `IfcPropertySet` / `IfcElementQuantity` lookup by element.
//! - Property *inheritance*: an occurrence inherits from its type
//!   (`IfcRelDefinesByType`), and the occurrence value wins. Getting this
//!   precedence wrong silently reports the wrong value, so it is tested
//!   directly rather than assumed.
//! - Unit resolution against `IfcUnitAssignment`, including prefixed and
//!   derived units. A quantity without its unit is a number, not a fact.
//!
//! # Status
//!
//! Reserved. See `docs/ROADMAP.md` Stage 3.
