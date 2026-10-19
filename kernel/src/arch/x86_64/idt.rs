//! Interrupt descriptor table: tells cpu which handler to run for each of the
//! 256 interrupt vectors. Vectors 0-32 are CPU exceptions, 32-256 will be
//! software and hardware interrupts.

use core::arch::asm;
use core::mem::size_of;

use super::exceptions as ex;
use super::gdt;

/// One 16-byte IDT entry ("gate"). Handler address split in 3
#[derive(Clone, Copy)]
#[repr(C)]
struct IdtEntry {
    offset_low: u16,
    selector: u16,
    /// Bits 0-3: IST slot (0 = keep current stack)
    ist: u8,
    /// Present bit, privilege level, gate type
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    _reserved: u32,
}

const _:() = assert!(size_of::<IdtEntry>() == 16);

const PRESENT: u8 = 1 << 7;

/// Interrupt gate: interrupts are disabled while handler runs
/// (trap gate, 0xF, would leave them enabled)
const INTERRUPT_GATE: u8 = 0xE;

const MISSING: IdtEntry = IdtEntry {
    offset_low: 0,
    selector: 0,
    ist: 0,
    type_attr: 0,
    offset_mid: 0,
    offset_high: 0,
    _reserved: 0,
};

impl IdtEntry {
    fn new(handler: *const (), ist: u8) -> Self {
        let address = handler as u64;
        IdtEntry {
            offset_low: address as u16,
            selector: gdt::KERNEL_CODE,
            ist,
            type_attr: PRESENT | INTERRUPT_GATE, // privilege level 0
            offset_mid: (address >> 16) as u16,
            offset_high: (address >> 32) as u32,
            _reserved: 0,
        }
    }
}

static mut IDT: [IdtEntry; 256] = [MISSING; 256];

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u64,
}

/// Fills in exception handler and loads IDT
/// call ONCE after gdt::init
pub unsafe fn init() {
    let no_ist = 0;
    let handlers: [(usize, *const (), u8); 18] = [
        (0, ex::divide_error as *const (), no_ist),
        (1, ex::debug as *const (), no_ist),
        (2, ex::non_maskable as *const (), no_ist),
        (3, ex::breakpoint as *const (), no_ist),
        (4, ex::overflow as *const (), no_ist),
        (5, ex::bound_range as *const (), no_ist),
        (6, ex::invalid_opcode as *const (), no_ist),
        (7, ex::device_not_available as *const (), no_ist),
        (8, ex::double_fault as *const (), gdt::DOUBLE_FAULT_IST),
        (10, ex::invalid_tss as *const (), no_ist),
        (11, ex::segment_not_present as *const (), no_ist),
        (12, ex::stack_segment as *const (), no_ist),
        (13, ex::general_protection as *const (), no_ist),
        (14, ex::page_fault as *const (), no_ist),
        (16, ex::x87_floating_point as *const (), no_ist),
        (17, ex::alignment_check as *const (), no_ist),
        (18, ex::machine_check as *const (), no_ist),
        (19, ex::simd_floating_point as *const (), no_ist),
    ];

    unsafe {
        for (vector, handler, ist) in handlers {
            IDT[vector] = IdtEntry::new(handler, ist);
        }

        let pointer = DescriptorTablePointer {
            limit: (size_of::<[IdtEntry; 256]>() - 1) as u16,
            base: (&raw const IDT) as u64,
        };
        asm!("lidt [{}]", in(reg) &pointer, options(readonly, nostack, preserves_flags));
    }
}