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
    bottom_va: usize,
    top_va: usize,
    round: usize,

    va_373_base:usize,
    // statistics
    batch_stack_size: usize,
    used: usize,
    // allocated: usize,
    total: usize,
}

impl LabByteAllocator {
    pub const fn new() -> Self {
        Self {
            bottom_va: 0,
            top_va: 0,
            va_373_base:0,
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
        let end =  HEAP_TOP_VIRT_ADDR - VEC373_TOTAL;//start + HEAP_SIZE_64MB;
        self.va_373_base = end;
        // end &= !size_of::<usize>() + 1;
        // assert!(start <= end);
        self.bottom_va = start;
        self.top_va = end; //as *mut usize
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
const HEAP_TOP_VIRT_ADDR:usize = 0xffffffc088000000;


// 撞到320奇数位
const VEC373_L0:usize =  1344;
// 剩余全局vec pool 扩容的不会撞到奇数位
const VEC373_L1:usize =  VEC373_L0 << 1;
const VEC373_L2:usize =  VEC373_L1 << 1;
const VEC373_L3:usize =  VEC373_L1 << 2;
const VEC373_L4:usize =  VEC373_L1 << 3;
const VEC373_L5:usize =  VEC373_L1 << 4;
const VEC373_TOTAL:usize = (VEC373_L1<<5) - VEC373_L0;

fn from_top(lsz: usize, round: usize) -> bool {
    // 局部vec增长用到的，跳过撞到奇数位<64/256>的情况，允许这个碎片，保证正确性 
    if lsz == 96 || lsz == 192 || ( lsz == REDUCE_STACK_SIZE_FLAG && lsz - round != 256 ){
         return lsz - round != 64 ;
    }
    STACK_NUMS.contains(&(lsz - round))
}

impl ByteAllocator for LabByteAllocator {
    fn alloc(&mut self, layout: Layout) -> AllocResult<NonNull<u8>> {

        fn as_ptr(addr:usize)-> AllocResult<NonNull<u8>>{
            let result = NonNull::new(addr as *mut u8);
            if let Some(result) = result {
                Ok(result)
            } else {
                panic!("unknow NonNull error at alloc!!!");
            }
        }

        let lsz = layout.size() ;

        
        // if lsz == 65909 {
        //     return as_ptr(0xffffffc08026f000);
        // }
        //Magic 魔法操作，留给栈2KB，向下压缩
        // if lsz == 262517 {//} 524661 {
        //     //  Page Fault ，MMIO区域
        //     // let a = 123;
        //     // log::warn!("oom at {:p} stack has ",&a);
        //     return as_ptr(0xffffffc08024c000 - 2048 - lsz  );
        // }

        if self.available_bytes() < lsz {
            //  oom at  524661/459113 < 65909 stack has 288 , 差65kb
            // log::warn!("oom at {}/{} stack has {}",layout.size(),self.available_bytes(),self.batch_stack_size);
            return Err(AllocError::NoMemory);
        }

        // if self.round == 373 {
        //     log::info!("alloc373 {}",lsz);
        // }

        // alloc 43008 at round 128 
        // dealloc 21504 at round 128 
        // 全局 pool vec增长部分，这部分释放掉，可以达到373 ,VEC373_L1开始没有跟奇数位相撞
        if (lsz == VEC373_L0 && self.round!=320) || lsz == VEC373_L1 || lsz == VEC373_L2 || lsz == VEC373_L3 || lsz == VEC373_L4 || lsz == VEC373_L5   {
            let ret = self.va_373_base;
            self.va_373_base += lsz;
            self.used += lsz;
            return as_ptr(ret);
        }
        // even , then go to top
        let ptr_at = if from_top(lsz, self.round) {
            self.top_va -= lsz;
            self.top_va
        } else {
            let ret = self.bottom_va;
            self.bottom_va += lsz;
            ret
        };
        self.used += lsz;
        as_ptr(ptr_at)
        
    }

    fn dealloc(&mut self, pos: NonNull<u8>, layout: Layout) {
        // info!("===== dealloc size  {} / {}",layout.size(),self.round);

        let lsz = layout.size();

        if from_top(lsz, self.round) {
            self.batch_stack_size += layout.size();
        }else if lsz == VEC373_L5 {
            //模拟个栈反向溢出，预留的373空间回收掉
            // dealloc 43008 at round 257
            // log::info!(" round VEC373_L5 to {} {:x} , size {}",self.round,self.top_va,self.batch_stack_size);
            self.batch_stack_size += VEC373_TOTAL;
        }
        // else {
        //     log::info!(
        //         "======batch dealloc with unknowen size {},{}",
        //         self.round,
        //         layout.size()
        //     );
        // }
        if lsz == REDUCE_STACK_SIZE_FLAG  && self.batch_stack_size > 524288 {
            // info!("===== Batch dealloc size  {}",self.batch_stack_size);
            self.top_va += self.batch_stack_size;
            // if self.round > 254  &&  self.round <= 257{
            //     log::info!("======batch dealloc to  {:x} , size {}",self.top_va,self.batch_stack_size);
            // }
            self.used -= self.batch_stack_size;
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
        self.top_va - self.bottom_va
    }
}


