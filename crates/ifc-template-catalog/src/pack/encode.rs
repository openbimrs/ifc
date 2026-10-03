//! Writing a container: distinct records by content, the string table,
//! then the bytes.

use std::collections::HashMap;

use super::{FORMAT_VERSION, MAGIC, SLOT_ABSENT, SLOT_HEX, SLOT_TABLE, SLOT_TEXT};
use crate::definition::{
    CatalogEdition, LocalizedText, PropertyDataType, PropertyKind, PropertySetType,
    PropertyTemplate, QuantityKind, QuantitySetType, QuantityTemplate, SetTemplate,
    SetTemplateKind, SourceManifest,
};

/// Encode official editions into one container. Editions keep the order
/// given; the caller sorts them for a canonical container.
pub(crate) fn encode(editions: &[(&SourceManifest, &[SetTemplate])]) -> Vec<u8> {
    // 1. Each distinct record once, keyed by its content (every string
    //    inline), in first-use order.
    let mut unique = Unique::default();
    let entries: Vec<Vec<u64>> = editions
        .iter()
        .map(|(_, templates)| templates.iter().map(|set| unique.set(set)).collect())
        .collect();

    // 2. The table: every string a distinct record or a manifest uses more
    //    than once, most used first (ties by byte order), so the commonest
    //    take a one-byte slot. A string used once stays inline, next to
    //    the text around it.
    let mut counts: HashMap<&str, u64> = HashMap::new();
    let mut tally = |s| *counts.entry(s).or_default() += 1;
    for (manifest, _) in editions {
        manifest_strings(manifest, &mut tally);
    }
    for (property, _) in &unique.properties {
        property_strings(property, &mut tally);
    }
    for quantity in &unique.quantities {
        quantity_strings(quantity, &mut tally);
    }
    for (set, _) in &unique.sets {
        set_strings(set, &mut tally);
    }
    let mut table: Vec<(&str, u64)> = counts.into_iter().filter(|(_, n)| *n > 1).collect();
    table.sort_unstable_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let ids: HashMap<&str, u64> = table
        .iter()
        .enumerate()
        .map(|(index, (s, _))| (*s, index as u64))
        .collect();
    let writer = Writer { table: Some(&ids) };

    // 3. Header, table, records, editions.
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    uvar(&mut out, FORMAT_VERSION);
    // Every header, then every payload, which compresses better than
    // interleaving them.
    uvar(&mut out, table.len() as u64);
    let mut payload = Vec::new();
    for (s, _) in &table {
        match hex_bytes(s) {
            Some(bytes) => {
                uvar(&mut out, ((bytes.len() as u64) << 1) | 1);
                payload.extend_from_slice(&bytes);
            }
            None => {
                uvar(&mut out, (s.len() as u64) << 1);
                payload.extend_from_slice(s.as_bytes());
            }
        }
    }
    out.extend_from_slice(&payload);
    let mut record = Vec::new();
    uvar(&mut out, unique.properties.len() as u64);
    for (property, children) in &unique.properties {
        record.clear();
        writer.property(&mut record, property, children);
        uvar(&mut out, record.len() as u64);
        out.extend_from_slice(&record);
    }
    uvar(&mut out, unique.quantities.len() as u64);
    for quantity in &unique.quantities {
        record.clear();
        writer.quantity(&mut record, quantity);
        uvar(&mut out, record.len() as u64);
        out.extend_from_slice(&record);
    }
    uvar(&mut out, unique.sets.len() as u64);
    for (set, members) in &unique.sets {
        record.clear();
        writer.set(&mut record, set, members);
        uvar(&mut out, record.len() as u64);
        out.extend_from_slice(&record);
    }
    uvar(&mut out, editions.len() as u64);
    for ((manifest, _), sets) in editions.iter().zip(&entries) {
        out.push(edition_tag(manifest.edition));
        writer.string(&mut out, &manifest.source_label);
        writer.string(&mut out, &manifest.source_url);
        writer.string(&mut out, &manifest.sha256);
        uvar(&mut out, manifest.property_set_count as u64);
        uvar(&mut out, manifest.quantity_set_count as u64);
        uvar(&mut out, sets.len() as u64);
        for id in sets {
            uvar(&mut out, *id);
        }
    }
    out
}

