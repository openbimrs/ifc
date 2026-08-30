//! Aggregates where the schema expects a scalar, and the reverse.
//!
//! # What this can and cannot check
//!
//! The EXPRESS parser records *whether* an attribute is an aggregate
//! (`LIST`/`SET`/`ARRAY`/`BAG`), not its bounds. So `LIST [1:?] OF X` and
//! `LIST [3:3] OF X` are indistinguishable here: a three-element list where
//! the schema demands exactly two is **not** caught, and this module does not
//! pretend otherwise.
//!
//! What is caught is the shape mismatch that actually occurs in the wild: a
//! scalar written where an aggregate belongs, or an aggregate written where a
//! scalar belongs. Both are usually a writer that guessed the slot layout.
//!
//! Bounds checking needs the parser to retain them. Until it does, a bounds
//! violation is unchecked rather than passed, and
//! [`crate::where_rule`] counts it.

use ifc_model::{Model, Value};
use ifc_schema::{Schema, TypeKind};

use crate::report::{Finding, Path, Report};

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
                attribute.aggregate || type_is_aggregate(schema, effective_type);
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

/// Whether a named type resolves to an aggregate declaration.
///
/// EXPRESS lets a `TYPE` alias an aggregate directly. The parser keeps such a
/// right-hand side as text, so this is a textual test against the resolved
/// alias chain rather than a structural one -- which is why it is confined to
/// this one question and not exposed.
fn type_is_aggregate(schema: &Schema, type_name: &str) -> bool {
    let mut current = type_name.to_string();
    for _ in 0..16 {
        let Some(definition) = schema.type_def(&current) else {
            return false;
        };
        let TypeKind::Defined(target) = &definition.kind else {
            return false;
        };
        let head = target.trim_start().to_ascii_uppercase();
        if head.starts_with("LIST")
            || head.starts_with("ARRAY")
            || head.starts_with("SET")
            || head.starts_with("BAG")
        {
            return true;
        }
        current = target.trim().to_string();
    }
    false
}
