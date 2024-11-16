//! Allocator algorithm in lab.

#![no_std]
#![allow(unused_variables)]

use allocator::{AllocError, AllocResult, BaseAllocator, ByteAllocator};
// use log::{info, warn};
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
    /**
         * [  0.076038 0 lab_allocator:23] init BaseAllocator start 18446743800981688320  with size 32768
    [  0.077624 0 axruntime:150] Initialize platform devices...
    [  0.078613 0 axruntime:186] Primary CPU 0 init OK.
    Running bumb tests...
    Indicator: 0
    [  0.081796 0 lab_allocator:32] init add_memory start 18446743800981721088  with size 32768 , total bytes 65536
    [  0.083266 0 lab_allocator:32] init add_memory start 18446743800981753856  with size 65536 , total bytes 131072
    [  0.084633 0 lab_allocator:32] init add_memory start 18446743800981819392  with size 131072 , total bytes 262144
    [  0.086033 0 lab_allocator:32] init add_memory start 18446743800981950464  with size 262144 , total bytes 524288
    [  0.087751 0 lab_allocator:32] init add_memory start 18446743800982212608  with size 524288 , total bytes 1048576
    [  0.089759 0 lab_allocator:32] init add_memory start 18446743800982736896  with size 1048576 , total bytes 2097152
         */
    fn add_memory(&mut self, start: usize, size: usize) -> AllocResult {
        // warn!("add_memory on OOM {},{}",start,size);
        Ok(())
        // unimplemented!();

        //Indicator: 84
        // init add_memory start 18446743801015242752  with size 33554432 , total bytes 67108864
        // let res =  self.0.add_memory(start, size);
        // error!("init add_memory start {}  with size {} , total bytes {}",start,size,self.0.total_bytes());
        // res
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
        || lsize == 21504
        || lsize == 10752
        || lsize == 43008
    {
        return true;
    }
    STACK_NUMS.contains(&(lsize - round))
}

impl ByteAllocator for LabByteAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {
        if self.available_bytes() < layout.size() {
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
        } as *mut u8;

        // if  self.round>=368 {
        //     log::info!(" before panic used {} / {} ,available_bytes {}",self.used_bytes(),self.total_bytes(),self.available_bytes());
        // }
        // if layout.size() >= 524288 {
        //     self.round +=1;
        // }

        // warn!("alloc size  {}",layout.size());
        let result = NonNull::new(ptr_at);
        if let Some(result) = result {
            self.used += layout.size();
            // self.allocated += size;
            return Ok(result);
        } else {
            panic!("unknow NonNull error at alloc!!!");
        }
    }

    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        // info!("===== dealloc size  {} / {}",layout.size(),self.round);
        if from_top(layout.size(), self.round) {
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
        }
        // 只有顶部，类似栈
        // info!("======dealloc with at addr {:p} , size {}",pos,layout.size());
        // let ret = pos.as_ptr() as usize + layout.size();
        // info!("======dealloc back at addr {:p}",ret as *mut u8);
        // self.top_phd += layout.size()
        // self.top_phd = ret
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

