//! Offset index: what is in a file, without decoding it.
//!
//! # Why this exists
//!
//! A type census, or picking 200 walls out of nine million records, needs
//! to know where each record is and what type it has -- not what it says.
//! [`Index::scan`] answers that from the borrowed source: it frames every
//! record with `openbim_step::scan` and keeps its id, byte span and type,
//! and nothing else. It does not copy the input and does not validate the
//! inside of a record, which makes it the cheapest way to look into a file.
//!
//! For a model that decodes on access with every record validated up front,
//! read with [`StepCodec`](crate::StepCodec): a strict read is lazy.
//!
//! # Trust
//!
//! Framing is `openbim_step::scan`: the header is parsed strictly,
//! everything between records goes through the real lexer, and junk,
//! missing section ends or a second file appended are errors rather than
//! skipped. Decoding a record ([`Index::entity`]) runs the same parser and
//! the same IFC conversion as a full read, on exactly the record's bytes.
//! `tests/index_agreement.rs` checks both against the eager reader on every
//! fixture in the repository.

use std::collections::HashMap;

use ifc_model::{Entity, EntityId, Model, Value};
use openbim_step::Span;

use crate::{parser, StepError};

/// What is in a file, without what it says.
///
/// Borrows the source bytes and stores, per record, its id, span and an
/// index into a table of distinct type names -- about 32 bytes per entity,
/// two orders of magnitude less than a decoded [`Model`].
///
/// Use [`Index::entity`] to decode one record, or [`Index::materialize_closure`]
/// to build a real [`Model`] from a chosen subset.
pub struct Index<'a> {
    source: &'a [u8],
    header: ifc_model::header::Header,
    ids: Vec<u64>,
    spans: Vec<Span>,
    type_ids: Vec<u32>,
    type_names: Vec<String>,
    /// Position of each id in the columns. Files list ids in increasing
    /// order in practice, where a binary search over `ids` needs nothing
    /// extra; this map exists only for a file that does not.
    unordered: Option<HashMap<u64, usize>>,
}

