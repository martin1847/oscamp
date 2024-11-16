#![no_std]

use core::mem::size_of;

use allocator::{BaseAllocator, ByteAllocator, PageAllocator};
use log::{error, warn};

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
pub struct EarlyAllocator<const PAGE_SIZE: usize>{
    bottom_phd: usize,
    top_phd: usize,
    bused:usize,
    pused:usize,
    total:usize
}

impl<const PAGE_SIZE: usize> EarlyAllocator<PAGE_SIZE>  {
    /// Creates a new empty `BitmapPageAllocator`.
    pub const fn new() -> Self {
        Self {
            bottom_phd: 0,
            top_phd: 0,
            bused: 0,
            pused: 0,total:0,
        }
    }
}

impl<const PAGE_SIZE: usize> BaseAllocator for EarlyAllocator<PAGE_SIZE> {
    fn init(&mut self, start: usize, size: usize) {
      // avoid unaligned access on some platforms
        let start = (start + size_of::<usize>() - 1) & (!size_of::<usize>() + 1);
        let mut end = start + size;
        end &= !size_of::<usize>() + 1;
        assert!(start <= end);
        error!("init  memory {}MB {} -> {}", size>>20 ,start,end);
        self.bottom_phd = start;
        self.top_phd = end; //as *mut usize
        self.total = end - start;
    }

    fn add_memory(&mut self, start: usize, size: usize) -> allocator::AllocResult {
        error!("add_memory not support!!!");
        unimplemented!()
    }
}

impl<const PAGE_SIZE: usize> ByteAllocator for EarlyAllocator<PAGE_SIZE> {
    fn alloc(&mut self, layout: core::alloc::Layout) -> allocator::AllocResult<core::ptr::NonNull<u8>> {
        
        if self.available_bytes() < layout.size() {
            return Err(allocator::AllocError::NoMemory);
        }

        

        let result = core::ptr::NonNull::new(self.bottom_phd as *mut u8);
        if let Some(result) = result {
            self.bused += layout.size();
            self.bottom_phd += layout.size();
            // self.allocated += size;
            return Ok(result);
        } else {
            panic!("unknow NonNull error at alloc!!!");
        }

    }

    fn dealloc(&mut self, pos: core::ptr::NonNull<u8>, layout: core::alloc::Layout) {
        // todo!()
        let ret = pos.as_ptr() as usize + layout.size();
        if ret == self.bottom_phd {
            warn!("dealloc .... {}",layout.size());
            self.bottom_phd -= layout.size();
            self.bused -= layout.size();
        }
    }

    fn total_bytes(&self) -> usize {
        self.total
    }

    fn used_bytes(&self) -> usize {
        self.bused 
    }

    fn available_bytes(&self) -> usize {
        self.total - self.bused - self.pused
    }
}

// const PAGE_SIZE: usize = 0x1000;


// const PAGE_SIZE: usize = 0x1000;

impl<const PAGE_SIZE: usize> PageAllocator for EarlyAllocator<PAGE_SIZE> {
    
    const PAGE_SIZE: usize = PAGE_SIZE;


    fn alloc_pages(&mut self, num_pages: usize, align_pow2: usize) -> allocator::AllocResult<usize> {
        // BitmapPageAllocator
        // todo!()
        if align_pow2 %  PAGE_SIZE != 0 {
            return Err(allocator::AllocError::InvalidParam);
        }
        let align_pow2 = align_pow2 / PAGE_SIZE;
        if !align_pow2.is_power_of_two() {
            return Err(allocator::AllocError::InvalidParam);
        }
        let _align_log2 = align_pow2.trailing_zeros() as usize;
        //不支持大页。
        warn!("alloc_pages {} ",num_pages);
        self.pused += num_pages * PAGE_SIZE;
        Ok(self.top_phd - num_pages * PAGE_SIZE)

    }

    fn dealloc_pages(&mut self, pos: usize, num_pages: usize) {
        self.pused -= num_pages * PAGE_SIZE;
        warn!("dealloc_pages {} ",num_pages);
        //不处理回收了
    }

    fn total_pages(&self) -> usize {
        self.total/PAGE_SIZE
    }

    fn used_pages(&self) -> usize {
        self.pused/PAGE_SIZE
    }

    fn available_pages(&self) -> usize {
        ( self.total -self.bused - self.pused)/PAGE_SIZE
    }
}
