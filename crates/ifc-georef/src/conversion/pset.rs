//! IFC2X3 georeferencing through the `ePSet_MapConversion` and
//! `ePSet_ProjectedCRS` property sets (#451).
//!
//! # Source
//!
//! IFC2X3 declares no `IfcMapConversion` or `IfcProjectedCRS`. The
//! buildingSMART Australasia "User Guide for Geo-referencing in IFC",
//! version 2.0 (January 2020), §5.1 "The IFC Site Concept", states the
//! IFC2X3 mapping:
//!
//! - Table 2 maps the IFC4 entities to two IFC2X3 property sets,
//!   `ePSet_ProjectedCRS` and `ePSet_MapConversion`.
//! - Table 3 lists `ePSet_MapConversion`: `Eastings`, `Northings`,
//!   `OrthogonalHeight` (`IfcLengthMeasure`), `XAxisAbscissa`,
//!   `XAxisOrdinate` and `Scale` (`IfcReal`): the `IfcMapConversion`
//!   attributes of the same names.
//! - Table 4 lists `ePSet_ProjectedCRS`: `Name` (`IfcLabel`, an EPSG
//!   name), `Description` (`IfcText`), `GeodeticDatum`, `VerticalDatum`,
//!   `MapProjection` and `MapZone` (`IfcIdentifier`): the
//!   `IfcProjectedCRS` attributes of the same names, without `MapUnit`.
//! - The note closing §5.1: "For the IFC2x3 implementation the ePSets are
//!   linked to ifcProject as the geo-referencing specification applies to
//!   the entire project (model)."
//!
//! Each property is read as the `IfcMapConversion` or `IfcProjectedCRS`
//! attribute it stands for and lowered by the same arithmetic
//! (`map::linear_part`), so an IFC2X3 file and its IFC4 equivalent
//! resolve to the same [`ProjectToMap`]. What IFC4 makes mandatory is
//! required here too: `Eastings`, `Northings`, `OrthogonalHeight` and the
//! CRS `Name`. The optional ones default as IFC4 defaults them (axis
//! `(1, 0)`, scale `1`). The guide's §5.2 example applies `Scale` to
//! eastings and northings only (its footnote 9); IFC4 `IfcMapConversion`
//! scales heights too. The sets follow IFC4, so an IFC2X3 file and its
//! IFC4 twin agree; the two readings differ only in height, and only when
//! `Scale` is not `1`.
//!
//! # Where the guide is silent: established practice
//!
//! These follow what IFC2X3 exporters and readers do (IfcOpenShell's
//! `ePSet_Georeferencing` templates and `util/geolocation.py`, tudelft3d
//! `ifcgref`, ifc-lite), not the guide:
//!
//! - **Name case.** The guide spells `ePSet_`; exporters also write
//!   `ePset_` and `EPset_`. Property-set names match ignoring ASCII case.
//!   Property names inside the sets match exactly.
//! - **`IfcSite` as a legacy host.** Some exporters attach the sets to
//!   `IfcSite`. When an `IfcProject` carries either set, only the project
//!   is read and every site is ignored. Otherwise each `IfcSite` that
//!   carries the sets yields its own operation (the guide's §5.1.2 allows
//!   one transformation per site). A pair is never joined across hosts: a
//!   host with one set and not the other is refused with
//!   [`GeorefError::IncompletePropertySets`].
//! - **`MapUnit`.** An optional `ePSet_ProjectedCRS` property naming an SI
//!   length unit as text (`METRE`, `MILLIMETRE`, ...), as IfcOpenShell
//!   writes it. When absent the map unit is the project length unit, the
//!   IFC4 rule for an omitted `IfcProjectedCRS.MapUnit`. `Eastings`,
//!   `Northings` and `OrthogonalHeight` are in the map unit, as in IFC4; a
//!   length property that states its own `Unit` must state that unit.
//!
//! # Precedence over the IFC4 entities
//!
//! The declared release decides, and the two are never merged: an IFC2X3
//! file is read only through these property sets (it cannot declare the
//! entities), and an IFC4 or IFC4X3 file only through its
//! `IfcCoordinateOperation` entities; property sets of these names in an
//! IFC4 or IFC4X3 file are not read for georeferencing.
//!
//! # Strictness
//!
//! A malformed set is refused, never partly read: two sets of one name on
//! a host ([`GeorefError::AmbiguousPropertySets`]), a repeated property
//! name (`IfcPropertySet.WR32`, [`GeorefError::RuleViolation`]), a required
//! property without a value ([`GeorefError::MissingProperty`]), and a
//! property that is not an `IfcPropertySingleValue`, a number that is not
//! the guide's measure type or not finite, or a text that is not text
//! ([`GeorefError::InvalidProperty`]). Unrecognised properties are ignored.

