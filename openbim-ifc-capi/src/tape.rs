//! The value tape: attribute values as flat C data.
//!
//! A value is a pre-order array of [`OpenbimIfcValueNode`]. A container node
//! (`list`, `typed`) is followed by its children; `list` records how many,
//! `typed` always has exactly one. Strings are not in the nodes: each node
//! carries an `(offset, len)` byte range into one shared UTF-8 buffer, which
//! is how text and names cross without any Rust-owned pointer.
//!
//! `$`, `*`, `.U.` and `.F.` are distinct kinds or payloads, never folded,
//! for the same reason as in the JavaScript and Python bindings (ADR 0013).

use openbim_ifc_binding_core::value::{Kind, Tagged, MAX_NESTING};
use openbim_ifc_binding_core::BindingError;

/// Kind codes. Plain integers, not a Rust enum, so an unknown value from C
/// is rejected rather than read as an invalid discriminant.
pub const OPENBIM_IFC_KIND_NULL: i32 = 0;
/// `*`.
pub const OPENBIM_IFC_KIND_DERIVED: i32 = 1;
/// `.T.`/`.F.`; `int_value` is 1 or 0.
pub const OPENBIM_IFC_KIND_BOOL: i32 = 2;
/// `.U.`.
pub const OPENBIM_IFC_KIND_UNKNOWN: i32 = 3;
/// Integer in `int_value`.
pub const OPENBIM_IFC_KIND_INTEGER: i32 = 4;
/// Finite real in `real_value`.
pub const OPENBIM_IFC_KIND_REAL: i32 = 5;
/// Text in the string range.
pub const OPENBIM_IFC_KIND_TEXT: i32 = 6;
/// Binary digits in the string range.
pub const OPENBIM_IFC_KIND_BINARY: i32 = 7;
/// Enumeration name in the string range.
pub const OPENBIM_IFC_KIND_ENUM: i32 = 8;
/// Entity id in `int_value` (non-negative).
pub const OPENBIM_IFC_KIND_REF: i32 = 9;
/// Aggregate; `child_count` children follow.
pub const OPENBIM_IFC_KIND_LIST: i32 = 10;
/// Typed wrapper; type name in the string range, one child follows.
pub const OPENBIM_IFC_KIND_TYPED: i32 = 11;

/// One node of a value tape. Fields a kind does not use are zero.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct OpenbimIfcValueNode {
    /// One of the `OPENBIM_IFC_KIND_*` codes.
    pub kind: i32,
    /// Children following a `list` node; 0 otherwise.
    pub child_count: u32,
    /// Payload of `bool` (0/1), `integer` and `ref`.
    pub int_value: i64,
    /// Payload of `real`.
    pub real_value: f64,
    /// Byte offset of this node's string in the string buffer.
    pub str_offset: u64,
    /// Byte length of this node's string (no NUL terminator).
    pub str_len: u64,
}

/// A value flattened to a tape: the nodes plus their shared string bytes.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Tape {
    /// Nodes in pre-order.
    pub nodes: Vec<OpenbimIfcValueNode>,
    /// UTF-8 bytes every string range points into.
    pub strings: Vec<u8>,
}

impl Tape {
    /// Flatten one value.
    pub fn encode(value: &Tagged) -> Self {
        let mut tape = Self::default();
        tape.push(value);
        tape
    }

    /// Flatten several values back to back, as `attributes` returns them.
    pub fn encode_all(values: &[Tagged]) -> Self {
        let mut tape = Self::default();
        for value in values {
            tape.push(value);
        }
        tape
    }

    fn string(&mut self, node: &mut OpenbimIfcValueNode, text: &str) {
        node.str_offset = self.strings.len() as u64;
        node.str_len = text.len() as u64;
        self.strings.extend_from_slice(text.as_bytes());
    }

    fn push(&mut self, value: &Tagged) {
        let mut node = OpenbimIfcValueNode {
            kind: kind_code(value.kind()),
            ..OpenbimIfcValueNode::default()
        };
        match value {
            Tagged::Null | Tagged::Derived | Tagged::Unknown => {}
            Tagged::Bool(b) => node.int_value = i64::from(*b),
            Tagged::Integer(i) => node.int_value = *i,
            Tagged::Real(r) => node.real_value = *r,
            Tagged::Text(s) | Tagged::Binary(s) | Tagged::Enum(s) => self.string(&mut node, s),
            // Ids come from `EntityId(u64)` but real files stay far below
            // 2^63; the decoder refuses a negative id instead of wrapping.
            Tagged::Ref(id) => node.int_value = i64::try_from(*id).unwrap_or(i64::MAX),
            Tagged::List(items) => {
                node.child_count = u32::try_from(items.len()).unwrap_or(u32::MAX);
                self.nodes.push(node);
                for item in items {
                    self.push(item);
                }
                return;
            }
            Tagged::Typed { type_name, value } => {
                self.string(&mut node, type_name);
                self.nodes.push(node);
                self.push(value);
                return;
            }
        }
        self.nodes.push(node);
    }
}

