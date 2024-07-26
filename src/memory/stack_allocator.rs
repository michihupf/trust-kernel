use super::{
    paging::{entry::EntryFlags, ActivePageTable, Page, PageIter},
    FrameAllocator, PAGE_SIZE,
};

pub struct StackAllocator {
    range: PageIter,
}

impl StackAllocator {
    pub fn new(pages: PageIter) -> StackAllocator {
        StackAllocator { range: pages }
    }

    pub fn alloc_stack<FA: FrameAllocator>(
        &mut self,
        active_table: &mut ActivePageTable,
        frame_allocator: &mut FA,
        n_pages: usize,
    ) -> Option<Stack> {
        if n_pages == 0 {
            return None; // zero sized stack is nonsensical
        }

        let mut range = self.range.clone();

        // try to allocate stack pages and guard page
        let guard_page = range.next();
        let stack_start = range.next();
        let stack_end = if n_pages == 1 {
            stack_start
        } else {
            // index starts at 0 and start is already allocated.
            range.nth(n_pages - 2)
        };

        match (guard_page, stack_start, stack_end) {
            (Some(_), Some(start), Some(end)) => {
                // success!
                self.range = range;

                // map stack pages to frames
                for page in Page::range_inclusive(start, end) {
                    active_table.map(page, EntryFlags::WRITABLE, frame_allocator);
                }

                // create a new stack
                let stack_top = end.start_address() + PAGE_SIZE;
                Some(Stack::new(stack_top, start.start_address()))
            }
            _ => None, // not enough space for stack
        }
    }
}

#[derive(Debug)]
pub struct Stack {
    top: usize,
    bottom: usize,
}

impl Stack {
    fn new(top: usize, bottom: usize) -> Stack {
        assert!(top > bottom);
        Stack { top, bottom }
    }

    pub fn top(&self) -> usize {
        self.top
    }

    pub fn bottom(&self) -> usize {
        self.bottom
    }
}
