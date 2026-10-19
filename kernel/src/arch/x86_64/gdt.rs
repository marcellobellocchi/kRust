//! Global descriptor table and Task State Segment.
//!
//! Replaces minimal boot GDT with the permanent one:
//! Kernel and user ring separation,
//! and a TSS providing a stack for double faults, and for ring 3 -> ring 0 switches

use core::arch::asm;
use core::mem::size_of;

//Selectors
pub const KERNEL_CODE: u16 = 0x08;
pub const KERNEL_DATA: u16 = 0x10;
pub const USER_DATA: u16 = 0x18 | 3; //privilege level 3
pub const USER_CODE: u16 = 0x20 | 3;
pub const TSS: u16 = 0x28;

//IST slot for double fault handler
pub const DOUBLE_FAULT_IST: u8 = 1;


//Segment descriptors
const PRESENT: u8 = 1 << 7;
const RING_3: u8 = 3 << 5;
const CODE_OR_DATA: u8 = 1 << 4;
const EXECUTABLE: u8 = 1 << 3;
const READ_WRITE: u8 = 1 << 1;
const ACCESSED: u8 = 1 << 0;

const GRANULARITY_4K: u8 = 1 << 3;
const SIZE_32: u8 = 1 << 2;
const LONG_MODE: u8 = 1 << 1;

const fn segment(access: u8, flags: u8) -> u64 {
    0xFFFF
        | (access as u64) << 40
        | 0xF << 48
        | (flags as u64) << 52
}

const KERNEL_CODE_SEGMENT: u64 =
    segment(PRESENT | CODE_OR_DATA | EXECUTABLE | READ_WRITE | ACCESSED, GRANULARITY_4K | LONG_MODE);
const KERNEL_DATA_SEGMENT: u64 =
    segment(PRESENT | CODE_OR_DATA | READ_WRITE | ACCESSED, GRANULARITY_4K | SIZE_32);
const USER_CODE_SEGMENT: u64 =
    segment(PRESENT | RING_3 | CODE_OR_DATA | EXECUTABLE | READ_WRITE | ACCESSED, GRANULARITY_4K | LONG_MODE);
const USER_DATA_SEGMENT: u64 =
    segment(PRESENT | RING_3 | CODE_OR_DATA | READ_WRITE | ACCESSED, GRANULARITY_4K | SIZE_32);

//Task state segment

#[repr(C, packed(4))]
pub struct TaskStateSegment {
    _reserved1: u32,

    /// Stacks loaded on privilege changes
    pub privilege_stacks: [u64; 3],
    _reserved2: u64,

    /// Interrupt Stack Table: stacks an IDT entry can force the CPU to switch to
    pub interrupt_stacks: [u64; 7],
    _reserved3: u64,
    _reserved4: u16,

    /// Offset of the I/O permission bitmap, pointing past the end means "none"
    pub iomap_base: u16,
}

const _:() = assert!(size_of::<TaskStateSegment>() == 104);

fn tss_descriptor(tss: *const TaskStateSegment) -> [u64; 2] {
    const TYPE_AVAILABLE_TSS: u8 = 0x9;
    let base = tss as u64;
    let limit = (size_of::<TaskStateSegment>() - 1) as u64;

    let low = (limit & 0xFFFF)
        | (base & 0xFF_FFFF) << 16
        | ((PRESENT | TYPE_AVAILABLE_TSS) as u64) << 40
        | ((limit >> 16) & 0xF) << 48
        | ((base >> 24) & 0xFF) << 56;
    let high = base >> 32;
    [low, high]
}

//storage

//cpu will write busy bit into the tss descriptor on ltr,
//tss insead will be updated on every switch to a user thread

const GDT_ENTRIES: usize = 7;
static mut GDT: [u64; GDT_ENTRIES] = [0; GDT_ENTRIES];

static mut TSS_DATA: TaskStateSegment = TaskStateSegment {
    _reserved1: 0,
    privilege_stacks: [0; 3],
    _reserved2: 0,
    interrupt_stacks: [0; 7],
    _reserved3: 0,
    _reserved4: 0,
    iomap_base: size_of::<TaskStateSegment>() as u16,
};

#[repr(C, align(16))]
struct Stack([u8; 16384]);

/// Used only by double fault handler, aka kernel stack overflow
/// Will manage to get a working stack instead of triple fault trainwreck
static mut DOUBLE_FAULT_STACK: Stack = Stack([0; 16384]);

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

//Initialization

/// Builds and loads kernel GDT and TSS, then reloads all segment registers
/// To only be called ONCE, on CPU bootstrap, before interrupts are enabled.
pub unsafe fn init() {
    unsafe{
        //stack grows down, IST entry is the top of the stack
        let df_stack_top = (&raw mut DOUBLE_FAULT_STACK) as u64 + size_of::<Stack>() as u64;
        TSS_DATA.interrupt_stacks[(DOUBLE_FAULT_IST - 1) as usize] = df_stack_top;

        let [tss_low, tss_high] = tss_descriptor(&raw const TSS_DATA);
        GDT = [
            0,
            KERNEL_CODE_SEGMENT,
            KERNEL_DATA_SEGMENT,
            USER_DATA_SEGMENT,
            USER_CODE_SEGMENT,
            tss_low,
            tss_high,
        ];

        let pointer = DescriptorTablePointer {
            limit: (size_of::<[u64; GDT_ENTRIES]>() - 1) as u16,
            base: (&raw const GDT) as u64,
        };
        asm!("lgdt [{}]", in(reg) &pointer, options(readonly, nostack, preserves_flags));

        asm!(
            "push {sel}",
            "lea {tmp}, [rip + 2f]",
            "push {tmp}",
            "retfq",
            "2:",
            sel = in(reg) KERNEL_CODE as u64,
            tmp = lateout(reg) _,
            options(preserves_flags),
        );

        asm!(
            "mov ds, {0:x}",
            "mov es, {0:x}",
            "mov ss, {0:x}",
            in(reg) KERNEL_DATA,
            options(nostack, preserves_flags),
        );

        asm!("ltr {0:x}", in(reg) TSS, options(nostack, preserves_flags));
    }
}