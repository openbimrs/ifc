//! Offset index: what is in a file, without decoding it.
//!
//! # Why this exists
//!
//! Building a [`Model`] decodes every attribute of every record into an
//! owned `Value`. On a 513 MB export that is 18 s and 2.3 GB. A consumer
//! that only needs a type census, or the 200 walls out of nine million
//! records, pays the full price for work it discards.
//!
//! This module answers those questions from byte offsets alone. Records are
//! decoded one at a time, on request, from the borrowed source.
//!
//! # Trust
//!
//! The scanner finds record boundaries without running the full lexer, so it
//! could in principle disagree with it. `tests/index_agreement.rs` asserts on
//! every fixture in the repository that the index finds exactly the ids the
//! eager parser finds, and that each lazily decoded entity equals its eager
//! counterpart. That test is the reason to believe this module.

use ifc_model::{Codec, Entity, EntityId, Model, Value};
use openbim_step::ParseOptions;

/// Advances past a STEP string literal, honouring the doubled-quote escape.
///
/// Returns the offset just past the closing quote. A string can contain `#`,
/// `;`, and `(`/`)`, so a scanner that does not skip strings wholesale will
/// split records in the middle of a name and silently lose data.
/// The doubled-quote branch below is deliberate but, for boundary
/// finding alone, provably redundant: quote runs in valid STEP are always
/// even, so a scanner that treated an escape as close-then-reopen would
/// re-synchronise before the terminating semicolon. Exhaustive search over
/// short records found no input where the two disagree, so no test can
/// kill a mutation of it. It stays because it makes this function correct
/// in isolation rather than correct by a whole-file parity argument.
fn skip_text(bytes: &[u8], mut i: usize) -> usize {
    i += 1;
    while i < bytes.len() {
        if bytes[i] == b'\'' {
            if bytes.get(i + 1) == Some(&b'\'') {
                i += 2;
                continue;
            }
            return i + 1;
        }
        i += 1;
    }
    i
}

/// Advances past a `/* ... */` comment.
fn skip_comment(bytes: &[u8], mut i: usize) -> usize {
    i += 2;
    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'/' {
            return i + 2;
        }
        i += 1;
    }
    bytes.len()
}

/// One scanned record: `#id=TYPE(...);`
struct Scanned {
    id: u64,
    start: usize,
    end: usize,
    type_start: usize,
    type_end: usize,
}

/// Walks DATA looking only for record starts and the semicolon that ends
/// each one. Everything between is skipped as opaque bytes, except strings
/// and comments, which must be stepped over so their contents are not
/// mistaken for structure.
fn scan_records(bytes: &[u8]) -> Vec<Scanned> {
    let mut out = Vec::new();
    let mut i = find_data_section(bytes);
    while i < bytes.len() {
        match bytes[i] {
            b'\'' => {
                i = skip_text(bytes, i);
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i = skip_comment(bytes, i);
            }
            b'#' => {
                let Some(rec) = scan_one(bytes, i) else {
                    i += 1;
                    continue;
                };
                i = rec.end;
                out.push(rec);
            }
            _ => {
                i += 1;
            }
        }
    }
    out
}

/// Reads one `#id=TYPE(...);` starting at `#`, or `None` if this `#` is a
/// reference rather than a record head.
fn scan_one(bytes: &[u8], start: usize) -> Option<Scanned> {
    let mut i = start + 1;
    let digits = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits {
        return None;
    }
    let id: u64 = std::str::from_utf8(&bytes[digits..i]).ok()?.parse().ok()?;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if bytes.get(i) != Some(&b'=') {
        return None;
    }
    i += 1;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let type_start = i;
    while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
        i += 1;
    }
    let type_end = i;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' => {
                i = skip_text(bytes, i);
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i = skip_comment(bytes, i);
            }
            b';' => {
                return Some(Scanned {
                    id,
                    start,
                    end: i + 1,
                    type_start,
                    type_end,
                })
            }
            _ => {
                i += 1;
            }
        }
    }
    None
}

