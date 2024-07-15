use super::{align_up, Locked};
use core::{
    alloc::{GlobalAlloc, Layout},
    ptr,
};

pub struct Allocator {
    // start of the heap
    heap_start: usize,
    // end of the heap
    heap_end: usize,
    // always points to the first usable byte in the heap
    next: usize,
    // counter for the active allocations
    allocations: usize,
}

impl Allocator {
    /// Creates a new empty Allocator.
    #[must_use]
    pub const fn empty() -> Self {
        Allocator {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    /// Initializes the Allocator with the given heap bounds.
    ///
    /// # Safety
    /// This method is unsafe as the caller must ensure that the given
    /// memory range is usable. Also, this method must be called no more
    /// than once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

unsafe impl GlobalAlloc for Locked<Allocator> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let mut allocator = self.lock();

        let alloc_start = align_up(allocator.next, layout.align());
        let Some(alloc_end) = alloc_start.checked_add(layout.size()) else {
            // allocation caused integer overflow
            return ptr::null_mut();
        };

        if alloc_end > allocator.heap_end {
            ptr::null_mut() // no heap memory left
        } else {
            allocator.next = alloc_end;
            allocator.allocations += 1;
            alloc_start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut allocator = self.lock();

        allocator.allocations -= 1;
        if allocator.allocations == 0 {
            allocator.next = allocator.heap_start;
        }
    }
}
