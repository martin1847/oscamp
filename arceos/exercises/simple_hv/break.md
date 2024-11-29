
## 正常运行

```s
lldb exercises/simple_hv/simple_hv_riscv64-qemu-virt.elf

(lldb) gdb-remote localhost:1234
(lldb) br set --name _restore_csrs

(lldb) n
Process 1 stopped

* thread #1, stop reason = instruction step over
    frame #0: 0xffffffc0802001d4 simple_hv_riscv64-qemu-virt.elf`_restore_csrs + 4
simple_hv_riscv64-qemu-virt.elf`_restore_csrs:
->  0xffffffc0802001d4 <+4>:  csrrw  t1, sstatus, t1
    0xffffffc0802001d8 <+8>:  sd     t1, 544(a0)
    0xffffffc0802001dc <+12>: csrr   t1, hstatus
    0xffffffc0802001e0 <+16>: sd     t1, 552(a0)
Target 0: (simple_hv_riscv64-qemu-virt.elf) stopped.
(lldb) register read t1
      t1 = 0x8000000200006120

(lldb) register read t1                        
      t1 = 0x8000000200006022
```

## 错误类型

[  0.617342 0:2 axruntime::lang_items:5] panicked at exercises/simple_hv/src/main.rs:178:13:
Unhandled trap: Exception(StorePageFault), sepc: 0xffffffc080200150, stval: 0x0
