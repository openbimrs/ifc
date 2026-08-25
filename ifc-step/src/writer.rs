//! Conversion from the IFC record model into generic STEP syntax.

use crate::StepError;
use ifc_model::{EntityId, Model, Value};
use openbim_step::{
    DataRecord, DataSection, Exchange, HeaderRecord, HeaderSection, InstanceId, Parameter,
};
use std::io::Write;

pub(crate) fn write(model: &Model, output: &mut dyn Write) -> Result<(), StepError> {
    let exchange = exchange_from_model(model);
    openbim_step::write(&exchange, output).map_err(|error| StepError::Io(error.to_string()))
}

fn exchange_from_model(model: &Model) -> Exchange {
    let header = model.header();
    let header = HeaderSection {
        records: vec![
            HeaderRecord {
                name: "FILE_DESCRIPTION".into(),
                parameters: vec![
                    Parameter::List(
                        header
                            .description
                            .iter()
                            .cloned()
                            .map(Parameter::Text)
                            .collect(),
                    ),
                    Parameter::Text(header.implementation_level.clone()),
                ],
            },
            HeaderRecord {
                name: "FILE_NAME".into(),
                parameters: vec![
                    Parameter::Text(header.name.clone()),
                    Parameter::Text(header.time_stamp.clone()),
                    Parameter::List(header.author.iter().cloned().map(Parameter::Text).collect()),
                    Parameter::List(
                        header
                            .organization
                            .iter()
                            .cloned()
                            .map(Parameter::Text)
                            .collect(),
                    ),
                    Parameter::Text(header.preprocessor_version.clone()),
                    Parameter::Text(header.originating_system.clone()),
                    Parameter::Text(header.authorization.clone()),
                ],
            },
            HeaderRecord {
                name: "FILE_SCHEMA".into(),
                parameters: vec![Parameter::List(
                    header.schema.iter().cloned().map(Parameter::Text).collect(),
                )],
            },
        ],
    };
    let data = DataSection {
        records: model
            .iter()
            .map(|(id, entity)| {
                DataRecord::simple(
                    InstanceId::from(id.0),
                    entity.type_name.to_string(),
                    entity.attributes.iter().map(value_to_parameter).collect(),
                )
            })
            .collect(),
    };
    Exchange { header, data }
}

fn value_to_parameter(value: &Value) -> Parameter {
    match value {
        Value::Null => Parameter::Null,
        Value::Derived => Parameter::Derived,
        Value::Bool(value) => Parameter::Bool(*value),
        Value::LogicalUnknown => Parameter::LogicalUnknown,
        Value::Integer(value) => Parameter::Integer(value.to_string()),
        Value::Real(value) => Parameter::Real(format_real(*value)),
        Value::Text(value) => Parameter::Text(value.to_string()),
        Value::Binary(value) => Parameter::Binary(value.to_string()),
        Value::Enum(value) => Parameter::Enum(value.to_string()),
        Value::Ref(EntityId(id)) => Parameter::Ref(InstanceId::from(*id)),
        Value::List(values) => Parameter::List(values.iter().map(value_to_parameter).collect()),
        Value::Typed { type_name, value } => Parameter::Typed {
            type_name: type_name.to_string(),
            value: Box::new(value_to_parameter(value)),
        },
    }
}

#[allow(clippy::float_cmp)]
fn format_real(value: f64) -> String {
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
