//! Boundary tests for the `replace` and propagation fields of the
//! authoring tape (#480), called exactly as C would call
//! `openbim_ifc_v0_1_model_author`: a `BOOL` field moves an element to
//! another container, its absence refuses, and a malformed flag or mode
//! is `InvalidValue`. The propagation itself is pinned in `openbim-ifc`.

use openbim_ifc_binding_core::value::Tagged;

use crate::capability_tests::{last_code, parse};
use crate::tape::Tape;
use crate::*;

fn fixture() -> OpenbimIfcModel {
    let path = format!(
        "{}/../../test/fixtures/synthetic-relations/cascade_ifc4.ifc",
        env!("CARGO_MANIFEST_DIR")
    );
    parse(&std::fs::read(path).expect("fixture"))
}

/// Run one operation: its ENUM name, then (name, value) fields.
fn author(model: OpenbimIfcModel, op: &str, fields: Vec<(&str, Tagged)>) -> OpenbimIfcStatus {
    let mut items = vec![Tagged::Enum(op.into())];
    for (name, value) in fields {
        items.push(Tagged::Text(name.into()));
        items.push(value);
    }
    let tape = Tape::encode(&Tagged::List(vec![Tagged::List(items)]));
    let mut ids = [0u64; 1];
    let mut count = 0;
    // SAFETY: the tape's buffers and the out-pointers are valid.
    unsafe {
        openbim_ifc_v0_1_model_author(
            model,
            tape.nodes.as_ptr(),
            tape.nodes.len(),
            tape.strings.as_ptr(),
            tape.strings.len(),
            ids.as_mut_ptr(),
            ids.len(),
            &mut count,
        )
    }
}

#[test]
fn a_bool_replace_moves_an_element_and_its_absence_refuses() {
    let model = fixture();
    // Wall B (#40) is contained in the storey (#14); the building (#13)
    // is another spatial structure element.
    let contain = |replace: Option<Tagged>| {
        let mut fields = vec![
            ("structure", Tagged::Ref(13)),
            ("elements", Tagged::List(vec![Tagged::Ref(40)])),
        ];
        if let Some(replace) = replace {
            fields.push(("replace", replace));
        }
        author(model, "CONTAIN", fields)
    };
    assert_eq!(contain(None), OpenbimIfcStatus::InvalidModel);
    assert_eq!(last_code(model), "invalid-model");
    assert_eq!(
        contain(Some(Tagged::Bool(false))),
        OpenbimIfcStatus::InvalidModel
    );
    assert_eq!(
        contain(Some(Tagged::Integer(1))),
        OpenbimIfcStatus::InvalidValue
    );
    assert_eq!(contain(Some(Tagged::Bool(true))), OpenbimIfcStatus::Ok);
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}

#[test]
fn assign_type_reads_its_flags_and_modes() {
    let model = fixture();
    let assign = |extra: Vec<(&'static str, Tagged)>| {
        let mut fields = vec![
            ("type_object", Tagged::Ref(66)),
            ("objects", Tagged::List(vec![Tagged::Ref(40)])),
        ];
        fields.extend(extra);
        author(model, "ASSIGN_TYPE", fields)
    };
    // Wall B is already typed by #66.
    assert_eq!(assign(vec![]), OpenbimIfcStatus::InvalidModel);
    assert_eq!(
        assign(vec![("predefined_type", Tagged::Text("always".into()))]),
        OpenbimIfcStatus::InvalidValue
    );
    assert_eq!(
        assign(vec![("layer_set_direction", Tagged::Text("AXIS4".into()))]),
        OpenbimIfcStatus::InvalidValue
    );
    assert_eq!(
        assign(vec![
            ("replace", Tagged::Bool(true)),
            ("map_representations", Tagged::Bool(true)),
            ("predefined_type", Tagged::Text("validate".into())),
        ]),
        OpenbimIfcStatus::Ok
    );
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}
