//! Aggregates where the schema expects a scalar, and the reverse.
//!
//! What is caught here is the shape mismatch that actually occurs in the
//! wild: a scalar written where an aggregate belongs, or an aggregate
//! written where a scalar belongs. Both are usually a writer that guessed
//! the slot layout. Bounds, inner nesting levels and element uniqueness are
//! `bounds`'s findings.

use ifc_model::{Model, Value};
use ifc_schema::Schema;

use crate::report::{Finding, Path, Report};
use crate::type_check::aliased_aggregate;

/// Reports scalar/aggregate shape mismatches against the schema.
pub fn aggregate_shape(model: &Model, schema: &Schema, report: &mut Report) {
    let mut ids: Vec<_> = model.iter().map(|(id, _)| id).collect();
    ids.sort_unstable();
    for id in ids {
        let Some(entity) = model.get(id) else {
            continue;
        };
        let declared = schema.attributes(&entity.type_name);
        for (index, value) in entity.attributes.iter().enumerate() {
            let Some(attribute) = declared.get(index) else {
                continue;
            };
            // `$` and `*` carry no shape; optionality and derivation are
            // checked in `required`.
            if matches!(value, Value::Null | Value::Derived) {
                continue;
            }
            let is_list = matches!(value.unwrap_typed(), Value::List(_));
            // A defined type can itself be an aggregate: IFC4 declares
            // `IfcComplexNumber = ARRAY [1:2] OF REAL` and four others. Such
            // an attribute is declared scalar yet legitimately holds a list,
            // so the declared type has to be resolved before judging shape.
            // A typed wrapper names its own type, which overrides the
            // declared slot type: `NominalValue : IfcValue` is a SELECT, and
            // only `IFCCOMPLEXNUMBER((0.,0.))` says which member was chosen.
            let effective_type = match value {
                Value::Typed { type_name, .. } => type_name.as_ref(),
                _ => attribute.type_name.as_str(),
            };
            let declared_is_aggregate =
                attribute.aggregate || aliased_aggregate(schema, effective_type).is_some();
            let path = || Path::Attribute {
                entity: id,
                index,
                name: Some(attribute.name.clone()),
            };
            if declared_is_aggregate && !is_list {
                report.push(Finding::error(
                    "structure.cardinality.expected_aggregate",
                    path(),
                    format!("{} is an aggregate, a scalar was written", attribute.name),
                ));
            } else if !declared_is_aggregate && is_list {
                report.push(Finding::error(
                    "structure.cardinality.unexpected_aggregate",
                    path(),
                    format!("{} is a scalar, an aggregate was written", attribute.name),
                ));
            }
        }
    }
}
