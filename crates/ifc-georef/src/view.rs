//! Pins the model's declared schema to IFC4 or IFC4X3, and names the
//! coordinate-operation entities each schema version actually declares.
//!
//! # Why this crate does not need per-attribute version dispatch
//!
//! `IfcCoordinateReferenceSystem`/`IfcProjectedCRS`'s attribute *counts*
//! differ between IFC4 and IFC4X3 (IFC4X3 drops `VerticalDatum` from the
//! base `IfcCoordinateReferenceSystem` and re-adds it one level down on
//! `IfcProjectedCRS`), but the two changes cancel out: the absolute
//! attribute slots `crs::projected` reads (`Name`=0, `Description`=1,
//! `GeodeticDatum`=2, `VerticalDatum`=3, `MapProjection`=4, `MapZone`=5,
//! `MapUnit`=6) are identical in both schemas. `IfcMapConversion`'s own six
//! attributes are unchanged too. So `resolve_project_to_map` and
//! `projected_crs` need no version branch for slots -- confirmed against
//! `IFC4.exp`/`IFC4X3_ADD2.exp` directly, and asserted against both bundled
//! schema tables by `crate::slot`'s tests.
//!
//! The one rule that does differ is `Name`: mandatory in IFC4, `OPTIONAL`
//! in IFC4X3 under `NameOrWKT : (HIINDEX(WellKnownText) = 1) OR
//! EXISTS(Name)`. `projected_crs` reads the header's release for that alone
//! (#142): under IFC4X3 an unnamed CRS reads when exactly one
//! `IfcWellKnownText` defines it; anything else requires the name.
//!
//! # What actually differs
//!
//! - **IFC2X3** declares no georeferencing entities at all (`IfcMapConversion`,
//!   `IfcProjectedCRS`, `IfcCoordinateReferenceSystem` do not exist in
//!   `IFC2X3_TC1.exp`). There is no reduced IFC2X3 path to offer.
//! - **IFC4** declares exactly one coordinate-operation shape:
//!   `IfcMapConversion` targeting `IfcProjectedCRS` (the only
//!   `IfcCoordinateReferenceSystem` subtype IFC4 has).
//! - **IFC4X3** adds three shapes IFC4 cannot express at all:
//!   `IfcMapConversionScaled` (per-axis scale factors), `IfcRigidOperation`
//!   (a pure offset with no scale/rotation, in length or plane-angle
//!   coordinates), and `IfcGeographicCRS` (a second
//!   `IfcCoordinateReferenceSystem` subtype, for geodetic rather than
//!   projected targets). The first two lower to a project-to-map transform
//!   (a rigid operation only with length coordinates); a plane-angle rigid
//!   operation onto an `IfcGeographicCRS` is read, not lowered, by
//!   [`crate::resolve_geographic_offset_in`]. Under an IFC4 header all
//!   three are refused with [`GeorefError::UnsupportedOperation`] naming
//!   the release that does not declare them.

use ifc_model::{EntityId, Model};
use ifc_schema::{ifc4, ifc4x3, Schema, SchemaVersion};

use crate::error::{GeorefError, GeorefResult};

/// A model whose declared schema has been pinned to IFC4 or IFC4X3.
#[derive(Debug, Clone, Copy)]
pub struct GeorefView<'m> {
    pub(crate) model: &'m Model,
    pub(crate) schema: &'static Schema,
    pub(crate) version: SchemaVersion,
}

impl<'m> GeorefView<'m> {
    /// Pin the model's declared schema and confirm it is IFC4 or IFC4X3.
    ///
    /// IFC2X3 is refused: it declares no `IfcMapConversion`,
    /// `IfcProjectedCRS`, or `IfcCoordinateReferenceSystem` at all, so there
    /// is no reduced behavior to offer instead of a typed refusal.
    pub fn for_model(model: &'m Model) -> GeorefResult<Self> {
        let token = match model.header().schema.as_slice() {
            [] => return Err(GeorefError::MissingSchema),
            [token] => token,
            tokens => {
                return Err(GeorefError::AmbiguousSchema {
                    tokens: tokens.to_vec(),
                });
            }
        };
        let version = SchemaVersion::from_header_token(token).ok_or_else(|| {
            GeorefError::UnsupportedSchema {
                token: token.clone(),
            }
        })?;
        let schema = match version {
            SchemaVersion::Ifc4 => ifc4(),
            SchemaVersion::Ifc4x3 => ifc4x3(),
            // IFC2X3 predates these layouts; IFC4X1 and IFC4X2 are bundled
            // by ifc-schema but not verified here. Refused, never aliased.
            _ => {
                return Err(GeorefError::UnsupportedSchema {
                    token: token.clone(),
                });
            }
            // A release added to `SchemaVersion` later has no reviewed
            // georeferencing layout: refused rather than read with another's.
            #[allow(unreachable_patterns)]
            _ => {
                return Err(GeorefError::UnsupportedSchema {
                    token: token.clone(),
                });
            }
        };
        Ok(Self {
            model,
            schema,
            version,
        })
    }

