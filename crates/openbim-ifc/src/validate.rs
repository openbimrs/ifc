//! Schema and integrity validation: [`ifc_validate`], with the PSD/QTO
//! catalog composed in.
//!
//! Everything in `ifc-validate` is here under the same names. With the
//! `property-catalog` or `property-catalog-runtime` feature (and a release,
//! which every validating build names), `validate`, `validate_with`,
//! `validate_declared` and `validate_with_options` also check every
//! `Pset_`/`Qto_` set name against the declared release's catalog (#511):
//! a set using the reserved prefix that the catalog does not define is a
//! warning, `property.reserved_set_name`. Without either
//! feature, or with a runtime catalog whose edition is not installed, such
//! names are reported once as unsupported -- not checked, never silently
//! passed. `ifc-validate` itself links no catalog (ADR 0017): the facade
//! composes the two.
//!
//! Which edition a release reads is the one property edits read: IFC2X3 TC1
//! as published, IFC4 ADD2 TC1 and IFC4X3 ADD2 with the built-in
//! corrections. IFC4X1 and IFC4X2 have no catalog.

pub use ifc_validate::*;

#[cfg(all(
    feature = "schema-api",
    any(feature = "property-catalog", feature = "property-catalog-runtime")
))]
mod with_catalog {
    use ifc_model::Model;
    use ifc_schema::{Schema, SchemaVersion};
    use ifc_template_catalog::catalog::{Catalog, CatalogProfile};
    use ifc_template_catalog::definition::CatalogEdition;
    use ifc_validate::{Budget, PropertySetCatalog, Report, ValidateError, ValidateOptions};

    /// The PSD/QTO catalog of each release this build carries.
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ReleaseCatalog;

    impl ReleaseCatalog {
        fn catalog(version: SchemaVersion) -> Option<Catalog> {
            let (edition, profile) = match version {
                SchemaVersion::Ifc2x3 => (CatalogEdition::Ifc2x3Tc1, CatalogProfile::Official),
                SchemaVersion::Ifc4 => (CatalogEdition::Ifc4Add2Tc1, CatalogProfile::Corrected),
                SchemaVersion::Ifc4x3 => (CatalogEdition::Ifc4x3Add2, CatalogProfile::Corrected),
                _ => return None,
            };
            #[cfg(feature = "property-catalog")]
            {
                ifc_template_catalog::embedded::load_catalog(edition, profile).ok()
            }
            #[cfg(not(feature = "property-catalog"))]
            {
                ifc_template_catalog::runtime::load_catalog(edition, profile).ok()
            }
        }
    }

    impl PropertySetCatalog for ReleaseCatalog {
        fn defines(&self, version: SchemaVersion, name: &str) -> Option<bool> {
            Self::catalog(version).map(|catalog| catalog.get(name).is_some())
        }
    }

    /// [`ifc_validate::validate`], with set names checked against the
    /// release's catalog.
    #[must_use]
    pub fn validate(model: &Model, schema: &Schema) -> Report {
        validate_with(model, schema, Budget::DEFAULT)
    }

    /// [`ifc_validate::validate_with`], with set names checked against the
    /// release's catalog.
    #[must_use]
    pub fn validate_with(model: &Model, schema: &Schema, budget: Budget) -> Report {
        validate_with_options(model, schema, &ValidateOptions::new().budget(budget))
    }

    /// [`ifc_validate::validate_with_options`], checking set names against
    /// the release's catalog unless `options` names another one.
    #[must_use]
    pub fn validate_with_options(
        model: &Model,
        schema: &Schema,
        options: &ValidateOptions<'_>,
    ) -> Report {
        let mut options = *options;
        if options.catalog.is_none() {
            options = options.catalog(&ReleaseCatalog);
        }
        ifc_validate::validate_with_options(model, schema, &options)
    }

    /// [`ifc_validate::validate_declared`], with set names checked against
    /// the release's catalog.
    ///
    /// # Errors
    ///
    /// As [`ifc_validate::validate_declared`].
    pub fn validate_declared(model: &Model) -> Result<Report, ValidateError> {
        let token = model
            .header()
            .schema_token()
            .ok_or(ValidateError::NoSchemaDeclared)?;
        let version = SchemaVersion::from_header_token(token).ok_or_else(|| {
            // A named release with no published EXPRESS (#467) is refused
            // as such, never read with a neighbour's tables.
            if ifc_schema::UnpublishedRelease::from_header_token(token).is_some() {
                ValidateError::UnpublishedSchema(token.to_string())
            } else {
                ValidateError::UnknownSchema(token.to_string())
            }
        })?;
        let schema = ifc_schema::for_version(version)
            .map_err(|_| ValidateError::UnbundledSchema(token.to_string()))?;
        Ok(validate(model, schema))
    }
}

#[cfg(all(
    feature = "schema-api",
    any(feature = "property-catalog", feature = "property-catalog-runtime")
))]
pub use with_catalog::{
    validate, validate_declared, validate_with, validate_with_options, ReleaseCatalog,
};
