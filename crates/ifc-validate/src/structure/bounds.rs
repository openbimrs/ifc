//! Aggregate bounds, nesting and element uniqueness (#111).
//!
//! The schema tables record every aggregation level of an attribute with its
//! kind and bounds, outermost first: `CoordList : LIST [1:?] OF LIST [3:3]
//! OF IfcLengthMeasure` has two levels. A value is walked level by level:
//!
//! - each level must be an aggregate (`structure.aggregate.nesting`); the
//!   outermost level is `cardinality`'s, so only inner levels are judged here;
//! - its size must lie within the declared bounds -- for an `ARRAY [l:u]`,
//!   exactly `u - l + 1` elements (`structure.aggregate.too_few`,
//!   `structure.aggregate.too_many`);
//! - a `SET`, or a level declared `UNIQUE`, must not repeat an element
//!   (`structure.aggregate.duplicate`).
//!
//! A bound written as an expression (a reference to another attribute) is
//! not evaluated and constrains nothing here; no bundled release declares
//! one on an explicit attribute. An aggregate reached through a defined type
//! (`IfcLineIndex = LIST [2:?] OF ...`) keeps its bounds in the type's text
//! and is not judged by this module.

use std::collections::HashMap;

use ifc_model::{EntityId, Model, Value};
use ifc_schema::{AggregateKind, Aggregation, Bound, Schema};

use crate::report::{Finding, Path, Report};

/// How deeply a value is walked; IFC nests at most three levels.
const MAX_LEVELS: usize = 8;

/// Reports aggregate values outside their declared bounds, nesting or
/// uniqueness.
pub fn aggregate_bounds(model: &Model, schema: &Schema, report: &mut Report) {
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
            if attribute.aggregation.is_empty() {
                continue;
            }
            // The outermost shape is `cardinality`'s finding; a scalar there
            // is not walked.
            let Value::List(items) = value else {
                continue;
            };
            let site = Site {
                entity: id,
                index,
                name: &attribute.name,
            };
            level(&site, &attribute.aggregation, items, 0, report);
        }
    }
}

/// Where a finding is reported.
struct Site<'a> {
    entity: EntityId,
    index: usize,
    name: &'a str,
}

impl Site<'_> {
    fn path(&self) -> Path {
        Path::Attribute {
            entity: self.entity,
            index: self.index,
            name: Some(self.name.to_owned()),
        }
    }
}

fn level(
    site: &Site<'_>,
    levels: &[Aggregation],
    items: &[Value],
    depth: usize,
    report: &mut Report,
) {
    let Some(aggregation) = levels.first() else {
        return;
    };
    if depth >= MAX_LEVELS {
        return;
    }
    size(site, aggregation, items.len(), depth, report);
    if aggregation.forbids_duplicates() {
        duplicates(site, aggregation, items, depth, report);
    }
    let inner = &levels[1..];
    if inner.is_empty() {
        return;
    }
    for item in items {
        match item {
            Value::List(nested) => level(site, inner, nested, depth + 1, report),
            // An unset element of an `ARRAY OF OPTIONAL` level is legal and
            // has no shape.
            Value::Null if aggregation.optional_elements => {}
            other => report.push(Finding::error(
                "structure.aggregate.nesting",
                site.path(),
                format!(
                    "{} nests {} aggregation levels; level {} holds {} instead of an aggregate",
                    site.name,
                    levels.len() + depth,
                    depth + 2,
                    crate::type_check::describe_value(other)
                ),
            )),
        }
    }
}

/// The size an aggregation level admits, as `(minimum, maximum)`.
fn admitted(aggregation: &Aggregation) -> (Option<u64>, Option<u64>) {
    let lower = aggregation.lower.as_integer();
    let upper = match aggregation.upper {
        Bound::Unbounded => None,
        ref bound => bound.as_integer(),
    };
    if aggregation.kind == AggregateKind::Array {
        // `ARRAY [l:u]` is indexed from l to u: exactly u - l + 1 elements.
        return match (lower, upper) {
            (Some(l), Some(u)) if u >= l => (Some(u - l + 1), Some(u - l + 1)),
            _ => (None, None),
        };
    }
    (lower, upper)
}

fn size(site: &Site<'_>, aggregation: &Aggregation, len: usize, depth: usize, report: &mut Report) {
    let (minimum, maximum) = admitted(aggregation);
    let len = len as u64;
    let where_ = if depth == 0 {
        String::new()
    } else {
        format!(" at nesting level {}", depth + 1)
    };
    if let Some(minimum) = minimum.filter(|&minimum| len < minimum) {
        report.push(Finding::error(
            "structure.aggregate.too_few",
            site.path(),
            format!(
                "{}{where_} declares {}; the file wrote {len} element(s), fewer than {minimum}",
                site.name,
                declaration(aggregation)
            ),
        ));
    }
    if let Some(maximum) = maximum.filter(|&maximum| len > maximum) {
        report.push(Finding::error(
            "structure.aggregate.too_many",
            site.path(),
            format!(
                "{}{where_} declares {}; the file wrote {len} element(s), more than {maximum}",
                site.name,
                declaration(aggregation)
            ),
        ));
    }
}

