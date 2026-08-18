//! `ifc-material` — material definitions and their association to elements.
//!
//! # Why this is its own crate
//!
//! IFC4 has **22 material entities**. Materials are not a cosmetic concern:
//! `IfcMaterialLayerSetUsage` defines the *actual layered build-up* of a wall
//! (its thicknesses, their order, and which side the reference line sits on),
//! which is what a thermal calculation, a quantity takeoff, and a correct
//! wall-axis geometry all read.
//!
//! # Scope
//!
//! - `IfcMaterial`, material lists, constituent sets
//! - Layer sets + `IfcMaterialLayerSetUsage` (offsets, priorities, direction)
//! - Profile sets for extruded members
//! - Material properties, and association to elements and types
//!
//! # The subtlety worth knowing
//!
//! Layer usage carries a *reference direction and offset*: the geometric axis
//! of a wall is generally not its centreline. Getting this wrong shifts every
//! layer boundary while the wall still looks correct at a glance.
