//! Boundary tests for unit conversion and `convert_length_unit` (#469),
//! called exactly as C would call them. The answers are pinned in
//! `openbim-ifc-binding-core`; these check the calling convention, that a
//! size query and a short buffer apply nothing, and that the applied report
//! fits the sizes the query asked for.

use std::ptr::{null, null_mut};

use openbim_ifc_binding_core::value::Tagged;

use crate::capability_tests::{last_code, parse};
use crate::tape::Reader;
use crate::*;

const MODEL: &str = "ISO-10303-21;
HEADER;
FILE_DESCRIPTION((''),'2;1');
FILE_NAME('','',(''),(''),'','','');
FILE_SCHEMA(('IFC4'));
ENDSEC;
DATA;
#1=IFCSIUNIT(*,.LENGTHUNIT.,.MILLI.,.METRE.);
#2=IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.);
#3=IFCMONETARYUNIT('EUR');
#4=IFCUNITASSIGNMENT((#1,#2,#3));
#5=IFCPROJECT('0000000000000000000005',$,'P',$,$,$,$,(#6),#4);
#6=IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,0.01,#7,$);
#7=IFCAXIS2PLACEMENT3D(#8,$,$);
#8=IFCCARTESIANPOINT((1000.,2500.,0.));
ENDSEC;
END-ISO-10303-21;
";

fn convert(value: f64, from: &str, to: &str) -> Result<f64, OpenbimIfcStatus> {
    let mut out = 0.0;
    // SAFETY: the strings and the out-pointer outlive the call.
    let status = unsafe {
        openbim_ifc_v0_1_convert_unit(
            value,
            from.as_ptr(),
            from.len(),
            to.as_ptr(),
            to.len(),
            &mut out,
        )
    };
    if status == OpenbimIfcStatus::Ok {
        Ok(out)
    } else {
        Err(status)
    }
}

