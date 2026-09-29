//! Absolute attribute slots the georeferencing readers index.
//!
//! Every position was read from `IFC4.exp` (IFC4 ADD2 TC1) and
//! `IFC4X3_ADD2.exp` and is asserted below against the bundled tables
//! (`ifc_schema::ifc4()`, `ifc4x3()`), so a slot that is wrong in either
//! version fails a test rather than silently reading the neighbouring
//! attribute. The readers index this module; never restate a slot number
//! at a call site.
//!
//! The IFC4 and IFC4X3 layouts of `IfcProjectedCRS` are declared
//! differently -- IFC4X3 moves `VerticalDatum` from
//! `IfcCoordinateReferenceSystem` down onto `IfcProjectedCRS` -- but the
//! absolute positions coincide, which is why one table serves both.

/// `IfcMapConversion`: `IfcCoordinateOperation` contributes `SourceCRS` and
/// `TargetCRS` at 0..1.
pub(crate) mod map_conversion {
    /// `SourceCRS : IfcCoordinateReferenceSystemSelect`.
    pub const SOURCE_CRS: usize = 0;
    /// `TargetCRS : IfcCoordinateReferenceSystem`.
    pub const TARGET_CRS: usize = 1;
    /// `Eastings : IfcLengthMeasure`.
    pub const EASTINGS: usize = 2;
    /// `Northings : IfcLengthMeasure`.
    pub const NORTHINGS: usize = 3;
    /// `OrthogonalHeight : IfcLengthMeasure`.
    pub const ORTHOGONAL_HEIGHT: usize = 4;
    /// `XAxisAbscissa : OPTIONAL IfcReal`.
    pub const X_AXIS_ABSCISSA: usize = 5;
    /// `XAxisOrdinate : OPTIONAL IfcReal`.
    pub const X_AXIS_ORDINATE: usize = 6;
    /// `Scale : OPTIONAL IfcReal`.
    pub const SCALE: usize = 7;
}

/// `IfcProjectedCRS`, inherited `IfcCoordinateReferenceSystem` slots first.
pub(crate) mod projected_crs {
    /// `Name : IfcLabel` (IFC4) / `OPTIONAL IfcLabel` (IFC4X3).
    pub const NAME: usize = 0;
    /// `Description : OPTIONAL IfcText`.
    pub const DESCRIPTION: usize = 1;
    /// `GeodeticDatum : OPTIONAL IfcIdentifier`.
    pub const GEODETIC_DATUM: usize = 2;
    /// `VerticalDatum : OPTIONAL IfcIdentifier`; inherited in IFC4, declared
    /// on `IfcProjectedCRS` itself in IFC4X3.
    pub const VERTICAL_DATUM: usize = 3;
    /// `MapProjection : OPTIONAL IfcIdentifier`.
    pub const MAP_PROJECTION: usize = 4;
    /// `MapZone : OPTIONAL IfcIdentifier`.
    pub const MAP_ZONE: usize = 5;
    /// `MapUnit : OPTIONAL IfcNamedUnit`.
    pub const MAP_UNIT: usize = 6;
}

/// `IfcWellKnownText` (IFC4X3 only): the forward side of
/// `IfcCoordinateReferenceSystem.WellKnownText`.
pub(crate) mod well_known_text {
    /// `WellKnownText : IfcWellKnownTextLiteral`.
    pub const WELL_KNOWN_TEXT: usize = 0;
    /// `CoordinateReferenceSystem : IfcCoordinateReferenceSystem`.
    pub const COORDINATE_REFERENCE_SYSTEM: usize = 1;
}

/// `IfcGeometricRepresentationContext`: `IfcRepresentationContext`
/// contributes `ContextIdentifier` and `ContextType` at 0..1.
pub(crate) mod geometric_context {
    /// `TrueNorth : OPTIONAL IfcDirection`.
    pub const TRUE_NORTH: usize = 5;
}

/// `IfcDirection`.
pub(crate) mod direction {
    /// `DirectionRatios : LIST [2:3] OF IfcReal`.
    pub const DIRECTION_RATIOS: usize = 0;
}

