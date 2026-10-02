//! Equivalence of paired STEP and XSD-configuration ifcXML files.
//!
//! The corpus is not committed: the paired examples are buildingSMART
//! documentation (CC BY-ND 4.0). The IFC4 ADD2 TC1 documentation ships 41
//! pairs in its Annex E (`html/annex/annex-e/*.ifc` and `*.ifcxml`); unpack
//! the release documentation and run
//!
//! ```text
//! IFCXML_PAIRED_CORPUS=<dir> cargo test -p ifc-xml --test xsd_corpus \
//!     -- --ignored --nocapture
//! ```
//!
//! Every `<name>.ifcxml` under the directory (recursively) with a
//! `<name>.ifc` beside it is read twice: the STEP file with the STEP codec,
//! the ifcXML file with [`XmlCodec::xsd`] for the release its STEP header
//! declares. Each pair is then
//!
//! - **equal**: the same entities up to renumbering;
//! - **equivalent**: equal once structurally identical duplicate STEP
//!   instances (which the ifcXML export shares) and, when the ifcXML holds
//!   nothing the STEP lacks, unreferenced STEP instances (which the ifcXML
//!   tree never reaches) are set aside;
//! - **refused**: the XSD reader returned a typed error;
//! - **mismatched**: both read, into different models.
//!
//! The test fails on a mismatch, never on a refusal: refusing what it cannot
//! read exactly is the reader's contract, reading something else is the
//! defect. A pair whose two *exports* differ in content is a mismatch the
//! reader cannot be blamed for; [`KNOWN_EXPORT_DIFFERENCES`] lists those of
//! the Annex E corpus with the reason and the exact number of differing
//! entities, so any further difference in those files still fails.
//!
//! Every Annex E `.ifcxml` writes reals with a decimal comma (`0,5`), which
//! is not an `xs:double`, so the reader refuses all 41 as published. Set
//! `IFCXML_CORPUS_DECIMAL_COMMA=repair` to rewrite `<digit>,<digit>` to
//! `<digit>.<digit>` in the bytes before reading, to test everything else.
//! Set `IFCXML_CORPUS_VERBOSE=1` to print each refusal.
//!
//! Equality is structural: the ifcXML form numbers entities in document
//! order and defines most in place without an id, so ids cannot be
//! compared. Each entity is reduced to a digest of its type and attribute
//! values, references replaced by the digest of their target, and the
//! models must hold the same multiset of digests. Reals compare bit for bit;
//! `SET` and `BAG` members compare as multisets, since the ifcXML form fills
//! a set reached through an inverse attribute in document order.
#![cfg(feature = "schema")]

use ifc_model::{Codec, EntityId, Model, Value};
use ifc_schema::{AggregateKind, Schema, SchemaVersion, TypeKind};
use ifc_step::StepCodec;
use ifc_xml::{XmlCodec, XmlProfile};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Annex E pairs whose two exports differ in content, as
/// `(file, differing STEP entities, differing ifcXML entities, reason)`.
/// Counts are the differing entities whose own references all agree, the
/// root causes; they are pinned so a new difference in the file still fails.
const KNOWN_EXPORT_DIFFERENCES: &[(&str, usize, usize, &str)] = &[
    (
        "air-terminal-library-object.ifcxml",
        4,
        4,
        "the ifcXML writes line breaks literally inside attribute values, \
         which XML 1.0 normalizes to spaces; the STEP strings keep them",
    ),
    (
        "beam-revolved-solid.ifcxml",
        1,
        1,
        "IfcMaterialProfile.Priority is 0. in STEP and omitted in ifcXML",
    ),
    (
        "beam-varying-cardinal-points.ifcxml",
        1,
        1,
        "IfcMaterialProfile.Priority is 0. in STEP and omitted in ifcXML",
    ),
    (
        "beam-varying-extrusion-paths.ifcxml",
        1,
        1,
        "IfcMaterialProfile.Priority is 0. in STEP and omitted in ifcXML",
    ),
    (
        "beam-varying-profiles.ifcxml",
        2,
        2,
        "IfcMaterialProfile.Priority is 0. in STEP and omitted in ifcXML; \
         the STEP IfcCircleProfileDef carries a fifth (hollow-profile) value",
    ),
    (
        "column-extruded-solid.ifcxml",
        1,
        1,
        "IfcMaterialProfile.Priority is 0. in STEP and omitted in ifcXML",
    ),
    (
        "reinforcing-assembly.ifcxml",
        1,
        1,
        "IfcMaterialProfile.Priority is 0. in STEP and omitted in ifcXML",
    ),
    (
        "slab-standard-case.ifcxml",
        1,
        1,
        "IfcMaterialLayer.IsVentilated is .U. in STEP and false in ifcXML",
    ),
    (
        "wall-standard-case.ifcxml",
        2,
        2,
        "IfcMaterialLayer.IsVentilated is .U. in STEP and false in ifcXML",
    ),
    (
        "tessellation-with-blob-texture.ifcxml",
        1,
        1,
        "the STEP IfcTriangulatedFaceSet carries a list-of-lists PnIndex \
         (the pre-ADD2 NormalIndex) the ifcXML omits",
    ),
    (
        "tessellation-with-image-texture.ifcxml",
        1,
        1,
        "the STEP IfcTriangulatedFaceSet carries a list-of-lists PnIndex \
         (the pre-ADD2 NormalIndex) the ifcXML omits",
    ),
    (
        "tessellation-with-pixel-texture.ifcxml",
        1,
        1,
        "the STEP IfcTriangulatedFaceSet carries a list-of-lists PnIndex \
         (the pre-ADD2 NormalIndex) the ifcXML omits",
    ),
    (
        "wall-with-opening-and-window.ifcxml",
        6,
        2,
        "the ifcXML omits both IfcRelDeclares.RelatingContext (the project \
         library is not exported), so STEP's unreferenced directions cannot \
         be set aside either",
    ),
];

