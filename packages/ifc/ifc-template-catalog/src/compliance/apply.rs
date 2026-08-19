use crate::definition::{
    PropertyKind, PropertyTemplate, QuantityTemplate, SetTemplate, SetTemplateKind,
};

/// Adapter boundary for creating authored data from a catalog template.
///
/// A sink owns transactionality: if a callback fails, it must roll back any
/// partial authored state itself.
pub trait TemplateSink {
    type Error;
    fn begin(&mut self, template: &SetTemplate) -> Result<(), Self::Error>;
    fn property(&mut self, path: &str, property: &PropertyTemplate) -> Result<(), Self::Error>;
    fn quantity(&mut self, _quantity: &QuantityTemplate) -> Result<(), Self::Error> {
        Ok(())
    }
    fn finish(&mut self, template: &SetTemplate) -> Result<(), Self::Error>;
}

pub fn apply_template<S: TemplateSink>(
    template: &SetTemplate,
    sink: &mut S,
) -> Result<(), S::Error> {
    sink.begin(template)?;
    match &template.kind {
        SetTemplateKind::Property { properties, .. } => {
            walk_properties("", properties, sink)?;
        }
        SetTemplateKind::Quantity { quantities, .. } => {
            for quantity in quantities {
                sink.quantity(quantity)?;
            }
        }
    }
    sink.finish(template)
}

fn walk_properties<S: TemplateSink>(
    parent: &str,
    properties: &[PropertyTemplate],
    sink: &mut S,
) -> Result<(), S::Error> {
    for property in properties {
        let path = if parent.is_empty() {
            property.name.clone()
        } else {
            format!("{parent}.{}", property.name)
        };
        sink.property(&path, property)?;
        if let PropertyKind::Complex { properties, .. } = &property.kind {
            walk_properties(&path, properties, sink)?;
        }
    }
    Ok(())
}
