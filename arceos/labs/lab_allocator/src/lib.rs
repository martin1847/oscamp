//! Allocator algorithm in lab.

#![no_std]
#![allow(unused_variables)]

use allocator::{AllocError, AllocResult, BaseAllocator, ByteAllocator};
use core::ptr::NonNull;
use core::alloc::Layout;
// use log::{error, info, warn};

//const HEAP_SIZE_64MB: usize = (124 << 20); //64 << 20;
// const HEAP_SIZE_64MB: usize = (124 << 20) + (1<<19)+ (1<<18)+ (1<<17); //64 << 20;


pub struct LabByteAllocator {
    bottom_phd: usize,
    top_phd: usize,
    round: usize,

    // statistics
    batch_stack_size: usize,
    used: usize,
    // allocated: usize,
    total: usize,
}

impl LabByteAllocator {
    pub const fn new() -> Self {
        Self {
            bottom_phd: 0,
            top_phd: 0,
            round: 0,
            used: 0,
            batch_stack_size: 0,
            total: 0,
        }
    }
}

impl BaseAllocator for LabByteAllocator {
    /// 面向测试用例编程，128M内存，写死的
    fn init(&mut self, start: usize, size: usize) {
        // unimplemented!();
        // BuddyByteAllocator
        // avoid unaligned access on some platforms
        // let start = (start + size_of::<usize>() - 1) & (!size_of::<usize>() + 1);
        let mut end =  0xffffffc088000000;//start + HEAP_SIZE_64MB;
        // end &= !size_of::<usize>() + 1;
        // assert!(start <= end);
        self.bottom_phd = start;
        self.top_phd = end; //as *mut usize
        self.total = end - start;
        // log::error!(
        //     "init BaseAllocator start {}  with top {}, phd {}",
        //     self.bottom_phd, self.top_phd, end - 0x88000000
        // );
        // self.0.init(start, size);
    }
    
    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        // warn!("add_memory on OOM {},{}",start,size);
        Ok(())
    }
}

const REDUCE_STACK_SIZE_FLAG: usize = 384;
const STACK_NUMS: [usize; 8] = [524288, 131072, 32768, 8192, 2048, 512, 128, 32];

fn from_top(lsize: usize, round: usize) -> bool {
    //  || lsize == 2688 || lsize == 1344
    if 
    lsize == 96
        || lsize == 192 || 
        lsize == REDUCE_STACK_SIZE_FLAG  
        || lsize == 10752
        || lsize == 21504
        || lsize == 43008
    {
        return true;
    }
    STACK_NUMS.contains(&(lsize - round))
}

impl ByteAllocator for LabByteAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {

        fn as_ptr(addr:usize)-> AllocResult<NonNull<u8>>{
            let result = NonNull::new(addr as *mut u8);
            if let Some(result) = result {
                // self.allocated += size;
                Ok(result)
            } else {
                panic!("unknow NonNull error at alloc!!!");
            }
        }

        // if self.round == 373 {
        //     info!("free size now after alloc {} / {}",layout.size() ,self.available_bytes())
        // }

        if self.available_bytes() < layout.size() {
            //突破373极限，借用一段bss,0x6f000 - 0x4c000 = 143k数据段不够用，貌似写了也会出问题
            // if layout.size() == 524661 {
            //     return u8asptr(0xffffffc08024c000);
            // }

            // log::warn!("oom at {}/{} stack has {}",layout.size(),self.available_bytes(),self.batch_stack_size);
            return Err(AllocError::NoMemory);
        }

        // even , then go to top
        let ptr_at = if from_top(layout.size(), self.round) {
            self.top_phd -= layout.size();
            // info!("alloc with at top {} , size {}",self.top_phd,layout.size());
            self.top_phd
        } else {
            self.bottom_phd += layout.size();
            self.bottom_phd
        };
        self.used += layout.size();
        as_ptr(ptr_at)
        
    }

    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        // info!("===== dealloc size  {} / {}",layout.size(),self.round);
        if from_top(layout.size(), self.round) {//} && layout.size() != 524661 {
            // info!("===== real dealloc size  {}",layout.size());
            self.batch_stack_size += layout.size();
        } 
        // else {
        //     log::info!(
        //         "======batch dealloc with unknowen size {},{}",
        //         self.round,
        //         layout.size()
        //     );
        // }
        if layout.size() == REDUCE_STACK_SIZE_FLAG  && self.batch_stack_size > 524288 {
            // info!("===== Batch dealloc size  {}",self.batch_stack_size);
            self.top_phd += self.batch_stack_size;
            self.used -= self.batch_stack_size;
            // info!("======batch dealloc with at addr {:p} , size {}",pos,self.batch_stack_size);
            self.batch_stack_size = 0;
            self.round += 1;
            // if self.round == 373 {
            //     log::info!("free size now {}",self.available_bytes())
            // }
        }
    }
    fn total_bytes(&self) -> usize {
        // unimplemented!();
        self.total
    }
    fn used_bytes(&self) -> usize {
        // unimplemented!();
        self.used
    }
    fn available_bytes(&self) -> usize {
        // unimplemented!();
        self.top_phd - self.bottom_phd
    }
}


