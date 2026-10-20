//! Which declared types are lengths, areas, volumes or parameters, read
//! from the release's tables; and the reviewed tables for the attributes
//! the measure types cannot classify.
//!
//! A measure type is classified by `ifc_properties::measure_unit_type`,
//! which derives the unit type from the release (`IfcXMeasure` takes
//! `XUNIT`, a defined type follows its base): `LENGTHUNIT`, `AREAUNIT` and
//! `VOLUMEUNIT` measures are rescaled. Nothing here lists a measure by name.
//!
//! What the measure types cannot say is listed below and tested against the
//! releases (`tests::every_real_and_parameter_attribute_is_reviewed`): every
//! attribute declared `REAL`, `NUMBER`, `IfcReal` or `IfcParameterValue`, or
//! a `SELECT` that admits `IfcParameterValue` outside `IfcValue`, in IFC2X3
//! TC1, IFC4 ADD2 TC1 and IFC4X3 ADD2, has a row; a release adding one
//! fails the test until it is reviewed.

use std::collections::HashMap;
use std::sync::Arc;

use ifc_properties::{measure_unit_type, MeasureUnitType, SchemaVersion};
use ifc_schema::{Schema, TypeKind};

/// What a numeric value of a type means for the conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Measure {
    /// In the project length unit.
    Length,
    /// In the project area unit.
    Area,
    /// In the project volume unit.
    Volume,
    /// An `IfcParameterValue`: its host decides.
    Parameter,
}

/// What an attribute's declared type is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Declared {
    /// A defined or simple type: an untyped number is this measure, or
    /// nothing to convert.
    Plain(Option<Measure>),
    /// A `SELECT`: values carry their type. An untyped number is the one
    /// numeric member when there is exactly one, else ambiguous.
    Select {
        /// The select's name, upper case.
        name: Arc<str>,
        /// The numeric member an untyped number must be, if unique.
        numeric: Option<Option<Measure>>,
    },
}

/// The role of a reviewed `REAL` attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RealRole {
    /// A ratio, direction, weight or factor: unchanged.
    Unitless,
    /// A length stated as a `REAL`.
    Length,
    /// Proportional to a distance: divided by the length factor.
    InverseLength,
    /// Proportional to a squared distance.
    InverseArea,
    /// `IfcMapConversion.Scale`: handled with the map unit (`georef.rs`).
    MapScale,
    /// Not carried: refused when present.
    Refuse,
}

