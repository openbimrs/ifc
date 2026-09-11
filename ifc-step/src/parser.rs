//! Conversion from generic STEP syntax into the IFC record model.

use crate::StepError;
use ifc_model::{Diagnostic, Entity, EntityId, Model, Value};
use openbim_step::{OnMalformed, Parameter, ParseOptions, StandardHeader};

pub(crate) fn parse(input: &[u8], options: ParseOptions) -> Result<Model, StepError> {
    // Records are converted as they arrive rather than collected first.
    // openbim_step::parse_with builds a Vec of every DataRecord, so the
    // generic records and the converted model are both fully resident at
    // peak. Converting inside the sink drops each record as soon as its
    // Entity exists, which removes that second copy.
    let mut sink = ModelSink {
        model: Model::new(),
        header: openbim_step::HeaderSection::default(),
        recovering: options.on_malformed_record == OnMalformed::Skip,
        error: None,
    };
    let diagnostics = openbim_step::parse_events_with(input, &mut sink, options)?;
    if let Some(error) = sink.error {
        return Err(error);
    }
    let mut model = sink.model;
    apply_header(model.header_mut(), sink.header.standard());
    // Diagnostics are only available once the parse finishes, so they are
    // appended after the records rather than interleaved with them.
    for diagnostic in &diagnostics {
        model.push_diagnostic(Diagnostic::warning(
            diagnostic.span().start..diagnostic.span().end,
            diagnostic.detail(),
        ));
    }
    Ok(model)
}

/// Converts records into the model as the parser emits them.
///
/// The sink owns the model under construction. A conversion failure cannot
/// abort the parse from inside the callback, so the first one is stored and
/// re-raised by the caller once the parse returns.
struct ModelSink {
    model: Model,
    header: openbim_step::HeaderSection,
    recovering: bool,
    error: Option<StepError>,
}

impl openbim_step::EventSink for ModelSink {
    fn event(&mut self, event: openbim_step::Event) {
        if self.error.is_some() {
            return;
        }
        match event {
            openbim_step::Event::HeaderRecord(record) => self.header.records.push(record),
            openbim_step::Event::DataRecord(instance) => self.record(instance),
            openbim_step::Event::StartHeader
            | openbim_step::Event::EndHeader
            | openbim_step::Event::StartData
            | openbim_step::Event::EndData => {}
        }
    }
}

impl ModelSink {
    fn record(&mut self, instance: openbim_step::DataRecord) {
        // A record can be syntactically valid STEP yet unrepresentable in
        // the IFC record model (an out-of-range id, a complex instance).
        // Under the recovery policy that is the same class of problem as a
        // damaged record and is reported rather than fatal.
        let id_text = instance.id.as_str().to_string();
        match convert(instance) {
            Ok((id, entity)) => self.model.insert(id, entity),
            Err(error) if self.recovering => {
                self.model.push_diagnostic(Diagnostic::unlocated(format!(
                    "skipped unrepresentable record #{id_text}: {error}"
                )));
            }
            Err(error) => self.error = Some(error),
        }
    }
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
