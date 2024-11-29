#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]
#![feature(asm_const)]
#![feature(riscv_ext_intrinsics)]

extern crate alloc;
#[cfg(feature = "axstd")]
extern crate axstd as std;
#[macro_use]
extern crate axlog;

mod csrs;
mod loader;
mod regs;
mod sbi;
mod task;
mod vcpu;

use crate::regs::GprIndex::{A0, A1};
use axhal::mem::{MemoryAddr, PhysAddr, VirtAddr, PAGE_SIZE_4K};
use axhal::paging::MappingFlags;
use axmm::AddrSpace;
use csrs::defs::hstatus;
use csrs::{RiscvCsrTrait, CSR};
use loader::load_vm_image;
use riscv::register::{scause, sstatus, stval};
use sbi::SbiMessage;
use tock_registers::LocalRegisterCopy;
use vcpu::VmCpuRegisters;
use vcpu::_run_guest;

const VM_ENTRY: usize = 0x8020_0000;

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    ax_println!("Hypervisor ...");

    // A new address space for vm.
    let mut uspace = axmm::new_user_aspace().unwrap();

    // Load vm binary file into address space.
    if let Err(e) = load_vm_image("/sbin/skernel2", &mut uspace) {
        panic!("Cannot load app! {:?}", e);
    }

    // Setup context to prepare to enter guest mode.
    let mut ctx = VmCpuRegisters::default();
    prepare_guest_context(&mut ctx);

    // Setup pagetable for 2nd address mapping.
    let ept_root = uspace.page_table_root();
    prepare_vm_pgtable(ept_root);

    // Kick off vm and wait for it to exit.
    while !run_guest(&mut ctx, &mut uspace) {}

    panic!("Hypervisor ok!");
}

fn prepare_vm_pgtable(ept_root: PhysAddr) {
    let hgatp = 8usize << 60 | usize::from(ept_root) >> 12;
    unsafe {
        core::arch::asm!(
            "csrw hgatp, {hgatp}",
            hgatp = in(reg) hgatp,
        );
        core::arch::riscv64::hfence_gvma_all();
    }
}

fn run_guest(ctx: &mut VmCpuRegisters, uspace: &mut AddrSpace) -> bool {
    unsafe {
        _run_guest(ctx);
    }

    vmexit_handler(ctx, uspace)
}

#[allow(unreachable_code)]
fn vmexit_handler(ctx: &mut VmCpuRegisters, uspace: &mut AddrSpace) -> bool {
    use scause::{Exception, Trap};

    let scause = scause::read();
    match scause.cause() {
        Trap::Exception(Exception::VirtualSupervisorEnvCall) => {
            let sbi_msg = SbiMessage::from_regs(ctx.guest_regs.gprs.a_regs()).ok();
            ax_println!("VmExit Reason: VSuperEcall: {:?}", sbi_msg);
            if let Some(msg) = sbi_msg {
                match msg {
                    SbiMessage::Reset(_) => {
                        let a0 = ctx.guest_regs.gprs.reg(A0);
                        let a1 = ctx.guest_regs.gprs.reg(A1);
                        ax_println!("a0 = {:#x}, a1 = {:#x}", a0, a1);
                        assert_eq!(a0, 0x6688);
                        assert_eq!(a1, 0x1234);
                        ax_println!("Shutdown vm normally!");
                        return true;
                    }
                    _ => todo!(),
                }
            } else {
                panic!("bad sbi message! ");
            }
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            // Bad instruction: 0xf14025f3 sepc: 0x80200000
            warn!(
                "Bad instruction: {:#x} sepc: {:#x}",
                stval::read(),
                ctx.guest_regs.sepc
            );
            //LoadGuestPageFault: stval0x40 sepc: 0x80200004
            // docker run --rm --mount type=bind,source="$(pwd)",dst=/ros  -w /ros myrisc/alpine:3 objdump -D target/riscv64gc-unknown-none-elf/release/skernel2
            /*
                         * target/riscv64gc-unknown-none-elf/release/skernel2:     file format elf64-littleriscv


            Disassembly of section .text:

            ffffffc080200000 <_start>:
            ffffffc080200000:	f14025f3          	csrr	a1,mhartid
            ffffffc080200004:	04003503          	ld	a0,64(zero) # 40 <_percpu_load_end+0x40>
            ffffffc080200008:	48a1                	li	a7,8
            ffffffc08020000a:	00000073          	ecall

                         */
            // 跳过越权出错的一行 csrr
            // mhartid 是一个 机器模式（M-mode） 下的寄存器，只有在 M-mode 下才能访问。
            // 程序计数器 sepc: 0x80200000
            ctx.guest_regs.sepc += 4;
            ctx.guest_regs.gprs.set_reg(A1, 0x1234);

            info!("ignore csrr, just next  程序计数器 sepc += 4 and set a1=0x1234");
            return false;
        }
        Trap::Exception(Exception::LoadGuestPageFault) => {
            // LoadGuestPageFault: stval0x40 sepc: 0x80200004
            // ld	a0,64(zero) # 40 <_percpu_load_end+0x40>
            let user_fault_vaddr: VirtAddr = stval::read().into();

            warn!(
                "LoadGuestPageFault: stval{:?} sepc: {:#x}",
                user_fault_vaddr, ctx.guest_regs.sepc
            );

            // let (paddr, _, _) = uspace
            //     .page_table()
            //     .query(stval::read().into())
            //     .unwrap_or_else(|_| panic!("Mapping failed for segment: {:#x}", stval::read()));

            uspace.map_alloc(
                user_fault_vaddr.align_down_4k(),
                PAGE_SIZE_4K,
                MappingFlags::READ | MappingFlags::WRITE | MappingFlags::USER,
                true,
            );

            let magic: usize = 0x6688;
            // let mut buf = [0u8; 8];
            let buf = unsafe {
                alloc::slice::from_raw_parts(
                    &magic as *const _ as *const u8,
                    core::mem::size_of::<usize>(),
                )
            };
            info!(
                "mmap addr at {:?},  a0=0x6688 with buf  {:?}",
                user_fault_vaddr, buf
            );
            uspace.write(user_fault_vaddr, buf);

            // ctx.guest_regs.sepc += 4;
            // ctx.guest_regs.gprs.set_reg(A0, 0x6688);
            // info!("TODO ! ignore mmap addr 64, just next  and set a0=0x6688");
            return false;
        }
        _ => {
            panic!(
                "Unhandled trap: {:?}, sepc: {:#x}, stval: {:#x}",
                scause.cause(),
                ctx.guest_regs.sepc,
                stval::read()
            );
        }
    }
    false
}

fn prepare_guest_context(ctx: &mut VmCpuRegisters) {
    // Set hstatus
    let mut hstatus =
        LocalRegisterCopy::<usize, hstatus::Register>::new(riscv::register::hstatus::read().bits());
    // Set Guest bit in order to return to guest mode.
    hstatus.modify(hstatus::spv::Guest);
    // Set SPVP bit in order to accessing VS-mode memory from HS-mode.
    hstatus.modify(hstatus::spvp::Supervisor);
    CSR.hstatus.write_value(hstatus.get());
    ctx.guest_regs.hstatus = hstatus.get();

    // Set sstatus in guest mode.
    let mut sstatus = sstatus::read();
    sstatus.set_spp(sstatus::SPP::Supervisor);
    ctx.guest_regs.sstatus = sstatus.bits();
    // Return to entry to start vm.
    ctx.guest_regs.sepc = VM_ENTRY;
}
