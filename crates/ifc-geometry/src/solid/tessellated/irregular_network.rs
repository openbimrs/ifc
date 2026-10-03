//! `IfcTriangulatedIrregularNetwork`: an IFC4X3 terrain mesh.
//!
//! The entity is a subtype of `IfcTriangulatedFaceSet` that adds exactly one
//! attribute, `Flags : LIST [1:?] OF IfcInteger`, and the `NotClosed` rule
//! (`Closed = FALSE`). Every inherited slot reads as on the supertype, so
//! [`TriangulatedIrregularNetwork::face_set`] hands out the supertype's view.
//!
//! # What the flags mean
//!
//! The EXPRESS source types `Flags` only as integers. Their meaning comes
//! from the IFC 4.3 documentation (buildingSMART `IFC4.3.x-development`,
//! `IfcTriangulatedIrregularNetwork.md` at `fb8f62fa`), one value per triangle:
//! `-2` invisible void, `-1` invisible hole, `0` no breaklines, and `1` to `7`
//! a bit set of breaklines on edges 1, 2 and 3. A void excludes the triangle;
//! a hole excludes it but lets another surface fill the gap.
//!
//! A breakline marks an edge the triangulation already has, so the triangle
//! surface is the same with or without it. An excluded triangle is not: the
//! surface has a gap there. [`TriangulatedIrregularNetwork::admit`] therefore
//! admits a network whose flags are all breakline codes and refuses one with
//! a void, a hole, or an undocumented value.

use crate::error::GeometryResult;
use crate::slots::Slots;
use ifc_model::{Entity, EntityId};

use super::{int_list, TriangulatedFaceSet};

/// The STEP type name.
pub const TYPE: &str = "IFCTRIANGULATEDIRREGULARNETWORK";

/// `IfcTriangulatedIrregularNetwork` attribute slots.
///
/// IFC4X3_ADD2: the five `IfcTriangulatedFaceSet` slots, then `Flags`.
pub(crate) mod slot {
    /// `Flags : LIST [1:?] OF IfcInteger`, absolute slot 5.
    pub const FLAGS: usize = 5;
}

/// Refusal for a void or hole triangle.
pub const EXCLUDED_TRIANGLE: &str = "a Flags value is -1 (hole) or -2 (void); the triangle is \
     excluded from the surface, and the neutral mesh has no face-exclusion or fall-back channel";

/// Refusal for a flag outside the documented codes.
pub const UNDOCUMENTED_FLAG: &str =
    "a Flags value is outside the documented codes -2 to 7, so its meaning is unknown";

/// `IfcTriangulatedIrregularNetwork`: a triangulated face set with per-triangle flags.
#[derive(Debug, Clone, Copy)]
pub struct TriangulatedIrregularNetwork<'m> {
    slots: Slots<'m>,
}

impl<'m> TriangulatedIrregularNetwork<'m> {
    /// Wrap an entity assumed to be an `IfcTriangulatedIrregularNetwork`.
    pub fn new(id: EntityId, entity: &'m Entity) -> Self {
        Self {
            slots: Slots::new(id, entity),
        }
    }

    /// The entity id.
    pub fn id(&self) -> EntityId {
        self.slots.id()
    }

    /// The inherited `IfcTriangulatedFaceSet` attributes.
    pub fn face_set(&self) -> TriangulatedFaceSet<'m> {
        TriangulatedFaceSet::new(self.slots.id(), self.slots.entity())
    }

    /// `Flags` exactly as written, one value per triangle.
    pub fn flags(&self) -> GeometryResult<Vec<i64>> {
        let value = self.slots.req(slot::FLAGS, "Flags")?;
        int_list(value.unwrap_typed())
            .ok_or_else(|| self.slots.degenerate("Flags is not a list of integers"))
    }

    /// Admit the network for lowering as its triangle surface, or refuse it.
    ///
    /// Breakline codes (`0` to `7`) are admitted: they mark existing edges and
    /// leave the surface unchanged. Voids and holes are refused because the
    /// surface excludes those triangles. Undocumented codes are refused.
    ///
    /// # Errors
    ///
    /// `Degenerate` when `Flags` does not hold one value per triangle;
    /// `Unsupported` for a void, a hole or an undocumented code.
    pub fn admit(&self, triangles: usize) -> GeometryResult<()> {
        let flags = self.flags()?;
        if flags.len() != triangles {
            return Err(self.slots.degenerate(format!(
                "Flags has {} values but CoordIndex declares {triangles} triangles",
                flags.len()
            )));
        }
        if flags.iter().any(|flag| !(-2..=7).contains(flag)) {
            return Err(self.slots.unsupported(UNDOCUMENTED_FLAG));
        }
        if flags.iter().any(|flag| *flag < 0) {
            return Err(self.slots.unsupported(EXCLUDED_TRIANGLE));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solid::testkit::{entity, int_grid as grid, ints, r};
    use crate::GeometryError;
    use ifc_model::Value;

    fn network(flags: &[i64]) -> Entity {
        entity(
            TYPE,
            vec![
                r(50),
                Value::Null,
                Value::Bool(false),
                grid(&[&[1, 2, 3], &[1, 3, 4]]),
                Value::Null,
                ints(flags),
            ],
        )
    }

    #[test]
    fn inherited_slots_read_through_the_face_set_view() {
        let e = network(&[0, 0]);
        let view = TriangulatedIrregularNetwork::new(EntityId(1), &e);
        assert_eq!(view.face_set().coordinates().unwrap(), EntityId(50));
        assert_eq!(view.face_set().closed(), Some(false));
        assert_eq!(view.flags().unwrap(), vec![0, 0]);
    }

    #[test]
    fn breakline_codes_are_admitted() {
        for flags in [[0, 0], [1, 7], [3, 4]] {
            let e = network(&flags);
            TriangulatedIrregularNetwork::new(EntityId(1), &e)
                .admit(2)
                .unwrap_or_else(|error| panic!("{flags:?} must be admitted: {error}"));
        }
    }

    #[test]
    fn voids_holes_and_unknown_codes_are_typed_refusals() {
        for (flags, detail) in [
            ([0, -1], EXCLUDED_TRIANGLE),
            ([-2, 0], EXCLUDED_TRIANGLE),
            ([8, 0], UNDOCUMENTED_FLAG),
            ([0, -3], UNDOCUMENTED_FLAG),
        ] {
            let e = network(&flags);
            let error = TriangulatedIrregularNetwork::new(EntityId(4), &e)
                .admit(2)
                .expect_err("must refuse");
            assert!(
                matches!(&error, GeometryError::Unsupported { detail: d, .. } if *d == detail),
                "{flags:?}: {error}"
            );
        }
    }

    #[test]
    fn a_flag_count_that_disagrees_with_the_triangles_is_degenerate() {
        let e = network(&[0]);
        let error = TriangulatedIrregularNetwork::new(EntityId(4), &e)
            .admit(2)
            .expect_err("one flag for two triangles");
        assert!(matches!(error, GeometryError::Degenerate { .. }), "{error}");
    }
}
