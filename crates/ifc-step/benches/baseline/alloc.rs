//! A counting global allocator: heap bytes live and their high-water mark.
//!
//! Counting is off by default, so the timing passes pay one relaxed load
//! per allocation and never contend on a shared counter (the lazy read and
//! `decode_all` allocate from up to eight threads). [`measure`] switches it
//! on around one call.
//!
//! What it sees: every byte requested through Rust's global allocator by
//! any thread of this process. What it does not see: allocator overhead
//! and fragmentation (glibc's own headers, retained arenas), thread stacks,
//! and file pages of a memory map, which live in the page cache rather
//! than the heap. A mapped read therefore shows only its index and slots.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering::Relaxed};

/// The bench binary's global allocator: [`System`] plus counters.
pub(crate) struct Counting;

static ON: AtomicBool = AtomicBool::new(false);
static LIVE: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);

fn grow(bytes: usize) {
    let live = LIVE.fetch_add(bytes as isize, Relaxed) + bytes as isize;
    PEAK.fetch_max(live, Relaxed);
}

fn shrink(bytes: usize) {
    LIVE.fetch_sub(bytes as isize, Relaxed);
}

// SAFETY: every call forwards to `System` with the caller's layout
// unchanged; the counters never affect what is returned.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded under the caller's contract.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() && ON.load(Relaxed) {
            grow(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: forwarded under the caller's contract.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() && ON.load(Relaxed) {
            grow(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: forwarded under the caller's contract.
        unsafe { System.dealloc(ptr, layout) };
        if ON.load(Relaxed) {
            shrink(layout.size());
        }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: forwarded under the caller's contract.
        let new = unsafe { System.realloc(ptr, layout, new_size) };
        if !new.is_null() && ON.load(Relaxed) {
            // A moving realloc briefly holds both blocks; counting only the
            // difference understates that instant by the old size.
            if new_size >= layout.size() {
                grow(new_size - layout.size());
            } else {
                shrink(layout.size() - new_size);
            }
        }
        new
    }
}

/// Heap growth of one call, in bytes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Memory {
    /// Live bytes after the call while its result is still held: what the
    /// result retains, net of anything the call freed.
    pub(crate) retained: isize,
    /// The high-water mark during the call, above the live bytes at its
    /// start.
    pub(crate) peak: isize,
}

/// Runs `routine` on `input` with counting on and reports its heap growth.
///
/// The result is dropped after counting stops, so freeing memory that was
/// allocated before the call cannot drive the counters negative.
pub(crate) fn measure<I, O>(input: I, routine: impl FnOnce(I) -> O) -> (Memory, O) {
    LIVE.store(0, Relaxed);
    PEAK.store(0, Relaxed);
    ON.store(true, Relaxed);
    let output = std::hint::black_box(routine(std::hint::black_box(input)));
    ON.store(false, Relaxed);
    let memory = Memory {
        retained: LIVE.load(Relaxed),
        peak: PEAK.load(Relaxed),
    };
    (memory, output)
}
