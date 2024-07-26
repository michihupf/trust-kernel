use area_frame_allocator::AreaFrameAllocator;
use multiboot2::BootInformation;
use paging::{entry::EntryFlags, ActivePageTable, Page, PhysAddr};
use stack_allocator::{Stack, StackAllocator};
// use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

pub mod area_frame_allocator;
pub mod paging;
pub mod stack_allocator;

pub use self::paging::remap_kernel;
pub use paging::test_paging;

pub use crate::heap::ALLOCATOR;

pub const PAGE_SIZE: usize = 4096;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB
pub const HEAP_END: usize = HEAP_START + HEAP_SIZE;

/// Initialize a new [`OffsetPageTable`].
///
/// # Safety
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
#[must_use]
// pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
//     let l4_page_table = active_l4_page_table(physical_memory_offset);
//     OffsetPageTable::new(l4_page_table, physical_memory_offset)
// }

/// Returns a mutable reference to the active level 4 page table.
///
/// # Safety
/// This function is unsafe as the caller must guarantee that the complete physical memory is
/// mapped to virutal memory at the passed `physical_memory_offset`. Also, this function must
/// only be called one to avoid undefined behaviour because of `&mut` reference aliasing.
// unsafe fn active_l4_page_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
//     use x86_64::registers::control::Cr3;

//     let (l4_table_frame, _flags) = Cr3::read();

//     let phys = l4_table_frame.start_address();
//     let virt = physical_memory_offset + phys.as_u64();
//     let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

//     &mut *page_table_ptr // unsafe
// }

// /// Creates an example mapping for the given page to frame `0xb8000` (the VGA text buffer).
// ///
// /// # Panics
// /// Panics when memory mapping fails.
// pub fn create_example_mapping(
//     page: Page,
//     mapper: &mut OffsetPageTable,
//     frame_allocator: &mut impl FrameAllocator<Size4KiB>,
// ) {
//     use x86_64::structures::paging::PageTableFlags as Flags;

//     let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
//     let flags = Flags::PRESENT | Flags::WRITABLE;

//     let map_to_result = unsafe {
//         // FIXME: this is not safe
//         mapper.map_to(page, frame, flags, frame_allocator)
//     };
//     map_to_result.expect("map_to failed").flush();
// }

// /// A [`FrameAllocator`] that always returns `None`.
// pub struct EmptyFrameAllocator;

// unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
//     fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
//         None
//     }
// }

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize,
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}

impl Frame {
    fn containing_address(addr: usize) -> Frame {
        Frame {
            number: addr / PAGE_SIZE,
        }
    }

    fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
        FrameIter { start, end }
    }

    fn clone(&self) -> Frame {
        Frame {
            number: self.number,
        }
    }

    fn start_address(&self) -> PhysAddr {
        self.number * PAGE_SIZE
    }
}

struct FrameIter {
    start: Frame,
    end: Frame,
}

impl Iterator for FrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.end {
            let frame = self.start.clone();
            self.start.number += 1;
            Some(frame)
        } else {
            None
        }
    }
}

pub struct MemoryController {
    active_table: ActivePageTable,
    frame_allocator: AreaFrameAllocator,
    stack_allocator: StackAllocator,
}

impl MemoryController {
    pub fn alloc_stack(&mut self, n_pages: usize) -> Option<Stack> {
        let &mut MemoryController {
            ref mut active_table,
            ref mut frame_allocator,
            ref mut stack_allocator,
        } = self;

        stack_allocator.alloc_stack(active_table, frame_allocator, n_pages)
    }
}

pub fn init(mbi: &BootInformation) -> MemoryController {
    let memory_map_tag = mbi.memory_map_tag().expect("Memory map tag required");
    let elf_sections = mbi.elf_sections().expect("Elf sections required");

    let kernel_start = elf_sections
        .clone()
        .filter(|s| s.is_allocated())
        .map(|s| s.start_address())
        .min()
        .unwrap() as usize;
    let kernel_end = elf_sections
        .filter(|s| s.is_allocated())
        .map(|s| s.end_address())
        .max()
        .unwrap() as usize;

    // Safety: memory areas will never be cleaned up or moved and USABLE areas are usable.
    let mut frame_allocator = unsafe {
        AreaFrameAllocator::new(
            kernel_start,
            kernel_end,
            mbi.start_address(),
            mbi.end_address(),
            memory_map_tag.memory_areas(),
        )
    };

    let mut active_table = paging::remap_kernel(&mut frame_allocator, mbi);

    let heap_start = Page::containing_address(HEAP_START);
    let heap_end = Page::containing_address(HEAP_END - 1);

    for page in Page::range_inclusive(heap_start, heap_end) {
        active_table.map(page, EntryFlags::WRITABLE, &mut frame_allocator);
    }

    // initialize the allocator
    {
        let locked_allocator = &mut *ALLOCATOR.lock();
        // SAFETY: init is only called here and [HEAP_START, HEAP_END) is usable.
        unsafe { locked_allocator.init(HEAP_START, HEAP_SIZE) }
    }

    let stack_allocator = {
        let stack_alloc_start = heap_end + 1;
        let stack_alloc_end = stack_alloc_start + 100;
        let stack_alloc_range = Page::range_inclusive(stack_alloc_start, stack_alloc_end);
        StackAllocator::new(stack_alloc_range)
    };

    MemoryController {
        active_table,
        frame_allocator,
        stack_allocator,
    }
}
