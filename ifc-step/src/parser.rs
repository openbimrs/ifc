//! Conversion from generic STEP syntax into the IFC record model.

use crate::StepError;
use ifc_model::{Diagnostic, Entity, EntityId, Model, Value};
use openbim_step::{OnMalformed, Parameter, ParseOptions, StandardHeader};

pub(crate) fn parse(input: &[u8], options: ParseOptions) -> Result<Model, StepError> {
    let outcome = openbim_step::parse_with(input, options)?;
    let mut model = Model::new();
    apply_header(model.header_mut(), outcome.exchange.header.standard());

    for diagnostic in &outcome.diagnostics {
        model.push_diagnostic(Diagnostic::warning(
            diagnostic.span().start..diagnostic.span().end,
            diagnostic.detail(),
        ));
    }

    let recovering = options.on_malformed_record == OnMalformed::Skip;
    for instance in outcome.exchange.data.records {
        // A record can be syntactically valid STEP yet unrepresentable in the
        // IFC record model (an out-of-range id, a complex instance). Under the
        // recovery policy that is the same class of problem as a damaged
        // record and is reported rather than fatal.
        let id_text = instance.id.as_str().to_string();
        match convert(instance) {
            Ok((id, entity)) => model.insert(id, entity),
            Err(error) if recovering => model.push_diagnostic(Diagnostic::unlocated(format!(
                "skipped unrepresentable record #{id_text}: {error}"
            ))),
            Err(error) => return Err(error),
        }
    }
    Ok(model)
}

fn convert(instance: openbim_step::DataRecord) -> Result<(EntityId, Entity), StepError> {
    let id = instance
        .id
        .as_str()
        .parse()
        .map_err(|_| StepError::Syntax {
            offset: 0,
            detail: "instance id exceeds the IFC record model range".into(),
        })?;
    let record = instance.as_simple().ok_or_else(|| StepError::Syntax {
        offset: 0,
        detail: "complex STEP instances are not representable in the IFC record model".into(),
    })?;
    let attributes = record
        .parameters
        .clone()
        .into_iter()
        .map(parameter_to_value)
        .collect::<Result<Vec<_>, _>>()?;
    Ok((EntityId(id), Entity::new(record.name.clone(), attributes)))
}

fn apply_header(header: &mut ifc_model::header::Header, source: StandardHeader) {
    if let Some(value) = source.description {
        header.description = value;
    }
    if let Some(value) = source.implementation_level {
        header.implementation_level = value;
    }
    if let Some(value) = source.name {
        header.name = value;
    }
    if let Some(value) = source.time_stamp {
        header.time_stamp = value;
    }
    if let Some(value) = source.author {
        header.author = value;
    }
    if let Some(value) = source.organization {
        header.organization = value;
    }
    if let Some(value) = source.preprocessor_version {
        header.preprocessor_version = value;
    }
    if let Some(value) = source.originating_system {
        header.originating_system = value;
    }
    if let Some(value) = source.authorization {
        header.authorization = value;
    }
    if let Some(value) = source.schema {
        header.schema = value;
    }
}

fn parameter_to_value(parameter: Parameter) -> Result<Value, StepError> {
    Ok(match parameter {
        Parameter::Null => Value::Null,
        Parameter::Derived => Value::Derived,
        Parameter::Bool(value) => Value::Bool(value),
        Parameter::LogicalUnknown => Value::LogicalUnknown,
        Parameter::Integer(value) => {
            Value::Integer(value.parse().map_err(|_| StepError::Syntax {
                offset: 0,
                detail: "integer exceeds the IFC record model range".into(),
            })?)
        }
        Parameter::Real(value) => Value::Real(value.parse().map_err(|_| StepError::Syntax {
            offset: 0,
            detail: "real exceeds the IFC record model range".into(),
        })?),
        Parameter::Text(value) => Value::Text(value.into()),
        Parameter::Binary(value) => Value::Binary(value.into()),
        Parameter::Enum(value) => Value::Enum(value.into()),
        Parameter::Ref(id) => Value::Ref(EntityId(id.as_str().parse().map_err(|_| {
            StepError::Syntax {
                offset: 0,
                detail: "reference id exceeds the IFC record model range".into(),
            }
        })?)),
        Parameter::List(values) => Value::List(
            values
                .into_iter()
                .map(parameter_to_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Parameter::Typed { type_name, value } => Value::Typed {
            type_name: type_name.into(),
            value: Box::new(parameter_to_value(*value)?),
        },
    })
}
