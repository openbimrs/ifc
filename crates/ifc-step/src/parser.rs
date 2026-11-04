//! Conversion from generic STEP syntax into the IFC record model.
//!
//! The nesting limit for aggregates and typed values is enforced by the
//! generic `openbim-step` parser and writer. Do not re-implement it here: a
//! second limit would disagree with the substrate's on some input.

use std::borrow::Cow;
use std::sync::Arc;

use crate::location;
use crate::StepError;
use ifc_model::{Diagnostic, DiagnosticKind, Entity, EntityId, Model, Value};
use openbim_step::{
    DiagnosticKind as StepKind, Instance, OnMalformed, Parameter, ParseOptions, Span,
    StandardHeader,
};

/// Borrowed parser events: text is a slice of the input unless the source
/// needed rewriting, so conversion allocates once per value, not twice.
type Text<'a> = Cow<'a, str>;

/// Reads `input` eagerly under `options`.
///
/// A REAL written without its decimal point (`1E-05`, #285) is the
/// substrate's to recognise: a strict parse refuses it with a typed error,
/// which [`StepError::from_step`] names, and a parse with
/// `ParseOptions::accept_real_without_point` (part of
/// `ParseOptions::lenient()`) reads it with a diagnostic, which
/// [`diagnostics`] words.
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
        floor: 0,
        records: 0,
        failed: None,
    };
    let diagnostics = openbim_step::parse_events_borrowed(input, &mut sink, options)
        .map_err(|error| StepError::from_step(input, error))?;
    if let Some(error) = sink.error {
        return Err(match sink.failed {
            Some((index, id)) => locate(input, error, index, &id),
            None => error,
        });
    }
    let mut model = sink.model;
    apply_header(model.header_mut(), sink.header.standard());
    // Diagnostics are only available once the parse finishes, so they are
    // appended after the records rather than interleaved with them.
    for diagnostic in self::diagnostics(input, &diagnostics, options) {
        model.push_diagnostic(diagnostic);
    }
    reserve_through(&mut model, sink.floor);
    Ok(model)
}

/// Whether [`parse`] would accept `input` under `options`, and its error
/// when not, without building a model.
///
/// The same events, the same recovery policy and the same relocation as
/// [`parse`]; each record is [`validate`]d where [`parse`] converts it, and
/// the two refuse exactly the same records.
pub(crate) fn check_input(input: &[u8], options: ParseOptions) -> Result<(), StepError> {
    let mut sink = CheckSink {
        recovering: options.on_malformed_record == OnMalformed::Skip,
        error: None,
        records: 0,
        failed: None,
    };
    openbim_step::parse_events_borrowed(input, &mut sink, options)
        .map_err(|error| StepError::from_step(input, error))?;
    match (sink.error, sink.failed) {
        (Some(error), Some((index, id))) => Err(locate(input, error, index, &id)),
        (Some(error), None) => Err(error),
        (None, _) => Ok(()),
    }
}

/// Validates records as the parser emits them; see [`check_input`].
struct CheckSink {
    recovering: bool,
    error: Option<StepError>,
    records: usize,
    failed: Option<(usize, String)>,
}

impl<'a> openbim_step::EventSink<Text<'a>> for CheckSink {
    fn event(&mut self, event: openbim_step::Event<Text<'a>>) {
        let openbim_step::Event::DataRecord(instance) = event else {
            return;
        };
        self.records += 1;
        // Recovery skips a refused record, as `ModelSink` does.
        if self.recovering || self.error.is_some() {
            return;
        }
        if let Err(error) = validate(&instance) {
            self.failed = Some((self.records - 1, instance.id.as_str().to_string()));
            self.error = Some(error);
        }
    }
}

/// Puts a conversion error at the offset of the record it concerns.
///
/// Conversion works on parsed records, which carry no offset, so such an error
/// says offset 0. The failing record is the `index`-th data record; `scan`
/// frames the same records, and the id guards against a disagreement.
fn locate(input: &[u8], error: StepError, index: usize, id: &str) -> StepError {
    let StepError::Syntax { detail, .. } = error else {
        return error;
    };
    let offset = openbim_step::scan(input)
        .ok()
        .and_then(|scanned| scanned.records().nth(index)?.ok())
        .filter(|record| record.id.as_str() == id)
        .map_or(0, |record| record.span.start);
    StepError::Syntax { offset, detail }
}

/// Keeps new entities off every id the source declared or referenced.
///
/// `floor` is the highest id the records declared (skipped ones included) or
/// referenced; the highest id a diagnostic names is added. Without this a
/// dropped record's id, or the id a dangling reference points at, would be
/// handed out by `push` and silently capture the reference.
pub(crate) fn reserve_through(model: &mut Model, floor: u64) {
    let named = model
        .diagnostics()
        .iter()
        .flat_map(|d| [d.instance(), d.referrer()])
        .flatten()
        .map(|id| id.0)
        .filter(|id| *id != u64::MAX)
        .max()
        .unwrap_or(0);
    // `u64::MAX` is no model id (see `model_id`); never reserve it.
    let floor = floor.max(named).min(u64::MAX - 1);
    if floor > 0 {
        model.reserve_ids_through(EntityId(floor));
    }
}