#[test]
#[ignore = "needs IFCXML_PAIRED_CORPUS: a directory of paired .ifc/.ifcxml files"]
fn paired_step_and_ifcxml_read_into_the_same_model() {
    let Some(root) = std::env::var_os("IFCXML_PAIRED_CORPUS").map(PathBuf::from) else {
        panic!("set IFCXML_PAIRED_CORPUS to a directory of paired .ifc/.ifcxml files");
    };
    let verbose = std::env::var_os("IFCXML_CORPUS_VERBOSE").is_some();
    let repair = std::env::var("IFCXML_CORPUS_DECIMAL_COMMA").is_ok_and(|value| value == "repair");
    let pairs = collect_pairs(&root);
    assert!(
        !pairs.is_empty(),
        "no .ifc/.ifcxml pairs under {}",
        root.display()
    );

    let mut equal = Vec::new();
    let mut equivalent = Vec::new();
    let mut known = Vec::new();
    let mut refused: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut failures = Vec::new();
    for (step_path, xml_path) in &pairs {
        let name = xml_path
            .strip_prefix(&root)
            .unwrap_or(xml_path)
            .display()
            .to_string();
        let step = StepCodec
            .read_bytes(&std::fs::read(step_path).expect("read .ifc"))
            .unwrap_or_else(|error| panic!("{name}: STEP read failed: {error}"));
        let token = step.header().schema_token().unwrap_or_default().to_string();
        let (version, profile) = match SchemaVersion::from_header_token(&token) {
            Some(SchemaVersion::Ifc4) => (SchemaVersion::Ifc4, XmlProfile::Ifc4Add2Tc1),
            Some(SchemaVersion::Ifc4x3) => (SchemaVersion::Ifc4x3, XmlProfile::Ifc4x3Add2),
            _ => panic!("{name}: no ifcXML profile for schema {token:?}"),
        };
        let schema = ifc_schema::for_version(version).expect("bundled schema");
        let codec = XmlCodec::xsd(Arc::new(schema.clone()), profile);
        let mut bytes = std::fs::read(xml_path).expect("read .ifcxml");
        if repair {
            repair_decimal_commas(&mut bytes);
        }
        let file = xml_path
            .file_name()
            .map(|file| file.to_string_lossy().into_owned())
            .unwrap_or_default();
        let listed = KNOWN_EXPORT_DIFFERENCES
            .iter()
            .find(|(listed, ..)| *listed == file);
        match ifc_xml::reader::read(&codec, &bytes) {
            Err(error) => {
                if verbose {
                    eprintln!("refused  {name}: {error}");
                }
                refused
                    .entry(class(error.root_cause()))
                    .or_default()
                    .push(name);
            }
            Ok(xml) => match (compare(schema, &step, &xml), listed) {
                (Outcome::Equal, None) => equal.push(name),
                (
                    Outcome::Equivalent {
                        orphans,
                        duplicates,
                    },
                    None,
                ) => equivalent.push(format!(
                    "{name} ({orphans} unreferenced, {duplicates} duplicate STEP instances)"
                )),
                (Outcome::Mismatch { step, xml, .. }, Some((_, want_step, want_xml, reason)))
                    if step == *want_step && xml == *want_xml =>
                {
                    known.push(format!("{name}: {reason}"));
                }
                (Outcome::Mismatch { detail, .. }, _) => failures.push(format!("{name}: {detail}")),
                (_, Some(_)) => failures.push(format!(
                    "{name} is listed in KNOWN_EXPORT_DIFFERENCES but now compares equal; \
                     remove the entry"
                )),
            },
        }
    }

    let refused_count: usize = refused.values().map(Vec::len).sum();
    eprintln!(
        "{} pairs: {} equal, {} equivalent, {} known export differences, \
         {refused_count} refused, {} failures",
        pairs.len(),
        equal.len(),
        equivalent.len(),
        known.len(),
        failures.len()
    );
    for name in &equivalent {
        eprintln!("  equivalent: {name}");
    }
    for name in &known {
        eprintln!("  known export difference: {name}");
    }
    for (cause, files) in &refused {
        eprintln!("  refused ({}) {cause}: {}", files.len(), files.join(", "));
    }
    assert!(
        failures.is_empty(),
        "{} pairs failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// `<digit>,<digit>` -> `<digit>.<digit>`, for the Annex E decimal commas.
fn repair_decimal_commas(bytes: &mut [u8]) {
    for index in 1..bytes.len().saturating_sub(1) {
        if bytes[index] == b','
            && bytes[index - 1].is_ascii_digit()
            && bytes[index + 1].is_ascii_digit()
        {
            bytes[index] = b'.';
        }
    }
}

/// The refusal's variant and message with values stripped, so equal causes
/// group.
fn class(error: &ifc_xml::XmlError) -> String {
    let text = error.to_string();
    let text: String = text
        .split('"')
        .enumerate()
        .map(|(index, part)| if index % 2 == 1 { "…" } else { part })
        .collect::<Vec<_>>()
        .join("\"");
    format!("{}: {text}", variant(error))
}

fn variant(error: &ifc_xml::XmlError) -> &'static str {
    use ifc_xml::XmlError::*;
    match error {
        InvalidScalar { .. } => "InvalidScalar",
        Unsupported { .. } => "Unsupported",
        UnknownAttribute { .. } => "UnknownAttribute",
        UnknownEntity { .. } => "UnknownEntity",
        TypeMismatch { .. } => "TypeMismatch",
        WrongForm { .. } => "WrongForm",
        InverseConflict { .. } => "InverseConflict",
        Namespace { .. } => "Namespace",
        _ => "other",
    }
}

fn collect_pairs(root: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("read corpus directory") {
            let path = entry.expect("corpus entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("ifcxml"))
            {
                let step = path.with_extension("ifc");
                if step.is_file() {
                    pairs.push((step, path));
                }
            }
        }
    }
    pairs.sort();
    pairs
}

/// How a pair compares.
enum Outcome {
    /// The same multiset of entity digests.
    Equal,
    /// Equal once STEP instances nothing references, and STEP instances
    /// structurally identical to one the ifcXML holds, are set aside.
    Equivalent { orphans: usize, duplicates: usize },
    /// Different: the root-cause entities on each side, and a description.
    Mismatch {
        step: usize,
        xml: usize,
        detail: String,
    },
}

/// Compare the two models.
fn compare(schema: &Schema, step: &Model, xml: &Model) -> Outcome {
    let (step_digests, xml_digests) = match (
        Digests::new(schema, step).all(),
        Digests::new(schema, xml).all(),
    ) {
        (Ok(step), Ok(xml)) => (step, xml),
        (Err(detail), _) | (_, Err(detail)) => {
            return Outcome::Mismatch {
                step: usize::MAX,
                xml: usize::MAX,
                detail,
            }
        }
    };
    let tally = |digests: &[(u64, EntityId)]| {
        let mut counts: HashMap<u64, Vec<EntityId>> = HashMap::new();
        for (digest, id) in digests {
            counts.entry(*digest).or_default().push(*id);
        }
        counts
    };
    let left = tally(&step_digests);
    let right = tally(&xml_digests);
    let surplus = |this: &HashMap<u64, Vec<EntityId>>, other: &HashMap<u64, Vec<EntityId>>| {
        let mut ids = Vec::new();
        for (digest, these) in this {
            let others = other.get(digest).map_or(0, Vec::len);
            if these.len() > others {
                ids.extend(these[others..].iter().map(|id| (*id, others > 0)));
            }
        }
        ids.sort();
        ids
    };
    // A STEP surplus whose digest the ifcXML also holds is a duplicate the
    // ifcXML export shares; every digest that refers to it is unchanged.
    let step_surplus = surplus(&left, &right);
    let duplicates = step_surplus.iter().filter(|(_, shared)| *shared).count();
    let unshared: Vec<EntityId> = step_surplus
        .iter()
        .filter(|(_, shared)| !shared)
        .map(|(id, _)| *id)
        .collect();
    let only_xml: Vec<EntityId> = surplus(&right, &left)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    if unshared.is_empty() && only_xml.is_empty() && duplicates == 0 {
        return Outcome::Equal;
    }
    // Only when the ifcXML holds nothing unexplained: otherwise a STEP
    // relationship nothing references may be the counterpart of one.
    let orphans = if only_xml.is_empty() {
        orphans(step, &unshared)
    } else {
        HashSet::new()
    };
    let only_step: Vec<EntityId> = unshared
        .iter()
        .copied()
        .filter(|id| !orphans.contains(id))
        .collect();
    if only_step.is_empty() && only_xml.is_empty() {
        return Outcome::Equivalent {
            orphans: orphans.len(),
            duplicates,
        };
    }
    let only_step = leaves(step, &only_step);
    let only_xml = leaves(xml, &only_xml);
    let show = |model: &Model, ids: &[EntityId]| {
        ids.iter()
            .take(4)
            .map(|id| {
                let entity = model.get(*id).expect("entity");
                let mut text = format!("{id}={}{:?}", entity.type_name, entity.attributes);
                text.truncate(400);
                text
            })
            .collect::<Vec<_>>()
            .join("\n      ")
    };
    Outcome::Mismatch {
        step: only_step.len(),
        xml: only_xml.len(),
        detail: format!(
            "{} STEP entities vs {} ifcXML; {} differing entities only in STEP, {} only \
             in ifcXML\n    STEP only:   {}\n    ifcXML only: {}",
            step.len(),
            xml.len(),
            only_step.len(),
            only_xml.len(),
            show(step, &only_step),
            show(xml, &only_xml)
        ),
    }
}

/// The differences that cause the others: entities none of whose
/// references reach another differing entity.
fn leaves(model: &Model, ids: &[EntityId]) -> Vec<EntityId> {
    let set: HashSet<EntityId> = ids.iter().copied().collect();
    ids.iter()
        .copied()
        .filter(|id| {
            let mut inner = false;
            for value in &model.get(*id).expect("entity").attributes {
                value.for_each_ref(&mut |target| inner |= set.contains(&target));
            }
            !inner
        })
        .collect()
}

/// The `candidates` that can be removed from `model` by repeatedly removing
/// one that no remaining entity references.
fn orphans(model: &Model, candidates: &[EntityId]) -> HashSet<EntityId> {
    let mut referrers: HashMap<EntityId, Vec<EntityId>> = HashMap::new();
    for (id, entity) in model.iter() {
        for value in &entity.attributes {
            value.for_each_ref(&mut |target| referrers.entry(target).or_default().push(id));
        }
    }
    let mut remaining: Vec<EntityId> = candidates.to_vec();
    let mut removed = HashSet::new();
    loop {
        let before = remaining.len();
        remaining.retain(|id| {
            let referenced = referrers
                .get(id)
                .is_some_and(|from| from.iter().any(|from| !removed.contains(from)));
            if !referenced {
                removed.insert(*id);
            }
            referenced
        });
        if remaining.len() == before {
            return removed;
        }
    }
}

/// Memoized structural digests of a model's entities.
struct Digests<'a> {
    schema: &'a Schema,
    model: &'a Model,
    memo: HashMap<EntityId, u64>,
    visiting: Vec<EntityId>,
}