/// The distinct records of a container, each with the ids of the records
/// it refers to, and the content keys that make them distinct.
#[derive(Default)]
struct Unique<'a> {
    properties: Vec<(&'a PropertyTemplate, Vec<u64>)>,
    quantities: Vec<&'a QuantityTemplate>,
    sets: Vec<(&'a SetTemplate, Vec<u64>)>,
    keys: HashMap<(u8, Vec<u8>), u64>,
}

impl<'a> Unique<'a> {
    const CONTENT: Writer<'static> = Writer { table: None };

    fn intern(&mut self, kind: u8, key: Vec<u8>, len: usize) -> Option<u64> {
        match self.keys.entry((kind, key)) {
            std::collections::hash_map::Entry::Occupied(found) => Some(*found.get()),
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(len as u64);
                None
            }
        }
    }

    fn property(&mut self, property: &'a PropertyTemplate) -> u64 {
        // Children first, so a complex property refers only backwards.
        let children: Vec<u64> = match &property.kind {
            PropertyKind::Complex { properties, .. } => properties
                .iter()
                .map(|child| self.property(child))
                .collect(),
            _ => Vec::new(),
        };
        let mut key = Vec::new();
        Self::CONTENT.property(&mut key, property, &children);
        let id = self.properties.len();
        self.intern(0, key, id).unwrap_or_else(|| {
            self.properties.push((property, children));
            id as u64
        })
    }

    fn quantity(&mut self, quantity: &'a QuantityTemplate) -> u64 {
        let mut key = Vec::new();
        Self::CONTENT.quantity(&mut key, quantity);
        let id = self.quantities.len();
        self.intern(1, key, id).unwrap_or_else(|| {
            self.quantities.push(quantity);
            id as u64
        })
    }

    fn set(&mut self, set: &'a SetTemplate) -> u64 {
        let members: Vec<u64> = match &set.kind {
            SetTemplateKind::Property { properties, .. } => properties
                .iter()
                .map(|property| self.property(property))
                .collect(),
            SetTemplateKind::Quantity { quantities, .. } => quantities
                .iter()
                .map(|quantity| self.quantity(quantity))
                .collect(),
        };
        let mut key = Vec::new();
        Self::CONTENT.set(&mut key, set, &members);
        let id = self.sets.len();
        self.intern(2, key, id).unwrap_or_else(|| {
            self.sets.push((set, members));
            id as u64
        })
    }
}

/// Writes records; without a table every string is inline, which makes the
/// bytes a content key.
struct Writer<'a> {
    table: Option<&'a HashMap<&'a str, u64>>,
}

