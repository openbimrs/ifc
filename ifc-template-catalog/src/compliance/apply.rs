use crate::definition::{
    PropertyKind, PropertyTemplate, QuantityTemplate, SetTemplate, SetTemplateKind,
};

/// Adapter boundary for creating authored data from a catalog template.
///
/// A sink owns transactionality: if a callback fails, it must roll back any
/// partial authored state itself.
pub trait TemplateSink {
    /// Failure type raised by any callback; also the type the sink rolls back on.
    type Error;
    /// Called once before any member callback for `template`. Sinks that
    /// open a transaction or authored-set shell should do it here.
    fn begin(&mut self, template: &SetTemplate) -> Result<(), Self::Error>;
    /// Called once per property in the template, `path` is the dotted
    /// ancestor chain for a nested `Complex` property (e.g. `"Usage.Sub"`).
    fn property(&mut self, path: &str, property: &PropertyTemplate) -> Result<(), Self::Error>;
    /// Called once per quantity in a quantity-set template. Default is a
    /// no-op, so sinks that only author property sets need not implement it.
    fn quantity(&mut self, _quantity: &QuantityTemplate) -> Result<(), Self::Error> {
        Ok(())
    }
    /// Called once after every member callback for `template` has returned `Ok`.
    fn finish(&mut self, template: &SetTemplate) -> Result<(), Self::Error>;
}

/// Walk `template`'s properties or quantities in order, calling `sink`'s
/// `begin`/`property`/`quantity`/`finish` callbacks. Stops and returns the
/// first callback error; the sink is responsible for any rollback.
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
