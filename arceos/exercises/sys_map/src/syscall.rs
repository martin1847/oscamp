#![allow(dead_code)]

use arceos_posix_api::{self as api, get_file_like};
use axerrno::LinuxError;
use axhal::arch::TrapFrame;
use axhal::mem::{phys_to_virt, virt_to_phys};
use axhal::paging::MappingFlags;
use axhal::trap::{register_trap_handler, SYSCALL};
use axtask::current;
use axtask::TaskExtRef;
use core::ffi::{c_char, c_int, c_void};
use memory_addr::{MemoryAddr, VirtAddr, PAGE_SIZE_4K};

const SYS_IOCTL: usize = 29;
const SYS_OPENAT: usize = 56;
const SYS_CLOSE: usize = 57;
const SYS_READ: usize = 63;
const SYS_WRITE: usize = 64;
const SYS_WRITEV: usize = 66;
const SYS_EXIT: usize = 93;
const SYS_EXIT_GROUP: usize = 94;
const SYS_SET_TID_ADDRESS: usize = 96;
const SYS_MMAP: usize = 222;

const AT_FDCWD: i32 = -100;

/// Macro to generate syscall body
///
/// It will receive a function which return Result<_, LinuxError> and convert it to
/// the type which is specified by the caller.
#[macro_export]
macro_rules! syscall_body {
    ($fn: ident, $($stmt: tt)*) => {{
        #[allow(clippy::redundant_closure_call)]
        let res = (|| -> axerrno::LinuxResult<_> { $($stmt)* })();
        match res {
            Ok(_) | Err(axerrno::LinuxError::EAGAIN) => debug!(concat!(stringify!($fn), " => {:?}"),  res),
            Err(_) => info!(concat!(stringify!($fn), " => {:?}"), res),
        }
        match res {
            Ok(v) => v as _,
            Err(e) => {
                -e.code() as _
            }
        }
    }};
}

bitflags::bitflags! {
    #[derive(Debug)]
    /// permissions for sys_mmap
    ///
    /// See <https://github.com/bminor/glibc/blob/master/bits/mman.h>
    struct MmapProt: i32 {
        /// Page can be read.
        const PROT_READ = 1 << 0;
        /// Page can be written.
        const PROT_WRITE = 1 << 1;
        /// Page can be executed.
        const PROT_EXEC = 1 << 2;
    }
}

impl From<MmapProt> for MappingFlags {
    fn from(value: MmapProt) -> Self {
        let mut flags = MappingFlags::USER;
        if value.contains(MmapProt::PROT_READ) {
            flags |= MappingFlags::READ;
        }
        if value.contains(MmapProt::PROT_WRITE) {
            flags |= MappingFlags::WRITE;
        }
        if value.contains(MmapProt::PROT_EXEC) {
            flags |= MappingFlags::EXECUTE;
        }
        flags
    }
}

bitflags::bitflags! {
    #[derive(Debug)]
    /// flags for sys_mmap
    ///
    /// See <https://github.com/bminor/glibc/blob/master/bits/mman.h>
    struct MmapFlags: i32 {
        /// Share changes
        const MAP_SHARED = 1 << 0;
        /// Changes private; copy pages on write.
        const MAP_PRIVATE = 1 << 1;
        /// Map address must be exactly as requested, no matter whether it is available.
        const MAP_FIXED = 1 << 4;
        /// Don't use a file.
        const MAP_ANONYMOUS = 1 << 5;
        /// Don't check for reservations.
        const MAP_NORESERVE = 1 << 14;
        /// Allocation is for a stack.
        const MAP_STACK = 0x20000;
    }
}

#[register_trap_handler(SYSCALL)]
fn handle_syscall(tf: &TrapFrame, syscall_num: usize) -> isize {
    ax_println!("handle_syscall [{}] ...", syscall_num);
    let ret = match syscall_num {
        SYS_IOCTL => sys_ioctl(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _) as _,
        SYS_SET_TID_ADDRESS => sys_set_tid_address(tf.arg0() as _),
        SYS_OPENAT => sys_openat(
            tf.arg0() as _,
            tf.arg1() as _,
            tf.arg2() as _,
            tf.arg3() as _,
        ),
        SYS_CLOSE => sys_close(tf.arg0() as _),
        SYS_READ => sys_read(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _),
        SYS_WRITE => sys_write(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _),
        SYS_WRITEV => sys_writev(tf.arg0() as _, tf.arg1() as _, tf.arg2() as _),
        SYS_EXIT_GROUP => {
            ax_println!("[SYS_EXIT_GROUP]: system is exiting ..");
            axtask::exit(tf.arg0() as _)
        }
        SYS_EXIT => {
            ax_println!("[SYS_EXIT]: system is exiting ..");
            axtask::exit(tf.arg0() as _)
        }
        SYS_MMAP => sys_mmap(
            tf.arg0() as _,
            tf.arg1() as _,
            tf.arg2() as _,
            tf.arg3() as _,
            tf.arg4() as _,
            tf.arg5() as _,
        ),
        _ => {
            ax_println!("Unimplemented syscall: {}", syscall_num);
            -LinuxError::ENOSYS.code() as _
        }
    };
    ret
}

