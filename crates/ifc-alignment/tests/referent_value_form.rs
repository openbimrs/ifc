//! Referent values carry the member of their SELECT (#201).
//!
//! `IfcPointByDistanceExpression.DistanceAlong` is an
//! `IfcCurveMeasureSelect = SELECT (IfcLengthMeasure, IfcParameterValue)`
//! and `IfcPropertySingleValue.NominalValue` an `IfcValue` SELECT, so both
//! are written as the typed parameter of the member the value is. The
//! point, the referent and `Pset_Stationing` exist only in IFC4X3, the one
//! release this authoring supports. The records are written to STEP, read
//! back with `ifc-step` and resolved by `station_equations`; the STEP text
//! is asserted directly.

use ifc_alignment::{
    axis2_placement_linear, linear_placement, point_by_distance, referent, station_equations,
    stationing, AlignmentUnits,
};
use ifc_model::codec::Codec;
use ifc_model::{Entity, EntityId, Model, Transaction, Value};
use ifc_step::StepCodec;

const UNITS: AlignmentUnits = AlignmentUnits {
    length_to_metres: 1.0,
    angle_to_radians: 1.0,
};

/// The STEP record of `id`, from the written text.
fn record(text: &str, id: EntityId) -> String {
    let prefix = format!("#{}=", id.0);
    text.lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no record {prefix} in\n{text}"))
        .to_owned()
}

/// A referent at distance 125 with station 1125, an incoming station of
/// 1100 and a decreasing stationing; the written STEP text and the ids.
fn authored() -> (String, EntityId, Vec<EntityId>) {
    let mut model = Model::default();
    model.header_mut().schema = vec!["IFC4X3_ADD2".to_owned()];
    let mut tx = Transaction::new(&model);
    let curve = tx.create(Entity::new("IFCPOLYLINE", vec![Value::List(vec![])]));
    let point = point_by_distance(&mut tx, 125.0, (Some(1.5), None, None), curve).expect("point");
    let axis = axis2_placement_linear(&mut tx, point, None, None).expect("axis");
    let placement = linear_placement(&mut tx, axis, None, None).expect("placement");
    let marker = referent(
        &mut tx,
        "0aBcDeFgHiJkLmNoPqRsTu",
        None,
        Some("STATION"),
        Some(placement),
    )
    .expect("referent");
    stationing(
        &mut tx,
        "1aBcDeFgHiJkLmNoPqRsTu",
        "2aBcDeFgHiJkLmNoPqRsTu",
        marker,
        1125.0,
        Some(1100.0),
        Some(false),
    )
    .expect("stationing");
    let properties = tx
        .edits()
        .iter()
        .filter_map(|edit| match edit {
            ifc_model::Edit::Create { id, entity } if entity.is_type("IFCPROPERTYSINGLEVALUE") => {
                Some(*id)
            }
            _ => None,
        })
        .collect();
    tx.commit(&mut model).expect("commit");
    let bytes = StepCodec.write_bytes(&model).expect("written");
    (String::from_utf8(bytes).expect("utf8"), point, properties)
}

fn resolve(text: &str) -> (f64, f64, Option<f64>, bool) {
    let model = StepCodec.read_bytes(text.as_bytes()).expect("read back");
    let found = station_equations(&model, UNITS).expect("resolves");
    assert_eq!(found.len(), 1);
    let equation = &found[0];
    (
        equation.distance_along,
        equation.station,
        equation.incoming_station,
        equation.has_increasing_station,
    )
}

#[test]
fn referent_values_are_their_select_members_and_read_back() {
    let (text, point, properties) = authored();
    let line = record(&text, point);
    assert!(
        line.contains("(IFCLENGTHMEASURE(125.") && line.contains("),1.5,$,$,#"),
        "DistanceAlong is typed, the offsets bare: {line}"
    );
    let lines: Vec<String> = properties.iter().map(|id| record(&text, *id)).collect();
    assert_eq!(lines.len(), 3);
    assert!(
        lines[0].contains("('Station',$,IFCLENGTHMEASURE(1125."),
        "{}",
        lines[0]
    );
    assert!(
        lines[1].contains("('IncomingStation',$,IFCLENGTHMEASURE(1100."),
        "{}",
        lines[1]
    );
    assert!(
        lines[2].contains("('HasIncreasingStation',$,IFCBOOLEAN(.F.),$)"),
        "{}",
        lines[2]
    );
    assert_eq!(resolve(&text), (125.0, 1125.0, Some(1100.0), false));
}

/// The reader accepts the typed form written now and the bare form written
/// before #201.
#[test]
fn the_reader_accepts_both_forms() {
    let (typed, _, _) = authored();
    let mut bare = typed.clone();
    for (from, to) in [
        ("IFCLENGTHMEASURE(125.)", "125."),
        ("IFCLENGTHMEASURE(1125.)", "1125."),
        ("IFCLENGTHMEASURE(1100.)", "1100."),
        ("IFCBOOLEAN(.F.)", ".F."),
    ] {
        assert!(bare.contains(from), "{from} in\n{bare}");
        bare = bare.replace(from, to);
    }
    assert!(!bare.contains("MEASURE(") && !bare.contains("IFCBOOLEAN("));
    assert_eq!(resolve(&bare), resolve(&typed));
    assert_eq!(resolve(&bare), (125.0, 1125.0, Some(1100.0), false));
}