use std::collections::BTreeMap;

use ifc_model::value::Value;
use ifc_model::{Entity, EntityId, Model};
use ifc_schema::{for_version, SchemaVersion};

use crate::context::OperationSource;
use crate::crs::{resolve_length_unit, LengthUnit, ProjectedCrs};
use crate::error::{GeorefError, GeorefResult};
use crate::slot::{property_set, property_single_value, rel_defines_by_properties};
use crate::view::declared_release;

use super::map::linear_part;
use super::operation::{OperationKind, ProjectToMap};

/// The property set standing for `IfcMapConversion` (guide Table 3).
pub const MAP_CONVERSION_PSET: &str = "ePSet_MapConversion";
/// The property set standing for `IfcProjectedCRS` (guide Table 4).
pub const PROJECTED_CRS_PSET: &str = "ePSet_ProjectedCRS";

/// Whether `model` holds any `IfcPropertySet` named `ePSet_MapConversion`
/// or `ePSet_ProjectedCRS` (ignoring ASCII case), attached or not.
///
/// A cheap presence test, so a caller can skip resolving the project
/// length unit for a model with nothing to read.
#[must_use]
pub fn has_property_set_georeferencing(model: &Model) -> bool {
    model
        .of_type("IFCPROPERTYSET")
        .any(|(_, set)| pset_kind(set).is_some())
}

/// Every IFC2X3 property-set georeferencing of `model`, resolved to
/// project-to-map parameters: one for the `IfcProject`, or else one per
/// `IfcSite` carrying the sets, in ascending host id.
///
/// `project_metres_per_unit` is the project's `IfcUnitAssignment` length
/// scale, as for [`crate::resolve_project_to_map`]. In the result,
/// [`ProjectToMap::source`] is [`OperationSource::PropertySet`] naming the
/// host, [`ProjectToMap::operation`] is the `ePSet_MapConversion` set,
/// [`ProjectedCrs::entity`] the `ePSet_ProjectedCRS` set, and the kind is
/// [`OperationKind::MapConversion`]. A model with neither set yields an
/// empty list. See the module documentation for the source, the rules
/// where the guide is silent and the precedence over IFC4 entities.
///
/// # Errors
///
/// [`GeorefError::MissingSchema`], [`GeorefError::AmbiguousSchema`] or
/// [`GeorefError::UnsupportedSchema`] unless the header declares exactly
/// IFC2X3 (and this build links its table); [`GeorefError::InvalidUnit`]
/// for a non-positive or non-finite project scale or an unusable
/// `MapUnit`; and the property-set refusals listed in the module
/// documentation.
pub fn resolve_property_set_georeferencing(
    model: &Model,
    project_metres_per_unit: f64,
) -> GeorefResult<Vec<ProjectToMap>> {
    let (token, version) = declared_release(model)?;
    if version != SchemaVersion::Ifc2x3 || for_version(version).is_err() {
        return Err(GeorefError::UnsupportedSchema {
            token: token.clone(),
        });
    }
    let hosts = hosts(model)?;
    if hosts.is_empty() {
        return Ok(Vec::new());
    }
    if !project_metres_per_unit.is_finite() || project_metres_per_unit <= 0.0 {
        let (&host, _) = hosts.iter().next().expect("not empty");
        return Err(GeorefError::InvalidUnit {
            entity: host,
            detail: "project length scale must be finite and positive",
        });
    }
    let project_unit = LengthUnit {
        name: "PROJECT_LENGTH_UNIT".into(),
        metres_per_unit: project_metres_per_unit,
    };
    let any_project = hosts.values().any(|sets| sets.is_project);
    hosts
        .iter()
        .filter(|(_, sets)| sets.is_project == any_project)
        .map(|(&host, sets)| resolve_host(model, host, sets, &project_unit))
        .collect()
}