/// Every `REAL`, `NUMBER` and `IfcReal` attribute of IFC2X3 TC1, IFC4 ADD2
/// TC1 and IFC4X3 ADD2, by declaring entity.
pub(super) const REVIEWED_REALS: &[(&str, &str, RealRole)] = &[
    // A dimensionless multiplier of the transformed lengths.
    (
        "IFCCARTESIANTRANSFORMATIONOPERATOR",
        "Scale",
        RealRole::Unitless,
    ),
    (
        "IFCCARTESIANTRANSFORMATIONOPERATOR2DNONUNIFORM",
        "Scale2",
        RealRole::Unitless,
    ),
    (
        "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM",
        "Scale2",
        RealRole::Unitless,
    ),
    (
        "IFCCARTESIANTRANSFORMATIONOPERATOR3DNONUNIFORM",
        "Scale3",
        RealRole::Unitless,
    ),
    // A unit's own definition, not a value in the project unit (#110).
    (
        "IFCCONVERSIONBASEDUNITWITHOFFSET",
        "ConversionOffset",
        RealRole::Unitless,
    ),
    ("IFCDIRECTION", "DirectionRatios", RealRole::Unitless),
    // "the maximum distance from an edge curve to the underlying face
    // surface" (IFC4 ADD2 TC1): a length.
    (
        "IFCGEOMETRICREPRESENTATIONCONTEXT",
        "Precision",
        RealRole::Length,
    ),
    (
        "IFCLIGHTSOURCEPOSITIONAL",
        "ConstantAttenuation",
        RealRole::Unitless,
    ),
    // "the attenuation in the lighting equation that is proportional to
    // the distance from the light source", and "to the square value of the
    // distance".
    (
        "IFCLIGHTSOURCEPOSITIONAL",
        "DistanceAttenuation",
        RealRole::InverseLength,
    ),
    (
        "IFCLIGHTSOURCEPOSITIONAL",
        "QuadricAttenuation",
        RealRole::InverseArea,
    ),
    (
        "IFCLIGHTSOURCESPOT",
        "ConcentrationExponent",
        RealRole::Unitless,
    ),
    ("IFCMAPCONVERSION", "Scale", RealRole::MapScale),
    // Together a direction in the map's horizontal plane.
    ("IFCMAPCONVERSION", "XAxisAbscissa", RealRole::Unitless),
    ("IFCMAPCONVERSION", "XAxisOrdinate", RealRole::Unitless),
    ("IFCMAPCONVERSIONSCALED", "FactorX", RealRole::Unitless),
    ("IFCMAPCONVERSIONSCALED", "FactorY", RealRole::Unitless),
    ("IFCMAPCONVERSIONSCALED", "FactorZ", RealRole::Unitless),
    ("IFCPOLYNOMIALCURVE", "CoefficientsX", RealRole::Refuse),
    ("IFCPOLYNOMIALCURVE", "CoefficientsY", RealRole::Refuse),
    ("IFCPOLYNOMIALCURVE", "CoefficientsZ", RealRole::Refuse),
    (
        "IFCRATIONALBSPLINECURVEWITHKNOTS",
        "WeightsData",
        RealRole::Unitless,
    ),
    (
        "IFCRATIONALBSPLINESURFACEWITHKNOTS",
        "WeightsData",
        RealRole::Unitless,
    ),
    ("IFCRATIONALBEZIERCURVE", "WeightsData", RealRole::Unitless),
    (
        "IFCSURFACESTYLEREFRACTION",
        "RefractionIndex",
        RealRole::Unitless,
    ),
    (
        "IFCSURFACESTYLEREFRACTION",
        "DispersionFactor",
        RealRole::Unitless,
    ),
    (
        "IFCTEXTURECOORDINATEGENERATOR",
        "Parameter",
        RealRole::Refuse,
    ),
];

