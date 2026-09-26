//! Source-order-preserving projection of the `IfcMaterialConstituentSet` SET,
//! and the opt-in fraction policy check.

use ifc_model::EntityId;

use crate::constituent::MaterialConstituent;
use crate::view::{borrowed_entity, optional_refs, optional_text, MaterialView};
use crate::{MaterialError, MaterialResult};

borrowed_entity!(MaterialConstituentSet, "IFCMATERIALCONSTITUENTSET");

impl<'m> MaterialConstituentSet<'m> {
    /// `IfcMaterialConstituentSet.Name`, if given.
    pub fn name(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALCONSTITUENTSET",
            self.id(),
            self.entity(),
            0,
            "Name",
        )
    }

    /// `IfcMaterialConstituentSet.Description`, if given.
    pub fn description(self) -> MaterialResult<Option<&'m str>> {
        optional_text(
            "IFCMATERIALCONSTITUENTSET",
            self.id(),
            self.entity(),
            1,
            "Description",
        )
    }

    /// `IfcMaterialConstituentSet.MaterialConstituents`, if the set has any
    /// members. `None` when the optional attribute slot is absent, distinct
    /// from an authored-but-empty list (which `optional_refs` rejects).
    pub fn constituent_ids(self) -> MaterialResult<Option<Vec<EntityId>>> {
        optional_refs(
            "IFCMATERIALCONSTITUENTSET",
            self.id(),
            self.entity(),
            2,
            "MaterialConstituents",
            1,
        )
    }
}

impl<'m> MaterialView<'m> {
    /// Iterates every `IfcMaterialConstituentSet` instance in the model.
    pub fn constituent_sets(self) -> impl Iterator<Item = MaterialConstituentSet<'m>> + 'm {
        self.model()
            .of_type("IFCMATERIALCONSTITUENTSET")
            .map(|(id, entity)| MaterialConstituentSet::from_known(id, entity))
    }
}

/// A constituent set whose stated fractions do not describe one whole.
///
/// A policy finding, not a schema violation: IFC4 ADD2 TC1 and IFC4X3 ADD2
/// declare no WHERE rule on `IfcMaterialConstituentSet`, and
/// `IfcMaterialConstituent.Fraction` is `OPTIONAL`, so a file producing this
/// diagnostic is valid. Only
/// [`MaterialView::constituent_fraction_diagnostic`] produces it, when a
/// caller asks; no accessor fails or normalises because of it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ConstituentFractionDiagnostic {
    /// Every constituent states a fraction, and their sum differs from 1 by
    /// more than the caller's tolerance.
    SumNotOne {
        /// The `IfcMaterialConstituentSet`.
        set: EntityId,
        /// The sum of the stated fractions.
        sum: f64,
        /// The tolerance the sum was checked against.
        tolerance: f64,
    },
    /// Some constituents state a fraction and others do not, so the set
    /// does not say how the whole divides.
    PartiallyStated {
        /// The `IfcMaterialConstituentSet`.
        set: EntityId,
        /// Constituents stating a fraction, in set order.
        stated: Vec<EntityId>,
        /// Constituents stating none, in set order.
        missing: Vec<EntityId>,
        /// The sum of the fractions that are stated.
        stated_sum: f64,
    },
}

impl<'m> MaterialView<'m> {
    /// Opt-in policy check that a constituent set's fractions make one whole.
    ///
    /// Returns `Ok(None)` when every constituent states a fraction and the
    /// sum is within `tolerance` of 1, when no constituent states one, and
    /// when the set lists no constituents. A sum outside the tolerance is
    /// [`ConstituentFractionDiagnostic::SumNotOne`]; a mix of stated and
    /// missing fractions is
    /// [`ConstituentFractionDiagnostic::PartiallyStated`]. The file is never
    /// corrected: fractions are reported as authored.
    ///
    /// A negative or NaN `tolerance` admits no sum, so every fully stated
    /// set is reported.
    ///
    /// # Errors
    ///
    /// The decode errors of the underlying accessors: a malformed member
    /// list, a member id absent from the model or not an
    /// `IfcMaterialConstituent`, or a fraction outside `0..=1`. The
    /// diagnostic itself is never an error.
    pub fn constituent_fraction_diagnostic(
        self,
        set: MaterialConstituentSet<'m>,
        tolerance: f64,
    ) -> MaterialResult<Option<ConstituentFractionDiagnostic>> {
        let Some(ids) = set.constituent_ids()? else {
            return Ok(None);
        };
        let mut stated = Vec::new();
        let mut missing = Vec::new();
        let mut sum = 0.0;
        for id in ids {
            let entity = self.entity(set.id(), id)?;
            if !entity.is_type("IFCMATERIALCONSTITUENT") {
                return Err(MaterialError::ReferenceType {
                    source_id: set.id(),
                    target: id,
                    expected: "IFCMATERIALCONSTITUENT",
                    actual: entity.type_name.to_string(),
                });
            }
            match MaterialConstituent::from_known(id, entity).fraction()? {
                Some(fraction) => {
                    stated.push(id);
                    sum += fraction;
                }
                None => missing.push(id),
            }
        }
        if stated.is_empty() {
            return Ok(None);
        }
        if !missing.is_empty() {
            return Ok(Some(ConstituentFractionDiagnostic::PartiallyStated {
                set: set.id(),
                stated,
                missing,
                stated_sum: sum,
            }));
        }
        // A NaN tolerance fails this comparison, so it reports.
        if (sum - 1.0_f64).abs() <= tolerance {
            return Ok(None);
        }
        Ok(Some(ConstituentFractionDiagnostic::SumNotOne {
            set: set.id(),
            sum,
            tolerance,
        }))
    }
}
