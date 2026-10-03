//! Reading a container: the string table and each record as a slice, then
//! one edition's records on demand.

use std::borrow::Cow;

use super::{
    PackError, FORMAT_VERSION, MAGIC, MAX_BYTES, SLOT_ABSENT, SLOT_HEX, SLOT_TABLE, SLOT_TEXT,
};
use crate::catalog::{Catalog, CatalogProfile};
use crate::definition::{
    Applicability, CatalogEdition, EnumerationConstant, LocalizedText, PropertyDataType,
    PropertyKind, PropertySetType, PropertyTemplate, QuantityKind, QuantitySetType,
    QuantityTemplate, SetTemplate, SetTemplateKind, SourceManifest, TemplateSource,
};

/// The editions a container holds, in container order, without decoding
/// their templates.
pub(crate) fn editions(bytes: &[u8]) -> Result<Vec<CatalogEdition>, PackError> {
    let container = Container::read(bytes)?;
    Ok(container
        .entries
        .iter()
        .map(|entry| entry.edition)
        .collect())
}

/// Decode every edition of a container into official-profile catalogs, in
/// container order.
pub(crate) fn decode_all(bytes: &[u8]) -> Result<Vec<Catalog>, PackError> {
    let container = Container::read(bytes)?;
    container
        .entries
        .iter()
        .map(|entry| container.catalog(entry))
        .collect()
}

/// Decode one edition of a container; `Ok(None)` when it holds no such
/// edition. Only that edition's records are decoded.
pub(crate) fn decode_one(
    bytes: &[u8],
    edition: CatalogEdition,
) -> Result<Option<Catalog>, PackError> {
    let container = Container::read(bytes)?;
    container
        .entries
        .iter()
        .find(|entry| entry.edition == edition)
        .map(|entry| container.catalog(entry))
        .transpose()
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn byte(&mut self) -> Result<u8, PackError> {
        let byte = *self
            .bytes
            .get(self.at)
            .ok_or(PackError::Malformed("truncated"))?;
        self.at += 1;
        Ok(byte)
    }

    fn uvar(&mut self) -> Result<u64, PackError> {
        let mut value = 0u64;
        let mut shift = 0;
        loop {
            let byte = self.byte()?;
            let bits = u64::from(byte & 0x7f);
            if shift == 63 && bits > 1 {
                return Err(PackError::Malformed("integer overflows 64 bits"));
            }
            value |= bits << shift;
            if byte & 0x80 == 0 {
                // A canonical encoding has no redundant high zero group.
                if byte == 0 && shift > 0 {
                    return Err(PackError::Malformed("non-canonical integer"));
                }
                return Ok(value);
            }
            shift += 7;
            if shift > 63 {
                return Err(PackError::Malformed("integer overflows 64 bits"));
            }
        }
    }

    fn usize(&mut self) -> Result<usize, PackError> {
        usize::try_from(self.uvar()?).map_err(|_| PackError::Malformed("integer exceeds usize"))
    }

    /// A count of items each at least one byte long, so a hostile count
    /// cannot reserve more than the input could hold.
    fn count(&mut self) -> Result<usize, PackError> {
        let count = self.usize()?;
        if count > self.bytes.len() - self.at {
            return Err(PackError::Malformed("count exceeds the remaining input"));
        }
        Ok(count)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], PackError> {
        let end = self
            .at
            .checked_add(len)
            .filter(|end| *end <= self.bytes.len())
            .ok_or(PackError::Malformed("truncated"))?;
        let slice = &self.bytes[self.at..end];
        self.at = end;
        Ok(slice)
    }

    fn finish(&self) -> Result<(), PackError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(PackError::Malformed("record has trailing bytes"))
        }
    }
}

struct Entry {
    edition: CatalogEdition,
    manifest: [String; 3],
    property_sets: usize,
    quantity_sets: usize,
    sets: Vec<usize>,
}

/// A read container: its strings, and each record as an undecoded slice,
/// so decoding one edition touches only that edition's records.
struct Container<'a> {
    strings: Vec<Cow<'a, str>>,
    properties: Vec<&'a [u8]>,
    quantities: Vec<&'a [u8]>,
    sets: Vec<&'a [u8]>,
    entries: Vec<Entry>,
}