/// Decodes the record framed by `span` under `options`, as the lazy reader
/// and [`crate::Index`] do, with the record's diagnostics.
pub(crate) fn decode(
    input: &[u8],
    span: openbim_step::Span,
    options: ParseOptions,
) -> Result<
    (
        openbim_step::DataRecord<Text<'_>>,
        Vec<openbim_step::Diagnostic>,
    ),
    StepError,
> {
    openbim_step::decode_record_borrowed_with(input, span, options)
        .map_err(|error| StepError::from_step(input, error))
}

/// Substrate diagnostics over `input` as model diagnostics, in order.
///
/// Each carries its kind, the bytes it covers, their line and column and a
/// quote of them. The instance at fault is the substrate's when it names
/// one. A skipped record's is the id the substrate read, also when a record
/// that was read holds it; none when no id was read or it does not fit the
/// model's range. A dangling reference names the missing id as its instance
/// and the record holding it, decoded from the span under `options`, as its
/// referrer. A REAL read without its decimal point quotes the token as
/// written.
pub(crate) fn diagnostics(
    input: &[u8],
    found: &[openbim_step::Diagnostic],
    options: ParseOptions,
) -> Vec<Diagnostic> {
    let starts: Vec<usize> = found.iter().map(|d| d.span().start).collect();
    let positions = location::positions(input, &starts);
    found
        .iter()
        .zip(positions)
        .map(|(found, position)| {
            let span = found.span();
            let range = span.start..span.end;
            let own = found.instance().and_then(|id| model_id(id.as_str()));
            let (kind, instance, referrer) = match found.kind() {
                // TODO(#668): a skip that swallowed several records names only
                // the first (openbimrs/step#20).
                StepKind::SkippedRecord => (DiagnosticKind::SkippedRecord, own.map(EntityId), None),
                StepKind::DuplicateId => (DiagnosticKind::DuplicateId, own.map(EntityId), None),
                StepKind::DanglingReference => (
                    DiagnosticKind::DanglingReference,
                    own.map(EntityId),
                    referrer(input, span, options),
                ),
                StepKind::RealWithoutPoint => (DiagnosticKind::RealWithoutPoint, None, None),
                _ => (DiagnosticKind::Other, None, None),
            };
            let excerpt = location::excerpt_of(input, range.clone());
            let detail = match (found.kind(), input.get(range.clone())) {
                (StepKind::RealWithoutPoint, Some(token)) => format!(
                    "read REAL `{}` without the decimal point ISO 10303-21 requires",
                    String::from_utf8_lossy(token)
                ),
                _ => found.detail().to_owned(),
            };
            let mut diagnostic = Diagnostic::new(kind, detail)
                .with_byte_range(range)
                .with_position(position)
                .with_excerpt(excerpt);
            if let Some(id) = instance {
                diagnostic = diagnostic.with_instance(id);
            }
            if let Some(id) = referrer {
                diagnostic = diagnostic.with_referrer(id);
            }
            diagnostic
        })
        .collect()
}

/// The id of the record framed by `span`, decoded under `options`.
fn referrer(input: &[u8], span: Span, options: ParseOptions) -> Option<EntityId> {
    let (record, _) = decode(input, span, options).ok()?;
    model_id(record.id.as_str()).map(EntityId)
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
    /// The highest id declared or referenced so far.
    floor: u64,
    /// Data records seen so far.
    records: usize,
    /// Position and id text of the record whose conversion failed.
    failed: Option<(usize, String)>,
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
        self.records += 1;
        if let Some(declared) = model_id(&id_text) {
            self.floor = self.floor.max(declared);
        }
        match convert_noting(instance, &mut self.floor) {
            Ok((id, entity)) => self.model.insert(id, entity),
            Err(error) if self.recovering => {
                let mut skipped = Diagnostic::new(
                    DiagnosticKind::SkippedRecord,
                    format!("skipped unrepresentable record #{id_text}: {error}"),
                );
                if let Some(id) = model_id(&id_text) {
                    skipped = skipped.with_instance(EntityId(id));
                }
                self.model.push_diagnostic(skipped);
            }
            Err(error) => {
                self.failed = Some((self.records - 1, id_text));
                self.error = Some(error);
            }
        }
    }
}

pub(crate) fn convert(
    instance: openbim_step::DataRecord<Text<'_>>,
) -> Result<(EntityId, Entity), StepError> {
    convert_noting(instance, &mut 0)
}

