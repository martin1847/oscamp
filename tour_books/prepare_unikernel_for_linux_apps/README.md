### 前言

本实验指导是为Unikernel内核模式下支持Linux多应用所做的前期准备。包括一系列基本实验与附加练习：

1. 基本实验：本指导书以增量的方式基本给出了1到5共5个实验的源码及过程，大家照着做一遍，以熟悉基本原理机制。
2. 附加练习：基于基本实验，根据自己的理解，增加一些实现，以达到练习要求目标。总共6个练习。



### 环境准备

注意：以下实验都是基于**ArceOS的主线仓库**，**不是**基于oscamp那个简化的版本仓库。

1. Fork ArceOS的工程，clone到本地。工程链接如下

   ```sh
   git@github.com:arceos-org/arceos.git
   ```

   通过`git log`查看commit id是否为*51a42ea4d65a53bf6b43fc35a27a3ff1e9e284c7*。如果不是，回退到这个commit，确保工作的基线与指导书一致。

2. 建立并切换到分支week2_base

   ```sh
   cd arceos
   git checkout main
   git checkout -b week2_base
   ```

   这个分支对应**基本实验**。开始实验时，每完成一个，就commit 一次，commit msg是"step N"，N是实验序号。

3. 建立并切换到分支week2_exercise

   ```rust
   git checkout main
   git checkout -b week2_exercise
   ```

   这个分支对应**附加练习**。根据每个附加练习的要求完成，每完成一个commit一次，commit msg是"exercise N"，N是练习序号。

4. 执行`make run ARCH=riscv64`测试一下环境，我们的实习平台是**riscv64-qemu-virt**。

   ```sh
          d8888                            .d88888b.   .d8888b.
         d88888                           d88P" "Y88b d88P  Y88b
        d88P888                           888     888 Y88b.
       d88P 888 888d888  .d8888b  .d88b.  888     888  "Y888b.
      d88P  888 888P"   d88P"    d8P  Y8b 888     888     "Y88b.
     d88P   888 888     888      88888888 888     888       "888
    d8888888888 888     Y88b.    Y8b.     Y88b. .d88P Y88b  d88P
   d88P     888 888      "Y8888P  "Y8888   "Y88888P"   "Y8888P"
   
   arch = riscv64
   platform = riscv64-qemu-virt
   target = riscv64gc-unknown-none-elf
   smp = 1
   build_mode = release
   log_level = warn
   
   Hello, world!
   ```

   看到这个输出表示环境正常。

   

### 实验1：从外部加载应用

实现加载器loader，从外部加载bin应用到ArceOS地址空间。

<img src=".\pictures\p1.png" alt="p1" style="zoom:48%;" />

1. 编写一个no_std应用作为实验对象，命名为hello_app，目录与本地的arceos目录**并列**。它的主文件main.rs如下

   ```rust
   #![no_std]
   #![no_main]
   
   use core::panic::PanicInfo;
   
   #[no_mangle]
   unsafe extern "C" fn _start() -> ! {
       core::arch::asm!(
           "wfi",
           options(noreturn)
       )
   }
   
   #[panic_handler]
   fn panic(_info: &PanicInfo) -> ! {
       loop {}
   }
   ```

   现在只有一行代码`wfi`。

2. 在hello_app根目录下加一个rust-toolchain.toml

   ```rust
   [toolchain]
   profile = "minimal"
   channel = "nightly"
   components = ["rust-src", "llvm-tools-preview", "rustfmt", "clippy"]
   targets = ["riscv64gc-unknown-none-elf"]
   ```

   定制默认的toolchain，关键是指定target = "riscv64gc-unknown-none-elf"。即riscv64体系结构的裸机程序。

3. 执行一系列命名，包括编译，转换和打包，生成可被ArceOS加载的image。

   ```sh
   cargo build --target riscv64gc-unknown-none-elf --release
   
   rust-objcopy --binary-architecture=riscv64 --strip-all -O binary target/riscv64gc-unknown-none-elf/release/hello_app ./hello_app.bin
   
   dd if=/dev/zero of=./apps.bin bs=1M count=32
   dd if=./hello_app.bin of=./apps.bin conv=notrunc
   
   mkdir -p ../arceos/payload
   mv ./apps.bin ../arceos/payload/apps.bin
   ```

   得到image文件apps.bin，上面最后两步把它转移到arceos/payload目录下，以方便启动。

   > 这步的一系列动作可以考虑写入一个shell脚本，便于今后执行。