impl<'a> Container<'a> {
    fn read(bytes: &'a [u8]) -> Result<Self, PackError> {
        if bytes.len() > MAX_BYTES {
            return Err(PackError::TooLarge(bytes.len()));
        }
        if !bytes.starts_with(&MAGIC) {
            return Err(PackError::BadMagic);
        }
        let mut reader = Reader::new(bytes);
        reader.at = MAGIC.len();
        let version = reader.uvar()?;
        if version != FORMAT_VERSION {
            return Err(PackError::UnsupportedVersion(version));
        }
        let strings = read_strings(&mut reader)?;
        let properties = read_records(&mut reader)?;
        let quantities = read_records(&mut reader)?;
        let sets = read_records(&mut reader)?;
        let mut container = Self {
            strings,
            properties,
            quantities,
            sets,
            entries: Vec::new(),
        };
        let count = reader.count()?;
        for _ in 0..count {
            let entry = container.read_entry(&mut reader)?;
            if container.entries.iter().any(|e| e.edition == entry.edition) {
                return Err(PackError::Malformed("an edition appears twice"));
            }
            container.entries.push(entry);
        }
        if reader.at != bytes.len() {
            return Err(PackError::TrailingBytes(bytes.len() - reader.at));
        }
        Ok(container)
    }

    fn catalog(&self, entry: &Entry) -> Result<Catalog, PackError> {
        let manifest = SourceManifest {
            edition: entry.edition,
            source_label: entry.manifest[0].clone(),
            source_url: entry.manifest[1].clone(),
            sha256: entry.manifest[2].clone(),
            property_set_count: entry.property_sets,
            quantity_set_count: entry.quantity_sets,
        };
        let templates = entry
            .sets
            .iter()
            .map(|id| self.set(*id))
            .collect::<Result<Vec<_>, _>>()?;
        Catalog::try_new(manifest, CatalogProfile::Official, templates).map_err(PackError::Catalog)
    }

    /// One string slot.
    fn optional(&self, reader: &mut Reader<'_>) -> Result<Option<String>, PackError> {
        match reader.uvar()? {
            SLOT_ABSENT => Ok(None),
            SLOT_TEXT => {
                let len = reader.usize()?;
                let bytes = reader.take(len)?;
                std::str::from_utf8(bytes)
                    .map(|text| Some(text.to_owned()))
                    .map_err(|_| PackError::Malformed("string is not UTF-8"))
            }
            SLOT_HEX => {
                let len = reader.usize()?;
                if len == 0 {
                    return Err(PackError::Malformed("empty hexadecimal string"));
                }
                Ok(Some(hex_text(reader.take(len)?)))
            }
            slot => usize::try_from(slot - SLOT_TABLE)
                .ok()
                .and_then(|index| self.strings.get(index))
                .map(|text| Some(text.to_string()))
                .ok_or(PackError::Malformed("string index out of range")),
        }
    }

    /// A string slot that must not be absent.
    fn string(&self, reader: &mut Reader<'_>) -> Result<String, PackError> {
        self.optional(reader)?
            .ok_or(PackError::Malformed("a required string is absent"))
    }

    fn aliases(&self, reader: &mut Reader<'_>) -> Result<Vec<LocalizedText>, PackError> {
        let count = reader.count()?;
        let mut aliases = Vec::with_capacity(count);
        for _ in 0..count {
            aliases.push(LocalizedText {
                language: self.optional(reader)?,
                text: self.string(reader)?,
            });
        }
        Ok(aliases)
    }

    fn data_type(&self, reader: &mut Reader<'_>) -> Result<PropertyDataType, PackError> {
        Ok(PropertyDataType {
            type_name: self.optional(reader)?,
            unit_type: self.optional(reader)?,
        })
    }

