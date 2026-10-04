//! Content checksums: how equivalence between benchmarks is asserted.
//!
//! [`model`] hashes everything a reader produces -- header schema, ids in
//! file order, type names and every attribute value, recursively -- so two
//! models with the same checksum hold the same graph. A read path that
//! dropped, reordered or mis-decoded one value would change it.

use ifc_model::{Model, Value};

/// 64-bit FNV-1a. Not cryptographic; it only has to make an accidental
/// collision between two different decodings implausible.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Fnv(u64);

impl Fnv {
    pub(crate) const fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    pub(crate) fn bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.bytes(&value.to_le_bytes());
    }

    pub(crate) const fn finish(self) -> u64 {
        self.0
    }
}

/// The content checksum of `model`, decoding every entity it touches.
pub(crate) fn model(model: &Model) -> u64 {
    let mut hash = Fnv::new();
    for schema in &model.header().schema {
        hash.bytes(schema.as_bytes());
    }
    hash.u64(model.len() as u64);
    for (id, entity) in model.iter() {
        hash.u64(id.0);
        hash.bytes(entity.type_name.as_bytes());
        hash.u64(entity.attributes.len() as u64);
        for value in &entity.attributes {
            self::value(&mut hash, value);
        }
    }
    hash.finish()
}

/// Hashes one value with a tag per variant. The match is exhaustive, so a
/// new `Value` variant fails to compile here until it is hashed.
fn value(hash: &mut Fnv, value: &Value) {
    match value {
        Value::Null => hash.bytes(b"$"),
        Value::Derived => hash.bytes(b"*"),
        Value::Bool(flag) => hash.bytes(if *flag { b"T" } else { b"F" }),
        Value::LogicalUnknown => hash.bytes(b"U"),
        Value::Integer(number) => {
            hash.bytes(b"i");
            hash.u64(*number as u64);
        }
        Value::Real(number) => {
            hash.bytes(b"r");
            hash.u64(number.to_bits());
        }
        Value::Text(text) => {
            hash.bytes(b"s");
            hash.bytes(text.as_bytes());
        }
        Value::Binary(text) => {
            hash.bytes(b"b");
            hash.bytes(text.as_bytes());
        }
        Value::Enum(text) => {
            hash.bytes(b"e");
            hash.bytes(text.as_bytes());
        }
        Value::Ref(id) => {
            hash.bytes(b"#");
            hash.u64(id.0);
        }
        Value::List(items) => {
            hash.bytes(b"(");
            hash.u64(items.len() as u64);
            for item in items {
                self::value(hash, item);
            }
        }
        Value::Typed { type_name, value } => {
            hash.bytes(b"t");
            hash.bytes(type_name.as_bytes());
            self::value(hash, value);
        }
    }
}

/// The checksum of a byte string.
pub(crate) fn bytes(bytes: &[u8]) -> u64 {
    let mut hash = Fnv::new();
    hash.bytes(bytes);
    hash.finish()
}