/// `IfcSIUnit`: `IfcNamedUnit` contributes `Dimensions` and `UnitType`.
pub(crate) mod si_unit {
    /// `UnitType : IfcUnitEnum`, inherited from `IfcNamedUnit`.
    pub const UNIT_TYPE: usize = 1;
    /// `Prefix : OPTIONAL IfcSIPrefix`.
    pub const PREFIX: usize = 2;
    /// `Name : IfcSIUnitName`.
    pub const NAME: usize = 3;
}

/// `IfcConversionBasedUnit`: same inherited `IfcNamedUnit` pair.
pub(crate) mod conversion_based_unit {
    /// `UnitType : IfcUnitEnum`, inherited from `IfcNamedUnit`.
    pub const UNIT_TYPE: usize = 1;
    /// `Name : IfcLabel`.
    pub const NAME: usize = 2;
    /// `ConversionFactor : IfcMeasureWithUnit`.
    pub const CONVERSION_FACTOR: usize = 3;
}

/// `IfcMeasureWithUnit`.
pub(crate) mod measure_with_unit {
    /// `ValueComponent : IfcValue`.
    pub const VALUE_COMPONENT: usize = 0;
    /// `UnitComponent : IfcUnit`.
    pub const UNIT_COMPONENT: usize = 1;
}

#[cfg(test)]
mod tests {
    use ifc_schema::{ifc2x3, ifc4, ifc4x3, Schema};

    use super::*;