    /// The pinned schema version.
    #[must_use]
    pub const fn version(&self) -> SchemaVersion {
        self.version
    }

    /// The pinned schema, for subtype-aware (`is_a`) queries.
    #[must_use]
    pub const fn schema(&self) -> &'static Schema {
        self.schema
    }

    /// Confirm `id` is declared in the pinned schema at all, distinguishing
    /// a version-specific entity (declared in IFC4X3, not IFC4) from a
    /// genuinely unknown or malformed type name.
    pub(crate) fn require_known_type(&self, id: EntityId) -> GeorefResult<&'m str> {
        let entity = self.model.get(id).ok_or(GeorefError::MissingEntity {
            referrer: id,
            missing: id,
        })?;
        let type_name = entity.type_name.as_ref();
        if self.schema.entity(type_name).is_none() {
            return Err(GeorefError::UnsupportedOperation {
                entity: id,
                actual: format!(
                    "{type_name} (not declared in {})",
                    self.version.release_id()
                ),
            });
        }
        Ok(type_name)
    }
}

#[cfg(test)]
mod tests {
    use ifc_model::Entity;

    use super::*;

    fn model_with_schema(token: &str) -> Model {
        let mut model = Model::new();
        model.header_mut().schema = vec![token.to_owned()];
        model
    }

    #[test]
    fn accepts_ifc4() {
        let model = model_with_schema("IFC4");
        let view = GeorefView::for_model(&model).expect("IFC4 is accepted");
        assert_eq!(view.version(), SchemaVersion::Ifc4);
    }

    #[test]
    fn accepts_ifc4x3_and_its_add2_variant() {
        for token in ["IFC4X3", "IFC4X3_ADD2"] {
            let model = model_with_schema(token);
            let view = GeorefView::for_model(&model).expect("IFC4X3 is accepted");
            assert_eq!(view.version(), SchemaVersion::Ifc4x3);
        }
    }

    #[test]
    fn refuses_ifc2x3_because_it_declares_no_georeferencing_entities() {
        let model = model_with_schema("IFC2X3");
        assert!(matches!(
            GeorefView::for_model(&model),
            Err(GeorefError::UnsupportedSchema { token }) if token == "IFC2X3"
        ));
    }

    #[test]
    fn refuses_an_unrecognized_schema_token() {
        let model = model_with_schema("IFC5");
        assert!(matches!(
            GeorefView::for_model(&model),
            Err(GeorefError::UnsupportedSchema { token }) if token == "IFC5"
        ));
    }

    #[test]
    fn refuses_a_missing_schema_declaration() {
        let model = Model::new();
        assert!(matches!(
            GeorefView::for_model(&model),
            Err(GeorefError::MissingSchema)
        ));
    }

    #[test]
    fn refuses_an_ambiguous_schema_declaration() {
        let mut model = Model::new();
        model.header_mut().schema = vec!["IFC4".to_owned(), "IFC4X3".to_owned()];
        assert!(matches!(
            GeorefView::for_model(&model),
            Err(GeorefError::AmbiguousSchema { tokens })
                if tokens == vec!["IFC4".to_owned(), "IFC4X3".to_owned()]
        ));
    }

    #[test]
    fn names_an_ifc4x3_only_coordinate_operation_when_read_under_ifc4() {
        let mut model = model_with_schema("IFC4");
        model.insert(EntityId(1), Entity::new("IFCGEOGRAPHICCRS", vec![]));
        let view = GeorefView::for_model(&model).expect("IFC4 is accepted");
        assert!(matches!(
            view.require_known_type(EntityId(1)),
            Err(GeorefError::UnsupportedOperation { entity, actual })
                if entity == EntityId(1) && actual.contains("IFCGEOGRAPHICCRS") && actual.contains("IFC4_ADD2_TC1")
        ));
    }

    #[test]
    fn recognizes_an_ifc4x3_only_coordinate_operation_when_read_under_ifc4x3() {
        let mut model = model_with_schema("IFC4X3");
        model.insert(EntityId(1), Entity::new("IFCGEOGRAPHICCRS", vec![]));
        let view = GeorefView::for_model(&model).expect("IFC4X3 is accepted");
        assert_eq!(
            view.require_known_type(EntityId(1))
                .expect("declared in IFC4X3"),
            "IFCGEOGRAPHICCRS"
        );
    }
}

#[cfg(test)]
mod intermediate_release_tests {
    use super::*;

    /// IFC4X1 and IFC4X2 have bundled tables but no verified layout here:
    /// refused with the unsupported-schema error, never read as IFC4/IFC4X3.
    #[test]
    fn ifc4x1_and_ifc4x2_are_refused_not_aliased() {
        for token in ["IFC4X1", "IFC4X2"] {
            let mut model = Model::new();
            model.header_mut().schema = vec![token.to_owned()];
            assert!(
                matches!(
                    GeorefView::for_model(&model),
                    Err(GeorefError::UnsupportedSchema { token: found }) if found == token
                ),
                "{token} must be refused"
            );
        }
    }
}
