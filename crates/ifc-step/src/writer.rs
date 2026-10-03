//! Conversion from the IFC record model into generic STEP syntax.

use crate::StepError;
use ifc_model::{EntityId, Model, Value};
use openbim_step::{
    DataRecord, DataSection, Exchange, HeaderRecord, HeaderSection, InstanceId, Parameter, Str,
};
use std::io::Write;

pub(crate) fn write(model: &Model, output: &mut dyn Write) -> Result<(), StepError> {
    reject_non_finite_reals(model)?;
    let exchange = exchange_from_model(model);
    openbim_step::write(&exchange, output).map_err(|error| StepError::Io(error.to_string()))
}

fn reject_non_finite_reals(model: &Model) -> Result<(), StepError> {
    for (id, entity) in model.iter() {
        for (slot, value) in entity.attributes.iter().enumerate() {
            if contains_non_finite_real(value) {
                return Err(StepError::NonFiniteReal { entity: id, slot });
            }
        }
    }
    Ok(())
}

fn contains_non_finite_real(value: &Value) -> bool {
    match value {
        Value::Real(value) => !value.is_finite(),
        Value::List(values) => values.iter().any(contains_non_finite_real),
        Value::Typed { value, .. } => contains_non_finite_real(value),
        _ => false,
    }
}

fn exchange_from_model(model: &Model) -> Exchange {
    let header = model.header();
    let header = HeaderSection {
        records: vec![
            HeaderRecord {
                name: "FILE_DESCRIPTION".into(),
                parameters: Box::new([
                    Parameter::List(header.description.iter().map(text).collect()),
                    text(&header.implementation_level),
                ]),
            },
            HeaderRecord {
                name: "FILE_NAME".into(),
                parameters: Box::new([
                    text(&header.name),
                    text(&header.time_stamp),
                    Parameter::List(header.author.iter().map(text).collect()),
                    Parameter::List(header.organization.iter().map(text).collect()),
                    text(&header.preprocessor_version),
                    text(&header.originating_system),
                    text(&header.authorization),
                ]),
            },
            HeaderRecord {
                name: "FILE_SCHEMA".into(),
                parameters: Box::new([Parameter::List(header.schema.iter().map(text).collect())]),
            },
        ],
    };
    let data = DataSection {
        records: model
            .iter()
            .map(|(id, entity)| {
                DataRecord::simple(
                    InstanceId::from(id.0),
                    Str::from(entity.type_name.clone()),
                    entity
                        .attributes
                        .iter()
                        .map(value_to_parameter)
                        .collect::<Box<[_]>>(),
                )
            })
            .collect(),
    };
    Exchange { header, data }
}

fn text(value: impl AsRef<str>) -> Parameter {
    Parameter::Text(value.as_ref().into())
}

fn value_to_parameter(value: &Value) -> Parameter {
    match value {
        Value::Null => Parameter::Null,
        Value::Derived => Parameter::Derived,
        Value::Bool(value) => Parameter::Bool(*value),
        Value::LogicalUnknown => Parameter::LogicalUnknown,
        Value::Integer(value) => Parameter::Integer(value.to_string().into()),
        Value::Real(value) => Parameter::Real(format_real(*value).into()),
        Value::Text(value) => Parameter::Text(Str::from(value.clone())),
        Value::Binary(value) => Parameter::Binary(Str::from(value.clone())),
        Value::Enum(value) => Parameter::Enum(Str::from(value.clone())),
        Value::Ref(EntityId(id)) => Parameter::Ref(InstanceId::from(*id)),
        Value::List(values) => Parameter::List(values.iter().map(value_to_parameter).collect()),
        Value::Typed { type_name, value } => Parameter::Typed {
            type_name: Str::from(type_name.clone()),
            value: Box::new(value_to_parameter(value)),
        },
    }
}

#[allow(clippy::float_cmp)]
fn format_real(value: f64) -> String {
    if value == 0.0 && value.is_sign_negative() {
        return "-0.".into();
    }
    if value == value.trunc() && value.abs() < 1e15 {
        #[allow(clippy::cast_possible_truncation)]
        let integer = value.trunc() as i64;
        format!("{integer}.")
    } else {
        let text = value.to_string();
        if text.contains(['.', 'e', 'E']) {
            text
        } else {
            format!("{text}.")
        }
    }
}
