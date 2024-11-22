use core::ptr::Unique;

use crate::{
    memory::{paging::ENTRY_COUNT, Frame, FrameAllocator, PAGE_SIZE},
    println,
};

use super::{
    entry::EntryFlags,
    table::{self, Level3, Level4, Table},
    Page, PhysAddr, VirtAddr,
};

pub struct Mapper {
    p4: Unique<Table<Level4>>,
}

impl Mapper {
    /// Creates a new Mapper.
    ///
    /// # Safety
    /// The caller must ensure that this is only done once!
    pub unsafe fn new() -> Mapper {
        Mapper {
            p4: Unique::new_unchecked(table::P4),
        }
    }

    /// P4 Table immutable accessor.
    pub fn p4(&self) -> &Table<Level4> {
        // SAFETY: safe as only valid pointer is created
        unsafe { self.p4.as_ref() }
    }

    /// P4 Table mutable accessor.
    pub fn p4_mut(&mut self) -> &mut Table<Level4> {
        // SAFETY: safe as only valid pointer is created
        unsafe { self.p4.as_mut() }
    }

    fn calculate_huge_page_frame(&self, p3: Option<&Table<Level3>>, page: Page) -> Option<Frame> {
        p3.and_then(|p3| {
            let p3_entry = &p3[page.p3_index()];
            // 1GiB page?
            if let Some(start_frame) = p3_entry.pointed_frame() {
                if p3_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                    // address must be 1GiB aligned
                    assert!(start_frame.number % (ENTRY_COUNT * ENTRY_COUNT) == 0);
                    return Some(Frame {
                        number: start_frame.number
                            + page.p2_index() * ENTRY_COUNT
                            + page.p1_index(),
                    });
                }
            }
            if let Some(p2) = p3.next_table(page.p3_index()) {
                let p2_entry = &p2[page.p2_index()];
                // 2MiB page?
                if let Some(start_frame) = p2_entry.pointed_frame() {
                    if p2_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                        // address must be 2MiB aligned
                        assert!(start_frame.number % ENTRY_COUNT == 0);
                        return Some(Frame {
                            number: start_frame.number + page.p1_index(),
                        });
                    }
                }
            }
            None
        })
    }

    /// Helper function to translate a `page` to its frame.
    pub fn translate_page(&self, page: Page) -> Option<Frame> {
        let p3 = self.p4().next_table(page.p4_index());

        p3.and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1[page.p1_index()].pointed_frame())
            .or_else(|| self.calculate_huge_page_frame(p3, page))
    }

    /// Translates a virtual address to a phyiscal one.
    pub fn translate(&self, addr: VirtAddr) -> Option<PhysAddr> {
        let offset = addr % PAGE_SIZE;
        self.translate_page(Page::containing(addr))
            .map(|frame| frame.start() + offset)
    }

    /// Maps a provided `page` to the provided `frame` with `flags`.
    pub fn map_to<A>(&mut self, page: Page, frame: Frame, flags: EntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let p4 = self.p4_mut();
        let p3 = p4.next_table_create(page.p4_index(), allocator);
        let p2 = p3.next_table_create(page.p3_index(), allocator);
        let p1 = p2.next_table_create(page.p2_index(), allocator);

        assert!(p1[page.p1_index()].is_unused());
        p1[page.p1_index()].set(frame, flags | EntryFlags::PRESENT);
    }

    /// Maps a provided `page` into a free frame with `flags`.
    pub fn map<A>(&mut self, page: Page, flags: EntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let frame = allocator.kalloc_frame().expect("no frames available");
        self.map_to(page, frame, flags, allocator);
    }

    /// Identity maps memory.
    pub fn id_map<A>(&mut self, frame: Frame, flags: EntryFlags, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        let page = Page::containing(frame.start());
        self.map_to(page, frame, flags, allocator);
    }

    /// Unmaps a `page`.
    ///
    /// # Panics
    /// This method will panic if one of the following conditions is met:
    /// - `page` is not mapped
    /// - `page` is a huge page
    pub fn unmap<A>(&mut self, page: Page, allocator: &mut A)
    where
        A: FrameAllocator,
    {
        assert!(self.translate(page.start()).is_some());

        let p1 = self
            .p4_mut()
            .next_table_mut(page.p4_index())
            .and_then(|p3| p3.next_table_mut(page.p3_index()))
            .and_then(|p2| p2.next_table_mut(page.p2_index()))
            .expect("mapping code does not support huge pages");

        let frame = p1[page.p1_index()].pointed_frame().unwrap();
        p1[page.p1_index()].set_unused();

        use x86_64::instructions::tlb;
        use x86_64::VirtAddr;

        tlb::flush(VirtAddr::new(page.start() as u64));

        // TODO free p(1,2,3) if empty
        allocator.kfree_frame(frame);
    }
}