4. 转移到ArceOS工程，在apps目录下，实现一个新的app，名为loader。仿照helloworld应用创建，它的main.rs如下

   ```rust
   #![cfg_attr(feature = "axstd", no_std)]
   #![cfg_attr(feature = "axstd", no_main)]
   
   #[cfg(feature = "axstd")]
   use axstd::println;
   
   const PLASH_START: usize = 0x22000000;
   
   #[cfg_attr(feature = "axstd", no_mangle)]
   fn main() {
       let apps_start = PLASH_START as *const u8;
       let apps_size = 32; // Dangerous!!! We need to get accurate size of apps.
   
       println!("Load payload ...");
   
       let code = unsafe { core::slice::from_raw_parts(apps_start, apps_size) };
       println!("content: {:?}: ", code);
   
       println!("Load payload ok!");
   }
   ```

   注意：qemu有两个pflash，其中第一个被保留做扩展的bios，我们只能用第二个，它的开始地址0x22000000。

5. ArceOS目前没有对pflash所在的地址空间进行映射，增加映射。

   在文件modules/axhal/src/platform/riscv64_qemu_virt/boot.rs中，恒等映射从0开始的1G空间。

   ```rust
   unsafe fn init_boot_page_table() {
       // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
       BOOT_PT_SV39[2] = (0x80000 << 10) | 0xef;
       // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
       BOOT_PT_SV39[0x102] = (0x80000 << 10) | 0xef;
   
       // 0x0000_0000..0x4000_0000, VRWX_GAD, 1G block
       BOOT_PT_SV39[0] = (0x00000 << 10) | 0xef;
   }
   ```

   注意，只有最后两行是我们新增的映射。这样ArceOS就可以访问pflash所在的地址空间。

6. 现在可以编译ArceOS了，修改一下Makefile的默认参数。看一下修改前后diff的结果

   ```makefile
    # General options
   -ARCH ?= x86_64
   +ARCH ?= riscv64
   
    # App options
   -A ?= apps/helloworld
   +A ?= apps/loader
   ```

   默认arch改为riscv64，默认应用改为apps/loader即我们的加载器。

7. 修改一下qemu的启动参数，让pflash加载之前的image就是那个apps.bin，然后启动ArceOS内核及loader应用。

   修改scripts/make/qemu.mk，在qemu启动参数中追加

   ``` makefile
   -drive if=pflash,file=$(CURDIR)/payload/apps.bin,format=raw,unit=1
   ```

8. 把apps/loader加到根目录Cargo.toml中的[workspace]下的members列表中。执行`make run`测试

   ```sh
   arch = riscv64
   platform = riscv64-qemu-virt
   target = riscv64gc-unknown-none-elf
   smp = 1
   build_mode = release
   log_level = warn
   
   Load payload ...
   content: [115, 0, 80, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]:
   Load payload ok!
   ```

   注意前6个字节，转为十六级制就是测试应用hello_app.bin的内容，可以用如下命令确认

   ```rust
   xxd -l ./hello_app.bin
   ```
   
   对比后可发现内容一致。应用打印的是十进制，xxd显示的十六进展，注意进制和大小端问题。



#### 练习1：

main函数中，固定设置app_size = 32，这个显然是不合理甚至危险的。

请为image设计一个头结构，包含应用的长度信息，loader在加载应用时获取它的实际大小。

执行通过。

#### 练习2：

在练习1的基础上，扩展image头结构，让image可以包含两个应用。

第二个应用包含唯一的汇编代码是`ebreak`。

如实验1的方式，让loader顺序打印出每一个应用的二进制代码。



### 实验2：把应用拷贝到执行区域并执行

目前应用已经被加载到pflash的地址区间内，但是处于只读状态，一旦执行到写数据的指令时，就会触发异常。

所以本实验就是把应用搬运到可读可写可执行的内存区域。

<img src=".\pictures\p2.png" style="zoom:50%;" />

1. 从pflash区域拷贝到0x8010_0000，即Kernel前面1M处作为应用的执行区，改造一下loader 的main.rs(这里只给出增量代码)

   ```rust
   #[cfg_attr(feature = "axstd", no_mangle)]
   fn main() {
       let load_start = PLASH_START as *const u8;
       let load_size = 32; // Dangerous!!! We need to get accurate size of apps.
   
       println!("Load payload ...");
   
       let load_code = unsafe { core::slice::from_raw_parts(load_start, load_size) };
       println!("load code {:?}; address [{:?}]", load_code, load_code.as_ptr());
   
       // app running aspace
       // SBI(0x80000000) -> App <- Kernel(0x80200000)
       // 0xffff_ffc0_0000_0000
       const RUN_START: usize = 0xffff_ffc0_8010_0000;
   
       let run_code = unsafe {
           core::slice::from_raw_parts_mut(RUN_START as *mut u8, load_size)
       };
       run_code.copy_from_slice(load_code);
       println!("run code {:?}; address [{:?}]", run_code, run_code.as_ptr());
   
       println!("Load payload ok!");
   }
   ```

   `make run`显示如下，代码被正常拷贝到目标区域。

   ```sh
   arch = riscv64
   platform = riscv64-qemu-virt
   target = riscv64gc-unknown-none-elf
   smp = 1
   build_mode = release
   log_level = warn
   
   Load payload ...
   load code [115, 0, 80, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]; address [0x22000000]
   run code [115, 0, 80, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]; address [0xffffffc080100000]
   Load payload ok!
   ```

   