/// Which georeferencing set a property set is, by name ignoring case.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    MapConversion,
    ProjectedCrs,
}

fn pset_kind(set: &Entity) -> Option<Kind> {
    let name = set.text(property_set::NAME)?;
    if name.eq_ignore_ascii_case(MAP_CONVERSION_PSET) {
        Some(Kind::MapConversion)
    } else if name.eq_ignore_ascii_case(PROJECTED_CRS_PSET) {
        Some(Kind::ProjectedCrs)
    } else {
        None
    }
}

/// The georeferencing sets attached to one host.
#[derive(Default)]
struct HostSets {
    is_project: bool,
    map_conversion: Vec<EntityId>,
    projected_crs: Vec<EntityId>,
}

/// Every `IfcProject` and `IfcSite` with a georeferencing set attached by
/// an `IfcRelDefinesByProperties`, keyed by host id. Other objects are not
/// hosts: the guide names the project, practice adds the site.
fn hosts(model: &Model) -> GeorefResult<BTreeMap<EntityId, HostSets>> {
    let mut hosts: BTreeMap<EntityId, HostSets> = BTreeMap::new();
    for (rel_id, rel) in model.of_type("IFCRELDEFINESBYPROPERTIES") {
        let Some(set_id) = rel.reference(rel_defines_by_properties::RELATING_PROPERTY_DEFINITION)
        else {
            continue;
        };
        let Some(kind) = model
            .get(set_id)
            .filter(|set| set.is_type("IFCPROPERTYSET"))
            .and_then(pset_kind)
        else {
            continue;
        };
        let related = rel
            .attribute(rel_defines_by_properties::RELATED_OBJECTS)
            .and_then(Value::as_list)
            .unwrap_or(&[]);
        for object in related.iter().filter_map(Value::as_ref_id) {
            let host = model.get(object).ok_or(GeorefError::MissingEntity {
                referrer: rel_id,
                missing: object,
            })?;
            let is_project = host.is_type("IFCPROJECT");
            if !is_project && !host.is_type("IFCSITE") {
                continue;
            }
            let sets = hosts.entry(object).or_default();
            sets.is_project = is_project;
            let list = match kind {
                Kind::MapConversion => &mut sets.map_conversion,
                Kind::ProjectedCrs => &mut sets.projected_crs,
            };
            if !list.contains(&set_id) {
                list.push(set_id);
            }
        }
    }
    Ok(hosts)
}

/// The one set of `name` on `host`, refusing none or several.
fn one(host: EntityId, name: &'static str, sets: &[EntityId]) -> GeorefResult<EntityId> {
    match sets {
        [] => Err(GeorefError::IncompletePropertySets {
            host,
            missing: name,
        }),
        [one] => Ok(*one),
        _ => {
            let mut sets = sets.to_vec();
            sets.sort_unstable();
            Err(GeorefError::AmbiguousPropertySets { host, name, sets })
        }
    }
}

