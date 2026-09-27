//! Conversion from generic STEP syntax into the IFC record model.

use std::borrow::Cow;
use std::sync::Arc;

use crate::StepError;
use ifc_model::{Diagnostic, Entity, EntityId, Model, Value};
use openbim_step::{OnMalformed, Parameter, ParseOptions, StandardHeader};

/// Borrowed parser events: text is a slice of the input unless the source
/// needed rewriting, so conversion allocates once per value, not twice.
type Text<'a> = Cow<'a, str>;

pub(crate) fn parse(input: &[u8], options: ParseOptions) -> Result<Model, StepError> {
    // Records are converted as they arrive rather than collected first.
    // openbim_step::parse_with builds a Vec of every DataRecord, so the
    // generic records and the converted model are both fully resident at
    // peak. Converting inside the sink drops each record as soon as its
    // Entity exists, which removes that second copy. The borrowed event API
    // additionally hands text over as input slices, so each value is
    // allocated once, directly in its model form.
    let mut sink = ModelSink {
        model: Model::new(),
        header: openbim_step::HeaderSection::default(),
        recovering: options.on_malformed_record == OnMalformed::Skip,
        error: None,
    };
    let diagnostics = openbim_step::parse_events_borrowed(input, &mut sink, options)?;
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

impl<'a> openbim_step::EventSink<Text<'a>> for ModelSink {
    fn event(&mut self, event: openbim_step::Event<Text<'a>>) {
        if self.error.is_some() {
            return;
        }
        match event {
            // The header is a handful of records; converting them to the
            // owned form keeps HeaderSection::standard() and its upper-case
            // name matching unchanged.
            openbim_step::Event::HeaderRecord(record) => {
                self.header.records.push(owned_header_record(record));
            }
            openbim_step::Event::DataRecord(instance) => self.record(instance),
            openbim_step::Event::StartHeader
            | openbim_step::Event::EndHeader
            | openbim_step::Event::StartData
            | openbim_step::Event::EndData => {}
        }
    }
}

impl ModelSink {
    fn record(&mut self, instance: openbim_step::DataRecord<Text<'_>>) {
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

pub(crate) fn convert(
    instance: openbim_step::DataRecord<Text<'_>>,
) -> Result<(EntityId, Entity), StepError> {
    let id = instance_id(&instance.id)?;
    simple(&instance)?;
    let record = instance
        .records
        .into_iter()
        .next()
        .expect("`simple` guarantees exactly one record");
    let attributes = values(record.parameters)?;
    Ok((id, Entity::new(upper_arc(record.name), attributes)))
}

/// Checks everything [`convert`] could reject, without building a value.
///
/// The lazy reader (`crate::lazy`) validates every record with this at load
/// time and decodes it with [`convert`] later, so the two must reject
/// exactly the same records. They share every fallible step -- the id, the
/// simple-instance rule, integers, reals and references go through the same
/// helpers -- and `tests/lazy_read.rs` checks the agreement on the corpus.
pub(crate) fn validate(
    instance: &openbim_step::DataRecord<Text<'_>>,
) -> Result<EntityId, StepError> {
    let id = instance_id(&instance.id)?;
    for parameter in &simple(instance)?.parameters {
        check(parameter)?;
    }
    Ok(id)
}

fn check(parameter: &Parameter<Text<'_>>) -> Result<(), StepError> {
    match parameter {
        Parameter::Integer(value) => integer(value).map(drop),
        Parameter::Real(value) => real(value).map(drop),
        Parameter::Ref(id) => reference(id).map(drop),
        Parameter::List(items) => items.iter().try_for_each(check),
        Parameter::Typed { value, .. } => check(value),
        Parameter::Null
        | Parameter::Derived
        | Parameter::Bool(_)
        | Parameter::LogicalUnknown
        | Parameter::Text(_)
        | Parameter::Binary(_)
        | Parameter::Enum(_) => Ok(()),
    }
}

fn instance_id(id: &openbim_step::InstanceId) -> Result<EntityId, StepError> {
    id.as_str()
        .parse()
        .map(EntityId)
        .map_err(|_| StepError::Syntax {
            offset: 0,
            detail: "instance id exceeds the IFC record model range".into(),
        })
}

fn simple<'r, 'a>(
    instance: &'r openbim_step::DataRecord<Text<'a>>,
) -> Result<&'r openbim_step::Record<Text<'a>>, StepError> {
    instance.as_simple().ok_or_else(|| StepError::Syntax {
        offset: 0,
        detail: "complex STEP instances are not representable in the IFC record model".into(),
    })
}

fn integer(value: &str) -> Result<i64, StepError> {
    value.parse().map_err(|_| StepError::Syntax {
        offset: 0,
        detail: "integer exceeds the IFC record model range".into(),
    })
}

fn real(value: &str) -> Result<f64, StepError> {
    value.parse().map_err(|_| StepError::Syntax {
        offset: 0,
        detail: "real exceeds the IFC record model range".into(),
    })
}

fn reference(id: &openbim_step::InstanceId) -> Result<EntityId, StepError> {
    id.as_str()
        .parse()
        .map(EntityId)
        .map_err(|_| StepError::Syntax {
            offset: 0,
            detail: "reference id exceeds the IFC record model range".into(),
        })
}

/// Record, type and enumeration names as the owned API delivered them:
/// ASCII upper case. Already-upper names (the norm) are moved, not copied.
fn upper(name: Text<'_>) -> String {
    if name.bytes().any(|byte| byte.is_ascii_lowercase()) {
        name.to_ascii_uppercase()
    } else {
        name.into_owned()
    }
}

fn upper_arc(name: Text<'_>) -> Arc<str> {
    if name.bytes().any(|byte| byte.is_ascii_lowercase()) {
        name.to_ascii_uppercase().into()
    } else {
        Arc::from(&*name)
    }
}

fn owned_header_record(record: openbim_step::HeaderRecord<Text<'_>>) -> openbim_step::HeaderRecord {
    openbim_step::HeaderRecord {
        name: upper(record.name),
        parameters: record.parameters.into_iter().map(owned_parameter).collect(),
    }
}

fn owned_parameter(parameter: Parameter<Text<'_>>) -> Parameter {
    match parameter {
        Parameter::Null => Parameter::Null,
        Parameter::Derived => Parameter::Derived,
        Parameter::Bool(value) => Parameter::Bool(value),
        Parameter::LogicalUnknown => Parameter::LogicalUnknown,
        Parameter::Integer(value) => Parameter::Integer(value.into_owned()),
        Parameter::Real(value) => Parameter::Real(value.into_owned()),
        Parameter::Text(value) => Parameter::Text(value.into_owned()),
        Parameter::Binary(value) => Parameter::Binary(value.into_owned()),
        Parameter::Enum(value) => Parameter::Enum(upper(value)),
        Parameter::Ref(id) => Parameter::Ref(id),
        Parameter::List(values) => {
            Parameter::List(values.into_iter().map(owned_parameter).collect())
        }
        Parameter::Typed { type_name, value } => Parameter::Typed {
            type_name: upper(type_name),
            value: Box::new(owned_parameter(*value)),
        },
    }
}

pub(crate) fn apply_header(header: &mut ifc_model::header::Header, source: StandardHeader) {
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

/// Converts a parameter list into an exactly sized `Vec<Value>`.
///
/// Deliberately not `into_iter().map().collect()`: `Parameter` and `Value`
/// have the same size, so that collect reuses the parser's buffer in place,
/// and the parser grows its buffers by doubling. Every entity would then
/// keep up to twice the memory its attributes need for the life of the
/// model -- measured at +14-41% resident on real files.
fn values(parameters: Vec<Parameter<Text<'_>>>) -> Result<Vec<Value>, StepError> {
    let mut out = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        out.push(parameter_to_value(parameter)?);
    }
    Ok(out)
}

fn parameter_to_value(parameter: Parameter<Text<'_>>) -> Result<Value, StepError> {
    Ok(match parameter {
        Parameter::Null => Value::Null,
        Parameter::Derived => Value::Derived,
        Parameter::Bool(value) => Value::Bool(value),
        Parameter::LogicalUnknown => Value::LogicalUnknown,
        Parameter::Integer(value) => Value::Integer(integer(&value)?),
        Parameter::Real(value) => Value::Real(real(&value)?),
        Parameter::Text(value) => Value::Text(Arc::from(&*value)),
        Parameter::Binary(value) => Value::Binary(Arc::from(&*value)),
        Parameter::Enum(value) => Value::Enum(upper_arc(value)),
        Parameter::Ref(id) => Value::Ref(reference(&id)?),
        Parameter::List(items) => Value::List(values(items)?),
        Parameter::Typed { type_name, value } => Value::Typed {
            type_name: upper_arc(type_name),
            value: Box::new(parameter_to_value(*value)?),
        },
    })
}