2. 然后从新的位置开始执行App的逻辑，在上面main函数的末尾追加代码执行。

   ```rust
       println!("Execute app ...");
   
       // execute app
       unsafe { core::arch::asm!("
           li      t2, {run_start}
           jalr    t2
           j       .",
           run_start = const RUN_START,
       )}
   ```

   `make run`显示"Execute app ..."之后卡住了，但这是正常的，注意汇编最后一句是无限循环。

   > 如果提示需要#![feature(asm_const)]之类的支持，按照提示处理。
   >
   > 另：qemu卡住后，退出到命令行的按键是，Ctrl+a后按x

3. 要想知道是否成功，需要通过qemu.log。ArceOS支持输出这种日志，为方便，直接改Makefile默认项。

   ```make
   -QEMU_LOG ?= n
   +QEMU_LOG ?= y
   ```

   再次`make run`，当前目录下产生qemu.log

   ```asm
   IN:
   Priv: 1; Virt: 0
   0xffffffc080100000:  10500073          wfi
   0xffffffc080100004:  0000              illegal
   ```

   可以看到，我们确实执行到了App的唯一一行代码`wfi`。

#### 练习3

批处理方式执行两个单行代码应用，第一个应用的单行代码是`noop`，第二个的是`wfi`.



### 实验3：通过ABI调用ArceOS功能

到目前为止，我们的外部应用hello_app还无法做实际的事情。原因就是，这个应用是独立于ArceOS之外编译的单独Image，现在ArceOS还没有为它提供调用接口。本实验中，我们先来做一个准备，为ArceOS增加简单的ABI接口支持，首先让内嵌应用Loader能够通过ABI方式调用功能；下个实验我们再进一步改成让外部应用通过ABI调用功能。

<img src=".\pictures\p3.png" alt="图片3" style="zoom:50%;" />

1. 在loader中引入abi_table，注册两个调用过程。一个是无参数的abi_hello，另一个是单参数的abi_putchar。在main.rs中增加

   ```rust
   const SYS_HELLO: usize = 1;
   const SYS_PUTCHAR: usize = 2;
   
   static mut ABI_TABLE: [usize; 16] = [0; 16];
   
   fn register_abi(num: usize, handle: usize) {
       unsafe { ABI_TABLE[num] = handle; }
   }
   
   fn abi_hello() {
       println!("[ABI:Hello] Hello, Apps!");
   }
   
   fn abi_putchar(c: char) {
       println!("[ABI:Print] {c}");
   }
   ```

2. 在ArceOS内嵌应用loader中，测试按照调用号调用ABI功能。我们可以分别测试一下两个功能。

   下面是在main()函数中调用的，改造原来的那几行汇编，变成下面这样

   ```rust
       register_abi(SYS_HELLO, abi_hello as usize);
       register_abi(SYS_PUTCHAR, abi_putchar as usize);
   
   	println!("Execute app ...");
       let arg0: u8 = b'A';
   
       // execute app
       unsafe { core::arch::asm!("
           li      t0, {abi_num}
           slli    t0, t0, 3
           la      t1, {abi_table}
           add     t1, t1, t0
           ld      t1, (t1)
           jalr    t1
           li      t2, {run_start}
           jalr    t2
           j       .",
           run_start = const RUN_START,
           abi_table = sym ABI_TABLE,
           //abi_num = const SYS_HELLO,
           abi_num = const SYS_PUTCHAR,
           in("a0") arg0,
       )}
   ```

   可以看到，在启动应用之前，我们在loader本地先测试了**SYS_PUTCHAR**的功能调用。如下是执行结果：

   ```sh
   arch = riscv64
   platform = riscv64-qemu-virt
   target = riscv64gc-unknown-none-elf
   smp = 1
   build_mode = release
   log_level = warn
   
   Load payload ...
   load code [[115, 0, 80, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]]; address [0x22000000]
   run code [[115, 0, 80, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]]; address [0xffffffc080100000]
   Load payload ok!
   
   Execute app ...
   [ABI:Print] A
   QEMU: Terminated
   ```

   看到打印出字符'A'，测试成功！

   打印后卡住了，还是用Ctrl+a后x退出。下面练习4就实验一下退出功能。


