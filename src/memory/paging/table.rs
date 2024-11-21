use core::{
    marker::PhantomData,
    ops::{Index, IndexMut},
};

use crate::memory::FrameAllocator;

use super::{
    entry::{Entry, EntryFlags},
    ENTRY_COUNT,
};

pub const P4: *mut Table<Level4> = 0xffff_ffff_ffff_f000 as *mut _;

pub trait TableLevel {}

pub enum Level4 {}
pub enum Level3 {}
pub enum Level2 {}
pub enum Level1 {}

impl TableLevel for Level4 {}
impl TableLevel for Level3 {}
impl TableLevel for Level2 {}
impl TableLevel for Level1 {}

pub trait HierachicalLevel: TableLevel {
    type NextLevel: TableLevel;
}

impl HierachicalLevel for Level4 {
    type NextLevel = Level3;
}
impl HierachicalLevel for Level3 {
    type NextLevel = Level2;
}
impl HierachicalLevel for Level2 {
    type NextLevel = Level1;
}

pub struct Table<L: TableLevel> {
    entries: [Entry; ENTRY_COUNT],
    level: PhantomData<L>,
}

impl<L> Index<usize> for Table<L>
where
    L: TableLevel,
{
    type Output = Entry;

    fn index(&self, index: usize) -> &Entry {
        &self.entries[index]
    }
}

impl<L> IndexMut<usize> for Table<L>
where
    L: TableLevel,
{
    fn index_mut(&mut self, index: usize) -> &mut Entry {
        &mut self.entries[index]
    }
}

impl<L> Table<L>
where
    L: TableLevel,
{
    pub fn zero(&mut self) {
        for entry in &mut self.entries {
            entry.set_unused();
        }
    }
}

impl<L> Table<L>
where
    L: HierachicalLevel,
{
    fn next_table_address(&self, index: usize) -> Option<usize> {
        let entry_flags = self[index].flags();
        if entry_flags.contains(EntryFlags::PRESENT) && !entry_flags.contains(EntryFlags::HUGE_PAGE)
        {
            // SAFETY: entry is PRESENT and not HUGE_PAGE
            Some(unsafe { self.next_table_address_unchecked(index) })
        } else {
            None
        }
    }

    /// Returns the address to the next lower table without performing checks on the table entry flags.
    ///
    /// # Safety
    /// The caller must ensure the entry has the PRESENT flag and
    /// is not a HUGE_PAGE.
    #[inline]
    unsafe fn next_table_address_unchecked(&self, index: usize) -> usize {
        ((self as *const _ as usize) << 9) | (index << 12)
    }

    #[must_use]
    pub fn next_table(&self, index: usize) -> Option<&Table<L::NextLevel>> {
        self.next_table_address(index)
            // Safety: address is a valid page table address.
            .map(|addr| unsafe { &*(addr as *const _) })
    }

    #[must_use]
    pub fn next_table_mut(&self, index: usize) -> Option<&mut Table<L::NextLevel>> {
        self.next_table_address(index)
            // Safety: address is a valid page table address.
            .map(|addr| unsafe { &mut *(addr as *mut _) })
    }

    /// Returns or creates a next table at `index`.
    ///
    /// # Panics
    /// This method panics when the page at index is [`HUGE_PAGE`].
    pub fn next_table_create<A>(
        &mut self,
        index: usize,
        allocator: &mut A,
    ) -> &mut Table<L::NextLevel>
    where
        A: FrameAllocator,
    {
        if self.next_table(index).is_none() {
            assert!(
                !self[index].flags().contains(EntryFlags::HUGE_PAGE),
                "mapping code does not support huge pages"
            );

            let frame = allocator.kalloc_frame().expect("no more frames available");
            self[index].set(frame, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            self.next_table_mut(index).unwrap().zero();
        }
        self.next_table_mut(index).unwrap()
    }
}