/// Offset just past `DATA;`, or 0 when the file has no DATA marker.
fn find_data_section(bytes: &[u8]) -> usize {
    let needle = b"DATA";
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if bytes[i..].starts_with(needle) {
            let mut j = i + needle.len();
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if bytes.get(j) == Some(&b';') {
                return j + 1;
            }
        }
        i += 1;
    }
    0
}

/// What is in a file, without what it says.
///
/// Borrows the source bytes and stores four parallel columns plus a table of
/// distinct type names. At roughly 24 bytes per entity this is two orders of
/// magnitude smaller than the decoded [`Model`].
///
/// Use [`Index::entity`] to decode one record, or [`Index::materialize_closure`]
/// to build a real [`Model`] from a chosen subset.
pub struct Index<'a> {
    source: &'a [u8],
    ids: Vec<u64>,
    starts: Vec<u64>,
    lens: Vec<u32>,
    type_ids: Vec<u32>,
    type_names: Vec<String>,
}

impl<'a> Index<'a> {
    /// Scans `source` for record boundaries. Does not decode attributes.
    #[must_use]
    pub fn scan(source: &'a [u8]) -> Self {
        let scanned = scan_records(source);
        let mut ids = Vec::with_capacity(scanned.len());
        let mut starts = Vec::with_capacity(scanned.len());
        let mut lens = Vec::with_capacity(scanned.len());
        let mut type_ids = Vec::with_capacity(scanned.len());
        let mut type_names: Vec<String> = Vec::new();
        let mut seen: std::collections::HashMap<&[u8], u32> = std::collections::HashMap::new();
        for rec in scanned {
            let name = &source[rec.type_start..rec.type_end];
            let next = type_names.len() as u32;
            let tid = *seen.entry(name).or_insert(next);
            if tid == next {
                type_names.push(String::from_utf8_lossy(name).to_ascii_uppercase());
            }
            ids.push(rec.id);
            starts.push(rec.start as u64);
            lens.push((rec.end - rec.start) as u32);
            type_ids.push(tid);
        }
        Self {
            source,
            ids,
            starts,
            lens,
            type_ids,
            type_names,
        }
    }