impl Writer<'_> {
    fn string(&self, out: &mut Vec<u8>, s: &str) {
        if let Some(index) = self.table.and_then(|table| table.get(s)) {
            uvar(out, SLOT_TABLE + index);
            return;
        }
        match hex_bytes(s) {
            Some(bytes) => {
                uvar(out, SLOT_HEX);
                uvar(out, bytes.len() as u64);
                out.extend_from_slice(&bytes);
            }
            None => {
                uvar(out, SLOT_TEXT);
                uvar(out, s.len() as u64);
                out.extend_from_slice(s.as_bytes());
            }
        }
    }

    fn optional(&self, out: &mut Vec<u8>, s: Option<&String>) {
        match s {
            None => uvar(out, SLOT_ABSENT),
            Some(s) => self.string(out, s),
        }
    }

    fn aliases(&self, out: &mut Vec<u8>, aliases: &[LocalizedText]) {
        uvar(out, aliases.len() as u64);
        for alias in aliases {
            self.optional(out, alias.language.as_ref());
            self.string(out, &alias.text);
        }
    }

    fn data_type(&self, out: &mut Vec<u8>, data_type: &PropertyDataType) {
        self.optional(out, data_type.type_name.as_ref());
        self.optional(out, data_type.unit_type.as_ref());
    }

    fn property(&self, out: &mut Vec<u8>, property: &PropertyTemplate, children: &[u64]) {
        self.string(out, &property.name);
        self.optional(out, property.guid.as_ref());
        self.optional(out, property.definition.as_ref());
        self.aliases(out, &property.name_aliases);
        self.aliases(out, &property.definition_aliases);
        match &property.kind {
            PropertyKind::SingleValue { data_type } => {
                out.push(0);
                self.data_type(out, data_type);
            }
            PropertyKind::BoundedValue { data_type } => {
                out.push(1);
                self.data_type(out, data_type);
            }
            PropertyKind::EnumeratedValue {
                enumeration_name,
                data_type,
                values,
                constants,
            } => {
                out.push(2);
                self.optional(out, enumeration_name.as_ref());
                match data_type {
                    None => out.push(0),
                    Some(data_type) => {
                        out.push(1);
                        self.data_type(out, data_type);
                    }
                }
                uvar(out, values.len() as u64);
                for value in values {
                    self.string(out, value);
                }
                uvar(out, constants.len() as u64);
                for constant in constants {
                    self.string(out, &constant.name);
                    self.optional(out, constant.definition.as_ref());
                    self.aliases(out, &constant.name_aliases);
                    self.aliases(out, &constant.definition_aliases);
                }
            }
            PropertyKind::ListValue { data_type } => {
                out.push(3);
                self.data_type(out, data_type);
            }
            PropertyKind::ReferenceValue { reference_type } => {
                out.push(4);
                self.string(out, reference_type);
            }
            PropertyKind::TableValue {
                defining_type,
                defined_type,
                expression,
            } => {
                out.push(5);
                self.data_type(out, defining_type);
                self.data_type(out, defined_type);
                self.optional(out, expression.as_ref());
            }
            PropertyKind::Complex { usage_name, .. } => {
                out.push(6);
                self.string(out, usage_name);
                uvar(out, children.len() as u64);
                for child in children {
                    uvar(out, *child);
                }
            }
        }
    }

    fn quantity(&self, out: &mut Vec<u8>, quantity: &QuantityTemplate) {
        self.string(out, &quantity.name);
        self.optional(out, quantity.definition.as_ref());
        self.aliases(out, &quantity.name_aliases);
        self.aliases(out, &quantity.definition_aliases);
        out.push(match quantity.kind {
            QuantityKind::Length => 0,
            QuantityKind::Area => 1,
            QuantityKind::Volume => 2,
            QuantityKind::Weight => 3,
            QuantityKind::Time => 4,
            QuantityKind::Count => 5,
            QuantityKind::Number => 6,
        });
    }

    fn set(&self, out: &mut Vec<u8>, set: &SetTemplate, members: &[u64]) {
        self.string(out, &set.name);
        self.optional(out, set.guid.as_ref());
        self.optional(out, set.definition.as_ref());
        self.aliases(out, &set.name_aliases);
        self.aliases(out, &set.definition_aliases);
        match &set.source {
            None => out.push(0),
            Some(source) => {
                out.push(1);
                self.string(out, &source.relative_path);
                self.string(out, &source.sha256);
            }
        }
        self.optional(out, set.raw_applicability.as_ref());
        uvar(out, set.applicability.len() as u64);
        for applicability in &set.applicability {
            self.string(out, &applicability.raw);
            self.string(out, &applicability.entity);
            self.optional(out, applicability.predefined_type.as_ref());
        }
        match &set.kind {
            SetTemplateKind::Property { set_type, .. } => {
                out.push(0);
                out.push(match set_type {
                    PropertySetType::TypeDrivenOverride => 0,
                    PropertySetType::TypeDrivenOnly => 1,
                    PropertySetType::OccurrenceDriven => 2,
                    PropertySetType::PerformanceDriven => 3,
                    PropertySetType::Unspecified => 4,
                });
            }
            SetTemplateKind::Quantity {
                set_type,
                method_of_measurement,
                ..
            } => {
                out.push(1);
                out.push(match set_type {
                    QuantitySetType::TypeDrivenOverride => 0,
                    QuantitySetType::TypeDrivenOnly => 1,
                    QuantitySetType::OccurrenceDriven => 2,
                    QuantitySetType::Unspecified => 3,
                });
                self.optional(out, method_of_measurement.as_ref());
            }
        }
        uvar(out, members.len() as u64);
        for member in members {
            uvar(out, *member);
        }
    }
}

// The strings each record (not the records it refers to) writes, for the
// table's use counts.