fn resolve_host(
    model: &Model,
    host: EntityId,
    sets: &HostSets,
    project_unit: &LengthUnit,
) -> GeorefResult<ProjectToMap> {
    // Both ambiguity checks before completeness, so a host with two of one
    // set and none of the other reports the ambiguity it has.
    if sets.map_conversion.len() > 1 {
        one(host, MAP_CONVERSION_PSET, &sets.map_conversion)?;
    }
    if sets.projected_crs.len() > 1 {
        one(host, PROJECTED_CRS_PSET, &sets.projected_crs)?;
    }
    let conversion =
        Properties::read(model, one(host, MAP_CONVERSION_PSET, &sets.map_conversion)?)?;
    let crs = Properties::read(model, one(host, PROJECTED_CRS_PSET, &sets.projected_crs)?)?;

    let target_crs = projected_crs(&crs)?;
    let map_unit = target_crs
        .map_unit
        .clone()
        .unwrap_or_else(|| project_unit.clone());

    let eastings = conversion.length(model, "Eastings", &map_unit)?;
    let northings = conversion.length(model, "Northings", &map_unit)?;
    let height = conversion.length(model, "OrthogonalHeight", &map_unit)?;
    let a = conversion.real("XAxisAbscissa")?.map_or(1.0, |(_, v)| v);
    let b = conversion.real("XAxisOrdinate")?.map_or(0.0, |(_, v)| v);
    let norm = a.hypot(b);
    if !norm.is_finite() || norm <= f64::EPSILON {
        return Err(GeorefError::DegenerateAxis {
            entity: conversion.set,
        });
    }
    let (a, b) = (a / norm, b / norm);
    let declared_scale = conversion.real("Scale")?.map_or(1.0, |(_, v)| v);
    if declared_scale <= 0.0 {
        return Err(GeorefError::InvalidScale {
            entity: conversion.set,
            value: declared_scale,
        });
    }
    let scale = declared_scale * map_unit.metres_per_unit / project_unit.metres_per_unit;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(GeorefError::InvalidScale {
            entity: conversion.set,
            value: scale,
        });
    }
    let linear = linear_part(scale, (a, b), (1.0, 1.0, 1.0));
    let metres = map_unit.metres_per_unit;
    let translation = [eastings * metres, northings * metres, height * metres];
    for (value, name) in translation
        .iter()
        .zip(["Eastings", "Northings", "OrthogonalHeight"])
    {
        if !value.is_finite() {
            let (property, _) = conversion.get(name).expect("read above");
            return Err(GeorefError::InvalidProperty {
                property_set: conversion.set,
                property,
                name,
                expected: "a length whose value in metres is finite",
            });
        }
    }

    Ok(ProjectToMap::new(
        OperationSource::PropertySet(host),
        conversion.set,
        OperationKind::MapConversion,
        target_crs,
        [eastings, northings, height],
        (project_unit.clone(), map_unit),
        declared_scale,
        (a, b),
        linear,
        translation,
    ))
}

/// `ePSet_ProjectedCRS` read as an `IfcProjectedCRS` (guide Table 4).
fn projected_crs(crs: &Properties) -> GeorefResult<ProjectedCrs> {
    let name = crs.text("Name")?.ok_or(GeorefError::MissingProperty {
        property_set: crs.set,
        name: "Name",
    })?;
    let map_unit = match crs.text("MapUnit")? {
        None => None,
        Some(text) => {
            let (property, _) = crs.get("MapUnit").expect("read above");
            Some(si_length_unit(&text).ok_or(GeorefError::InvalidProperty {
                property_set: crs.set,
                property,
                name: "MapUnit",
                expected: "the name of an SI length unit, such as METRE or MILLIMETRE",
            })?)
        }
    };
    Ok(ProjectedCrs {
        entity: crs.set,
        name: Some(name),
        description: crs.text("Description")?,
        geodetic_datum: crs.text("GeodeticDatum")?,
        vertical_datum: crs.text("VerticalDatum")?,
        map_projection: crs.text("MapProjection")?,
        map_zone: crs.text("MapZone")?,
        map_unit,
        well_known_text: None,
    })
}