    /// `(entity, attribute count, [(slot, attribute name)])`.
    type Layout = (&'static str, usize, &'static [(usize, &'static str)]);

    /// Every slot a reader indexes, with the attribute name the schema
    /// must declare there and the entity's total attribute count. The
    /// count pins that no attribute was inserted ahead of the named ones.
    const LAYOUTS: &[Layout] = &[
        (
            "IfcMapConversion",
            8,
            &[
                (map_conversion::SOURCE_CRS, "SourceCRS"),
                (map_conversion::TARGET_CRS, "TargetCRS"),
                (map_conversion::EASTINGS, "Eastings"),
                (map_conversion::NORTHINGS, "Northings"),
                (map_conversion::ORTHOGONAL_HEIGHT, "OrthogonalHeight"),
                (map_conversion::X_AXIS_ABSCISSA, "XAxisAbscissa"),
                (map_conversion::X_AXIS_ORDINATE, "XAxisOrdinate"),
                (map_conversion::SCALE, "Scale"),
            ],
        ),
        (
            "IfcProjectedCRS",
            7,
            &[
                (projected_crs::NAME, "Name"),
                (projected_crs::DESCRIPTION, "Description"),
                (projected_crs::GEODETIC_DATUM, "GeodeticDatum"),
                (projected_crs::VERTICAL_DATUM, "VerticalDatum"),
                (projected_crs::MAP_PROJECTION, "MapProjection"),
                (projected_crs::MAP_ZONE, "MapZone"),
                (projected_crs::MAP_UNIT, "MapUnit"),
            ],
        ),
        (
            "IfcGeometricRepresentationContext",
            6,
            &[(geometric_context::TRUE_NORTH, "TrueNorth")],
        ),
        (
            "IfcDirection",
            1,
            &[(direction::DIRECTION_RATIOS, "DirectionRatios")],
        ),
        (
            "IfcSIUnit",
            4,
            &[
                (si_unit::UNIT_TYPE, "UnitType"),
                (si_unit::PREFIX, "Prefix"),
                (si_unit::NAME, "Name"),
            ],
        ),
        (
            "IfcConversionBasedUnit",
            4,
            &[
                (conversion_based_unit::UNIT_TYPE, "UnitType"),
                (conversion_based_unit::NAME, "Name"),
                (conversion_based_unit::CONVERSION_FACTOR, "ConversionFactor"),
            ],
        ),
        (
            "IfcMeasureWithUnit",
            2,
            &[
                (measure_with_unit::VALUE_COMPONENT, "ValueComponent"),
                (measure_with_unit::UNIT_COMPONENT, "UnitComponent"),
            ],
        ),
    ];

    fn assert_layouts(schema: &Schema) {
        let release = schema.name();
        for (entity, arity, slots) in LAYOUTS {
            assert!(
                schema.entity(entity).is_some(),
                "{release} does not declare {entity}"
            );
            let names = schema.attribute_names(entity);
            assert_eq!(names.len(), *arity, "{release} {entity} arity: {names:?}");
            for (slot, name) in *slots {
                assert_eq!(
                    names.get(*slot).copied(),
                    Some(*name),
                    "{release} {entity} slot {slot}: {names:?}"
                );
            }
        }
    }

    #[test]
    fn slots_match_the_bundled_ifc4_schema() {
        assert_layouts(ifc4());
    }

    #[test]
    fn slots_match_the_bundled_ifc4x3_schema() {
        assert_layouts(ifc4x3());
    }

    /// `IfcProjectedCRS` is declared differently in the two versions even
    /// though the absolute layout coincides. Pin the declaration difference
    /// too, so a future schema regeneration that changes it is noticed here
    /// rather than assumed away by the shared table above.
    #[test]
    fn vertical_datum_moves_between_versions_without_moving_its_slot() {
        let crs = "IfcCoordinateReferenceSystem";
        assert_eq!(
            ifc4().attribute_names(crs),
            ["Name", "Description", "GeodeticDatum", "VerticalDatum"]
        );
        assert_eq!(
            ifc4x3().attribute_names(crs),
            ["Name", "Description", "GeodeticDatum"]
        );
    }

    /// The IFC4X3-only coordinate operations this crate refuses today
    /// (`GeorefError::UnsupportedOperation`). Pinned here so later work
    /// lowering them starts from checked positions, and so a
    /// `IfcMapConversionScaled` provably shares every slot the
    /// `IfcMapConversion` reader indexes.
    #[test]
    fn ifc4x3_only_operations_are_absent_from_ifc4_and_laid_out_in_ifc4x3() {
        for entity in [
            "IfcMapConversionScaled",
            "IfcRigidOperation",
            "IfcGeographicCRS",
        ] {
            assert!(ifc4().entity(entity).is_none(), "IFC4 declares {entity}");
            assert!(ifc4x3().entity(entity).is_some(), "IFC4X3 lacks {entity}");
        }
        let scaled = ifc4x3().attribute_names("IfcMapConversionScaled");
        assert_eq!(
            scaled[..8],
            ifc4x3().attribute_names("IfcMapConversion")[..],
            "IfcMapConversionScaled must extend IfcMapConversion's layout"
        );
        assert_eq!(scaled[8..], ["FactorX", "FactorY", "FactorZ"]);
        assert!(ifc4().entity("IfcWellKnownText").is_none());
        let wkt = ifc4x3().attribute_names("IfcWellKnownText");
        assert_eq!(wkt.len(), 2);
        assert_eq!(
            wkt[well_known_text::WELL_KNOWN_TEXT],
            "WellKnownText",
            "{wkt:?}"
        );
        assert_eq!(
            wkt[well_known_text::COORDINATE_REFERENCE_SYSTEM],
            "CoordinateReferenceSystem",
            "{wkt:?}"
        );
        assert_eq!(
            ifc4x3().attribute_names("IfcRigidOperation"),
            [
                "SourceCRS",
                "TargetCRS",
                "FirstCoordinate",
                "SecondCoordinate",
                "Height"
            ]
        );
        assert_eq!(
            ifc4x3().attribute_names("IfcGeographicCRS"),
            [
                "Name",
                "Description",
                "GeodeticDatum",
                "PrimeMeridian",
                "AngleUnit",
                "HeightUnit"
            ]
        );
    }

    /// IFC2X3 is refused by `GeorefView` because it declares none of the
    /// coordinate-operation entities; keep that claim tied to the table.
    #[test]
    fn ifc2x3_declares_no_coordinate_operation() {
        for entity in [
            "IfcCoordinateOperation",
            "IfcMapConversion",
            "IfcCoordinateReferenceSystem",
            "IfcProjectedCRS",
        ] {
            assert!(
                ifc2x3().entity(entity).is_none(),
                "IFC2X3 declares {entity}"
            );
        }
    }
}
