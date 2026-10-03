//! Lazy read: validate every record at load, decode each on first access.
//!
//! # What a lazy read does
//!
//! [`read`] walks the file with `openbim_step::scan` and, for every record,
//! parses it under the read's options (`decode_record_borrowed_with`) and
//! checks everything the IFC conversion could reject
//! ([`parser::validate`]). It then registers the
//! record's byte span and type name with [`Model::insert_lazy`] and drops the
//! parsed record: no `Value` is built and nothing is inserted decoded. The
//! model keeps the source bytes; [`StepSource`] decodes a span with the same
//! [`parser::convert`] the eager reader uses the first time an entity is
//! accessed.
//!
//! # Why it cannot change the result
//!
//! - Syntax: `openbim_step::scan` + `decode_record_with` succeed for every
//!   record exactly when a whole-file parse with the same options succeeds,
//!   with the same records and the same diagnostics (that crate's
//!   invariant). The diagnostics -- under the strict policy only a REAL read
//!   without its decimal point -- are added to the model in file order, as
//!   the eager read adds them.
//! - Conversion: `validate` and `convert` share every fallible step.
//! - Anything that fails takes the eager path: the file is read again by
//!   [`parser::parse`], so an error is the eager reader's error, byte for
//!   byte, and a disagreement could only cost time, never change an answer.
//!
//! Only the abort policy without reference checks loads lazily. Recovery
//! and reference diagnostics come from the whole-file parser, so those
//! options read eagerly.

use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use ifc_model::{Entity, EntityId, EntitySource, Model};
use openbim_step::{OnMalformed, ParseOptions, Span};

use crate::{parser, StepError};

/// The bytes a lazily loaded model decodes from.
pub(crate) enum Bytes {
    /// Read into memory; the model owns them.
    Owned(Vec<u8>),
    /// A memory-mapped file. The caller of the mapped read promised it will
    /// not change while the model lives.
    Mapped(memmap2::Mmap),
}

impl Bytes {
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes,
            Self::Mapped(map) => map,
        }
    }
}

/// Decodes the entities of a lazily loaded STEP model.
pub(crate) struct StepSource {
    bytes: Bytes,
    /// The options the records were validated under.
    options: ParseOptions,
}

impl std::fmt::Debug for StepSource {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.bytes {
            Bytes::Owned(_) => "owned",
            Bytes::Mapped(_) => "mapped",
        };
        formatter
            .debug_struct("StepSource")
            .field("bytes", &kind)
            .field("len", &self.bytes.as_slice().len())
            .field("options", &self.options)
            .finish()
    }
}

impl EntitySource for StepSource {
    fn decode(&self, span: Range<usize>) -> Entity {
        const CHANGED: &str = "a validated STEP record no longer decodes: \
            the source changed after the model was read (a mapped file must \
            not be modified while its model is alive)";
        let (record, _) = parser::decode(
            self.bytes.as_slice(),
            Span::new(span.start, span.end),
            self.options,
        )
        .expect(CHANGED);
        parser::convert(record).expect(CHANGED).1
    }
}

/// Whether `options` read lazily; see the module documentation.
pub(crate) fn is_lazy(options: ParseOptions) -> bool {
    options.on_malformed_record == OnMalformed::Abort && !options.check_references
}

/// Reads `bytes` lazily under `options`, which [`is_lazy`] accepts.
pub(crate) fn read(bytes: Bytes, options: ParseOptions) -> Result<Model, StepError> {
    debug_assert!(is_lazy(options));
    let source = Arc::new(StepSource { bytes, options });
    if let Some(model) = index(&source) {
        return Ok(model);
    }
    // Some record did not validate. The eager reader reports it exactly as
    // it always has -- and should it accept the file after all, its model
    // is the right answer.
    parser::parse(source.bytes.as_slice(), options)
}

/// Inputs below this size validate on one thread: spawning costs more than
/// it saves.
const PARALLEL_BYTES: usize = 4 << 20;

/// The lazily loaded model, or `None` at the first record that the strict
/// eager read would not accept.
///
/// `scan` frames every record without parsing it. Each record is then
/// parsed from its own span and validated -- independently of every other
/// record, so the frames are split across threads -- and finally the model
/// is filled in file order, which keeps [`Model::insert_lazy`]'s
/// duplicate-id and ordering rules exactly those of the eager read.
fn index(source: &Arc<StepSource>) -> Option<Model> {
    let input = source.bytes.as_slice();
    let scanned = openbim_step::scan(input).ok()?;
    let mut frames = Vec::new();
    for record in scanned.records() {
        let record = record.ok()?;
        frames.push(record.span);
    }
    let records = validate_all(input, &frames, source.options, threads(input.len()))?;
    let shared: Arc<dyn EntitySource> = source.clone();
    let mut model = Model::with_source(shared);
    model.reserve(frames.len());
    parser::apply_header(model.header_mut(), scanned.header().standard());
    let mut diagnostics = Vec::new();
    for (span, (id, name, found)) in frames.iter().zip(records) {
        model.insert_lazy(id, &name, span.start..span.end);
        diagnostics.extend(found);
    }
    // After the records and in file order, as the eager read reports them.
    for found in &diagnostics {
        model.push_diagnostic(parser::diagnostic(input, found));
    }
    Some(model)
}

/// One validated record: its id, its type name as parsed (borrowed from
/// the input unless the source had to be rewritten) and its diagnostics.
type Validated<'a> = (EntityId, Cow<'a, str>, Vec<openbim_step::Diagnostic>);

/// Parses and validates every framed record under `options`, or `None` if
/// any record fails.
fn validate_all<'a>(
    input: &'a [u8],
    frames: &[Span],
    options: ParseOptions,
    threads: usize,
) -> Option<Vec<Validated<'a>>> {
    let one = |span: &Span| -> Option<Validated<'a>> {
        let (record, found) = parser::decode(input, *span, options).ok()?;
        let id = parser::validate(&record).ok()?;
        let openbim_step::Instance::Simple(record) = record.instance else {
            return None;
        };
        Some((id, record.name, found))
    };
    if threads <= 1 || frames.len() < 2 {
        return frames.iter().map(one).collect();
    }
    let chunk = frames.len().div_ceil(threads);
    std::thread::scope(|scope| {
        let parts: Vec<_> = frames
            .chunks(chunk)
            .map(|part| scope.spawn(move || part.iter().map(one).collect::<Option<Vec<_>>>()))
            .collect();
        let mut all = Vec::with_capacity(frames.len());
        for part in parts {
            all.extend(part.join().ok()??);
        }
        Some(all)
    })
}

/// Threads for validating an input of `len` bytes.
fn threads(len: usize) -> usize {
    if cfg!(target_family = "wasm") || len < PARALLEL_BYTES {
        return 1;
    }
    std::thread::available_parallelism().map_or(1, |n| n.get().min(8))
}