/// `METRE`, or an `IfcSIPrefix` followed by `METRE`, as written in
/// `IfcSIUnitName` and `IfcSIPrefix` (ignoring ASCII case).
fn si_length_unit(text: &str) -> Option<LengthUnit> {
    const PREFIXES: &[(&str, f64)] = &[
        ("", 1.0),
        ("EXA", 1e18),
        ("PETA", 1e15),
        ("TERA", 1e12),
        ("GIGA", 1e9),
        ("MEGA", 1e6),
        ("KILO", 1e3),
        ("HECTO", 1e2),
        ("DECA", 1e1),
        ("DECI", 1e-1),
        ("CENTI", 1e-2),
        ("MILLI", 1e-3),
        ("MICRO", 1e-6),
        ("NANO", 1e-9),
        ("PICO", 1e-12),
        ("FEMTO", 1e-15),
        ("ATTO", 1e-18),
    ];
    let upper = text.trim().to_ascii_uppercase();
    let prefix = upper.strip_suffix("METRE")?;
    PREFIXES
        .iter()
        .find(|(p, _)| *p == prefix)
        .map(|&(_, metres_per_unit)| LengthUnit {
            name: upper.clone(),
            metres_per_unit,
        })
}

/// The properties of one georeferencing set, by name.
struct Properties<'m> {
    set: EntityId,
    by_name: BTreeMap<&'m str, (EntityId, &'m Entity)>,
}

impl<'m> Properties<'m> {
    /// Index `HasProperties` by name, refusing a dangling or non-reference
    /// member and a repeated name (`WR32`).
    fn read(model: &'m Model, set: EntityId) -> GeorefResult<Self> {
        let entity = model.get(set).ok_or(GeorefError::MissingEntity {
            referrer: set,
            missing: set,
        })?;
        let members = entity
            .attribute(property_set::HAS_PROPERTIES)
            .and_then(Value::as_list)
            .ok_or(GeorefError::InvalidAttribute {
                entity: set,
                index: property_set::HAS_PROPERTIES,
                name: "HasProperties",
            })?;
        let mut by_name = BTreeMap::new();
        for member in members {
            let id = member.as_ref_id().ok_or(GeorefError::InvalidAttribute {
                entity: set,
                index: property_set::HAS_PROPERTIES,
                name: "HasProperties",
            })?;
            let property = model.get(id).ok_or(GeorefError::MissingEntity {
                referrer: set,
                missing: id,
            })?;
            let Some(name) = property.text(property_single_value::NAME) else {
                return Err(GeorefError::InvalidAttribute {
                    entity: id,
                    index: property_single_value::NAME,
                    name: "Name",
                });
            };
            if by_name.insert(name, (id, property)).is_some() {
                return Err(GeorefError::RuleViolation {
                    entity: set,
                    rule: "WR32 : IfcUniquePropertyName(HasProperties)",
                });
            }
        }
        Ok(Self { set, by_name })
    }

    fn get(&self, name: &str) -> Option<(EntityId, &'m Entity)> {
        self.by_name.get(name).copied()
    }

