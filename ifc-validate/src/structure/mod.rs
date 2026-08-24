//! Structure validation capability.

//! ## Internal split
//!
//! - `reference.rs`: dangling and wrong-kind reference checks.
//! - `cardinality.rs`: aggregate cardinality checks.
//! - `required.rs`: required, optional, and derived slot-state checks.
//! - `unique.rs`: UNIQUE and global identity checks.

mod cardinality;
mod reference;
mod required;
mod unique;
