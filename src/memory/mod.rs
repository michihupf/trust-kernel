use area_frame_allocator::AreaFrameAllocator;
use multiboot2::BootInformation;
use paging::{entry::EntryFlags, ActivePageTable, Page, PhysAddr};
use stack_allocator::{Stack, StackAllocator};
// use bootloader::bootinfo::{MemoryMap, MemoryRegionType};

pub mod area_frame_allocator;
pub mod heap;
pub mod paging;
pub mod stack_allocator;

use crate::status_print;

pub use self::paging::remap_kernel;
pub use paging::test_paging;

pub use heap::ALLOCATOR;

pub const PAGE_SIZE: usize = 4096;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB
pub const HEAP_END: usize = HEAP_START + HEAP_SIZE;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: usize,
}

pub trait FrameAllocator {
    fn kalloc_frame(&mut self) -> Option<Frame>;
    fn kfree_frame(&mut self, frame: Frame);
}

impl Frame {
    fn containing(addr: PhysAddr) -> Frame {
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

    fn start(&self) -> PhysAddr {
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

    pub fn id_map(&mut self, addr: PhysAddr, flags: EntryFlags) {
        self.active_table.map_to(
            Page::containing_address(addr),
            Frame::containing(addr),
            flags,
            &mut self.frame_allocator,
        );
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

    // SAFETY: memory areas will never be cleaned up or moved and USABLE areas are usable.
    let mut frame_allocator = unsafe {
        AreaFrameAllocator::new(
            kernel_start,
            kernel_end,
            mbi.start_address(),
            mbi.end_address(),
            memory_map_tag.memory_areas(),
        )
    };

    // prepare remapping
    status_print!("enabling NO_EXECUTE" => crate::enable_nxe_bit());
    status_print!("enabling write protection" => crate::enable_wp_bit());

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