impl<'a> Digests<'a> {
    fn new(schema: &'a Schema, model: &'a Model) -> Self {
        Self {
            schema,
            model,
            memo: HashMap::new(),
            visiting: Vec::new(),
        }
    }

    fn all(mut self) -> Result<Vec<(u64, EntityId)>, String> {
        let ids: Vec<EntityId> = self.model.ids().collect();
        ids.into_iter()
            .map(|id| self.entity(id).map(|digest| (digest, id)))
            .collect()
    }

    fn entity(&mut self, id: EntityId) -> Result<u64, String> {
        if let Some(digest) = self.memo.get(&id) {
            return Ok(*digest);
        }
        if self.visiting.contains(&id) {
            return Err(format!("reference cycle through {id}"));
        }
        let entity = self
            .model
            .get(id)
            .ok_or_else(|| format!("dangling reference to {id}"))?;
        self.visiting.push(id);
        let type_name = entity.type_name.to_ascii_uppercase();
        let kinds: Vec<Vec<AggregateKind>> = self
            .schema
            .attributes(&type_name)
            .iter()
            .map(|attribute| {
                attribute
                    .aggregation
                    .iter()
                    .map(|level| level.kind)
                    .collect()
            })
            .collect();
        let mut hasher = DefaultHasher::new();
        type_name.hash(&mut hasher);
        entity.attributes.len().hash(&mut hasher);
        for (slot, value) in entity.attributes.iter().enumerate() {
            let levels = kinds.get(slot).map_or(&[][..], Vec::as_slice);
            self.value(value, levels)?.hash(&mut hasher);
        }
        self.visiting.pop();
        let digest = hasher.finish();
        self.memo.insert(id, digest);
        Ok(digest)
    }