fn manifest_strings<'a>(manifest: &'a SourceManifest, f: &mut impl FnMut(&'a str)) {
    f(&manifest.source_label);
    f(&manifest.source_url);
    f(&manifest.sha256);
}

fn alias_strings<'a>(aliases: &'a [LocalizedText], f: &mut impl FnMut(&'a str)) {
    for alias in aliases {
        if let Some(language) = &alias.language {
            f(language);
        }
        f(&alias.text);
    }
}

fn data_type_strings<'a>(data_type: &'a PropertyDataType, f: &mut impl FnMut(&'a str)) {
    data_type.type_name.as_deref().map(&mut *f);
    data_type.unit_type.as_deref().map(&mut *f);
}

fn property_strings<'a>(property: &'a PropertyTemplate, f: &mut impl FnMut(&'a str)) {
    f(&property.name);
    property.guid.as_deref().map(&mut *f);
    property.definition.as_deref().map(&mut *f);
    alias_strings(&property.name_aliases, f);
    alias_strings(&property.definition_aliases, f);
    match &property.kind {
        PropertyKind::SingleValue { data_type }
        | PropertyKind::BoundedValue { data_type }
        | PropertyKind::ListValue { data_type } => data_type_strings(data_type, f),
        PropertyKind::EnumeratedValue {
            enumeration_name,
            data_type,
            values,
            constants,
        } => {
            enumeration_name.as_deref().map(&mut *f);
            if let Some(data_type) = data_type {
                data_type_strings(data_type, f);
            }
            for value in values {
                f(value);
            }
            for constant in constants {
                f(&constant.name);
                constant.definition.as_deref().map(&mut *f);
                alias_strings(&constant.name_aliases, f);
                alias_strings(&constant.definition_aliases, f);
            }
        }
        PropertyKind::ReferenceValue { reference_type } => f(reference_type),
        PropertyKind::TableValue {
            defining_type,
            defined_type,
            expression,
        } => {
            data_type_strings(defining_type, f);
            data_type_strings(defined_type, f);
            expression.as_deref().map(&mut *f);
        }
        // Its children are records of their own.
        PropertyKind::Complex { usage_name, .. } => f(usage_name),
    }
}

fn quantity_strings<'a>(quantity: &'a QuantityTemplate, f: &mut impl FnMut(&'a str)) {
    f(&quantity.name);
    quantity.definition.as_deref().map(&mut *f);
    alias_strings(&quantity.name_aliases, f);
    alias_strings(&quantity.definition_aliases, f);
}

fn set_strings<'a>(set: &'a SetTemplate, f: &mut impl FnMut(&'a str)) {
    f(&set.name);
    set.guid.as_deref().map(&mut *f);
    set.definition.as_deref().map(&mut *f);
    alias_strings(&set.name_aliases, f);
    alias_strings(&set.definition_aliases, f);
    if let Some(source) = &set.source {
        f(&source.relative_path);
        f(&source.sha256);
    }
    set.raw_applicability.as_deref().map(&mut *f);
    for applicability in &set.applicability {
        f(&applicability.raw);
        f(&applicability.entity);
        applicability.predefined_type.as_deref().map(&mut *f);
    }
    // Its members are records of their own.
    if let SetTemplateKind::Quantity {
        method_of_measurement: Some(method),
        ..
    } = &set.kind
    {
        f(method);
    }
}

/// The bytes of a nonempty, even-length, lowercase hexadecimal string; it
/// decodes back to the same text, so storing it as bytes is lossless.
fn hex_bytes(s: &str) -> Option<Vec<u8>> {
    let digits = s.as_bytes();
    if digits.is_empty() || digits.len() % 2 != 0 {
        return None;
    }
    let nibble = |digit: u8| match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    };
    digits
        .chunks_exact(2)
        .map(|pair| Some((nibble(pair[0])? << 4) | nibble(pair[1])?))
        .collect()
}

fn uvar(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn edition_tag(edition: CatalogEdition) -> u8 {
    match edition {
        CatalogEdition::Ifc2x3Tc1 => 0,
        CatalogEdition::Ifc4Add2Tc1 => 1,
        CatalogEdition::Ifc4x3Add2 => 2,
    }
}
