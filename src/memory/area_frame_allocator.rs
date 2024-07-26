use multiboot2::MemoryArea;

use super::{Frame, FrameAllocator};

/// A [`FrameAllocator`] that returns usable frames from the bootloader's memory map.
pub struct AreaFrameAllocator {
    // frame counter
    next_free_frame: Frame,
    // memory area that contains `next_free_frame`
    current_area: Option<&'static MemoryArea>,
    // all memory areas
    areas: *const [MemoryArea],
    // avoid returning used fields
    kernel_start: Frame,
    // avoid returning used fields
    kernel_end: Frame,
    // avoid returning used fields
    mbi_start: Frame,
    // avoid returning used fields
    mbi_end: Frame,
}

impl AreaFrameAllocator {
    /// Create a [`FrameAllocator`] from the passed memory map.
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the passed memory
    /// map is valid. The main requirement is that all frames that are marked as `USABLE`
    /// in it are really unused.
    #[must_use]
    pub unsafe fn new(
        kernel_start: usize,
        kernel_end: usize,
        mbi_start: usize,
        mbi_end: usize,
        memory_areas: *const [MemoryArea],
    ) -> Self {
        let mut allocator = AreaFrameAllocator {
            next_free_frame: Frame::containing_address(0),
            current_area: None,
            areas: memory_areas,
            kernel_start: Frame::containing_address(kernel_start),
            kernel_end: Frame::containing_address(kernel_end),
            mbi_start: Frame::containing_address(mbi_start),
            mbi_end: Frame::containing_address(mbi_end),
        };

        allocator.pick_next_area(); // pick next area so current_area is correctly set
        allocator
    }
}

impl FrameAllocator for AreaFrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        if let Some(area) = self.current_area {
            let frame = Frame {
                number: self.next_free_frame.number,
            };

            let current_area_last_frame = {
                let addr = area.start_address() + area.size() - 1;
                Frame::containing_address(addr as usize)
            };

            if frame > current_area_last_frame {
                // current area has been used up
                self.pick_next_area();
            } else if frame >= self.kernel_start && frame <= self.kernel_end {
                // frame is used by kernel
                self.next_free_frame.number = self.kernel_end.number + 1;
            } else if frame >= self.mbi_start && frame <= self.mbi_end {
                // frame is used by mbi
                self.next_free_frame.number = self.mbi_end.number + 1;
            } else {
                // move on to next frame
                self.next_free_frame.number += 1;
                return Some(frame);
            }

            self.allocate_frame()
        } else {
            None // no frames left
        }
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        // TODO
    }
}

impl AreaFrameAllocator {
    fn pick_next_area(&mut self) {
        // Safety: self.areas is always pointing to our memory areas after initialization.
        let areas = unsafe { &*self.areas };
        self.current_area = areas
            .iter()
            .filter(|area| {
                let addr = area.start_address() + area.size() - 1;
                Frame::containing_address(addr as usize) >= self.next_free_frame
            })
            .min_by_key(|area| area.start_address());

        if let Some(area) = self.current_area {
            let start_frame = Frame::containing_address(area.start_address() as usize);
            if self.next_free_frame < start_frame {
                self.next_free_frame = start_frame;
            }
        }
    }
}