    fn property(&self, id: usize) -> Result<PropertyTemplate, PackError> {
        let record = self
            .properties
            .get(id)
            .ok_or(PackError::Malformed("property index out of range"))?;
        let mut reader = Reader::new(record);
        let reader = &mut reader;
        let name = self.string(reader)?;
        let guid = self.optional(reader)?;
        let definition = self.optional(reader)?;
        let name_aliases = self.aliases(reader)?;
        let definition_aliases = self.aliases(reader)?;
        let kind = match reader.byte()? {
            0 => PropertyKind::SingleValue {
                data_type: self.data_type(reader)?,
            },
            1 => PropertyKind::BoundedValue {
                data_type: self.data_type(reader)?,
            },
            2 => {
                let enumeration_name = self.optional(reader)?;
                let data_type = match reader.byte()? {
                    0 => None,
                    1 => Some(self.data_type(reader)?),
                    _ => return Err(PackError::Malformed("invalid option tag")),
                };
                let count = reader.count()?;
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(self.string(reader)?);
                }
                let count = reader.count()?;
                let mut constants = Vec::with_capacity(count);
                for _ in 0..count {
                    constants.push(EnumerationConstant {
                        name: self.string(reader)?,
                        definition: self.optional(reader)?,
                        name_aliases: self.aliases(reader)?,
                        definition_aliases: self.aliases(reader)?,
                    });
                }
                PropertyKind::EnumeratedValue {
                    enumeration_name,
                    data_type,
                    values,
                    constants,
                }
            }
            3 => PropertyKind::ListValue {
                data_type: self.data_type(reader)?,
            },
            4 => PropertyKind::ReferenceValue {
                reference_type: self.string(reader)?,
            },
            5 => PropertyKind::TableValue {
                defining_type: self.data_type(reader)?,
                defined_type: self.data_type(reader)?,
                expression: self.optional(reader)?,
            },
            6 => {
                let usage_name = self.string(reader)?;
                let count = reader.count()?;
                let mut properties = Vec::with_capacity(count);
                for _ in 0..count {
                    let child = reader.usize()?;
                    // Only earlier records, so the recursion ends.
                    if child >= id {
                        return Err(PackError::Malformed("complex property refers forwards"));
                    }
                    properties.push(self.property(child)?);
                }
                PropertyKind::Complex {
                    usage_name,
                    properties,
                }
            }
            _ => return Err(PackError::Malformed("invalid property kind")),
        };
        reader.finish()?;
        Ok(PropertyTemplate {
            name,
            guid,
            definition,
            name_aliases,
            definition_aliases,
            kind,
        })
    }

    fn quantity(&self, id: usize) -> Result<QuantityTemplate, PackError> {
        let record = self
            .quantities
            .get(id)
            .ok_or(PackError::Malformed("quantity index out of range"))?;
        let mut reader = Reader::new(record);
        let reader = &mut reader;
        let quantity = QuantityTemplate {
            name: self.string(reader)?,
            definition: self.optional(reader)?,
            name_aliases: self.aliases(reader)?,
            definition_aliases: self.aliases(reader)?,
            kind: match reader.byte()? {
                0 => QuantityKind::Length,
                1 => QuantityKind::Area,
                2 => QuantityKind::Volume,
                3 => QuantityKind::Weight,
                4 => QuantityKind::Time,
                5 => QuantityKind::Count,
                6 => QuantityKind::Number,
                _ => return Err(PackError::Malformed("invalid quantity kind")),
            },
        };
        reader.finish()?;
        Ok(quantity)
    }

    fn set(&self, id: usize) -> Result<SetTemplate, PackError> {
        let record = self
            .sets
            .get(id)
            .ok_or(PackError::Malformed("set index out of range"))?;
        let mut reader = Reader::new(record);
        let reader = &mut reader;
        let name = self.string(reader)?;
        let guid = self.optional(reader)?;
        let definition = self.optional(reader)?;
        let name_aliases = self.aliases(reader)?;
        let definition_aliases = self.aliases(reader)?;
        let source = match reader.byte()? {
            0 => None,
            1 => Some(TemplateSource {
                relative_path: self.string(reader)?,
                sha256: self.string(reader)?,
            }),
            _ => return Err(PackError::Malformed("invalid option tag")),
        };
        let raw_applicability = self.optional(reader)?;
        let count = reader.count()?;
        let mut applicability = Vec::with_capacity(count);
        for _ in 0..count {
            applicability.push(Applicability {
                raw: self.string(reader)?,
                entity: self.string(reader)?,
                predefined_type: self.optional(reader)?,
            });
        }
        let kind = match reader.byte()? {
            0 => {
                let set_type = match reader.byte()? {
                    0 => PropertySetType::TypeDrivenOverride,
                    1 => PropertySetType::TypeDrivenOnly,
                    2 => PropertySetType::OccurrenceDriven,
                    3 => PropertySetType::PerformanceDriven,
                    4 => PropertySetType::Unspecified,
                    _ => return Err(PackError::Malformed("invalid property set type")),
                };
                let count = reader.count()?;
                let mut properties = Vec::with_capacity(count);
                for _ in 0..count {
                    let member = reader.usize()?;
                    properties.push(self.property(member)?);
                }
                SetTemplateKind::Property {
                    set_type,
                    properties,
                }
            }
            1 => {
                let set_type = match reader.byte()? {
                    0 => QuantitySetType::TypeDrivenOverride,
                    1 => QuantitySetType::TypeDrivenOnly,
                    2 => QuantitySetType::OccurrenceDriven,
                    3 => QuantitySetType::Unspecified,
                    _ => return Err(PackError::Malformed("invalid quantity set type")),
                };
                let method_of_measurement = self.optional(reader)?;
                let count = reader.count()?;
                let mut quantities = Vec::with_capacity(count);
                for _ in 0..count {
                    let member = reader.usize()?;
                    quantities.push(self.quantity(member)?);
                }
                SetTemplateKind::Quantity {
                    set_type,
                    method_of_measurement,
                    quantities,
                }
            }
            _ => return Err(PackError::Malformed("invalid set kind")),
        };
        reader.finish()?;
        Ok(SetTemplate {
            name,
            guid,
            definition,
            name_aliases,
            definition_aliases,
            source,
            raw_applicability,
            applicability,
            kind,
        })
    }

    fn read_entry(&self, reader: &mut Reader<'_>) -> Result<Entry, PackError> {
        let edition = match reader.byte()? {
            0 => CatalogEdition::Ifc2x3Tc1,
            1 => CatalogEdition::Ifc4Add2Tc1,
            2 => CatalogEdition::Ifc4x3Add2,
            _ => return Err(PackError::Malformed("unknown edition")),
        };
        let manifest = [
            self.string(reader)?,
            self.string(reader)?,
            self.string(reader)?,
        ];
        let property_sets = reader.usize()?;
        let quantity_sets = reader.usize()?;
        let count = reader.count()?;
        let mut sets = Vec::with_capacity(count);
        for _ in 0..count {
            let id = reader.usize()?;
            if id >= self.sets.len() {
                return Err(PackError::Malformed("set index out of range"));
            }
            sets.push(id);
        }
        Ok(Entry {
            edition,
            manifest,
            property_sets,
            quantity_sets,
            sets,
        })
    }
}

