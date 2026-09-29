//! Public coordinate-reference-system values.
//!
//! A CRS is read as the file states it: authority, name, datum and map unit.
//! Nothing here looks a CRS up over the network or binds a reprojection
//! library; an application that needs EPSG parameters resolves the returned
//! identifier itself.

mod projected;
mod unit;

pub use projected::ProjectedCrs;
pub use unit::LengthUnit;

pub(crate) use projected::projected_crs;
