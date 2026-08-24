//! IFC GlobalId: the 22-character compressed GUID.
//!
//! # Why this is not standard base-64
//!
//! IFC packs a 128-bit UUID into 22 characters using its own alphabet ordered
//! `0-9 A-Z a-z _ $`, processed as four 6-digit base-64 groups. It is *not*
//! RFC 4648, and feeding it to a general base-64 decoder produces silent
//! garbage rather than an error — which is exactly the kind of bug that
//! surfaces as "some elements mysteriously fail to match" months later.
//!
//! Every IFC root object carries one, and it is the only stable cross-file
//! identity an element has, so correctness here underpins diffing, clash
//! tracking, and BCF issue references.

/// The IFC base-64 alphabet, in IFC's own order.
const ALPHABET: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";

/// Reverse lookup, built at compile time.
const fn reverse_table() -> [i8; 256] {
    let mut table = [-1i8; 256];
    let mut i = 0;
    while i < 64 {
        table[ALPHABET[i] as usize] = i as i8;
        i += 1;
    }
    table
}

const REVERSE: [i8; 256] = reverse_table();

/// A 22-character IFC GlobalId.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Guid([u8; 22]);

impl Guid {
    /// Wrap 22 ASCII characters, validating the alphabet.
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 22 {
            return None;
        }
        if bytes.iter().any(|&b| REVERSE[b as usize] < 0) {
            return None;
        }
        let mut buf = [0u8; 22];
        buf.copy_from_slice(bytes);
        Some(Self(buf))
    }

    /// The GlobalId as text.
    pub fn as_str(&self) -> &str {
        // SAFETY-free: every byte was validated against an ASCII alphabet in
        // `parse`, and `from_uuid` only emits alphabet bytes.
        std::str::from_utf8(&self.0).expect("alphabet is ASCII by construction")
    }

    /// Compress a raw 128-bit UUID into the IFC form.
    pub fn from_uuid(uuid: [u8; 16]) -> Self {
        let mut num = 0u128;
        for b in uuid {
            num = (num << 8) | b as u128;
        }
        // 22 base-64 digits cover 132 bits; the leading digit carries the
        // remaining 2 bits of the 128-bit value.
        let mut out = [b'0'; 22];
        let mut n = num;
        for slot in out.iter_mut().rev() {
            *slot = ALPHABET[(n & 0x3f) as usize];
            n >>= 6;
        }
        Self(out)
    }

    /// Expand back to the raw 128-bit UUID.
    pub fn to_uuid(self) -> [u8; 16] {
        let mut num = 0u128;
        for &b in &self.0 {
            num = (num << 6) | (REVERSE[b as usize] as u128);
        }
        let mut out = [0u8; 16];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = ((num >> (8 * (15 - i))) & 0xff) as u8;
        }
        out
    }
}

impl std::fmt::Display for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_length_and_foreign_characters() {
        assert!(Guid::parse("tooshort").is_none());
        // '+' and '/' are standard base-64 but NOT in IFC's alphabet.
        assert!(Guid::parse("0123456789ABCDEFGHIJ+/").is_none());
    }

    #[test]
    fn accepts_a_real_globalid_from_the_fixture_corpus() {
        assert!(Guid::parse("2O2Fr$t4X7Zf8NOew3FLOH").is_some());
    }

    /// The property that matters: compress then expand must be identity, or
    /// element identity silently changes on export.
    #[test]
    fn uuid_roundtrip_is_lossless() {
        let uuid: [u8; 16] = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        assert_eq!(Guid::from_uuid(uuid).to_uuid(), uuid);
    }

    #[test]
    fn text_roundtrip_is_lossless() {
        let g = Guid::parse("2O2Fr$t4X7Zf8NOew3FLOH").unwrap();
        assert_eq!(Guid::from_uuid(g.to_uuid()), g);
    }
}
