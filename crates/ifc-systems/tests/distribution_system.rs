//! #231: `System::long_name` and `System::predefined_type` for
//! `IfcDistributionSystem` and `IfcDistributionCircuit`, read by attribute
//! name in the declared release, under IFC4 and IFC4X3.
//!
//! Both releases declare `IfcDistributionSystem` as `IfcSystem` plus
//! `LongName : OPTIONAL IfcLabel; PredefinedType : OPTIONAL
//! IfcDistributionSystemEnum`, and `IfcDistributionCircuit` as a subtype
//! adding nothing. The fixtures are parsed from STEP so the header is real.

use ifc_model::{Codec, EntityId, Model};
use ifc_systems::{systems, System};

fn step(schema: &str, data: &str) -> Model {
    let text = format!(
        "ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((''),'2;1');\n\
         FILE_NAME('t','2026-09-30T00:00:00',(''),(''),'','','');\n\
         FILE_SCHEMA(({schema}));\nENDSEC;\nDATA;\n{data}\nENDSEC;\nEND-ISO-10303-21;\n"
    );
    let model = ifc_step::StepCodec
        .read_bytes(text.as_bytes())
        .expect("fixture parses");
    assert!(model.diagnostics().is_empty(), "{:?}", model.diagnostics());
    model
}

/// A plain system, a distribution system, a circuit, a distribution system
/// with both attributes unset, a building system and a zone. `circuit_type`
/// is the circuit's `PredefinedType` token.
fn data(circuit_type: &str) -> String {
    format!(
        "#1=IFCOWNERHISTORY($,$,$,$,$,$,$,0);
#10=IFCSYSTEM('s',#1,'Plain',$,$);
#11=IFCDISTRIBUTIONSYSTEM('d',#1,'Heating',$,$,'Heating loop 1',.HEATING.);
#12=IFCDISTRIBUTIONCIRCUIT('c',#1,'Circuit',$,$,'Circuit 1',.{circuit_type}.);
#13=IFCDISTRIBUTIONSYSTEM('e',#1,'Empty',$,$,$,$);
#14=IFCBUILDINGSYSTEM('b',#1,'Facade',$,$,.SHADING.,'Facade long');
#15=IFCZONE('z',#1,'Zone',$,$,'Zone long');"
    )
}

fn by_id(found: &[System], id: u64) -> &System {
    found
        .iter()
        .find(|s| s.id == EntityId(id))
        .unwrap_or_else(|| panic!("#{id} is a system"))
}

fn check(model: &Model, circuit_type: &str) {
    let (found, anomalies) = systems(model).unwrap();
    assert!(anomalies.is_empty(), "{anomalies:?}");
    assert_eq!(found.len(), 6);

    let heating = by_id(&found, 11);
    assert_eq!(heating.long_name.as_deref(), Some("Heating loop 1"));
    assert_eq!(heating.predefined_type.as_deref(), Some("HEATING"));

    // The subtype reads the inherited positions.
    let circuit = by_id(&found, 12);
    assert_eq!(circuit.type_name, "IFCDISTRIBUTIONCIRCUIT");
    assert_eq!(circuit.long_name.as_deref(), Some("Circuit 1"));
    assert_eq!(circuit.predefined_type.as_deref(), Some(circuit_type));

    // Left empty by the file.
    let empty = by_id(&found, 13);
    assert_eq!(empty.long_name, None);
    assert_eq!(empty.predefined_type, None);

    // Not an IfcDistributionSystem: nothing is read, even where the type
    // declares a LongName or PredefinedType of its own.
    for id in [10, 14, 15] {
        let other = by_id(&found, id);
        assert_eq!(other.long_name, None, "#{id}");
        assert_eq!(other.predefined_type, None, "#{id}");
    }
}

#[test]
fn ifc4_distribution_systems_expose_long_name_and_predefined_type() {
    check(&step("'IFC4'", &data("ELECTRICAL")), "ELECTRICAL");
}

/// `RETURN_CIRCUIT` is an IFC4X3-only `IfcDistributionSystemEnum` value.
#[test]
fn ifc4x3_distribution_systems_expose_long_name_and_predefined_type() {
    check(
        &step("'IFC4X3_ADD2'", &data("RETURN_CIRCUIT")),
        "RETURN_CIRCUIT",
    );
}

/// A wrong-kind value in a slot is not read as the attribute: an enumeration
/// is no label and text is no enumeration token.
#[test]
fn wrong_kind_values_are_not_read() {
    let model = step(
        "'IFC4'",
        "#1=IFCOWNERHISTORY($,$,$,$,$,$,$,0);
#11=IFCDISTRIBUTIONSYSTEM('d',#1,'Heating',$,$,.HEATING.,'HEATING');",
    );
    let (found, _) = systems(&model).unwrap();
    assert_eq!(found[0].long_name, None);
    assert_eq!(found[0].predefined_type, None);
}