    fn value(&mut self, value: &Value, levels: &[AggregateKind]) -> Result<u64, String> {
        let mut hasher = DefaultHasher::new();
        match value {
            Value::Null => 0u8.hash(&mut hasher),
            Value::Derived => 1u8.hash(&mut hasher),
            Value::Bool(value) => (2u8, value).hash(&mut hasher),
            Value::LogicalUnknown => 3u8.hash(&mut hasher),
            Value::Integer(value) => (4u8, value).hash(&mut hasher),
            Value::Real(value) => (5u8, value.to_bits()).hash(&mut hasher),
            Value::Text(value) => (6u8, &**value).hash(&mut hasher),
            Value::Binary(value) => (7u8, value.to_ascii_uppercase()).hash(&mut hasher),
            Value::Enum(value) => (8u8, value.to_ascii_uppercase()).hash(&mut hasher),
            Value::Ref(id) => (9u8, self.entity(*id)?).hash(&mut hasher),
            Value::List(items) => {
                let (outer, inner) = levels
                    .split_first()
                    .map_or((None, &[][..]), |(outer, inner)| (Some(*outer), inner));
                let mut digests = items
                    .iter()
                    .map(|item| self.value(item, inner))
                    .collect::<Result<Vec<_>, _>>()?;
                if matches!(outer, Some(AggregateKind::Set | AggregateKind::Bag)) {
                    digests.sort_unstable();
                }
                (10u8, digests).hash(&mut hasher);
            }
            Value::Typed { type_name, value } => {
                let upper = type_name.to_ascii_uppercase();
                let levels = self.type_levels(&upper);
                (11u8, &upper, self.value(value, &levels)?).hash(&mut hasher);
            }
        }
        Ok(hasher.finish())
    }

    /// The aggregation kinds a defined type aliases, outermost first.
    fn type_levels(&self, name: &str) -> Vec<AggregateKind> {
        let Some(definition) = self.schema.type_def(name) else {
            return Vec::new();
        };
        let TypeKind::Defined(_) = &definition.kind else {
            return Vec::new();
        };
        let resolved = self.schema.resolve_defined(name).to_ascii_uppercase();
        resolved
            .split_whitespace()
            .filter_map(|word| match word.split('[').next().unwrap_or_default() {
                "SET" => Some(AggregateKind::Set),
                "BAG" => Some(AggregateKind::Bag),
                "LIST" => Some(AggregateKind::List),
                "ARRAY" => Some(AggregateKind::Array),
                _ => None,
            })
            .collect()
    }
}