/// Where an `IfcParameterValue` gets its meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Host {
    /// Always unitless.
    Unitless,
    /// In the parameter space of the curve this attribute names.
    Curve(&'static str),
    /// In the `u` direction of the surface this attribute names.
    SurfaceU(&'static str),
    /// In the `v` direction of the surface this attribute names.
    SurfaceV(&'static str),
    /// IFC defines no parameter space here.
    Undefined(&'static str),
}

/// Every attribute of the three releases that can hold an
/// `IfcParameterValue` other than as an `IfcValue`, by declaring entity.
pub(super) const PARAMETER_HOSTS: &[(&str, &str, Host)] = &[
    ("IFCTRIMMEDCURVE", "Trim1", Host::Curve("BasisCurve")),
    ("IFCTRIMMEDCURVE", "Trim2", Host::Curve("BasisCurve")),
    ("IFCSWEPTDISKSOLID", "StartParam", Host::Curve("Directrix")),
    ("IFCSWEPTDISKSOLID", "EndParam", Host::Curve("Directrix")),
    (
        "IFCSURFACECURVESWEPTAREASOLID",
        "StartParam",
        Host::Curve("Directrix"),
    ),
    (
        "IFCSURFACECURVESWEPTAREASOLID",
        "EndParam",
        Host::Curve("Directrix"),
    ),
    (
        "IFCFIXEDREFERENCESWEPTAREASOLID",
        "StartParam",
        Host::Curve("Directrix"),
    ),
    (
        "IFCFIXEDREFERENCESWEPTAREASOLID",
        "EndParam",
        Host::Curve("Directrix"),
    ),
    (
        "IFCDIRECTRIXCURVESWEPTAREASOLID",
        "StartParam",
        Host::Curve("Directrix"),
    ),
    (
        "IFCDIRECTRIXCURVESWEPTAREASOLID",
        "EndParam",
        Host::Curve("Directrix"),
    ),
    (
        "IFCPOINTONCURVE",
        "PointParameter",
        Host::Curve("BasisCurve"),
    ),
    (
        "IFCPOINTBYDISTANCEEXPRESSION",
        "DistanceAlong",
        Host::Curve("BasisCurve"),
    ),
    (
        "IFCPOINTONSURFACE",
        "PointParameterU",
        Host::SurfaceU("BasisSurface"),
    ),
    (
        "IFCPOINTONSURFACE",
        "PointParameterV",
        Host::SurfaceV("BasisSurface"),
    ),
    (
        "IFCRECTANGULARTRIMMEDSURFACE",
        "U1",
        Host::SurfaceU("BasisSurface"),
    ),
    (
        "IFCRECTANGULARTRIMMEDSURFACE",
        "U2",
        Host::SurfaceU("BasisSurface"),
    ),
    (
        "IFCRECTANGULARTRIMMEDSURFACE",
        "V1",
        Host::SurfaceV("BasisSurface"),
    ),
    (
        "IFCRECTANGULARTRIMMEDSURFACE",
        "V2",
        Host::SurfaceV("BasisSurface"),
    ),
    // Knot vectors: the parameter space itself.
    ("IFCBSPLINECURVEWITHKNOTS", "Knots", Host::Unitless),
    ("IFCBSPLINESURFACEWITHKNOTS", "UKnots", Host::Unitless),
    ("IFCBSPLINESURFACEWITHKNOTS", "VKnots", Host::Unitless),
    // The segment's own parametric length in the composite curve.
    (
        "IFCREPARAMETRISEDCOMPOSITECURVESEGMENT",
        "ParamLength",
        Host::Unitless,
    ),
    // Unit normal components.
    ("IFCTRIANGULATEDFACESET", "Normals", Host::Unitless),
    // Texture space.
    ("IFCTEXTUREVERTEX", "Coordinates", Host::Unitless),
    ("IFCTEXTUREVERTEXLIST", "TexCoordsList", Host::Unitless),
    (
        "IFCCURVESEGMENT",
        "SegmentStart",
        Host::Undefined(
            "IFC4.3 ADD2 IfcCurveSegment: until implementer agreements define a parametric \
             space for every ParentCurve, SegmentStart and SegmentLength shall be \
             IfcLengthMeasure",
        ),
    ),
    (
        "IFCCURVESEGMENT",
        "SegmentLength",
        Host::Undefined(
            "IFC4.3 ADD2 IfcCurveSegment: until implementer agreements define a parametric \
             space for every ParentCurve, SegmentStart and SegmentLength shall be \
             IfcLengthMeasure",
        ),
    ),
];

/// The row of `table` for an attribute of `entity_type`: declared by the
/// entity itself or a supertype.
pub(super) fn lookup<T: Copy>(
    schema: &Schema,
    table: &[(&str, &str, T)],
    entity_type: &str,
    attribute: &str,
) -> Option<T> {
    table
        .iter()
        .find(|(entity, name, _)| {
            name.eq_ignore_ascii_case(attribute) && schema.is_a(entity_type, entity)
        })
        .map(|(_, _, role)| *role)
}

/// Classifies type names against one release, caching each answer.
pub(super) struct Classifier {
    version: SchemaVersion,
    schema: &'static Schema,
    measures: HashMap<Arc<str>, Option<Measure>>,
    declared: HashMap<Arc<str>, Declared>,
}

impl Classifier {
    pub(super) fn new(version: SchemaVersion, schema: &'static Schema) -> Self {
        Self {
            version,
            schema,
            measures: HashMap::new(),
            declared: HashMap::new(),
        }
    }

    pub(super) fn schema(&self) -> &'static Schema {
        self.schema
    }

    /// What a number of the named type measures, if anything the
    /// conversion touches.
    pub(super) fn measure(&mut self, type_name: &str) -> Option<Measure> {
        if let Some(known) = self.measures.get(type_name) {
            return *known;
        }
        let answer = self.classify_measure(type_name);
        self.measures.insert(type_name.into(), answer);
        answer
    }

    fn classify_measure(&self, type_name: &str) -> Option<Measure> {
        let mut current = type_name.to_ascii_uppercase();
        for _ in 0..16 {
            if current == "IFCPARAMETERVALUE" {
                return Some(Measure::Parameter);
            }
            match measure_unit_type(self.version, &current) {
                Ok(MeasureUnitType::Named(unit_type)) => {
                    return match unit_type.as_ref() {
                        "LENGTHUNIT" => Some(Measure::Length),
                        "AREAUNIT" => Some(Measure::Area),
                        "VOLUMEUNIT" => Some(Measure::Volume),
                        _ => None,
                    };
                }
                Ok(_) => return None,
                Err(_) => {}
            }
            match self.defined_base(&current) {
                Some(base) => current = base,
                None => return None,
            }
        }
        None
    }

    /// The type a defined type aliases, when that is itself a type.
    fn defined_base(&self, name: &str) -> Option<String> {
        let TypeKind::Defined(rhs) = &self.schema.type_def(name)?.kind else {
            return None;
        };
        let base = rhs
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .find(|part| !part.is_empty() && !is_aggregate_keyword(part))?
            .to_ascii_uppercase();
        self.schema.type_def(&base).map(|_| base)
    }

    /// What an attribute's declared type is.
    pub(super) fn declared(&mut self, type_name: &str) -> Declared {
        if let Some(known) = self.declared.get(type_name) {
            return known.clone();
        }
        let upper = type_name.to_ascii_uppercase();
        let answer = match self
            .schema
            .type_def(&upper)
            .map(|definition| &definition.kind)
        {
            Some(TypeKind::Select(_)) => {
                let mut members = Vec::new();
                self.numeric_members(&upper, &mut members, 0);
                members.sort();
                members.dedup();
                let numeric = match members.as_slice() {
                    [only] => Some(self.measure(only)),
                    _ => None,
                };
                Declared::Select {
                    name: upper.as_str().into(),
                    numeric,
                }
            }
            _ => Declared::Plain(self.measure(&upper)),
        };
        self.declared.insert(type_name.into(), answer.clone());
        answer
    }

    /// The non-entity, non-enumeration members of a select, flattened.
    fn numeric_members(&self, select: &str, out: &mut Vec<String>, depth: usize) {
        if depth > 16 {
            return;
        }
        let Some(TypeKind::Select(members)) = self.schema.type_def(select).map(|d| &d.kind) else {
            return;
        };
        for member in members {
            let upper = member.to_ascii_uppercase();
            match self.schema.type_def(&upper).map(|d| &d.kind) {
                Some(TypeKind::Select(_)) => self.numeric_members(&upper, out, depth + 1),
                Some(TypeKind::Defined(_)) if self.is_numeric(&upper) => out.push(upper),
                _ => {}
            }
        }
    }

    /// Whether a defined type ends in `REAL`, `NUMBER` or `INTEGER`.
    fn is_numeric(&self, name: &str) -> bool {
        let base = self.schema.resolve_defined(name).to_ascii_uppercase();
        ["REAL", "NUMBER", "INTEGER"]
            .iter()
            .any(|simple| base.split_whitespace().last() == Some(simple) || base == *simple)
    }
}

fn is_aggregate_keyword(word: &str) -> bool {
    matches!(
        word.to_ascii_uppercase().as_str(),
        "LIST" | "SET" | "ARRAY" | "BAG" | "OF" | "UNIQUE" | "OPTIONAL"
    ) || word.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn releases() -> Vec<(SchemaVersion, &'static Schema)> {
        [
            SchemaVersion::Ifc2x3,
            SchemaVersion::Ifc4,
            SchemaVersion::Ifc4x3,
        ]
        .into_iter()
        .filter_map(|version| {
            ifc_schema::for_version(version)
                .ok()
                .map(|schema| (version, schema))
        })
        .collect()
    }

    /// Whether a select admits `IfcParameterValue` other than through
    /// `IfcValue` or `IfcMeasureValue` (a property value may be any
    /// measure, and a parameter value there has no host to read).
    fn admits_parameter(schema: &Schema, select: &str, depth: usize) -> bool {
        if depth > 16
            || select.eq_ignore_ascii_case("IFCVALUE")
            || select.eq_ignore_ascii_case("IFCMEASUREVALUE")
        {
            return false;
        }
        let Some(TypeKind::Select(members)) = schema.type_def(select).map(|d| &d.kind) else {
            return false;
        };
        members.iter().any(|member| {
            member.eq_ignore_ascii_case("IFCPARAMETERVALUE")
                || admits_parameter(schema, member, depth + 1)
        })
    }

    #[test]
    fn every_real_and_parameter_attribute_is_reviewed() {
        let releases = releases();
        assert!(releases.len() >= 3, "the three exact releases are bundled");
        let mut reals = 0;
        let mut parameters = 0;
        for (version, schema) in releases {
            for entity in schema.entities() {
                for attribute in &entity.attributes {
                    let ty = attribute.type_name.to_ascii_uppercase();
                    let owner = entity.name.to_ascii_uppercase();
                    if matches!(ty.as_str(), "REAL" | "NUMBER" | "IFCREAL") {
                        reals += 1;
                        assert!(
                            REVIEWED_REALS
                                .iter()
                                .any(|(e, a, _)| *e == owner
                                    && a.eq_ignore_ascii_case(&attribute.name)),
                            "{version:?}: {owner}.{} is {ty} and not reviewed",
                            attribute.name
                        );
                    }
                    if ty == "IFCPARAMETERVALUE" || admits_parameter(schema, &ty, 0) {
                        parameters += 1;
                        assert!(
                            PARAMETER_HOSTS
                                .iter()
                                .any(|(e, a, _)| *e == owner
                                    && a.eq_ignore_ascii_case(&attribute.name)),
                            "{version:?}: {owner}.{} holds an IfcParameterValue and has no host",
                            attribute.name
                        );
                    }
                }
            }
        }
        // The scan reached the attributes it exists for.
        assert!(reals >= 55, "{reals} REAL attributes");
        assert!(parameters >= 45, "{parameters} parameter attributes");
        eprintln!("{reals} REAL, {parameters} parameter attributes");
    }

    #[test]
    fn every_reviewed_row_names_an_attribute_of_some_release() {
        for (entity, attribute) in REVIEWED_REALS
            .iter()
            .map(|(e, a, _)| (*e, *a))
            .chain(PARAMETER_HOSTS.iter().map(|(e, a, _)| (*e, *a)))
        {
            assert!(
                releases()
                    .iter()
                    .any(|(_, schema)| schema.entity(entity).is_some_and(|def| def
                        .attributes
                        .iter()
                        .any(|declared| declared.name.eq_ignore_ascii_case(attribute)))),
                "{entity}.{attribute} is declared by no release"
            );
        }
    }

    #[test]
    fn measures_are_classified_from_the_release() {
        for (version, schema) in releases() {
            let mut classifier = Classifier::new(version, schema);
            assert_eq!(
                classifier.measure("IFCLENGTHMEASURE"),
                Some(Measure::Length)
            );
            assert_eq!(
                classifier.measure("IFCPOSITIVELENGTHMEASURE"),
                Some(Measure::Length)
            );
            assert_eq!(classifier.measure("IFCAREAMEASURE"), Some(Measure::Area));
            assert_eq!(
                classifier.measure("IFCVOLUMEMEASURE"),
                Some(Measure::Volume)
            );
            assert_eq!(
                classifier.measure("IFCPARAMETERVALUE"),
                Some(Measure::Parameter)
            );
            assert_eq!(classifier.measure("IFCPLANEANGLEMEASURE"), None);
            assert_eq!(classifier.measure("IFCREAL"), None);
            assert_eq!(classifier.measure("IFCRATIOMEASURE"), None);
            // A select's single numeric member is what an untyped number is.
            assert_eq!(
                classifier.declared("IFCTRIMMINGSELECT"),
                Declared::Select {
                    name: "IFCTRIMMINGSELECT".into(),
                    numeric: Some(Some(Measure::Parameter)),
                }
            );
            assert!(matches!(
                classifier.declared("IFCVALUE"),
                Declared::Select { numeric: None, .. }
            ));
        }
    }
}
