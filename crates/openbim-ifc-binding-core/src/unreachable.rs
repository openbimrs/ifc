//! Products no viewer will draw (feature `unreachable`).
//!
//! The facade's `unreachable_products` joins the spatial tree and the
//! representation contexts. Its findings cross as plain records with a
//! stable `reason` code; without the feature the operation refuses with
//! `feature-disabled`.

use crate::{BindingError, IfcModel};

/// One product a viewer will not draw, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreachableProduct {
    /// The product's entity id.
    pub id: u64,
    /// Stable reason code: `not-contained-in-spatial-structure`,
    /// `no-representation-in-model-context` or
    /// `representation-without-context`.
    pub reason: String,
    /// For `no-representation-in-model-context`: the target views the
    /// product's geometry was found in instead. Empty otherwise.
    pub found_views: Vec<String>,
    /// A one-line explanation for a CLI or a UI warning.
    pub message: String,
}

impl IfcModel {
    /// Products that no viewer will draw, with the reason, in id order.
    ///
    /// Openings, assembly parts, spatial containers and products without a
    /// representation are not reported: they legitimately sit outside the
    /// containment tree.
    pub fn unreachable_products(&self) -> Result<Vec<UnreachableProduct>, BindingError> {
        #[cfg(feature = "unreachable")]
        {
            use ifc::Unreachable;
            Ok(ifc::unreachable_products(&self.inner)
                .into_iter()
                .map(|(id, why)| {
                    let message = why.message();
                    let (reason, found_views) = match why {
                        Unreachable::NotContainedInSpatialStructure => {
                            ("not-contained-in-spatial-structure", Vec::new())
                        }
                        Unreachable::NoRepresentationInModelContext { found } => {
                            ("no-representation-in-model-context", found)
                        }
                        Unreachable::RepresentationWithoutContext => {
                            ("representation-without-context", Vec::new())
                        }
                        // A reason added after this binding: the message
                        // still explains it.
                        _ => ("other", Vec::new()),
                    };
                    UnreachableProduct {
                        id: id.0,
                        reason: reason.to_owned(),
                        found_views,
                        message,
                    }
                })
                .collect())
        }
        #[cfg(not(feature = "unreachable"))]
        {
            Err(BindingError::FeatureDisabled("unreachable"))
        }
    }
}

/// The products as one tagged `list`, each a `list` of four values: id
/// (`ref`), reason (text), found views (`list` of text) and
/// message (text). The C binding's tape form.
pub fn products_to_tagged(products: &[UnreachableProduct]) -> crate::value::Tagged {
    use crate::value::Tagged;
    Tagged::List(
        products
            .iter()
            .map(|product| {
                Tagged::List(vec![
                    Tagged::Ref(product.id),
                    Tagged::Text(product.reason.clone()),
                    Tagged::List(
                        product
                            .found_views
                            .iter()
                            .map(|view| Tagged::Text(view.clone()))
                            .collect(),
                    ),
                    Tagged::Text(product.message.clone()),
                ])
            })
            .collect(),
    )
}
