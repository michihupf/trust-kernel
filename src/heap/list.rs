use super::Locked;
use crate::heap::align_up;
use core::{
    alloc::{GlobalAlloc, Layout},
    mem, ptr,
};

struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    /// Creates a new list node with size `size`.
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }

    /// Returns the start adress of the associated memory region.
    fn start_addr(&self) -> usize {
        ptr::from_ref::<Self>(self) as usize
    }

    /// Returns the end address of the associated memory region
    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct Allocator {
    head: ListNode,
}

impl Allocator {
    /// Creates an empty Allocator
    #[must_use]
    pub const fn empty() -> Self {
        Allocator {
            head: ListNode::new(0),
        }
    }

    /// Initializes the Allocator with the given heap bounds.
    ///
    /// # Safety
    /// This method is unsafe as the caller must ensure that the given
    /// memory range is usable. Also, this method must be called no more
    /// than once.
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_mem_region(heap_start, heap_size);
    }

    /// Adds the given memory region to the front of the list
    unsafe fn add_free_mem_region(&mut self, addr: usize, size: usize) {
        // ensure that freed region is large enough to hold the ListNode
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        let mut node = ListNode::new(size);
        // TODO use a sorted linked list to be able to merge consecutive freed memory blocks
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr);
    }

    /// Finds a free memory region with the given `size` and `align`ment and removes it from the list.
    ///
    /// Returns a tuple of [`ListNode`] and the start adress of the allocation.
    fn find_free_mem_region(
        &mut self,
        size: usize,
        align: usize,
    ) -> Option<(&'static mut ListNode, usize)> {
        // traverse the list starting from the head node
        let mut cur = &mut self.head;
        while let Some(ref mut region) = cur.next {
            if let Some(alloc_start) = Self::alloc_from_region(region, size, align) {
                // region is okay to be allocated
                let next = region.next.take();
                let ret = Some((cur.next.take().unwrap(), alloc_start));
                cur.next = next;
                return ret;
            }
            // region is not okay to be allocated. traverse further
            cur = cur.next.as_mut().unwrap();
        }

        // did not find a memory region that was big enough
        None
    }

    /// Tries to allocate using the given `region`.
    ///
    /// Returns the start address of the `region` is sufficient to be allocated for a given `size` and `align`ment.
    /// Returns None when the memory region was insufficient.
    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Option<usize> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size)?;

        if alloc_end > region.end_addr() {
            // memory region is too small
            return None;
        }

        let remaining_size = region.end_addr() - alloc_end;
        if remaining_size > 0 && remaining_size < mem::size_of::<ListNode>() {
            // remainging free region is too small to hold a ListNode
            // fail the allocation because we can't split into used and free part
            return None;
        }

        Some(alloc_start)
    }

    /// Adjusts the `layout` so that the allocated memory region is capable of storing a [`ListNode`].
    ///
    /// Returns the adjusted size and alignment as a (size, align) tuple.
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("memory alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<Allocator> {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // re-align so that a ListNode may be stored
        let (size, align) = Allocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_free_mem_region(size, align) {
            let Some(alloc_end) = alloc_start.checked_add(size) else {
                // integer overflow indicates that we are out of memory
                return ptr::null_mut();
            };
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                allocator.add_free_mem_region(alloc_end, excess_size);
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // re-align so that a ListNode may be stored
        let (size, _align) = Allocator::size_align(layout);
        // deallocate
        self.lock().add_free_mem_region(ptr as usize, size);
    }
}