/// [`convert`], raising `refs` to the highest reference id it meets.
fn convert_noting(
    instance: openbim_step::DataRecord<Text<'_>>,
    refs: &mut u64,
) -> Result<(EntityId, Entity), StepError> {
    let id = instance_id(&instance.id)?;
    let Instance::Simple(record) = instance.instance else {
        return Err(complex());
    };
    let attributes = values(record.parameters, refs)?;
    Ok((id, Entity::new(upper_arc(record.name), attributes)))
}

/// Checks everything [`convert`] could reject, without building a value.
///
/// The lazy reader (`crate::lazy`) validates every record with this at load
/// time and decodes it with [`convert`] later, so the two must reject
/// exactly the same records. They share every fallible step -- the id, the
/// simple-instance rule, integers, reals and references go through the same
/// helpers -- and `tests/lazy_read.rs` checks the agreement on the corpus.
///
/// Returns the record's id and the highest id it references (0 for none).
pub(crate) fn validate(
    instance: &openbim_step::DataRecord<Text<'_>>,
) -> Result<(EntityId, u64), StepError> {
    let id = instance_id(&instance.id)?;
    let mut refs = 0;
    for parameter in &simple(instance)?.parameters {
        check(parameter, &mut refs)?;
    }
    Ok((id, refs))
}

fn check(parameter: &Parameter<Text<'_>>, refs: &mut u64) -> Result<(), StepError> {
    match parameter {
        Parameter::Integer(value) => integer(value).map(drop),
        Parameter::Real(value) => real(value).map(drop),
        Parameter::Ref(id) => {
            *refs = (*refs).max(reference(id)?.0);
            Ok(())
        }
        Parameter::List(items) => items.iter().try_for_each(|item| check(item, refs)),
        Parameter::Typed { value, .. } => check(value, refs),
        Parameter::Null
        | Parameter::Derived
        | Parameter::Bool(_)
        | Parameter::LogicalUnknown
        | Parameter::Text(_)
        | Parameter::Binary(_)
        | Parameter::Enum(_) => Ok(()),
    }
}

/// An instance id as the record model holds it: a `u64` below `u64::MAX`,
/// because the next free id above `u64::MAX` does not exist.
fn model_id(text: &str) -> Option<u64> {
    text.parse().ok().filter(|id| *id != u64::MAX)
}

fn instance_id(id: &openbim_step::InstanceId) -> Result<EntityId, StepError> {
    model_id(id.as_str())
        .map(EntityId)
        .ok_or_else(|| StepError::Syntax {
            offset: 0,
            detail: "instance id exceeds the IFC record model range".into(),
        })
}

fn simple<'r, 'a>(
    instance: &'r openbim_step::DataRecord<Text<'a>>,
) -> Result<&'r openbim_step::Record<Text<'a>>, StepError> {
    instance.as_simple().ok_or_else(complex)
}

fn complex() -> StepError {
    StepError::Syntax {
        offset: 0,
        detail: "complex STEP instances are not representable in the IFC record model".into(),
    }
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
    model_id(id.as_str())
        .map(EntityId)
        .ok_or_else(|| StepError::Syntax {
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
        name: upper(record.name).into(),
        parameters: record
            .parameters
            .into_vec()
            .into_iter()
            .map(owned_parameter)
            .collect(),
    }
}

fn owned_parameter(parameter: Parameter<Text<'_>>) -> Parameter {
    match parameter {
        Parameter::Null => Parameter::Null,
        Parameter::Derived => Parameter::Derived,
        Parameter::Bool(value) => Parameter::Bool(value),
        Parameter::LogicalUnknown => Parameter::LogicalUnknown,
        Parameter::Integer(value) => Parameter::Integer((*value).into()),
        Parameter::Real(value) => Parameter::Real((*value).into()),
        Parameter::Text(value) => Parameter::Text((*value).into()),
        Parameter::Binary(value) => Parameter::Binary((*value).into()),
        Parameter::Enum(value) => Parameter::Enum(upper(value).into()),
        Parameter::Ref(id) => Parameter::Ref(id),
        Parameter::List(values) => {
            Parameter::List(values.into_vec().into_iter().map(owned_parameter).collect())
        }
        Parameter::Typed { type_name, value } => Parameter::Typed {
            type_name: upper(type_name).into(),
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
fn values(parameters: Box<[Parameter<Text<'_>>]>, refs: &mut u64) -> Result<Vec<Value>, StepError> {
    let mut out = Vec::with_capacity(parameters.len());
    for parameter in parameters.into_vec() {
        out.push(parameter_to_value(parameter, refs)?);
    }
    Ok(out)
}

fn parameter_to_value(parameter: Parameter<Text<'_>>, refs: &mut u64) -> Result<Value, StepError> {
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
        Parameter::Ref(id) => {
            let id = reference(&id)?;
            *refs = (*refs).max(id.0);
            Value::Ref(id)
        }
        Parameter::List(items) => Value::List(values(items, refs)?),
        Parameter::Typed { type_name, value } => Value::Typed {
            type_name: upper_arc(type_name),
            value: Box::new(parameter_to_value(*value, refs)?),
        },
    })
}
