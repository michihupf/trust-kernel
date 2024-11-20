pub mod bump;
pub mod list;

use self::list::Allocator;
use core::{alloc::GlobalAlloc, ptr::null_mut};

/// A wrapper around [`spin::Mutex`] to permit trait implementation.
pub struct Locked<T> {
    inner: spin::Mutex<T>,
}

impl<T> Locked<T> {
    pub const fn new(inner: T) -> Self {
        Locked {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> spin::MutexGuard<T> {
        self.inner.lock()
    }
}

/// Aligns the given address upwards to the alignment `align` given `align` is a power of 2.
fn align_up(addr: usize, align: usize) -> usize {
    let offset = (addr as *const u8).align_offset(align);
    addr + offset
}

// /// Aligns the given address downwards to the alignment `align` given `align` is a power of 2.
// fn align_down(addr: usize, align: usize) -> usize {
//     if align.is_power_of_two() {
//         addr & !(align - 1)
//     } else if align == 0 {
//         addr
//     } else {
//         panic!("`align` was not power of 2");
//     }
// }

/// A dummy heap allocator that always returns a null pointer signaling a
/// failure to allocate heap memory.
pub struct DummyAllocator;

// SAFETY: GlobalAlloc is unsafe, as the caller needs to ensure memory safety by providing a sane memory layout.
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: core::alloc::Layout) -> *mut u8 {
        null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        panic!("dealloc should never be called")
    }
}

#[global_allocator]
pub static ALLOCATOR: Locked<Allocator> = Locked::new(Allocator::empty());