/// One table: a count, then each record as its length and its bytes.
fn read_records<'a>(reader: &mut Reader<'a>) -> Result<Vec<&'a [u8]>, PackError> {
    let count = reader.count()?;
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let len = reader.usize()?;
        records.push(reader.take(len)?);
    }
    Ok(records)
}

fn read_strings<'a>(reader: &mut Reader<'a>) -> Result<Vec<Cow<'a, str>>, PackError> {
    let count = reader.count()?;
    let mut headers = Vec::with_capacity(count);
    for _ in 0..count {
        let header = reader.uvar()?;
        let len = usize::try_from(header >> 1)
            .map_err(|_| PackError::Malformed("string length exceeds usize"))?;
        headers.push((len, header & 1 == 1));
    }
    let mut strings = Vec::with_capacity(count);
    for (len, hex) in headers {
        let bytes = reader.take(len)?;
        if hex {
            if bytes.is_empty() {
                return Err(PackError::Malformed("empty hexadecimal string"));
            }
            strings.push(Cow::Owned(hex_text(bytes)));
        } else {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| PackError::Malformed("string is not UTF-8"))?;
            strings.push(Cow::Borrowed(text));
        }
    }
    Ok(strings)
}

/// The lowercase hexadecimal text of `bytes`.
fn hex_text(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0xf)]));
    }
    text
}
