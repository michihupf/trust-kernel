use core::ops::{Add, Deref, DerefMut};

use entry::EntryFlags;
use mapper::Mapper;
use multiboot2::BootInformation;
use tmp_page::TemporaryPage;
use x86_64::structures::paging::PhysFrame;

use crate::println;

use super::{Frame, FrameAllocator, PAGE_SIZE};

pub mod entry;
pub mod mapper;
pub mod table;
pub mod tmp_page;

/// Page table entry count
const ENTRY_COUNT: usize = 512;

pub type PhysAddr = usize;
pub type VirtAddr = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
    number: usize,
}

impl Page {
    pub fn containing_address(addr: VirtAddr) -> Page {
        assert!(
            !(0x0000_8000_0000_0000..0xffff_8000_0000_0000).contains(&addr),
            "invalid address: 0x{addr:x}"
        );
        Page {
            number: addr / PAGE_SIZE,
        }
    }

    pub fn range_inclusive(start: Page, end: Page) -> PageIter {
        PageIter { start, end }
    }

    pub fn start_address(&self) -> VirtAddr {
        self.number * PAGE_SIZE
    }

    fn p4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }

    fn p3_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }

    fn p2_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }

    fn p1_index(&self) -> usize {
        self.number & 0o777
    }
}

impl Add<usize> for Page {
    type Output = Page;

    fn add(self, rhs: usize) -> Self::Output {
        Page {
            number: self.number + rhs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PageIter {
    start: Page,
    end: Page,
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.end {
            let page = self.start;
            self.start.number += 1;
            Some(page)
        } else {
            None
        }
    }
}

pub fn test_paging<A>(allocator: &mut A)
where
    A: FrameAllocator,
{
    // Safety: This is the only active page table!
    let mut page_table = unsafe { ActivePageTable::new() };

    let addr = 42 * 512 * 512 * 4096; // 42-th P3 entry
    let page = Page::containing_address(addr);
    let frame = allocator.allocate_frame().expect("no more frames");

    println!("None = {:?}, map to {frame:?}", page_table.translate(addr));

    page_table.map_to(page, frame, EntryFlags::empty(), allocator);

    println!("Some = {:?}", page_table.translate(addr));
    println!("next free frame: {:?}", allocator.allocate_frame());

    page_table.unmap(Page::containing_address(addr), allocator);
    // println!("None = {:?}", page_table.translate(addr));

    // println!("{:x}", unsafe {
    // *(Page::containing_address(addr).start_address() as *const u64)
    // });
}

pub struct ActivePageTable {
    mapper: Mapper,
}

impl Deref for ActivePageTable {
    type Target = Mapper;

    fn deref(&self) -> &Mapper {
        &self.mapper
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Mapper {
        &mut self.mapper
    }
}

impl ActivePageTable {
    unsafe fn new() -> ActivePageTable {
        ActivePageTable {
            mapper: Mapper::new(),
        }
    }

    /// Executes a closure `f` as if `table` was the active page table.
    pub fn with<F>(&mut self, table: &mut InactivePageTable, tmp_page: &mut TemporaryPage, f: F)
    where
        F: FnOnce(&mut Mapper),
    {
        use x86_64::instructions::tlb;

        // FIXME when rewriting read from Cr3
        let backup = self.p4()[511].pointed_frame().unwrap();

        // map temporary page to current p4 table
        let p4_table = tmp_page.map_table_frame(backup.clone(), self);

        // overwrite recursive mapping
        self.mapper.p4_mut()[511].set(
            table.p4_frame.clone(),
            EntryFlags::PRESENT | EntryFlags::WRITABLE,
        );
        tlb::flush_all();

        // execute f in the new context
        f(&mut self.mapper);

        // restore recursive mapping to original
        p4_table[511].set(backup, EntryFlags::PRESENT | EntryFlags::WRITABLE);
        tlb::flush_all();

        tmp_page.unmap(self);
    }

    /// Switches the active page table with `new_table` and returns the old one.
    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        use x86_64::registers::control;
        use x86_64::PhysAddr;

        let old_table = InactivePageTable {
            p4_frame: self.p4()[511].pointed_frame().unwrap(),
        };

        let flags = control::Cr3::read().1;
        // Safety: new table is a valid P4 table.
        unsafe {
            control::Cr3::write(
                PhysFrame::from_start_address_unchecked(PhysAddr::new(
                    new_table.p4_frame.start_address() as u64,
                )),
                flags,
            );
        }

        old_table
    }
}

pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(
        frame: Frame,
        active_table: &mut ActivePageTable,
        tmp_page: &mut TemporaryPage,
    ) -> InactivePageTable {
        {
            let table = tmp_page.map_table_frame(frame.clone(), active_table);
            table.zero();
            // setup recursive mapping for table:
            table[511].set(frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
        }
        tmp_page.unmap(active_table);

        InactivePageTable { p4_frame: frame }
    }
}

pub fn remap_kernel<A>(allocator: &mut A, mbi: &BootInformation) -> ActivePageTable
where
    A: FrameAllocator,
{
    let mut tmp_page = TemporaryPage::new(Page { number: 0xcafebabe }, allocator);
    // Safety: used to set the new active page table below. It is the only one!
    let mut active_table = unsafe { ActivePageTable::new() };
    let mut new_table = {
        let frame = allocator.allocate_frame().expect("no more frames");
        InactivePageTable::new(frame, &mut active_table, &mut tmp_page)
    };

    active_table.with(&mut new_table, &mut tmp_page, |mapper| {
        let elf_sections = mbi.elf_sections().expect("Memory map tag required");

        for section in elf_sections {
            use entry::EntryFlags;

            if !section.is_allocated() {
                // section is not loaded to memory
                continue;
            }
            assert!(
                section.start_address() % PAGE_SIZE as u64 == 0,
                "sections need to be page aligned"
            );

            println!(
                "mapping section at addr: {:#x}, size: {:#x}",
                section.start_address(),
                section.size()
            );

            let flags = EntryFlags::from_elf_section(&section);

            let start_frame = Frame::containing_address(section.start_address() as usize);
            let end_frame = Frame::containing_address(section.end_address() as usize - 1);
            for frame in Frame::range_inclusive(start_frame, end_frame) {
                mapper.id_map(frame, flags, allocator);
            }
        }

        // identity map the VGA text buffer
        let vga_buffer_frame = Frame::containing_address(0xb8000);
        mapper.id_map(vga_buffer_frame, EntryFlags::WRITABLE, allocator);

        // identity map the MBI
        let mbi_start = Frame::containing_address(mbi.start_address());
        let mbi_end = Frame::containing_address(mbi.end_address() - 1);
        for frame in Frame::range_inclusive(mbi_start, mbi_end) {
            mapper.id_map(frame, EntryFlags::PRESENT, allocator);
        }
    });

    let old_table = active_table.switch(new_table);

    // turn old P4 table into guard page
    let old_p4 = Page::containing_address(old_table.p4_frame.start_address());
    active_table.unmap(old_p4, allocator);
    println!("guard page at {:#x}", old_p4.start_address());

    active_table
}
