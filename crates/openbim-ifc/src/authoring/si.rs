//! Reading a batch's numbers in SI (#638).
//!
//! [`SiInput`] runs between handle resolution and planning: it converts the
//! numbers an operation states into the unit that applies, and refuses an
//! operation that would change the units it converts into. The unit that
//! applies is resolved by `ifc-properties` against the model as it was
//! before the batch, so a batch cannot create the project or its units and
//! read numbers in SI at once; a raw batch creates those first.
//!
//! Only numbers the caller states are converted. Values the planner derives
//! or copies out of the model (a copied product, a propagated type) are
//! already in the model's units.

mod guard;

use std::collections::HashMap;
use std::sync::Arc;

use ifc_model::{EntityId, Model};
use ifc_properties::{si_conversion, SchemaVersion, UnitConversion, UnitConversionError};
use ifc_schema::Schema;

use super::si_refusal::SiInputRefusal;
use super::{AuthorOp, AuthoringFailure as F, CopyPlacement, Overlay};
use crate::unit_rules::classify::Classifier;
use crate::unit_rules::space::attribute;

/// The point whose coordinates a placement location and an offset state.
const POINT: &str = "IFCCARTESIANPOINT";
const LENGTH: &str = "IFCLENGTHMEASURE";

/// A batch's SI conversion: the release's tables, and the conversions
/// resolved so far against the base model.
pub(super) struct SiInput<'m> {
    model: &'m Model,
    classifier: Classifier,
    conversions: HashMap<(Arc<str>, Option<EntityId>), UnitConversion>,
}

impl<'m> SiInput<'m> {
    /// Check the batch can be read in SI against `model`.
    ///
    /// # Errors
    ///
    /// [`SiInputRefusal::UnsupportedRelease`], [`SiInputRefusal::ReleaseMismatch`],
    /// [`SiInputRefusal::ContextUnits`] or [`SiInputRefusal::ParameterSpaceCurve`].
    pub(super) fn new(model: &'m Model, schema: &Schema) -> Result<Self, F> {
        let refuse = |reason| F::SiInput {
            entity_type: None,
            attribute: None,
            reason,
        };
        let unsupported = || {
            refuse(SiInputRefusal::UnsupportedRelease {
                schema: schema.name().to_owned(),
            })
        };
        let version = schema
            .version()
            .filter(|version| {
                matches!(
                    version,
                    SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3
                )
            })
            .ok_or_else(unsupported)?;
        let declared = &model.header().schema;
        if !matches!(declared.as_slice(), [token] if SchemaVersion::from_header_token(token) == Some(version))
        {
            return Err(refuse(SiInputRefusal::ReleaseMismatch {
                declared: declared.join(", "),
                schema: schema.name().to_owned(),
            }));
        }
        let tables = ifc_schema::for_version(version).map_err(|_| unsupported())?;
        for library in model.ids_of_type("IFCPROJECTLIBRARY") {
            if attribute(tables, model, *library, "UnitsInContext").is_some_and(|value| {
                !matches!(value, ifc_model::Value::Null | ifc_model::Value::Derived)
            }) {
                return Err(refuse(SiInputRefusal::ContextUnits { context: *library }));
            }
        }
        // A p-curve's reference curve states `(u, v)` as lengths, so on a
        // surface that is not length-parameterised in both directions an
        // edit of its points cannot be converted by what they are: the batch
        // is refused, as `convert_length_unit` refuses the model.
        for pcurve in model.ids_of_type("IFCPCURVE") {
            let found = crate::unit_rules::space::pcurve_surface(tables, model, *pcurve);
            if let Err((surface, surface_type)) = found {
                return Err(F::SiInput {
                    entity_type: Some("IFCPCURVE".to_owned()),
                    attribute: Some("BasisSurface".to_owned()),
                    reason: SiInputRefusal::ParameterSpaceCurve {
                        surface,
                        surface_type: surface_type.to_string(),
                    },
                });
            }
        }
        Ok(Self {
            model,
            classifier: Classifier::new(version, tables),
            conversions: HashMap::new(),
        })
    }

    /// `op` with the numbers it states converted from SI.
    ///
    /// # Errors
    ///
    /// [`F::SiInput`] for an operation that changes the units, and
    /// [`F::UnitConversion`] for a number that cannot be converted.
    pub(super) fn convert(&mut self, view: &Overlay<'_>, mut op: AuthorOp) -> Result<AuthorOp, F> {
        self.guard(view, &op)?;
        match &mut op {
            AuthorOp::Placement { location, .. } => {
                for coordinate in location.iter_mut() {
                    *coordinate = self.length(*coordinate)?;
                }
            }
            AuthorOp::CopyProduct { options, .. } => {
                if let CopyPlacement::Offset(offset) = &mut options.placement {
                    for coordinate in offset.iter_mut() {
                        *coordinate = self.length(*coordinate)?;
                    }
                }
            }
            _ => {}
        }
        Ok(op)
    }

    /// A length of a point's coordinates, from SI into the project length
    /// unit.
    fn length(&mut self, value: f64) -> Result<f64, F> {
        self.convert_number(value, LENGTH, None, POINT, "Coordinates")
    }

    /// `value` in SI as the unit that applies to `measure` (with `unit` the
    /// explicit one, if stated) has it.
    fn convert_number(
        &mut self,
        value: f64,
        measure: &str,
        unit: Option<EntityId>,
        entity_type: &str,
        attribute: &str,
    ) -> Result<f64, F> {
        let failed = |error| F::UnitConversion {
            entity_type: entity_type.to_owned(),
            attribute: attribute.to_owned(),
            measure: measure.to_owned(),
            error: Box::new(error),
        };
        if !value.is_finite() {
            return Err(failed(UnitConversionError::NonFinite { value }));
        }
        let key = (Arc::<str>::from(measure.to_ascii_uppercase()), unit);
        let conversion = match self.conversions.get(&key) {
            Some(conversion) => *conversion,
            None => {
                let conversion = si_conversion(self.model, measure, unit).map_err(failed)?;
                self.conversions.insert(key, conversion);
                conversion
            }
        };
        Ok(if conversion.is_identity() {
            value
        } else {
            conversion.apply(value)
        })
    }
}