#[allow(unused_variables)]
fn sys_mmap(
    // NULL in c
    _addr: *mut usize,
    len: usize,
    port: i32,
    flags: i32,
    fd: i32,
    _offset: isize,
) -> isize {
    // unimplemented!("no sys_mmap!");
    if len == 0 {
        warn!("kernel: len  == 0 !");
        return -1;
    }
    if port & !0x7 != 0 {
        warn!("kernel: port mask must be 0 {}!", port);
        return -1;
    }
    if port & 0x7 == 0 {
        warn!("kernel: port not vaild , R = 0 : {}!", port);
        return -1;
    }
    // if start & (PAGE_SIZE - 1) != 0 {
    //     warn!("kernel: start not aligend!  {}!", start);
    //     return -1;
    // }
    let task = axtask::current();
    let mut uspace = task.task_ext().aspace.lock();
    // warn!("find_free_area AddrRange size:{}, {:?}", len, uspace);
    // cat /proc/sys/vm/mmap_min_addr
    // 4096 DEFAULT_MMAP_MIN_ADDR
    // https://elixir.bootlin.com/linux/v6.12.1/source/mm/Kconfig#L743

    // https://github.com/torvalds/linux/blob/master/mm/mmap.c#L726
    // let mmap_min_addr = 4096.into();
    let mmap_min_addr = (uspace.end() - uspace.size() / 3).align_up_4k();
    let size_align = memory_addr::align_up(len, PAGE_SIZE_4K);
    let addr_src = uspace
        .find_free_area(
            mmap_min_addr,
            size_align,
            memory_addr::AddrRange::new(uspace.base(), uspace.end()),
        )
        .unwrap();

    warn!(
        "uspace ip -> 0x{:x}  sp -> 0x{:x} , mmap_min_addr -> {:?}",
        task.task_ext().uctx.get_ip(),
        task.task_ext().uctx.get_sp(),
        mmap_min_addr
    );

    let addr_at = addr_src.align_up_4k();
    warn!(
        "addr_src  {:?} -> algin  {:?} uspace PA  {:?} -> {:?} ",
        addr_src,
        addr_at,
        virt_to_phys(uspace.base()),
        virt_to_phys(uspace.end())
    );
    // warn!("map_alloc  align {:?} -> {:?}", addr_src, addr_at);
    // len also need 4k align
    if uspace
        .map_alloc(
            addr_at,
            size_align,
            MappingFlags::from(MmapProt::from_bits_truncate(port)),
            true,
        )
        .is_err()
    {
        warn!("map_alloc error !!!! when sys_mmap!!! ");
        return 0;
    }

    if size_align > PAGE_SIZE_4K {
        let mut buf = alloc::vec![0; len];
        // for each page to write with offset.
        get_file_like(fd).and_then(|f| f.read(&mut buf));
        uspace.write(addr_at, &buf);
    } else {
        let (paddr, _, _) = uspace.page_table().query(addr_at).unwrap();
        let kernel_vaddr = phys_to_virt(paddr);
        warn!(
            "[ write single page directly ] user vaddr {:?} -> paddr : {:?} -> kernel vaddr {:?}",
            addr_at, paddr, kernel_vaddr
        );
        // single page, write directly
        // get_file_like(fd).a
        get_file_like(fd).and_then(|f| {
            f.read(unsafe { alloc::slice::from_raw_parts_mut(kernel_vaddr.as_mut_ptr(), len) })
        });
    }

    // let disk_addr: VirtAddr = 0xffffffc040006000.into();
    // let slice = unsafe { alloc::slice::from_raw_parts(disk_addr.as_ptr_of::<u16>(), 20) };

    // warn!("found disk.img first 20 bytes {:?}", slice);
    // for byte in slice {
    //     warn!("{:x}", byte);
    // }

    addr_at.as_usize() as isize
}

fn sys_openat(dfd: c_int, fname: *const c_char, flags: c_int, mode: api::ctypes::mode_t) -> isize {
    assert_eq!(dfd, AT_FDCWD);
    api::sys_open(fname, flags, mode) as isize
}

fn sys_close(fd: i32) -> isize {
    api::sys_close(fd) as isize
}

fn sys_read(fd: i32, buf: *mut c_void, count: usize) -> isize {
    api::sys_read(fd, buf, count)
}

fn sys_write(fd: i32, buf: *const c_void, count: usize) -> isize {
    api::sys_write(fd, buf, count)
}

fn sys_writev(fd: i32, iov: *const api::ctypes::iovec, iocnt: i32) -> isize {
    unsafe { api::sys_writev(fd, iov, iocnt) }
}

fn sys_set_tid_address(tid_ptd: *const i32) -> isize {
    let curr = current();
    curr.task_ext().set_clear_child_tid(tid_ptd as _);
    curr.id().as_u64() as isize
}

fn sys_ioctl(_fd: i32, _op: usize, _argp: *mut c_void) -> i32 {
    ax_println!("Ignore SYS_IOCTL");
    0
}
