//! The compact snapshot container (format version 3).
//!
//! A container holds one or more official editions over shared tables:
//!
//! 1. a string table holding every string the container's distinct records
//!    use more than once, each stored once, most used first (ties by byte
//!    order) so the commonest take a one-byte reference;
//! 2. property, quantity and set records, each distinct record stored once
//!    across every edition in the container and referenced by index;
//! 3. one entry per edition: its [`SourceManifest`] and its sets in order.
//!
//! A string field is a slot: absent (`0`), inline text (`1`, length,
//! UTF-8), inline hexadecimal (`2`, length, bytes) or a table entry
//! (`3 + index`). A string used once stays inline next to the text around
//! it, which keeps the container as compressible as the text itself. A
//! lowercase hexadecimal string (a GUID or SHA-256), inline or in the
//! table, is stored as its bytes and read back as the same text.
//!
//! Integers are unsigned LEB128. Each record carries its length, so
//! decoding one edition of a container reads only that edition's records.
//! A complex property refers only to records before it, so the record
//! graph is acyclic by construction.
//!
//! The encoding is a lossless format shift: decoding yields exactly the
//! templates and manifests that were encoded, field for field. The encoder
//! is deterministic: equal input produces equal bytes.

mod decode;
mod encode;

pub(crate) use decode::{decode_all, decode_one, editions};
pub(crate) use encode::encode;

use crate::catalog::CatalogError;

/// Leading magic shared with the earlier bincode artifacts.
pub(crate) const MAGIC: [u8; 8] = *b"NEHPSDQ\0";
/// The container version this build reads and writes.
pub(crate) const FORMAT_VERSION: u64 = 3;
/// Resource budget for one container.
pub(crate) const MAX_BYTES: usize = 8 * 1024 * 1024;

/// Why a container could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PackError {
    TooLarge(usize),
    BadMagic,
    UnsupportedVersion(u64),
    Malformed(&'static str),
    TrailingBytes(usize),
    Catalog(CatalogError),
}

/// A string slot: absent, inline text, inline hexadecimal bytes, or a
/// table entry (`SLOT_TABLE + index`).
pub(crate) const SLOT_ABSENT: u64 = 0;
pub(crate) const SLOT_TEXT: u64 = 1;
pub(crate) const SLOT_HEX: u64 = 2;
pub(crate) const SLOT_TABLE: u64 = 3;
