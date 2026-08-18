//! The STEP value model.
//!
//! One attribute slot, as it appears in a record. Deliberately a borrowed,
//! non-owning view: a 500 MB model has tens of millions of these, so an owned
//! `String` per string attribute is not affordable.
//!
//! Not yet implemented — Stage 1 in `docs/ROADMAP.md`.

/// A single attribute value inside an entity record.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    /// `$` — omitted optional.
    Unset,
    /// `*` — derived in a subtype.
    Derived,
    /// `#123`
    Ref(u64),
    /// `.T.` / `.F.` / `.U.`
    Logical(Option<bool>),
    /// `.SOMENAME.`
    Enum(&'a [u8]),
    /// Integer or real, unparsed.
    Number(&'a [u8]),
    /// Still-escaped string literal; decode with [`crate::escape`].
    Text(&'a [u8]),
    /// `(...)` — an aggregate.
    List(Vec<Value<'a>>),
    /// `IFCTYPE(value)` — a select/defined-type wrapper.
    Typed {
        /// The wrapping type name.
        name: &'a [u8],
        /// The wrapped value.
        inner: Box<Value<'a>>,
    },
}