fn format(
    value: f64,
    unit: &str,
    style: u32,
    places: u32,
    display: Option<&str>,
    flags: u32,
) -> Result<String, OpenbimIfcStatus> {
    let (display_ptr, display_len) = display.map_or((null(), 0), |d| (d.as_ptr(), d.len()));
    let mut need = 0;
    // SAFETY: the size query: null buffer, capacity 0.
    let status = unsafe {
        openbim_ifc_v0_1_format_length(
            value,
            unit.as_ptr(),
            unit.len(),
            style,
            places,
            display_ptr,
            display_len,
            flags,
            null_mut(),
            0,
            &mut need,
        )
    };
    if status != OpenbimIfcStatus::BufferTooSmall {
        return Err(status);
    }
    let mut buffer = vec![0u8; need];
    // SAFETY: the buffer holds `need` bytes.
    let status = unsafe {
        openbim_ifc_v0_1_format_length(
            value,
            unit.as_ptr(),
            unit.len(),
            style,
            places,
            display_ptr,
            display_len,
            flags,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut need,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    buffer.pop();
    Ok(String::from_utf8(buffer).unwrap())
}

fn count(model: OpenbimIfcModel) -> usize {
    let mut count = 0;
    // SAFETY: valid out-pointer.
    unsafe { openbim_ifc_v0_1_model_len(model, &mut count) };
    count
}

/// The size query, then the call with exactly the sizes it asked for.
fn convert_length_unit(
    model: OpenbimIfcModel,
    target: &str,
    flags: u32,
) -> Result<Tagged, OpenbimIfcStatus> {
    let (mut need_nodes, mut need_strings) = (0, 0);
    // SAFETY: null buffers with capacity 0 are the size query.
    let status = unsafe {
        openbim_ifc_v0_1_model_convert_length_unit(
            model,
            target.as_ptr(),
            target.len(),
            flags,
            null_mut(),
            0,
            &mut need_nodes,
            null_mut(),
            0,
            &mut need_strings,
        )
    };
    if status != OpenbimIfcStatus::BufferTooSmall {
        return Err(status);
    }
    let mut nodes = vec![OpenbimIfcValueNode::default(); need_nodes];
    let mut strings = vec![0u8; need_strings];
    // SAFETY: the buffers hold the sizes asked for.
    let status = unsafe {
        openbim_ifc_v0_1_model_convert_length_unit(
            model,
            target.as_ptr(),
            target.len(),
            flags,
            nodes.as_mut_ptr(),
            nodes.len(),
            &mut need_nodes,
            strings.as_mut_ptr(),
            strings.len(),
            &mut need_strings,
        )
    };
    if status != OpenbimIfcStatus::Ok {
        return Err(status);
    }
    Ok(Reader::new(&nodes[..need_nodes], &strings[..need_strings])
        .single()
        .expect("a well-formed tape"))
}

fn field(record: &Tagged, index: usize) -> &Tagged {
    let Tagged::List(fields) = record else {
        panic!("a record: {record:?}");
    };
    &fields[index]
}

#[test]
fn units_convert_and_format_without_a_model() {
    assert_eq!(convert(1.0, "foot", "MILLIMETRE"), Ok(304.8));
    assert_eq!(convert(1500.0, "MILLI METRE", "METRE"), Ok(1.5));
    assert_eq!(
        convert(1.0, "METRE", "SQUARE_METRE"),
        Err(OpenbimIfcStatus::TypeMismatch)
    );
    assert_eq!(
        convert(1.0, "furlong", "METRE"),
        Err(OpenbimIfcStatus::InvalidValue)
    );
    assert_eq!(
        convert(1.0, "degree Fahrenheit", "KELVIN"),
        Err(OpenbimIfcStatus::Unsupported)
    );
    let bad = [0xff_u8];
    let mut out = 0.0;
    // SAFETY: valid pointers and lengths.
    let status =
        unsafe { openbim_ifc_v0_1_convert_unit(1.0, bad.as_ptr(), 1, bad.as_ptr(), 1, &mut out) };
    assert_eq!(status, OpenbimIfcStatus::InvalidArgument);

    assert_eq!(
        format(
            444.5,
            "MILLIMETRE",
            OPENBIM_IFC_LENGTH_FORMAT_FEET_INCHES,
            8,
            None,
            OPENBIM_IFC_LENGTH_FORMAT_SUPPRESS_ZERO_INCHES
        )
        .as_deref(),
        Ok("1' - 5 1/2\"")
    );
    assert_eq!(
        format(
            1250.0,
            "MILLIMETRE",
            OPENBIM_IFC_LENGTH_FORMAT_DECIMAL,
            2,
            Some("METRE"),
            0
        )
        .as_deref(),
        Ok("1.25")
    );
    assert_eq!(
        format(66.5, "inch", OPENBIM_IFC_LENGTH_FORMAT_INCHES, 2, None, 0).as_deref(),
        Ok("66 1/2\"")
    );
    assert_eq!(
        format(1.0, "METRE", 9, 2, None, 0),
        Err(OpenbimIfcStatus::InvalidArgument)
    );
    assert_eq!(
        format(1.0, "METRE", 0, 2, None, 1 << 4),
        Err(OpenbimIfcStatus::InvalidArgument)
    );
}

#[test]
fn a_model_converts_its_length_unit_and_a_size_query_applies_nothing() {
    let model = parse(MODEL.as_bytes());
    let mut value = 0.0;
    // SAFETY: valid out-pointer.
    let status = unsafe { openbim_ifc_v0_1_model_convert_unit_value(model, 2.0, 1, 1, &mut value) };
    assert_eq!((status, value), (OpenbimIfcStatus::Ok, 2.0));
    // SAFETY: valid out-pointer.
    let status = unsafe { openbim_ifc_v0_1_model_convert_unit_value(model, 2.0, 3, 1, &mut value) };
    assert_eq!(status, OpenbimIfcStatus::Unsupported);
    assert_eq!(last_code(model), "unsupported");

    let before = count(model);
    let target = "METRE";
    // A short buffer applies nothing.
    let (mut need_nodes, mut need_strings) = (0, 0);
    let mut one = [OpenbimIfcValueNode::default(); 1];
    let mut byte = [0u8; 1];
    // SAFETY: the buffers hold their stated capacities.
    let status = unsafe {
        openbim_ifc_v0_1_model_convert_length_unit(
            model,
            target.as_ptr(),
            target.len(),
            0,
            one.as_mut_ptr(),
            1,
            &mut need_nodes,
            byte.as_mut_ptr(),
            1,
            &mut need_strings,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::BufferTooSmall);
    assert_eq!(count(model), before);

    let dry = convert_length_unit(model, target, OPENBIM_IFC_LENGTH_UNIT_DRY_RUN).unwrap();
    assert_eq!(field(&dry, 14), &Tagged::Bool(false));
    assert_eq!(count(model), before);

    let applied = convert_length_unit(model, target, 0).unwrap();
    assert_eq!(field(&applied, 0), &Tagged::Text("MILLI METRE".into()));
    assert_eq!(field(&applied, 2), &Tagged::Real(0.001));
    assert_eq!(field(&applied, 14), &Tagged::Bool(true));
    // The millimetre went; a metre came.
    assert_eq!(count(model), before);
    let mut nodes = vec![OpenbimIfcValueNode::default(); 8];
    let mut strings = vec![0u8; 8];
    let (mut nn, mut ns) = (0, 0);
    // SAFETY: the buffers hold their stated capacities.
    let status = unsafe {
        openbim_ifc_v0_1_entity_attribute(
            model,
            8,
            0,
            nodes.as_mut_ptr(),
            nodes.len(),
            &mut nn,
            strings.as_mut_ptr(),
            strings.len(),
            &mut ns,
        )
    };
    assert_eq!(status, OpenbimIfcStatus::Ok);
    assert_eq!(
        Reader::new(&nodes[..nn], &strings[..ns]).single().unwrap(),
        Tagged::List(vec![
            Tagged::Real(1.0),
            Tagged::Real(2.5),
            Tagged::Real(0.0)
        ])
    );

    assert_eq!(
        convert_length_unit(model, "SQUARE_METRE", 0),
        Err(OpenbimIfcStatus::InvalidValue)
    );
    assert_eq!(
        convert_length_unit(model, target, 1 << 3),
        Err(OpenbimIfcStatus::InvalidArgument)
    );
    assert_eq!(openbim_ifc_v0_1_model_destroy(model), OpenbimIfcStatus::Ok);
}