    /// The `NominalValue` of single-value property `name`: `None` when the
    /// property is absent or its value unset.
    fn nominal(
        &self,
        name: &'static str,
        expected: &'static str,
    ) -> GeorefResult<Option<(EntityId, &'m Entity, &'m Value)>> {
        let Some((id, property)) = self.get(name) else {
            return Ok(None);
        };
        if !property.is_type("IFCPROPERTYSINGLEVALUE") {
            return Err(self.invalid(id, name, expected));
        }
        match property.attribute(property_single_value::NOMINAL_VALUE) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => Ok(Some((id, property, value))),
        }
    }

    fn invalid(
        &self,
        property: EntityId,
        name: &'static str,
        expected: &'static str,
    ) -> GeorefError {
        GeorefError::InvalidProperty {
            property_set: self.set,
            property,
            name,
            expected,
        }
    }

    /// A finite number wrapped in `measure`, the guide's data type.
    fn measure(
        &self,
        name: &'static str,
        measure: &'static str,
        expected: &'static str,
    ) -> GeorefResult<Option<(EntityId, &'m Entity, f64)>> {
        let Some((id, property, value)) = self.nominal(name, expected)? else {
            return Ok(None);
        };
        match value {
            Value::Typed { type_name, value } if type_name.eq_ignore_ascii_case(measure) => {
                match value.as_f64() {
                    Some(number) if number.is_finite() => Ok(Some((id, property, number))),
                    _ => Err(self.invalid(id, name, expected)),
                }
            }
            _ => Err(self.invalid(id, name, expected)),
        }
    }

    /// A required `IfcLengthMeasure` in `map_unit`. A property `Unit`, when
    /// stated, must resolve to the map unit.
    fn length(
        &self,
        model: &Model,
        name: &'static str,
        map_unit: &LengthUnit,
    ) -> GeorefResult<f64> {
        const EXPECTED: &str = "an IfcPropertySingleValue with a finite IfcLengthMeasure";
        let (id, property, value) = self.measure(name, "IFCLENGTHMEASURE", EXPECTED)?.ok_or(
            GeorefError::MissingProperty {
                property_set: self.set,
                name,
            },
        )?;
        match property.attribute(property_single_value::UNIT) {
            None | Some(Value::Null) => {}
            Some(unit) => {
                let unit = unit.as_ref_id().ok_or(GeorefError::InvalidAttribute {
                    entity: id,
                    index: property_single_value::UNIT,
                    name: "Unit",
                })?;
                let stated = resolve_length_unit(model, unit)?;
                if stated.metres_per_unit != map_unit.metres_per_unit {
                    return Err(self.invalid(
                        id,
                        name,
                        "a length in the map unit: its Unit states another",
                    ));
                }
            }
        }
        Ok(value)
    }

    /// An optional `IfcReal`, refusing a stated `Unit` (a real has none).
    fn real(&self, name: &'static str) -> GeorefResult<Option<(EntityId, f64)>> {
        const EXPECTED: &str = "an IfcPropertySingleValue with a finite IfcReal and no Unit";
        let Some((id, property, value)) = self.measure(name, "IFCREAL", EXPECTED)? else {
            return Ok(None);
        };
        if !matches!(
            property.attribute(property_single_value::UNIT),
            None | Some(Value::Null)
        ) {
            return Err(self.invalid(id, name, EXPECTED));
        }
        Ok(Some((id, value)))
    }

    /// An optional text: `IfcLabel`, `IfcText` or `IfcIdentifier`.
    fn text(&self, name: &'static str) -> GeorefResult<Option<String>> {
        const EXPECTED: &str =
            "an IfcPropertySingleValue with an IfcLabel, IfcText or IfcIdentifier";
        let Some((id, _, value)) = self.nominal(name, EXPECTED)? else {
            return Ok(None);
        };
        match value {
            Value::Typed { type_name, value }
                if ["IFCLABEL", "IFCTEXT", "IFCIDENTIFIER"]
                    .iter()
                    .any(|t| type_name.eq_ignore_ascii_case(t)) =>
            {
                value
                    .as_text()
                    .map(|text| Some(text.to_owned()))
                    .ok_or_else(|| self.invalid(id, name, EXPECTED))
            }
            _ => Err(self.invalid(id, name, EXPECTED)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::si_length_unit;

    #[test]
    fn map_unit_names_resolve_only_si_length_units() {
        assert_eq!(si_length_unit("METRE").unwrap().metres_per_unit, 1.0);
        assert_eq!(si_length_unit("millimetre").unwrap().metres_per_unit, 1e-3);
        assert_eq!(si_length_unit("KILOMETRE").unwrap().metres_per_unit, 1e3);
        for refused in ["FOOT", "METER", "SQUARE_METRE", "MILLI", "", "XMETRE"] {
            assert!(si_length_unit(refused).is_none(), "{refused}");
        }
    }
}