#### 练习4

支持3号调用 - SYS_TERMINATE功能调用，作用是让ArceOS退出，相当于OS关机。



### 实验4：正式在App中调用ABI

上个实验已经实现了ABI机制，本实验我们让外部应用正式使用ABI。这里需要解决一个问题，外部应用必须获得**ABI入口表的基地址**，才能以调用号为偏移，找到对应的功能。因为loader是ArceOS内嵌应用，它知道这个地址，我们让它把地址传过来。

<img src=".\pictures\p4.png" alt="图片4" style="zoom:50%;" />

1. 在loader的main函数中，把直接调用abi的代码删除，改为如下代码

   ```rust
       println!("Execute app ...");
   
       // execute app
       unsafe { core::arch::asm!("
           la      a7, {abi_table}
           li      t2, {run_start}
           jalr    t2
           j       .",
           run_start = const RUN_START,
           abi_table = sym ABI_TABLE,
       )}
   ```

   loader不再调用abi，只是把ABI_TABLE的地址传给外部应用hello_app。注意：我们传递地址用的是a7寄存器。

2. 应用hello_app通过ABI获取ArceOS服务，修改它的main.rs：

   ```rust
   #![feature(asm_const)]
   #![no_std]
   #![no_main]
   
   //const SYS_HELLO: usize = 1;
   const SYS_PUTCHAR: usize = 2;
   
   #[no_mangle]
   unsafe extern "C" fn _start() -> ! {
       let arg0: u8 = b'C';
       core::arch::asm!("
           li      t0, {abi_num}
           slli    t0, t0, 3
           add     t1, a7, t0
           ld      t1, (t1)
           jalr    t1
           wfi",
           abi_num = const SYS_PUTCHAR,
           in("a0") arg0,
           options(noreturn),
       )
   }
   
   use core::panic::PanicInfo;
   
   #[panic_handler]
   fn panic(_info: &PanicInfo) -> ! {
       loop {}
   }
   ```

   可以看到，我们从a7寄存器获得了ABI_TABLE的基地址，再结合调用号就可以获得对应功能的入口。

   注意：调用号乘以8才是偏移(64位系统的函数指针8个字节)。

3. **重新执行**hello_app的编译转换步骤，见实验1的第3步。

   之前如果已经把步骤写入shell脚本，这步就比较简单。

4. 执行`make run`，测试结果：

   ```sh
   Execute app ...
   [ABI:Print] C
   QEMU: Terminated
   ```

   打印字符'C'，成功！

#### 练习5

按照如下要求改造应用hello_app：

1. 把三个功能调用的汇编实现封装为函数，以普通函数方式调用。例如，SYS_PUTCHAR封装为`fn putchar(c: char)`。

2. 基于打印字符函数putchar实现一个高级函数`fn puts(s: &str)`，可以支持输出字符串。

3. 应用hello_app的执行顺序是，Hello功能，打印字符串功能和退出功能。

> 别忘了应用修改后，还要执行实验1的第3步完成编译转换和覆盖旧应用。如果当时封装了shell脚本，这步比较方便。



### 实验5：支持内核和应用分离的地址空间及切换

目前，ArceOS Unikernel是单地址空间。我们希望为每个外部应用建立独立的地址空间，当应用被调度时，切换到此应用的地址空间上。这样对每个应用，就可以采用固定的地址空间布局。现在从0x4000_0000地址开始的1G区域空闲，那我们就以它作为应用的地址空间。

<img src=".\pictures\p5.png" alt="图片5" style="zoom:50%;" />

