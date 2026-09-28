//! The IFC release a release-aware writer authors against (#200).
//!
//! Most writers here need no release: an `IfcParameterValue` attribute is
//! declared with that defined type in IFC2X3, IFC4 and IFC4X3 alike, so its
//! value is written bare everywhere. A few attributes change kind or
//! optionality between releases, and only those writers take the model and
//! bind its release:
//!
//! ```text
//! IfcSurfaceCurveSweptAreaSolid.StartParam / EndParam
//! IfcFixedReferenceSweptAreaSolid.StartParam / EndParam
//!   IFC2X3  IfcParameterValue           (required; the fixed-reference
//!                                         sweep is not declared at all)
//!   IFC4    OPTIONAL IfcParameterValue  -> 0.5
//!   IFC4X3  OPTIONAL IfcCurveMeasureSelect, inherited from
//!           IfcDirectrixCurveSweptAreaSolid
//!           = SELECT (IfcLengthMeasure, IfcParameterValue)
//!                                       -> IFCPARAMETERVALUE(0.5)
//!
//! IfcSweptDiskSolid.StartParam / EndParam (#210)
//!   IFC2X3  IfcParameterValue           (required)
//!   IFC4    OPTIONAL IfcParameterValue
//!   IFC4X3  OPTIONAL IfcParameterValue
//! ```
//!
//! ISO 10303-21 writes a typed parameter only where the declared type is a
//! SELECT, so the form is read from the bound table, never assumed.
//!
//! Binding, from `FILE_SCHEMA`, as `ifc-properties` and `ifc-material` bind
//! their authoring:
//! - one recognised declaration binds that release's bundled table;
//! - one unrecognised declaration, or several, fail with
//!   [`GeometryError::AuthoringSchemaUnbound`];
//! - none at all (an in-memory `Model::default()`) binds IFC4.

use ifc_model::{Model, Value};
use ifc_schema::{for_version, Schema, SchemaVersion, TypeKind};

use crate::error::GeometryError;

use super::invalid;

/// The bound release and its bundled table.
#[derive(Debug, Clone, Copy)]
pub(super) struct Release {
    version: SchemaVersion,
    schema: &'static Schema,
}

/// Bind `model`'s declared release to author `type_name` in it.
///
/// Refuses a release that does not declare `type_name`.
pub(super) fn bind(model: &Model, type_name: &'static str) -> Result<Release, GeometryError> {
    let unbound = |detail: String| GeometryError::AuthoringSchemaUnbound { type_name, detail };
    let version = match model.header().schema.as_slice() {
        [] => SchemaVersion::Ifc4,
        [token] => SchemaVersion::from_header_token(token)
            .ok_or_else(|| unbound(format!("FILE_SCHEMA declares unknown release {token:?}")))?,
        tokens => {
            return Err(unbound(format!(
                "FILE_SCHEMA declares {} releases, not one",
                tokens.len()
            )))
        }
    };
    // IFC4X1 and IFC4X2 are bundled by ifc-schema, but no geometry writer is
    // verified against their tables: refused, never aliased to a neighbour.
    if !matches!(
        version,
        SchemaVersion::Ifc2x3 | SchemaVersion::Ifc4 | SchemaVersion::Ifc4x3
    ) {
        return Err(unbound(format!(
            "geometry authoring is not verified for {version:?}"
        )));
    }
    let schema =
        for_version(version).map_err(|_| unbound(format!("no bundled table for {version:?}")))?;
    if schema
        .entity(type_name)
        .is_none_or(|entity| entity.abstract_)
    {
        return Err(GeometryError::AuthoringEntityNotInSchema {
            type_name,
            schema: version,
        });
    }
    Ok(Release { version, schema })
}

impl Release {
    /// The bound release.
    pub(super) fn version(self) -> SchemaVersion {
        self.version
    }

    /// The bound release's bundled table.
    pub(super) fn schema(self) -> &'static Schema {
        self.schema
    }

    /// An `IfcParameterValue` in the form `type_name.attribute` declares it
    /// in this release, or `$` for `None`.
    ///
    /// Bare where the declared type is a defined type; the typed parameter
    /// `IFCPARAMETERVALUE(..)` where it is a SELECT that admits it.
    pub(super) fn parameter(
        self,
        type_name: &'static str,
        attribute: &'static str,
        value: Option<f64>,
    ) -> Result<Value, GeometryError> {
        let Some(value) = value else {
            return Ok(Value::Null);
        };
        let declared = self
            .schema
            .attributes(type_name)
            .into_iter()
            .find(|declared| declared.name.eq_ignore_ascii_case(attribute))
            .ok_or_else(|| {
                invalid(
                    type_name,
                    attribute,
                    format!("{:?} declares no such attribute", self.version),
                )
            })?;
        let declared = declared.type_name.as_str();
        if !self.schema.accepts_type(declared, "IfcParameterValue") {
            return Err(invalid(
                type_name,
                attribute,
                format!("{declared} does not admit an IfcParameterValue"),
            ));
        }
        let select = self
            .schema
            .type_def(declared)
            .is_some_and(|definition| matches!(definition.kind, TypeKind::Select(_)));
        Ok(if select {
            Value::Typed {
                type_name: "IFCPARAMETERVALUE".into(),
                value: Box::new(Value::Real(value)),
            }
        } else {
            Value::Real(value)
        })
    }

    /// Refuse a record of `type_name` that leaves an attribute this release
    /// requires unset, such as the IFC2X3 `Position` or `StartParam`.
    pub(super) fn require(
        self,
        type_name: &'static str,
        attrs: &[Value],
    ) -> Result<(), GeometryError> {
        let declared = self.schema.attributes(type_name);
        if declared.len() != attrs.len() {
            return Err(invalid(
                type_name,
                "*",
                format!(
                    "{:?} declares {} attributes, the record has {}",
                    self.version,
                    declared.len(),
                    attrs.len()
                ),
            ));
        }
        for (declaration, value) in declared.into_iter().zip(attrs) {
            if !declaration.optional && *value == Value::Null {
                return Err(invalid(
                    type_name,
                    declaration.name.as_str(),
                    format!("{:?} requires it", self.version),
                ));
            }
        }
        Ok(())
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
                matches!(bind(&model, "IfcGrid"), Err(GeometryError::AuthoringSchemaUnbound { .. }) if true),
                "{token} must be refused"
            );
        }
    }
}