fn duplicates(
    site: &Site<'_>,
    aggregation: &Aggregation,
    items: &[Value],
    depth: usize,
    report: &mut Report,
) {
    // Keyed by the value's rendering: `Value` holds reals, so it has no
    // `Eq`/`Hash`, and an equal rendering is an equal value.
    let mut first: HashMap<String, usize> = HashMap::with_capacity(items.len());
    for (position, item) in items.iter().enumerate() {
        if matches!(item, Value::Null) {
            continue;
        }
        if let Some(earlier) = first.insert(format!("{item:?}"), position) {
            let what = if aggregation.kind == AggregateKind::Set {
                "a SET"
            } else {
                "declared UNIQUE"
            };
            let level = if depth == 0 {
                String::new()
            } else {
                format!(" at nesting level {}", depth + 1)
            };
            report.push(Finding::error(
                "structure.aggregate.duplicate",
                site.path(),
                format!(
                    "{}{level} is {what}, but element {position} repeats element {earlier}",
                    site.name
                ),
            ));
            // One finding per aggregate: a list repeating one element many
            // times is one defect.
            return;
        }
    }
}

fn declaration(aggregation: &Aggregation) -> String {
    let kind = match aggregation.kind {
        AggregateKind::List => "LIST",
        AggregateKind::Set => "SET",
        AggregateKind::Bag => "BAG",
        AggregateKind::Array => "ARRAY",
        _ => "AGGREGATE",
    };
    let bound = |bound: &Bound| match bound {
        Bound::Integer(value) => value.to_string(),
        Bound::Unbounded => "?".to_owned(),
        Bound::Expression(text) => text.clone(),
        _ => "?".to_owned(),
    };
    format!(
        "{kind} [{}:{}]",
        bound(&aggregation.lower),
        bound(&aggregation.upper)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ifc_model::Entity;

    fn real(value: f64) -> Value {
        Value::Real(value)
    }

    fn rules(report: &Report) -> Vec<&str> {
        report.findings().iter().map(|f| f.rule.as_str()).collect()
    }

    fn point(coordinates: Vec<Value>) -> Report {
        let schema = ifc_schema::ifc4();
        let mut model = Model::new();
        model.push(Entity::new(
            "IFCCARTESIANPOINT",
            vec![Value::List(coordinates)],
        ));
        let mut report = Report::new();
        aggregate_bounds(&model, schema, &mut report);
        report
    }

    #[test]
    fn a_list_outside_its_bounds_is_reported() {
        assert!(rules(&point(vec![real(0.0), real(1.0)])).is_empty());
        assert_eq!(rules(&point(Vec::new())), ["structure.aggregate.too_few"]);
        assert_eq!(
            rules(&point(vec![real(0.0); 4])),
            ["structure.aggregate.too_many"]
        );
    }

    fn point_list(rows: Vec<Value>) -> Report {
        let schema = ifc_schema::ifc4();
        let mut model = Model::new();
        model.push(Entity::new(
            "IFCCARTESIANPOINTLIST3D",
            vec![Value::List(rows), Value::Null],
        ));
        let mut report = Report::new();
        aggregate_bounds(&model, schema, &mut report);
        report
    }

    #[test]
    fn a_nested_level_is_checked_for_shape_and_bounds() {
        let row = |n: usize| Value::List(vec![real(0.0); n]);
        assert!(rules(&point_list(vec![row(3), row(3)])).is_empty());
        assert_eq!(
            rules(&point_list(vec![row(3), row(2)])),
            ["structure.aggregate.too_few"]
        );
        assert_eq!(
            rules(&point_list(vec![row(3), real(1.0)])),
            ["structure.aggregate.nesting"]
        );
    }

    #[test]
    fn a_unique_list_may_not_repeat_an_element() {
        let schema = ifc_schema::ifc4();
        let run = |refs: &[u64]| {
            let mut model = Model::new();
            model.push(Entity::new(
                "IFCPOLYLOOP",
                vec![Value::List(
                    refs.iter().map(|&id| Value::Ref(EntityId(id))).collect(),
                )],
            ));
            let mut report = Report::new();
            aggregate_bounds(&model, schema, &mut report);
            report
        };
        assert!(rules(&run(&[1, 2, 3])).is_empty());
        assert_eq!(rules(&run(&[1, 2, 1])), ["structure.aggregate.duplicate"]);
    }
}