1. 在应用loader中，为应用hello_app建立独立的页表(仅有一级)，并实现初始化和切换函数。main.rs最后追加如下：

   ```rust
   //
   // App aspace
   //
   
   #[link_section = ".data.app_page_table"]
   static mut APP_PT_SV39: [u64; 512] = [0; 512];
   
   unsafe fn init_app_page_table() {
       // 0x8000_0000..0xc000_0000, VRWX_GAD, 1G block
       APP_PT_SV39[2] = (0x80000 << 10) | 0xef;
       // 0xffff_ffc0_8000_0000..0xffff_ffc0_c000_0000, VRWX_GAD, 1G block
       APP_PT_SV39[0x102] = (0x80000 << 10) | 0xef;
   
       // 0x0000_0000..0x4000_0000, VRWX_GAD, 1G block
       APP_PT_SV39[0] = (0x00000 << 10) | 0xef;
   
       // For App aspace!
       // 0x4000_0000..0x8000_0000, VRWX_GAD, 1G block
       APP_PT_SV39[1] = (0x80000 << 10) | 0xef;
   }
   
   unsafe fn switch_app_aspace() {
       use riscv::register::satp;
       let page_table_root = APP_PT_SV39.as_ptr() as usize - axconfig::PHYS_VIRT_OFFSET;
       satp::set(satp::Mode::Sv39, 0, page_table_root >> 12);
       riscv::asm::sfence_vma_all();
   }
   ```

   APP_PT_SV39的链接位置".data.app_page_table"，定义在modules/axhal/linker.lds.S中：

   ```rust
            _sdata = .;
            *(.data.boot_page_table)
            . = ALIGN(4K);
            *(.data.app_page_table)
            . = ALIGN(4K);
            *(.data .data.*)
   ```

   就紧跟在系统页表位置*(.data.boot_page_table)的下面。**注意**，咱们增加的只有中间两行。

   此外，代码中引用了两个外部的crate，是axconfig和riscv，修改loader的Cargo.toml

   ```rust
   [dependencies]
   axstd = { path = "../../ulib/axstd", optional = true }
   axconfig = { path = "../../modules/axconfig" }
   
   [target.'cfg(any(target_arch = "riscv32", target_arch = "riscv64"))'.dependencies]
   riscv = "0.10"
   ```

2. 虽然已经建立了应用的页表，但我们先不切换，直接去访问应用的地址空间0x4010_0000，去看看这将会导致什么样的状况。

   在loader的main函数中修改如下：

   ```diff
       - const RUN_START: usize = 0xffff_ffc0_8010_0000;
       + const RUN_START: usize = 0x4010_0000;
   ```

   > 注意：这个0x4010_0000所在的1G空间在原始的内核地址空间中是不存在的

   执行`make run`，系统异常**STORE_FAULT**，因为没有启用应用的地址空间映射。

   ```rust
   Unhandled trap Exception(StorePageFault) @ 0xffffffc080202aa8:
   TrapFrame {
       regs: GeneralRegisters {
           ra: 0xffffffc0802005f4,
           sp: 0xffffffc080247d30,
           gp: 0x0,
           tp: 0x0,
           t0: 0x20,
           t1: 0xffffffc080202b38,
           t2: 0x40100000,
           s0: 0xffffffc0802001aa,
           s1: 0xffffffc080200488,
           a0: 0x40100000,
           a1: 0x22000000,
           a2: 0x28e428904300513,
           a3: 0x40100020,
           a4: 0x2,
           a5: 0xffffffc0802018be,
           a6: 0x20,
           a7: 0x22000000,
           s2: 0x1,
           s3: 0xffffffc080247db0,
           s4: 0xffffffc080247d48,
           s5: 0x3,
           s6: 0xffffffc080247d58,
           s7: 0x2,
           s8: 0x40100000,
           s9: 0x20,
           s10: 0x0,
           s11: 0x0,
           t3: 0x10,
           t4: 0xffffffc080203fe0,
           t5: 0x27,
           t6: 0x1,
       },
       sepc: 0xffffffc080202aa8,
       sstatus: 0x8000000200006100,
   }
   ```

3. 现在正式切换地址空间。在拷贝Image到0x4010_0000的地址之前，切换到应用的地址空间。

   即在`const RUN_START: usize = 0x4010_0000;`代码行之前，先调用下面的两行：

   ```rust
       // switch aspace from kernel to app
       unsafe { init_app_page_table(); }
       unsafe { switch_app_aspace(); }
   ```

4. 执行`make run`

   ```rust
   Load payload ...
   load code [19, 5, 48, 4, 137, 66, 142, 2, 51, 131, 88, 0, 3, 51, 3, 0, 2, 147, 115, 0, 80, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]; address [0x22000000]
   run code [19, 5, 48, 4, 137, 66, 142, 2, 51, 131, 88, 0, 3, 51, 3, 0, 2, 147, 115, 0, 80, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]; address [0x40100000]
   Load payload ok!
   Execute app ...
   [ABI:Print] C
   ```

   又能看到打印字符了，切换地址空间成功！


#### 练习6

1. 仿照hello_app再实现一个应用，唯一功能是打印字符'D'。

2. 现在有两个应用，让它们分别有自己的地址空间。
3. 让loader顺序加载、执行这两个应用。这里有个问题，第一个应用打印后，不能进行无限循环之类的阻塞，想办法让控制权回到loader，再由loader执行下一个应用。