impl<'a> Index<'a> {
    /// Frames every record of `source`. Does not decode attributes.
    ///
    /// # Errors
    ///
    /// A header or framing defect: the wrong physical-file marker, a
    /// malformed header, anything between records that is not a record, a
    /// missing `ENDSEC`, content after the end marker, or an instance id or
    /// type the IFC record model cannot hold (an id beyond `u64`, a complex
    /// instance). Defects inside a record surface when it is decoded.
    pub fn scan(source: &'a [u8]) -> Result<Self, StepError> {
        let scanned = openbim_step::scan(source)?;
        let mut header = ifc_model::header::Header::default();
        parser::apply_header(&mut header, scanned.header().standard());
        let mut ids = Vec::new();
        let mut spans = Vec::new();
        let mut type_ids = Vec::new();
        let mut type_names: Vec<String> = Vec::new();
        let mut seen: HashMap<String, u32> = HashMap::new();
        for record in scanned.records() {
            let record = record?;
            let id = record.id.as_str().parse::<u64>().map_err(|_| {
                unrepresentable(
                    record.span,
                    "instance id exceeds the IFC record model range",
                )
            })?;
            let Some(name) = record.name else {
                return Err(unrepresentable(
                    record.span,
                    "complex STEP instances are not representable in the IFC record model",
                ));
            };
            let upper = name.to_ascii_uppercase();
            let next = u32::try_from(type_names.len()).expect("fewer than 2^32 distinct types");
            let type_id = *seen.entry(upper.clone()).or_insert(next);
            if type_id == next {
                type_names.push(upper);
            }
            ids.push(id);
            spans.push(record.span);
            type_ids.push(type_id);
        }
        let unordered = (!ids.windows(2).all(|pair| pair[0] < pair[1])).then(|| {
            // Later records win, as they do in a model read.
            ids.iter()
                .enumerate()
                .map(|(position, id)| (*id, position))
                .collect()
        });
        Ok(Self {
            source,
            header,
            ids,
            spans,
            type_ids,
            type_names,
            unordered,
        })
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

    /// The file header, as a model read would report it.
    #[must_use]
    pub fn header(&self) -> &ifc_model::header::Header {
        &self.header
    }

    /// Every id, in file order.
    pub fn ids(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.ids.iter().copied().map(EntityId)
    }

    /// Upper-cased type name of `id`, without decoding it.
    #[must_use]
    pub fn type_of(&self, id: EntityId) -> Option<&str> {
        let position = self.position(id)?;
        Some(&self.type_names[self.type_ids[position] as usize])
    }

    /// How many records of each type. The census a viewer opens with.
    #[must_use]
    pub fn count_by_type(&self) -> std::collections::BTreeMap<&str, usize> {
        let mut out = std::collections::BTreeMap::new();
        for type_id in &self.type_ids {
            *out.entry(self.type_names[*type_id as usize].as_str())
                .or_insert(0) += 1;
        }
        out
    }

    /// Ids of every record whose type matches `name`, case-insensitively,
    /// in file order.
    #[must_use]
    pub fn ids_of_type(&self, name: &str) -> Vec<EntityId> {
        let upper = name.to_ascii_uppercase();
        let Some(type_id) = self.type_names.iter().position(|n| *n == upper) else {
            return Vec::new();
        };
        let type_id = u32::try_from(type_id).expect("fewer than 2^32 distinct types");
        self.type_ids
            .iter()
            .enumerate()
            .filter(|(_, t)| **t == type_id)
            .map(|(position, _)| EntityId(self.ids[position]))
            .collect()
    }

    fn position(&self, id: EntityId) -> Option<usize> {
        match &self.unordered {
            Some(map) => map.get(&id.0).copied(),
            None => self.ids.binary_search(&id.0).ok(),
        }
    }

    /// Decodes one record into an [`Entity`]: the same parser and IFC
    /// conversion a full read uses, on exactly the record's bytes.
    ///
    /// # Errors
    ///
    /// When the record is malformed or holds a value the IFC record model
    /// cannot represent -- exactly what would make a strict read fail.
    pub fn entity(&self, id: EntityId) -> Result<Option<Entity>, StepError> {
        let Some(position) = self.position(id) else {
            return Ok(None);
        };
        let record = openbim_step::decode_record_borrowed(self.source, self.spans[position])?;
        Ok(Some(parser::convert(record)?.1))
    }

    /// Builds a [`Model`] from exactly `wanted`, and nothing else, with the
    /// file's header. Ids the file does not contain are skipped.
    ///
    /// # Dangling references
    ///
    /// The result is a real `Model` but not a self-contained one: an entity
    /// that referenced `#7` still says `Ref(7)` even when `#7` was not
    /// requested. Traversal will not resolve it. Use
    /// [`Index::materialize_closure`] unless you specifically want the raw
    /// subset and will handle missing targets yourself.
    ///
    /// # Errors
    ///
    /// The first wanted record that does not decode; see [`Index::entity`].
    pub fn materialize(&self, wanted: &[EntityId]) -> Result<Model, StepError> {
        self.build(wanted.iter().copied())
    }

    /// Builds a [`Model`] containing `wanted` and everything they reach.
    ///
    /// Follows references transitively, so the result has no dangling `Ref`
    /// except to ids the file itself does not define. This is the
    /// constructor to reach for: it is what makes a subset independently
    /// usable.
    ///
    /// # Errors
    ///
    /// The first reached record that does not decode; see [`Index::entity`].
    pub fn materialize_closure(&self, wanted: &[EntityId]) -> Result<Model, StepError> {
        let mut needed: std::collections::BTreeSet<u64> = wanted.iter().map(|i| i.0).collect();
        let mut frontier: Vec<u64> = needed.iter().copied().collect();
        while let Some(id) = frontier.pop() {
            let Some(entity) = self.entity(EntityId(id))? else {
                continue;
            };
            for value in &entity.attributes {
                collect_refs(value, &mut needed, &mut frontier);
            }
        }
        self.build(needed.into_iter().map(EntityId))
    }

    /// Decodes `wanted` into a model, in file order, with the file's header.
    fn build(&self, wanted: impl Iterator<Item = EntityId>) -> Result<Model, StepError> {
        let mut positions: Vec<usize> = wanted.filter_map(|id| self.position(id)).collect();
        positions.sort_unstable();
        positions.dedup();
        let mut model = Model::new();
        *model.header_mut() = self.header.clone();
        for position in positions {
            let record = openbim_step::decode_record_borrowed(self.source, self.spans[position])?;
            let (id, entity) = parser::convert(record)?;
            model.insert(id, entity);
        }
        Ok(model)
    }
}

fn unrepresentable(span: Span, detail: &str) -> StepError {
    StepError::Syntax {
        offset: span.start,
        detail: detail.into(),
    }
}

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