    /// Number of records found.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether the file has no data records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Every id, in file order.
    pub fn ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.ids.iter().copied().map(EntityId)
    }

    /// Upper-cased type name of `id`, without decoding it.
    #[must_use]
    pub fn type_of(&self, id: EntityId) -> Option<&str> {
        let pos = self.position(id)?;
        Some(&self.type_names[self.type_ids[pos] as usize])
    }

    /// How many records of each type. The census a viewer opens with.
    #[must_use]
    pub fn count_by_type(&self) -> std::collections::BTreeMap<&str, usize> {
        let mut out = std::collections::BTreeMap::new();
        for tid in &self.type_ids {
            *out.entry(self.type_names[*tid as usize].as_str())
                .or_insert(0) += 1;
        }
        out
    }

    /// Ids of every record whose type matches `name`, case-insensitively.
    #[must_use]
    pub fn ids_of_type(&self, name: &str) -> Vec<EntityId> {
        let upper = name.to_ascii_uppercase();
        let Some(tid) = self.type_names.iter().position(|n| *n == upper) else {
            return Vec::new();
        };
        let tid = tid as u32;
        self.type_ids
            .iter()
            .enumerate()
            .filter(|(_, t)| **t == tid)
            .map(|(i, _)| EntityId(self.ids[i]))
            .collect()
    }

    fn position(&self, id: EntityId) -> Option<usize> {
        self.ids.iter().position(|i| *i == id.0)
    }

    /// Decodes one record into an [`Entity`].
    ///
    /// The record is wrapped in a minimal synthetic file and handed to the
    /// real parser, so a lazily decoded entity cannot drift from its eager
    /// counterpart: there is only one decoder.
    #[must_use]
    pub fn entity(&self, id: EntityId) -> Option<Entity> {
        let pos = self.position(id)?;
        let start = self.starts[pos] as usize;
        let end = start + self.lens[pos] as usize;
        let model = self.decode_span(start, end)?;
        model.get(id).cloned()
    }

    /// Wraps one or more raw records in a minimal STEP envelope and parses
    /// them. The header is synthetic; only the DATA section is real.
    fn decode_span(&self, start: usize, end: usize) -> Option<Model> {
        let mut buf = Vec::with_capacity(end - start + 160);
        buf.extend_from_slice(ENVELOPE_HEAD);
        buf.extend_from_slice(&self.source[start..end]);
        buf.extend_from_slice(ENVELOPE_TAIL);
        crate::StepReader::new(ParseOptions::strict())
            .read_bytes(&buf)
            .ok()
    }

    /// Builds a [`Model`] from exactly `wanted`, and nothing else.
    ///
    /// # Dangling references
    ///
    /// The result is a real `Model` but not a self-contained one: an entity
    /// that referenced `#7` still says `Ref(7)` even when `#7` was not
    /// requested. Traversal will not resolve it. Use
    /// [`Index::materialize_closure`] unless you specifically want the raw
    /// subset and will handle missing targets yourself.
    #[must_use]
    pub fn materialize(&self, wanted: &[EntityId]) -> Model {
        self.build(wanted)
    }

    /// Builds a [`Model`] containing `wanted` and everything they reach.
    ///
    /// Follows references transitively, so the result has no dangling
    /// `Ref`. This is the constructor to reach for: it is what makes a
    /// subset independently usable.
    #[must_use]
    pub fn materialize_closure(&self, wanted: &[EntityId]) -> Model {
        let mut needed: std::collections::BTreeSet<u64> = wanted.iter().map(|i| i.0).collect();
        let mut frontier: Vec<u64> = needed.iter().copied().collect();
        while let Some(id) = frontier.pop() {
            let Some(entity) = self.entity(EntityId(id)) else {
                continue;
            };
            for value in &entity.attributes {
                collect_refs(value, &mut needed, &mut frontier);
            }
        }
        let ids: Vec<EntityId> = needed.into_iter().map(EntityId).collect();
        self.build(&ids)
    }

    /// Concatenates the raw bytes of `wanted` and parses them in one pass.
    fn build(&self, wanted: &[EntityId]) -> Model {
        let mut buf = Vec::new();
        buf.extend_from_slice(ENVELOPE_HEAD);
        for id in wanted {
            if let Some(pos) = self.position(*id) {
                let start = self.starts[pos] as usize;
                let end = start + self.lens[pos] as usize;
                buf.extend_from_slice(&self.source[start..end]);
                buf.push(b'\n');
            }
        }
        buf.extend_from_slice(ENVELOPE_TAIL);
        crate::StepReader::new(ParseOptions::strict())
            .read_bytes(&buf)
            .unwrap_or_default()
    }
}

/// Minimal valid STEP prologue. The header is synthetic because the index
/// decodes data records in isolation; callers that need real header fields
/// read them from an eagerly parsed model.
const ENVELOPE_HEAD: &[u8] = b"ISO-10303-21;\nHEADER;\nFILE_DESCRIPTION((\'\'),\'2;1\');\nFILE_NAME(\'\',\'\',(\'\'),(\'\'),\'\',\'\',\'\');\nFILE_SCHEMA((\'IFC4\'));\nENDSEC;\nDATA;\n";
const ENVELOPE_TAIL: &[u8] = b"ENDSEC;\nEND-ISO-10303-21;\n";

/// Adds every `Ref` reachable from `value` to the work set.
fn collect_refs(
    value: &Value,
    needed: &mut std::collections::BTreeSet<u64>,
    frontier: &mut Vec<u64>,
) {
    match value {
        Value::Ref(id) => {
            if needed.insert(id.0) {
                frontier.push(id.0);
            }
        }
        Value::List(items) => {
            for v in items {
                collect_refs(v, needed, frontier);
            }
        }
        Value::Typed { value, .. } => collect_refs(value, needed, frontier),
        _ => {}
    }
}
