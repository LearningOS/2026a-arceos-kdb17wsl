#![no_std]

use allocator::{BaseAllocator, ByteAllocator, PageAllocator};

/// Early memory allocator
/// Use it before formal bytes-allocator and pages-allocator can work!
/// This is a double-end memory range:
/// - Alloc bytes forward
/// - Alloc pages backward
///
/// [ bytes-used | avail-area | pages-used ]
/// |            | -->    <-- |            |
/// start       b_pos        p_pos       end
///
/// For bytes area, 'count' records number of allocations.
/// When it goes down to ZERO, free bytes-used area.
/// For pages area, it will never be freed!
///
pub struct EarlyAllocator<const SIZE: usize> {
    start: usize,
    end: usize,
    bytes_pos: usize,
    pages_pos: usize,
    bytes_count: usize,
    pages_count: usize,
}

impl<const SIZE: usize> EarlyAllocator<SIZE> {
    pub const fn new() -> Self {
        Self {
            start: 0,
            end: 0,
            bytes_pos: 0,
            pages_pos: 0,
            bytes_count: 0,
            pages_count: 0,
        }
    }
}

impl<const SIZE: usize> BaseAllocator for EarlyAllocator<SIZE> {
    fn init(&mut self, start: usize, size: usize) {
        // todo!()
        self.start = start;
        self.end = start + size;
        self.bytes_pos = start;
        self.pages_pos = start + size;
        self.bytes_count = 0;
        self.pages_count = 0;
    }

    fn add_memory(&mut self, start: usize, size: usize) -> allocator::AllocResult {
        let end = start + size;
        if start == end {
            return Err(allocator::AllocError::InvalidParam);
        }

        if end == self.start {
            self.start = start;
        } else if start == self.end {
            self.end = end;
        } else {
            return Err(allocator::AllocError::InvalidParam);
        }

        Ok(())
    }
}

impl<const SIZE: usize> ByteAllocator for EarlyAllocator<SIZE> {
    fn alloc(
        &mut self,
        layout: core::alloc::Layout,
    ) -> allocator::AllocResult<core::ptr::NonNull<u8>> {
        let align = layout.align();
        let aligned_pos = (self.bytes_pos + align - 1) & !(align - 1);
        let alloc_end = aligned_pos + layout.size();
        if alloc_end > self.pages_pos {
            return Err(allocator::AllocError::NoMemory);
        }
        self.bytes_pos = alloc_end;
        self.bytes_count += 1;
        Ok(core::ptr::NonNull::new(aligned_pos as *mut u8).unwrap())
    }

    fn dealloc(&mut self, pos: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        let pos = pos.as_ptr() as usize;
        if pos < self.start || pos >= self.bytes_pos {
            return;
        }
        self.bytes_count -= 1;
        if self.bytes_count == 0 {
            self.bytes_pos = self.start;
        }
    }

    fn total_bytes(&self) -> usize {
        self.end - self.start
    }

    fn used_bytes(&self) -> usize {
        self.bytes_pos - self.start
    }

    fn available_bytes(&self) -> usize {
        self.pages_pos - self.bytes_pos
    }
}

impl<const SIZE: usize> PageAllocator for EarlyAllocator<SIZE> {
    const PAGE_SIZE: usize = SIZE;

    fn alloc_pages(
        &mut self,
        num_pages: usize,
        align_pow2: usize,
    ) -> allocator::AllocResult<usize> {
        let align = 1 << align_pow2;
        let alloc_end = (self.pages_pos - num_pages * Self::PAGE_SIZE) & !(align - 1);
        if alloc_end < self.bytes_pos {
            return Err(allocator::AllocError::NoMemory);
        }
        self.pages_count += num_pages;
        self.pages_pos = alloc_end;
        Ok(alloc_end)
    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        let size = num_pages * Self::PAGE_SIZE;
        let end = pos + size;

        if pos != self.pages_pos || end > self.end || pos < self.bytes_pos {
            return;
        }

        self.pages_pos = end;
        self.pages_count -= num_pages;
    }

    fn total_pages(&self) -> usize {
        return self.end - self.start / Self::PAGE_SIZE;
    }

    fn used_pages(&self) -> usize {
        return (self.end - self.pages_pos) / Self::PAGE_SIZE;
    }

    fn available_pages(&self) -> usize {
        return (self.pages_pos - self.bytes_pos) / Self::PAGE_SIZE;
    }
}