/// The C code for a kind.
pub const fn kind_code(kind: Kind) -> i32 {
    match kind {
        Kind::Null => OPENBIM_IFC_KIND_NULL,
        Kind::Derived => OPENBIM_IFC_KIND_DERIVED,
        Kind::Bool => OPENBIM_IFC_KIND_BOOL,
        Kind::Unknown => OPENBIM_IFC_KIND_UNKNOWN,
        Kind::Integer => OPENBIM_IFC_KIND_INTEGER,
        Kind::Real => OPENBIM_IFC_KIND_REAL,
        Kind::Text => OPENBIM_IFC_KIND_TEXT,
        Kind::Binary => OPENBIM_IFC_KIND_BINARY,
        Kind::Enum => OPENBIM_IFC_KIND_ENUM,
        Kind::Ref => OPENBIM_IFC_KIND_REF,
        Kind::List => OPENBIM_IFC_KIND_LIST,
        Kind::Typed => OPENBIM_IFC_KIND_TYPED,
    }
}

fn invalid(detail: impl Into<String>) -> BindingError {
    BindingError::InvalidValue(detail.into())
}

/// Reads values back off a caller's tape, checking every field.
pub struct Reader<'a> {
    nodes: &'a [OpenbimIfcValueNode],
    strings: &'a [u8],
    next: usize,
}

impl<'a> Reader<'a> {
    /// Read from `nodes`, whose string ranges index into `strings`.
    pub fn new(nodes: &'a [OpenbimIfcValueNode], strings: &'a [u8]) -> Self {
        Self {
            nodes,
            strings,
            next: 0,
        }
    }

    /// Decode exactly one value, then require the tape to be used up.
    pub fn single(mut self) -> Result<Tagged, BindingError> {
        let value = self.value(0)?;
        self.finish()?;
        Ok(value)
    }

    /// Decode `count` values back to back, then require the tape used up.
    pub fn many(mut self, count: usize) -> Result<Vec<Tagged>, BindingError> {
        let values = (0..count)
            .map(|_| self.value(0))
            .collect::<Result<Vec<_>, _>>()?;
        self.finish()?;
        Ok(values)
    }

    /// Trailing nodes mean the caller's counts disagree with the tape;
    /// refusing them catches an off-by-one in the host instead of ignoring it.
    fn finish(self) -> Result<(), BindingError> {
        if self.next == self.nodes.len() {
            Ok(())
        } else {
            Err(invalid(format!(
                "tape has {} nodes but only {} were used",
                self.nodes.len(),
                self.next
            )))
        }
    }

    fn string(&self, node: &OpenbimIfcValueNode) -> Result<String, BindingError> {
        let start = usize::try_from(node.str_offset).map_err(|_| invalid("string offset"))?;
        let len = usize::try_from(node.str_len).map_err(|_| invalid("string length"))?;
        let end = start
            .checked_add(len)
            .filter(|end| *end <= self.strings.len())
            .ok_or_else(|| invalid("string range outside the string buffer"))?;
        std::str::from_utf8(&self.strings[start..end])
            .map(str::to_owned)
            .map_err(|_| invalid("string is not UTF-8"))
    }

    fn no_string(node: &OpenbimIfcValueNode) -> Result<(), BindingError> {
        if node.str_len == 0 {
            Ok(())
        } else {
            Err(invalid("this kind carries no string"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Tagged, BindingError> {
        if depth > MAX_NESTING {
            return Err(invalid(format!("nesting deeper than {MAX_NESTING}")));
        }
        let node = *self
            .nodes
            .get(self.next)
            .ok_or_else(|| invalid("tape ends before the value does"))?;
        self.next += 1;
        if node.kind != OPENBIM_IFC_KIND_LIST && node.child_count != 0 {
            return Err(invalid("only a list node may have children"));
        }
        let string_kind = matches!(
            node.kind,
            OPENBIM_IFC_KIND_TEXT
                | OPENBIM_IFC_KIND_BINARY
                | OPENBIM_IFC_KIND_ENUM
                | OPENBIM_IFC_KIND_TYPED
        );
        if !string_kind {
            Self::no_string(&node)?;
        }
        Ok(match node.kind {
            OPENBIM_IFC_KIND_NULL => Tagged::Null,
            OPENBIM_IFC_KIND_DERIVED => Tagged::Derived,
            OPENBIM_IFC_KIND_UNKNOWN => Tagged::Unknown,
            OPENBIM_IFC_KIND_BOOL => match node.int_value {
                0 => Tagged::Bool(false),
                1 => Tagged::Bool(true),
                other => return Err(invalid(format!("bool must be 0 or 1, got {other}"))),
            },
            OPENBIM_IFC_KIND_INTEGER => Tagged::Integer(node.int_value),
            OPENBIM_IFC_KIND_REAL => Tagged::Real(node.real_value),
            OPENBIM_IFC_KIND_TEXT => Tagged::Text(self.string(&node)?),
            OPENBIM_IFC_KIND_BINARY => Tagged::Binary(self.string(&node)?),
            OPENBIM_IFC_KIND_ENUM => Tagged::Enum(self.string(&node)?),
            OPENBIM_IFC_KIND_REF => Tagged::Ref(
                u64::try_from(node.int_value)
                    .map_err(|_| invalid("ref id must not be negative"))?,
            ),
            OPENBIM_IFC_KIND_LIST => {
                // Bounded by the tape: each child consumes a node, so a huge
                // count fails at "tape ends" instead of allocating up front.
                let mut items = Vec::new();
                for _ in 0..node.child_count {
                    items.push(self.value(depth + 1)?);
                }
                Tagged::List(items)
            }
            OPENBIM_IFC_KIND_TYPED => Tagged::Typed {
                type_name: self.string(&node)?,
                value: Box::new(self.value(depth + 1)?),
            },
            other => return Err(invalid(format!("unknown kind code {other}"))),
        })
    }
}

#[cfg(test)]
mod tests;
