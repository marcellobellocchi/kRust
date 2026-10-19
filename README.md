# kRust

A kernel written in Rust, built to need as little assembly and external code as possible.

## Project layout

The code is separated by **what it depends on**: hardware-specific code in one place, generic kernel logic in another, and pure algorithms in crates that can be tested on the host machine.

```
kRust/
├── Cargo.toml                  # workspace: lists kernel, libs/*, user/*
├── rust-toolchain.toml         # pinned nightly + components (rust-src, llvm-tools)
├── run.sh                      # builds an ISO from the latest binary, runs QEMU
│
├── kernel/
│   ├── Cargo.toml
│   ├── linker.ld
│   ├── build.rs                # passes linker.ld to the linker
│   └── src/
│       ├── main.rs             # kmain: init order of all subsystems
│       ├── panic.rs
│       ├── log.rs              # print!/println!, log levels
│       │
│       ├── arch/
│       │   ├── mod.rs          # picks the arch with #[cfg], re-exports its API
│       │   └── x86_64/
│       │       ├── boot/
│       │       │   ├── boot.s          # 32→64-bit stub
│       │       │   ├── tables.rs       # boot page tables, GDT, stack
│       │       │   └── multiboot2.rs   # parses GRUB info → generic BootInfo
│       │       ├── cpu.rs              # MSRs, control registers, CPUID
│       │       ├── port.rs             # inb/outb
│       │       ├── gdt.rs              # GDT + TSS
│       │       ├── idt.rs
│       │       ├── exceptions.rs
│       │       ├── pic.rs, apic.rs, ioapic.rs
│       │       ├── paging.rs           # x86 page table entry format
│       │       ├── context.rs          # context switch (naked fn)
│       │       ├── syscall.rs          # syscall entry stub
│       │       └── smp.rs              # starting other cores
│       │
│       ├── boot.rs             # BootInfo type: memory map, framebuffer, RSDP
│       ├── mm/                 # frame allocator, address spaces, heap, mmap
│       ├── sync/               # spinlocks (IRQ-safe), mutex, rwlock, wait queues
│       ├── task/               # processes, threads, scheduler
│       ├── syscall/            # syscall table and handlers
│       ├── time/               # clocks, timers, sleep
│       ├── ipc/                # pipes, signals, shared memory
│       ├── fs/                 # VFS + tmpfs, devfs, procfs, fat, ext2
│       ├── net/                # network stack glue
│       ├── loader/             # ELF loading
│       └── drivers/
│           ├── acpi/
│           ├── pci/
│           ├── serial.rs
│           ├── console/        # framebuffer text console
│           ├── input/          # PS/2, later USB HID
│           ├── usb/            # xHCI
│           ├── block/          # virtio-blk, AHCI, NVMe
│           └── net/            # virtio-net, e1000
│
├── libs/                       # no_std crates with no hardware access
│   ├── abi/                    # syscall numbers + shared structs (kernel AND userspace)
│   ├── elf/
│   ├── bitmap-alloc/
│   └── ...
│
└── user/                       # userspace programs
    ├── rt/                     # minimal runtime: _start, syscall wrappers
    ├── init/
    └── shell/
```

## Design rules

### 1. Only `arch/` touches the CPU directly
Code outside `arch/` calls functions such as `arch::interrupts::disable()` or `arch::paging::map(...)`, never `asm!` or x86 structures.

This keeps the boot method replaceable: switching to UEFI means adding `arch/x86_64/boot/uefi.rs` that produces the same `BootInfo`, then deleting `boot.s` and `multiboot2.rs`. Nothing else changes. It also keeps a port to another architecture (such as aarch64) possible.

### 2. Generic kernel logic sits at the top level of `src/`
Policy lives outside `arch/`, mechanism lives inside it:
- The scheduler in `task/` decides *which* thread runs; `arch/x86_64/context.rs` only knows *how* to switch registers.
- `mm/` decides *what* to map; `arch/x86_64/paging.rs` knows the bit layout of an x86 page table entry.

### 3. Pure algorithms go in `libs/`
An ELF parser, a bitmap allocator or an ext2 parser doesn't need a CPU. As separate `no_std` crates, they can be unit-tested with plain `cargo test` on the host, which is much faster than testing inside QEMU.

### 4. `libs/abi` is the contract with userspace
Syscall numbers and the structs passed through syscalls are defined once and used by both the kernel and `user/rt`, so the two sides can't drift apart.

### 5. Drivers are grouped by function
`drivers/block/` holds every disk driver behind one block-device interface, so the filesystem doesn't care whether it runs on NVMe or virtio.

## Conventions

- **Grow the tree as features arrive.** Directories are created when the first code for them is written, not in advance.
- **One module style:** either `mm/mod.rs` or `mm.rs` next to an `mm/` folder, used consistently across the project.
- **Assembly lives in `arch/`**, next to the Rust code that includes it (`include_str!("boot.s")`).

## Building

The kernel builds for bare metal, while `libs/` tests run on the host. For this reason `build.target` is **not** set globally in `.cargo/config.toml`; the target is passed when building the kernel:

```sh
cargo build -p kernel --target x86_64-unknown-none
```

Unit tests for the host-testable crates:

```sh
cargo test -p elf -p bitmap-alloc
```

## Running

`run.sh` takes the most recently built kernel binary (debug or release), builds a bootable ISO in `target/iso/`, and offers to launch it in QEMU. Since the workspace root has no `[package]`, the kernel's binary name is passed explicitly:

```sh
KERNEL_NAME=kernel ./run.sh        # build ISO, then ask whether to launch
KERNEL_NAME=kernel ./run.sh -l     # launch the most recent ISO
KERNEL_NAME=kernel ./run.sh -u     # boot with UEFI firmware (OVMF)
KERNEL_NAME=kernel ./run.sh -g     # wait for GDB on port 1234
```

Run `./run.sh -h` for all options.

**Requirements:** `grub-mkrescue` (GRUB, `xorriso`, `mtools`) and `qemu-system-x86_64`. UEFI boot also needs GRUB's EFI files (`grub-efi-amd64-bin` on Debian/Ubuntu) and OVMF.