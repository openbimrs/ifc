//! Every read and write of caller memory, each behind its own check.
//!
//! The ABI functions never dereference a pointer themselves; they call these
//! helpers, so the unsafe surface is this file. Each helper states the
//! caller contract it relies on in a `# Safety` section and checks null and
//! length before touching memory.

use crate::status::OpenbimIfcStatus;

/// Borrow `len` bytes from `ptr`. A null pointer is allowed only for `len`
/// 0, which yields an empty slice.
///
/// # Safety
/// If non-null, `ptr` must be valid for reads of `len` bytes for the
/// duration of the call and not mutated concurrently.
pub(crate) unsafe fn bytes<'a>(ptr: *const u8, len: usize) -> Result<&'a [u8], OpenbimIfcStatus> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(OpenbimIfcStatus::NullPointer);
    }
    // SAFETY: non-null, and the caller guarantees `len` readable bytes.
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

/// Borrow `len` bytes from `ptr` as UTF-8 text.
///
/// # Safety
/// As for [`bytes`].
pub(crate) unsafe fn text<'a>(ptr: *const u8, len: usize) -> Result<&'a str, OpenbimIfcStatus> {
    // SAFETY: forwarded caller contract.
    let raw = unsafe { bytes(ptr, len) }?;
    std::str::from_utf8(raw).map_err(|_| OpenbimIfcStatus::InvalidArgument)
}

/// Borrow `len` elements of `T` from `ptr`; null only for `len` 0.
///
/// # Safety
/// If non-null, `ptr` must be aligned and valid for reads of `len` values
/// of `T` for the duration of the call.
pub(crate) unsafe fn slice<'a, T>(ptr: *const T, len: usize) -> Result<&'a [T], OpenbimIfcStatus> {
    if len == 0 {
        return Ok(&[]);
    }
    if ptr.is_null() {
        return Err(OpenbimIfcStatus::NullPointer);
    }
    // SAFETY: non-null, and the caller guarantees `len` aligned readable values.
    Ok(unsafe { std::slice::from_raw_parts(ptr, len) })
}

/// Write one value to `out`.
///
/// # Safety
/// If non-null, `out` must be aligned and valid for one write of `T`.
pub(crate) unsafe fn put<T>(out: *mut T, value: T) -> Result<(), OpenbimIfcStatus> {
    if out.is_null() {
        return Err(OpenbimIfcStatus::NullPointer);
    }
    // SAFETY: non-null, and the caller guarantees aligned writable storage.
    unsafe { out.write(value) };
    Ok(())
}

/// Copy `source` into a caller buffer of `capacity` elements, after writing
/// the required length to `out_required`.
///
/// `buffer` may be null only when `capacity` is 0: that is the size query,
/// which still reports `BufferTooSmall` if anything would have been copied.
///
/// # Safety
/// `out_required` must be as for [`put`]. If non-null, `buffer` must be
/// aligned and valid for writes of `capacity` values of `T`.
pub(crate) unsafe fn fill<T: Copy>(
    source: &[T],
    buffer: *mut T,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    if capacity != 0 && buffer.is_null() {
        return OpenbimIfcStatus::NullPointer;
    }
    // SAFETY: forwarded caller contract for `out_required`.
    if let Err(status) = unsafe { put(out_required, source.len()) } {
        return status;
    }
    if capacity < source.len() {
        return OpenbimIfcStatus::BufferTooSmall;
    }
    if !source.is_empty() {
        // SAFETY: `buffer` is non-null (capacity >= len > 0), holds at least
        // `source.len()` values, and cannot overlap a Rust-owned `source`.
        unsafe { std::ptr::copy_nonoverlapping(source.as_ptr(), buffer, source.len()) };
    }
    OpenbimIfcStatus::Ok
}

/// Copy `text` plus a trailing NUL into a caller byte buffer; the required
/// size includes the NUL, so C can use the result as a string directly.
///
/// # Safety
/// As for [`fill`].
pub(crate) unsafe fn fill_c_string(
    text: &str,
    buffer: *mut u8,
    capacity: usize,
    out_required: *mut usize,
) -> OpenbimIfcStatus {
    let mut owned = Vec::with_capacity(text.len() + 1);
    owned.extend_from_slice(text.as_bytes());
    owned.push(0);
    // SAFETY: forwarded caller contract.
    unsafe { fill(&owned, buffer, capacity, out_required) }
}
